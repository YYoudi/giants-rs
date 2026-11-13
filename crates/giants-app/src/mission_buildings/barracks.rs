//! Ordinary state98 initial branch and real object-bound audio requests.
//! Damage, later spawning/AI and full world-update admission remain separate.
use giants_game::{
    building_barracks::InitialParameters,
    declared_residents::{Key, Scope},
    resident_states::Session,
};

pub(crate) struct Initial {
    keys: Vec<Key>,
    stock: InitialParameters,
    consumed: bool,
    failure: Option<String>,
}

impl Initial {
    pub(crate) fn prepare(
        plan: &giants_game::instances::Plan,
        globals: &giants_formats::global_data::GlobalData,
    ) -> Result<Self, String> {
        let world = plan
            .world_entries
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e.declaration.object_type, 1060 | 1226))
            .map(|(index, _)| Key {
                scope: Scope::World,
                index,
            });
        let mission = plan
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e.serialized.object_type, 1060 | 1226))
            .map(|(index, _)| Key {
                scope: Scope::Mission,
                index,
            });
        Ok(Self {
            keys: world.chain(mission).collect(),
            stock: InitialParameters::load(globals)?,
            consumed: false,
            failure: None,
        })
    }
    pub(crate) fn start(
        &mut self,
        session: &mut Session,
        events: &mut Vec<crate::mission_audio::Event>,
    ) -> Result<(), String> {
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        if self.consumed {
            return Ok(());
        }
        let owners = self
            .keys
            .iter()
            .map(|key| {
                session
                    .declaration(*key)
                    .ok_or_else(|| format!("barracks source owner missing: {key:?}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Exact authored keys, including declarations that failed construction:
        // a partial resident list cannot turn into an empty successful start.
        self.consumed = true;
        for owner in owners {
            let result =
                session.initialize_barracks(owner, self.stock, false, &mut |owner, sound| {
                    events.push(crate::mission_audio::Event::ActorSound {
                        owner,
                        sound,
                        kind: giants_game::actor_sound::Kind::Owner,
                    });
                    Ok(())
                });
            if let Err(error) = result {
                self.failure = Some(error.clone());
                return Err(error);
            }
            println!(
                "mission_barracks_initialized owner={owner:?} selector={} remaining={} initial_branch=true audio_request=true backend_acknowledged=false later_primary=false",
                session.barracks_state(owner)?.selector,
                session.barracks_state(owner)?.remaining
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; actual authored owners, stock and audio resources, not device/GPU/normal hit acceptance"]
    fn installed_barracks_initial_queues_real_owner_sounds_once() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_BUILDING_ROOT").unwrap());
        let menu = giants_game::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            giants_game::mission_loading::prepare(&root, menu.request("Story2", 1).unwrap())
                .unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let globals =
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
                .unwrap();
        let mut initial = Initial::prepare(&plan, &globals).unwrap();
        assert_eq!(initial.keys.len(), 2);
        let mut session = Session::new(412);
        let mut events = Vec::new();
        assert!(initial.start(&mut session, &mut events).is_err());
        assert!(events.is_empty());
        giants_game::declared_residents::construct_acquired(&plan, &mut session);
        initial.start(&mut session, &mut events).unwrap();
        assert_eq!(events.len(), 2);
        let resources = plan.resources();
        let registry =
            giants_game::gameplay_sound_bank::Registry::prepare(&resources.classes.visits, false)
                .unwrap();
        let (language, _) = giants_game::gameplay_sound_bank::voice_language(
            &resources.archives,
            &resources.language,
        )
        .unwrap();
        for event in &events {
            let crate::mission_audio::Event::ActorSound { owner, sound, kind } = event else {
                panic!("initial sound lost its object owner");
            };
            assert_eq!(*kind, giants_game::actor_sound::Kind::Owner);
            assert_eq!(session.barracks_state(*owner).unwrap().phase, 1);
            assert!(session.actor_world_tick(*owner).is_err());
            let source = registry
                .resolve(&resources.archives, &language, *sound)
                .unwrap()
                .unwrap();
            assert!(!source.pcm.samples.is_empty());
        }
        initial.start(&mut session, &mut events).unwrap();
        assert_eq!(events.len(), 2);
        session.clear();
        for event in events {
            let crate::mission_audio::Event::ActorSound { owner, .. } = event else {
                unreachable!()
            };
            assert!(session.barracks_state(owner).is_err());
        }
    }
}
