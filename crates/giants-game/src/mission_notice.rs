//! Local pickup notice request, separate from inventory and actor construction.
//! Callers supply real mission state and gates; no trigger or clock is invented.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct State {
    pub pending: u32,
    pub counter: u32,
}
#[derive(Debug, Clone, Copy)]
pub struct Gates {
    /// Three independent nonzero suppression words; broader modes unverified.
    pub suppress: [u32; 3],
    pub time: f32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lookup {
    pub key: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// Native message route, not an inferred priority or duration.
    pub route: u32,
    /// Original encoding and control bytes retained for the actual consumer.
    pub text: Vec<u8>,
}
impl State {
    /// Native load entry clears pending/time, not the existing counter. The
    /// mission loader may change both words before reporting success/failure.
    pub fn begin_load(&mut self, time: &mut f32) {
        self.pending = 0;
        *time = 0.;
    }
    /// Only after successful parsing; preserve the loader's final pending word.
    pub fn parsed(&mut self) {
        self.counter = 0;
    }
    /// Consume pending before suppression. Prefix validation precedes mutation
    /// for the bounded ASCII profile. A key is a request, not a displayed message.
    pub fn pickup(&mut self, prefix: &[u8], gates: Gates) -> Result<Option<Lookup>, &'static str> {
        if prefix.len() > 31 || !prefix.is_ascii() || prefix.contains(&0) {
            return Err("notice prefix outside bounded mission-name profile");
        }
        if self.pending == 0 {
            return Ok(None);
        }
        self.pending = 0;
        if gates.suppress.into_iter().any(|value| value != 0) || gates.time > 0. {
            return Ok(None);
        }
        self.counter = self.counter.wrapping_add(1);
        let mut key = prefix.to_vec();
        // The original formatter consumes this word through signed %d.
        key.extend_from_slice((self.counter as i32).to_string().as_bytes());
        Ok(Some(Lookup { key }))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    pub prefix: &'a [u8],
    pub gates: Gates,
}
#[derive(Debug, PartialEq, Eq)]
enum Stage {
    Context,
    Lookup(Lookup),
    Display(Notice),
    Finished,
}
/// Retain the actual request after consuming global pending. Retrying a failed
/// lookup/post must not turn that request into a successful no-pending branch.
#[derive(Debug, PartialEq, Eq)]
pub struct Delivery {
    stage: Stage,
}
impl Default for Delivery {
    fn default() -> Self {
        Self {
            stage: Stage::Context,
        }
    }
}
impl Delivery {
    pub fn finished(&self) -> bool {
        matches!(self.stage, Stage::Finished)
    }
    /// Callers must provide their authoritative state/context and real services.
    /// No default gates, prefix, translation or successful display is invented.
    pub fn advance(
        &mut self,
        state: &mut State,
        context: Option<Context<'_>>,
        mut lookup: impl FnMut(&[u8]) -> Result<Option<Vec<u8>>, String>,
        mut post: impl FnMut(&Notice) -> Result<(), String>,
    ) -> Result<(), String> {
        loop {
            match &self.stage {
                Stage::Finished => return Ok(()),
                Stage::Context => {
                    if state.pending == 0 {
                        self.stage = Stage::Finished;
                        continue;
                    }
                    let context = context.ok_or("mission notice context is not supplied")?;
                    self.stage = match state.pickup(context.prefix, context.gates)? {
                        Some(request) => Stage::Lookup(request),
                        None => Stage::Finished,
                    };
                }
                Stage::Lookup(request) => {
                    let translation = lookup(&request.key)?;
                    self.stage = match request.resolve(translation.as_deref())? {
                        Some(notice) => Stage::Display(notice),
                        None => Stage::Finished,
                    };
                }
                Stage::Display(notice) => {
                    post(notice)?;
                    self.stage = Stage::Finished;
                }
            }
        }
    }
}
impl Lookup {
    /// Missing lookup uses the key. The checked C-locale path ignores ASCII
    /// case; lookup precedence and the actual message queue remain external.
    pub fn resolve(&self, translation: Option<&[u8]>) -> Result<Option<Notice>, &'static str> {
        let translated = translation.unwrap_or(&self.key);
        if translated.contains(&0) {
            return Err("notice translation contains an interior terminator");
        }
        Ok(
            (!translated.eq_ignore_ascii_case(&self.key)).then(|| Notice {
                route: 6,
                text: translated.to_vec(),
            }),
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn load_entry_and_success_reset_different_words_at_different_phases() {
        let mut state = State {
            pending: 7,
            counter: 42,
        };
        let mut time = f32::NAN;
        state.begin_load(&mut time);
        assert_eq!(
            state,
            State {
                pending: 0,
                counter: 42
            }
        );
        assert_eq!(time.to_bits(), 0);
        state.pending = 9;
        state.counter = 43;
        state.parsed();
        assert_eq!(
            state,
            State {
                pending: 9,
                counter: 0
            }
        );
    }
    #[test]
    fn failed_lookup_and_post_retain_the_request_and_do_not_consume_later_pending() {
        let mut state = State {
            pending: 1,
            counter: 9,
        };
        let mut delivery = Delivery::default();
        let context = Context {
            prefix: b"mission",
            gates: Gates {
                suppress: [0; 3],
                time: 0.,
            },
        };
        assert!(
            delivery
                .advance(
                    &mut state,
                    None,
                    |_| panic!("missing context must block lookup"),
                    |_| panic!("must not post")
                )
                .is_err()
        );
        assert_eq!(
            state,
            State {
                pending: 1,
                counter: 9
            }
        );
        assert!(
            delivery
                .advance(
                    &mut state,
                    Some(context),
                    |key| {
                        assert_eq!(key, b"mission10");
                        Err("lookup unavailable".into())
                    },
                    |_| panic!("must not post")
                )
                .is_err()
        );
        assert_eq!(
            state,
            State {
                pending: 0,
                counter: 10
            }
        );
        assert!(!delivery.finished());
        state.pending = 7;
        assert!(
            delivery
                .advance(
                    &mut state,
                    None,
                    |_| Ok(Some(b"message".to_vec())),
                    |notice| {
                        assert_eq!(notice.route, 6);
                        Err("display unavailable".into())
                    }
                )
                .is_err()
        );
        assert_eq!(
            state,
            State {
                pending: 7,
                counter: 10
            }
        );
        assert!(!delivery.finished());
        let mut posts = 0;
        delivery
            .advance(
                &mut state,
                None,
                |_| panic!("lookup must not repeat"),
                |notice| {
                    assert_eq!(notice.text, b"message");
                    posts += 1;
                    Ok(())
                },
            )
            .unwrap();
        delivery
            .advance(
                &mut state,
                None,
                |_| panic!("finished lookup"),
                |_| panic!("finished display"),
            )
            .unwrap();
        assert_eq!(posts, 1);
        assert!(delivery.finished());
        assert_eq!(state.pending, 7);
    }
    #[test]
    fn known_clear_state_needs_no_prefix_gates_lookup_or_display() {
        let mut state = State {
            pending: 0,
            counter: u32::MAX,
        };
        let mut delivery = Delivery::default();
        delivery
            .advance(
                &mut state,
                None,
                |_| panic!("not read"),
                |_| panic!("not read"),
            )
            .unwrap();
        assert!(delivery.finished());
        assert_eq!(state.counter, u32::MAX);
    }
    #[test]
    fn blocked_notice_consumes_pending_without_advancing_or_backfilling() {
        let mut state = State {
            pending: 7,
            counter: 9,
        };
        let blocked = Gates {
            suppress: [0, 1, 0],
            time: -1.,
        };
        assert!(state.pickup(b"mission", blocked).unwrap().is_none());
        assert_eq!(
            state,
            State {
                pending: 0,
                counter: 9
            }
        );
        assert!(
            state
                .pickup(
                    b"mission",
                    Gates {
                        suppress: [0; 3],
                        time: 0.
                    }
                )
                .unwrap()
                .is_none()
        );
        let old = state;
        assert!(state.pickup(b"bad\0prefix", blocked).is_err());
        assert_eq!(state, old);
    }
    #[test]
    fn signed_counter_missing_translation_and_raw_message_bytes_remain_distinct() {
        let mut state = State {
            pending: 1,
            counter: 0x7fffffff,
        };
        let request = state
            .pickup(
                b"story",
                Gates {
                    suppress: [0; 3],
                    time: -0.,
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(request.key, b"story-2147483648");
        assert_eq!(state.counter, 0x80000000);
        assert_eq!(request.resolve(None).unwrap(), None);
        assert_eq!(request.resolve(Some(b"STORY-2147483648")).unwrap(), None);
        assert_eq!(
            request.resolve(Some(b"raw\x01\xe9")).unwrap(),
            Some(Notice {
                route: 6,
                text: b"raw\x01\xe9".to_vec()
            })
        );
        assert!(request.resolve(Some(b"bad\0tail")).is_err());
    }
}
