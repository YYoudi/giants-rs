use super::*;
use crate::vimp_threat::{Parameters, Peer, Signals, Sources, State};

struct Lists<'a> {
    session: &'a Session,
    nearby: Vec<Id>,
    peers: Vec<Id>,
}
impl Sources for Lists<'_> {
    fn nearby(&mut self, index: usize) -> Result<Option<[f32; 3]>, String> {
        self.nearby
            .get(index)
            .map(|&id| {
                self.session
                    .get(id)
                    .and_then(|a| a.construction_fields.as_ref())
                    .map(|f| f.position)
                    .ok_or_else(|| "Vimp threat nearby owner/fields absent".into())
            })
            .transpose()
    }
    fn peer(&mut self, index: usize) -> Result<Option<Peer>, String> {
        self.peers
            .get(index)
            .map(|&id| {
                let a = self.session.get(id).ok_or("Vimp threat peer absent")?;
                let mode = a.vimp.ok_or("Vimp threat peer is not source owned")?.mode;
                // Native dispatch checks the mode before reading this position.
                let position = if matches!(mode, 2..=4) {
                    a.construction_fields
                        .as_ref()
                        .ok_or("Vimp threat peer fields absent")?
                        .position
                } else {
                    [0.; 3]
                };
                Ok(Peer { mode, position })
            })
            .transpose()
    }
}
impl Session {
    pub fn vimp_threat_steps(&self, id: Id) -> Option<u64> {
        let actor = self.get(id)?;
        actor.vimp?;
        Some(actor.vimp_threat_steps)
    }
    /// Isolated consideration with actual owned list order. It writes signal
    /// flags/noise memory only; response, caller timing and world admission are
    /// separate. Nonzero linked state still requires its owned link producer.
    pub fn consider_vimp_threats_offline(
        &mut self,
        id: Id,
        number: u64,
        signals: Signals,
        globals: &giants_formats::global_data::GlobalData,
    ) -> Result<bool, String> {
        use crate::vimp_wander::{put, scalar, word};
        let actor = self.get(id).ok_or("Vimp threat owner absent or foreign")?;
        let vimp = actor
            .vimp
            .ok_or("Vimp threat requires source construction")?;
        if actor.class != 8
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 21
            || !vimp.initialized
            || vimp.wander_pending
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.world_tick_running
            || actor.vimp_active_running
            || actor.vimp_frame_stopped
            || actor.vimp_threat_failure.is_some()
            || actor.vimp_threat_steps != number
            || number == u64::MAX
        {
            return Err(
                "Vimp threat consideration requires fresh isolated owned invocation".into(),
            );
        }
        let body = actor
            .seed
            .storage
            .data()
            .ok_or("Vimp threat payload absent")?;
        if body.len() != 216 || word(body, 0x8c) != 0 {
            return Err("Vimp threat linked state requires its owned producer".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Vimp threat fields absent")?;
        let state = State {
            position: fields.position,
            priority: word(body, 0xd4) as i32,
            flags: fields.control_flags,
            remaining_signal: scalar(body, 0xc4),
            linked_seconds: scalar(body, 0x90),
            linked_present: false,
            heard_at: scalar(body, 0x3c),
            noise_level: scalar(body, 0x94),
        };
        let parameter = |at| globals.scalar(at).map_err(|e| e.to_string());
        let parameters = Parameters {
            nearby_radius: parameter(0x998)?,
            peer_radius: parameter(0x99c)?,
            noise_radius: parameter(0x9a4)?,
            noise_threshold: parameter(0x968)?,
        };
        let result = crate::vimp_threat::consider(
            state,
            signals,
            parameters,
            &mut Lists {
                session: self,
                nearby: self.list(11),
                peers: self.list(10),
            },
        );
        let (state, considered, error) = match result {
            Ok(o) => (o.state, o.considered, None),
            Err(e) => (e.state, false, Some(e.message)),
        };
        let actor = self.residents.get_mut(&id.serial).unwrap();
        actor.construction_fields.as_mut().unwrap().control_flags = state.flags;
        let body = actor.seed.storage.data_mut().unwrap();
        put(body, 0x3c, state.heard_at.to_bits());
        put(body, 0x94, state.noise_level.to_bits());
        actor.vimp_threat_steps += 1;
        actor.vimp_threat_failure = error.clone();
        if error.is_some() {
            actor.vimp_frame_stopped = true;
        }
        error.map_or(Ok(considered), Err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vimp_wander::{put, scalar};
    #[test]
    #[ignore = "requires GIANTS_NPC_ROOT; owned source cohort and supplied noise, no world admission"]
    fn source_vimp_threat_remembers_noise_once_and_refuses_unowned_links() {
        let (mut session, _, globals, _, children, _) =
            super::super::vimp_history_tests::source_births(true);
        let id = children[0];
        let actor = session.get(id).unwrap();
        let body = actor.seed.storage.data().unwrap();
        let at = scalar(body, 0x3c) + 1.;
        let old_level = scalar(body, 0x94);
        let position = actor.construction_fields.as_ref().unwrap().position;
        let signals = Signals {
            now: at + 0.5,
            noise_at: at,
            noise_position: position,
        };
        assert!(globals.scalar(0x9a4).unwrap() > 0.);
        assert!(
            session
                .consider_vimp_threats_offline(Id { ticket: 999, ..id }, 0, signals, &globals)
                .is_err()
        );
        assert!(
            session
                .consider_vimp_threats_offline(id, 1, signals, &globals)
                .is_err()
        );
        let body = session
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap();
        put(body, 0x8c, 1); // Supplied unsupported link, never treated as an owned address.
        assert!(
            session
                .consider_vimp_threats_offline(id, 0, signals, &globals)
                .is_err()
        );
        assert_eq!(session.vimp_threat_steps(id), Some(0));
        put(
            session
                .residents
                .get_mut(&id.serial)
                .unwrap()
                .seed
                .storage
                .data_mut()
                .unwrap(),
            0x8c,
            0,
        );
        let expected = f64::from(old_level) + 1. >= f64::from(globals.scalar(0x968).unwrap());
        assert_eq!(
            session
                .consider_vimp_threats_offline(id, 0, signals, &globals)
                .unwrap(),
            expected
        );
        let actor = session.get(id).unwrap();
        assert_eq!(scalar(actor.seed.storage.data().unwrap(), 0x3c), at);
        assert_eq!(
            scalar(actor.seed.storage.data().unwrap(), 0x94),
            (f64::from(old_level) + 1.) as f32
        );
        assert_eq!(
            actor.construction_fields.as_ref().unwrap().control_flags,
            if expected { 2 } else { 0 }
        );
        assert!(
            !session
                .consider_vimp_threats_offline(id, 1, signals, &globals)
                .unwrap()
        );
        assert_eq!(session.vimp_threat_steps(id), Some(2));
        assert_eq!(
            scalar(session.get(id).unwrap().seed.storage.data().unwrap(), 0x94),
            (f64::from(old_level) + 1.) as f32
        );
    }
    #[test]
    #[ignore = "requires GIANTS_NPC_ROOT; injected finite-profile failure after owned writes"]
    fn source_vimp_threat_late_failure_preserves_signal_and_noise_stamp() {
        let (mut session, factory, globals, _, children, _) =
            super::super::vimp_history_tests::source_births(true);
        let id = children[0];
        let actor = session.residents.get_mut(&id.serial).unwrap();
        let body = actor.seed.storage.data_mut().unwrap();
        let at = scalar(body, 0x3c) + 1.;
        let level = scalar(body, 0x94);
        put(body, 0xc4, 1_f32.to_bits());
        let signals = Signals {
            now: at,
            noise_at: at,
            noise_position: [f32::NAN, 0., 0.],
        };
        let error = session
            .consider_vimp_threats_offline(id, 0, signals, &globals)
            .unwrap_err();
        assert!(error.contains("finite profile"));
        let actor = session.get(id).unwrap();
        assert_eq!(
            actor.construction_fields.as_ref().unwrap().control_flags,
            0x20
        );
        assert_eq!(scalar(actor.seed.storage.data().unwrap(), 0x3c), at);
        assert_eq!(scalar(actor.seed.storage.data().unwrap(), 0x94), level);
        assert_eq!(session.vimp_threat_steps(id), Some(1));
        assert!(
            session
                .consider_vimp_threats_offline(id, 1, signals, &globals)
                .is_err()
        );
        assert!(session.prepare_vimp_world_history(id, 0, &factory).is_err());
        assert_eq!(session.vimp_threat_steps(id), Some(1));
    }
}
