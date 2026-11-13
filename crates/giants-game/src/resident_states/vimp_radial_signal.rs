//! Radial-call Vimp signal writer, separate from its living response consumer.
//! Prototype PS2 search route `vimp_logic` remains an unknown correspondence;
//! the retained PC helper alias and binary identity stay in the private record.
use super::*;

impl Session {
    /// Copy the actual source actor's current origin and the already-produced
    /// world elapsed time into the acquired Vimp signal slots. This selects no
    /// reaction, assigns no movement and acknowledges no damage or readiness.
    pub fn record_vimp_radial_signal(
        &mut self,
        target: Id,
        source: Id,
        elapsed_seconds: f32,
    ) -> Result<(), String> {
        let source = self
            .get(source)
            .ok_or("Vimp signal source absent or foreign")?;
        if !source.acquired_construction || source.seed.storage.profile() != Profile::Debug {
            return Err("Vimp radial signal source construction not acquired".into());
        }
        let position = source
            .construction_fields
            .as_ref()
            .ok_or("Vimp signal source fields absent")?
            .position;
        let actor = self
            .get(target)
            .ok_or("Vimp signal recipient absent or foreign")?;
        if !actor.acquired_construction
            || actor.class != 8
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 21
            || actor.seed.storage.data().is_none_or(|b| b.len() != 216)
            || actor.vimp.is_none()
            || actor.vimp_active_running
        {
            return Err("Vimp radial signal requires its acquired owned payload".into());
        }
        if !elapsed_seconds.is_finite()
            || elapsed_seconds < 0.
            || !position.into_iter().all(f32::is_finite)
        {
            return Err("Vimp radial signal outside finite current-world profile".into());
        }
        let body = self
            .residents
            .get_mut(&target.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap();
        for (offset, value) in [
            (0xc4, elapsed_seconds),
            (0xc8, position[0]),
            (0xcc, position[1]),
            (0xd0, position[2]),
        ] {
            body[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires GIANTS_NPC_ROOT; real Vimp/source constructors, signal stores only, not ordinary radial damage"]
    fn source_radial_signal_preserves_other_state_and_requires_owned_source() {
        let (mut session, _, _, _, children, _) =
            super::super::vimp_history_tests::source_births(true);
        let parent = children[1];
        let target = children[0];
        let source_position = session
            .get(parent)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        let before = session
            .get(target)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        let channels = session.get(target).unwrap().channels;
        let prior_mode = session.vimp(target).unwrap().mode;
        session
            .record_vimp_radial_signal(target, parent, 0.)
            .unwrap();
        let body = session.get(target).unwrap().seed.storage.data().unwrap();
        assert_eq!(&body[..0xc4], &before[..0xc4]);
        assert_eq!(&body[0xd4..], &before[0xd4..]);
        assert_eq!(u32::from_le_bytes(body[0xc4..0xc8].try_into().unwrap()), 0);
        for (i, value) in source_position.into_iter().enumerate() {
            assert_eq!(
                u32::from_le_bytes(body[0xc8 + i * 4..0xcc + i * 4].try_into().unwrap()),
                value.to_bits()
            );
        }
        assert_eq!(session.get(target).unwrap().channels, channels);
        assert_eq!(session.vimp(target).unwrap().mode, prior_mode);
        session
            .record_vimp_radial_signal(target, parent, 2.5)
            .unwrap();
        let before_error = session
            .get(target)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        assert!(
            Session::new(target.ticket() + 1)
                .record_vimp_radial_signal(target, parent, 3.)
                .is_err()
        );
        assert!(
            session
                .record_vimp_radial_signal(target, parent, f32::NAN)
                .is_err()
        );
        assert_eq!(
            session.get(target).unwrap().seed.storage.data().unwrap(),
            before_error
        );
        session.clear();
        assert!(
            session
                .record_vimp_radial_signal(target, parent, 3.)
                .is_err()
        );
    }
}
