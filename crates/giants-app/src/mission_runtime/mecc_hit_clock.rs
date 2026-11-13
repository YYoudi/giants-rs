//! Actual ordinary partial-controller voice/pain clock binding. This consumes
//! the same source clock sample as the admitted NPC components, independently
//! of their availability; no full native primary/world admission is asserted.
use super::*;

impl Runtime {
    pub(crate) fn advance_normal_mecc_hit_clock(
        &mut self,
        ticket: u64,
        step: u64,
    ) -> Result<bool, String> {
        let active = self
            .active
            .as_mut()
            .filter(|a| a.ticket == ticket)
            .ok_or("Mecc hit clock mission ticket absent")?;
        let owner = active.player_owner()?;
        if active.owned_mecc.is_some() || active.controlled_mecc_owner != Some(owner) {
            return Err("Mecc hit clock requires the current ordinary partial controller".into());
        }
        let (sample, clock) = active
            .payload
            .signals
            .current()
            .ok_or("Mecc hit clock actual global sample absent")?;
        let first = active.resident_states.mecc_countdown_world().is_none();
        let advanced = self.normal_mecc_hit_clock.advance(
            &mut active.resident_states,
            owner,
            step,
            sample,
            clock.seconds,
        )?;
        if first && advanced {
            println!(
                "mission_mecc_hit_clock_bound ticket={ticket} owner={owner:?} sample={sample} shared_bits={} global_seconds_bits={} partial_prefix=true",
                active
                    .resident_states
                    .mecc_countdown_world()
                    .unwrap()
                    .to_bits(),
                clock.seconds.to_bits()
            );
        }
        Ok(advanced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires GIANTS_NPC_ROOT; real Wife construction, partial controller/global samples and retained application clock across two Session teardowns; no GPU/OS input"]
    fn source_wife_normal_mecc_hit_clock_survives_menu_session_reentry() {
        use bevy::ecs::world::CommandQueue;
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
        let source = giants_game::world_source::resolve(&root, "Story2").unwrap();
        let menu = giants_game::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
            u32::MAX,
            true,
        )
        .unwrap();
        let packet = Arc::new(
            giants_game::mission_loading::prepare(&root, menu.request("Story2", 1).unwrap())
                .unwrap(),
        );
        let asset = Arc::new(packet.world_asset(&root).unwrap());
        let mut runtime = Runtime::default();
        let mut previous_timer = 10f32;
        let mut previous_owner = None;
        for ticket in [9601, 9602] {
            let plan = packet.instance_plan(&root, "English").unwrap();
            let mut payload = super::super::tests::payload();
            payload.player = giants_game::player_initial::Prepared::load(
                &root,
                &plan,
                packet.world.as_ref().unwrap(),
                packet.mission.as_ref().unwrap(),
            )
            .map(Arc::new)
            .map_err(|e| e.to_string());
            payload.world = Ok(asset.clone());
            let mut controller = crate::mecc_dev_scene::State::load_mission(
                &root,
                &asset,
                &packet,
                payload.player.as_ref().unwrap(),
            )
            .unwrap();
            controller.sample_pose().unwrap();
            payload.plan = plan;
            payload.packet = packet.clone();
            let mut world = World::new();
            let mut queue = CommandQueue::default();
            let (scene_root, actors, blocked, residents, bindings) =
                super::super::spawn(&mut Commands::new(&mut queue, &world), ticket, &mut payload);
            queue.apply(&mut world);
            runtime.active = Some(Active {
                ticket,
                root: scene_root,
                actors,
                resident_states: residents,
                blocked,
                payload,
                start: StartState {
                    intent: LaunchIntent::SavedMission,
                    progress: Progress::load(None, None).unwrap(),
                },
                reported: false,
                pickup: None,
                landing: None,
                campaign: None,
                cinematic_epoch: giants_game::animation_epoch::Stamp::default(),
                variant_random: giants_game::audio_parameters::Random(1),
                audio_registry: None,
                bindings,
                controlled_mecc_owner: None,
                owned_mecc: None,
            });
            let owner = runtime.active.as_ref().unwrap().player_owner().unwrap();
            assert_ne!(previous_owner, Some(owner));
            assert!(
                runtime
                    .active
                    .as_ref()
                    .unwrap()
                    .resident_states
                    .mecc_countdown_world()
                    .is_none()
            );
            let rng = runtime.active.as_ref().unwrap().variant_random.0;
            for step in 1..=60 {
                runtime
                    .sample_world_clock(ticket, 1000 + step as u32 * 16)
                    .unwrap();
                let motion = controller
                    .step(1. / 120., Vec2::ZERO, &asset.terrain, false, false)
                    .unwrap();
                runtime
                    .publish_controlled_mecc_motion(ticket, step, motion)
                    .unwrap();
                assert!(runtime.advance_normal_mecc_hit_clock(ticket, step).unwrap());
                assert!(!runtime.advance_normal_mecc_hit_clock(ticket, step).unwrap());
                if step == 1 {
                    let a = runtime.active.as_ref().unwrap();
                    let current = a.resident_states.mecc_countdown_world().unwrap();
                    // The actual first clock sample is not assumed zero. The
                    // second ticket must continue below its retained value,
                    // rather than reinitializing the shared timer to ten.
                    assert!(current <= previous_timer);
                    assert!(current > previous_timer - 1.);
                    assert_eq!(
                        runtime.normal_mecc_hit_clock.saved().to_bits(),
                        previous_timer.to_bits()
                    );
                }
            }
            let active = runtime.active.as_ref().unwrap();
            let current = active.resident_states.mecc_countdown_world().unwrap();
            assert!(current < previous_timer);
            assert_eq!(active.variant_random.0, rng);
            super::super::clear(&mut runtime, &mut Commands::new(&mut queue, &world));
            queue.apply(&mut world);
            assert!(runtime.active.is_none());
            assert_eq!(
                runtime.normal_mecc_hit_clock.saved().to_bits(),
                current.to_bits()
            );
            assert!(runtime.normal_mecc_hit_clock.failure().is_none());
            previous_timer = current;
            previous_owner = Some(owner);
        }
        println!(
            "normal_mecc_hit_clock source_wife=true fresh_tickets=2 shared_timer_retained=true actual_global_samples=120 no_duplicate_subtraction=true rng_unchanged=true rendered=false windows_input=false"
        );
    }
}
