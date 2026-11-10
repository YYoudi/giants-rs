//! Admission prefix of the acquired completion handler. Snapshot fields retain
//! their tested roles; names do not establish complete global lifetimes.
//! Required callbacks may mutate the snapshot and replace the current record.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    pub index: i32,
    pub completed: i32,
    pub required: i32,
    /// Required only when the actual current index is positive.
    pub previous_kind: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub initial_gate: u32,
    pub mode: u32,
    pub post_precheck_gates: [u32; 2],
    pub post_record_gates: [u32; 2],
    pub record: Option<Record>,
    pub first_visit: u32,
    pub first_request: [u32; 3],
}

pub trait Services {
    fn precheck(&mut self, state: &mut State) -> Result<(), String>;
    fn begin_previous_record(&mut self, state: &mut State, index: i32) -> Result<(), String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Inactive,
    Countdown,
    /// Requires the following record/message/progression owner; not completion.
    AdvanceRecord,
}

impl State {
    /// Execute the outer prefix. Completed marker/callback effects survive later
    /// errors. A failed enclosing turn must be quarantined against replay.
    pub fn advance(&mut self, services: &mut impl Services) -> Result<Decision, String> {
        if self.initial_gate != 0 {
            return Ok(Decision::Inactive);
        }
        if self.mode == 0 {
            services.precheck(self)?;
        }
        if self.post_precheck_gates.into_iter().any(|v| v != 0)
            || self.record.is_none()
            || self.post_record_gates.into_iter().any(|v| v != 0)
        {
            return Ok(Decision::Inactive);
        }
        if self.first_visit == 0 {
            self.first_visit = 1;
            if self.mode == 0 {
                self.first_request = [1, 1, 0];
                let previous = self.record.as_ref().unwrap().index.wrapping_sub(1);
                services.begin_previous_record(self, previous)?;
            }
        }
        // The first-record service can replace the global record. Earlier outer
        // gates are not retested; only the record is reloaded at this boundary.
        let record = self
            .record
            .as_ref()
            .ok_or("completion record retired during first-record callback")?;
        if record.completed < record.required {
            return Ok(Decision::Inactive);
        }
        if record.index <= 0 {
            return Ok(Decision::AdvanceRecord);
        }
        let previous = record
            .previous_kind
            .ok_or("completion requires the actual preceding record kind")?;
        Ok(if previous == 2 {
            Decision::Countdown
        } else {
            Decision::AdvanceRecord
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initial() -> State {
        State {
            initial_gate: 0,
            mode: 0,
            post_precheck_gates: [0; 2],
            post_record_gates: [0; 2],
            record: Some(Record {
                index: 2,
                completed: 3,
                required: 3,
                previous_kind: Some(2),
            }),
            first_visit: 0,
            first_request: [9; 3],
        }
    }

    #[derive(Default)]
    struct Provider {
        calls: Vec<i32>,
        fail: bool,
    }
    impl Services for Provider {
        fn precheck(&mut self, state: &mut State) -> Result<(), String> {
            self.calls.push(-1);
            // This earlier guard must not be retested after the callback.
            state.initial_gate = 7;
            Ok(())
        }
        fn begin_previous_record(&mut self, state: &mut State, index: i32) -> Result<(), String> {
            self.calls.push(index);
            state.record = Some(Record {
                index: 3,
                completed: 0,
                required: 1,
                previous_kind: Some(2),
            });
            if self.fail {
                Err("required record service failed".into())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn precheck_precedes_later_gates_and_keeps_its_effects() {
        let mut state = initial();
        state.post_precheck_gates[0] = 1;
        let mut p = Provider::default();
        assert_eq!(state.advance(&mut p).unwrap(), Decision::Inactive);
        assert_eq!(p.calls, [-1]);
        assert_eq!(state.initial_gate, 7);
        assert_eq!(state.first_visit, 0);
    }

    #[test]
    fn first_record_callback_reloads_record_without_retesting_outer_gates() {
        let mut state = initial();
        let mut p = Provider::default();
        assert_eq!(state.advance(&mut p).unwrap(), Decision::Inactive);
        assert_eq!(p.calls, [-1, 1]);
        assert_eq!(state.first_visit, 1);
        assert_eq!(state.first_request, [1, 1, 0]);
        assert_eq!(state.record.unwrap().index, 3);
    }

    #[test]
    fn failed_first_record_callback_retains_markers_and_replacement() {
        let mut state = initial();
        let mut p = Provider {
            fail: true,
            ..Default::default()
        };
        assert!(state.advance(&mut p).is_err());
        assert_eq!(state.first_visit, 1);
        assert_eq!(state.first_request, [1, 1, 0]);
        assert_eq!(state.record.unwrap().index, 3);
    }
}
