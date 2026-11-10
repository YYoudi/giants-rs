use super::*;
use crate::mission_runtime::{self as runtime, Runtime};
use bevy::{ecs::world::CommandQueue, prelude::*};

fn advance(runtime: &mut Runtime, ids: &[Id], number: u64, terrain: &giants_formats::gti::Terrain) {
    let active = runtime.active.as_mut().unwrap();
    let controlled = active.player_owner().unwrap();
    let npcs = active.payload.npcs.as_mut().unwrap().as_mut().unwrap();
    let query = giants_game::terrain_queries::StaticTerrainQuery::new(
        terrain,
        1. / terrain.spacing,
        giants_game::arithmetic::Precision::Double53,
    )
    .unwrap();
    // Explicit CPU callback inputs. This is not common-world admission and
    // does not acknowledge updates of the other mission actor families.
    let mut stamp = giants_game::animation_epoch::Stamp(200 + number as u32 * 100);
    for &id in ids {
        if number != 0 {
            active
                .resident_states
                .prepare_vimp_world_history(id, number, &npcs.factory)
                .unwrap();
        }
        let frame = giants_game::vimp_frame::Frame {
            number,
            controlled_actor: Some(controlled),
            tick: giants_game::vimp_wander::Tick {
                global_delta: 1. / 60.,
                control_factor: 0.5,
                skip_heading: false,
                slow_areas: &[],
                now: 2.,
                noise_time: -10000.,
                noise_position: [50000., 50000., 0.],
            },
        };
        let mut ground = |point| {
            giants_game::physical_ground::Ground::from_sample(query.sample(point)?, terrain.width)
        };
        let mut services = giants_game::vimp_frame::Services {
            animation: giants_game::vimp_animation::Services {
                blending: giants_game::animation_channels::Blending::Disabled,
                contexts: &mut npcs.bound.as_mut().unwrap().contexts,
                random: &mut active.variant_random,
                stamp: &mut stamp,
            },
            ground: &mut ground,
            recovery_sounds: None,
        };
        if number == 0 {
            active
                .resident_states
                .advance_vimp_newborn_world(id, frame, &npcs.factory, &npcs.globals, &mut services)
                .unwrap();
        } else {
            active
                .resident_states
                .advance_vimp_world_after_history(
                    id,
                    frame,
                    &npcs.factory,
                    &npcs.globals,
                    &mut services,
                )
                .unwrap();
        }
    }
}
#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; canonical Story2 construction with supplied offline birth/callback admission and quiet clocks; CPU Bevy assets only"]
fn source_npc_completed_poses_feed_common_render_assets_and_retire() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let source = giants_game::world_source::resolve(&root, "Story2").unwrap();
    let menu = giants_game::mission_selection::MissionMenu::parse(
        &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
        u32::MAX,
        true,
    )
    .unwrap();
    let packet = Arc::new(
        giants_game::mission_loading::prepare(&root, menu.request("Story2", 0).unwrap()).unwrap(),
    );
    let asset = Arc::new(giants_game::load_world(&root, "Story2").unwrap());
    for ticket in [3511, 3512] {
        let plan = packet.instance_plan(&root, "English").unwrap();
        let mut payload = runtime::tests::payload();
        payload.npcs = super::State::prepare(&root, &plan);
        payload.player = giants_game::player_initial::Prepared::load(
            &root,
            &plan,
            packet.world.as_ref().unwrap(),
            packet.mission.as_ref().unwrap(),
        )
        .map(Arc::new)
        .map_err(|e| e.to_string());
        payload.plan = plan;
        payload.packet = packet.clone();
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let (scene_root, actors, blocked, residents, bindings) =
            runtime::spawn(&mut Commands::new(&mut queue, &world), ticket, &mut payload);
        queue.apply(&mut world);
        let mut runtime = Runtime {
            active: Some(runtime::Active {
                ticket,
                root: scene_root,
                actors,
                resident_states: residents,
                blocked,
                payload,
                start: runtime::StartState {
                    intent: giants_game::mission_loading::LaunchIntent::SavedMission,
                    progress: giants_game::mission_selection::Progress::load(None, None).unwrap(),
                },
                reported: false,
                pickup: None,
                landing: None,
                variant_random: giants_game::audio_parameters::Random(1),
                audio_registry: None,
                bindings,
                controlled_mecc_owner: None,
                owned_mecc: None,
            }),
            ..Default::default()
        };
        assert!(runtime.npc_draws(ticket).unwrap().is_empty());
        assert!(runtime.npc_draws(ticket + 1).is_err());
        let active = runtime.active.as_mut().unwrap();
        let npcs = active.payload.npcs.as_mut().unwrap().as_mut().unwrap();
        let parent = npcs
            .bound
            .as_ref()
            .unwrap()
            .declarations
            .iter()
            .find(|(_, _, class)| *class == 49)
            .unwrap()
            .1;
        struct Ground<'a>(giants_game::terrain_queries::StaticTerrainQuery<'a>);
        impl giants_game::herd_generator::Ground for Ground<'_> {
            fn height(&mut self, p: [f32; 3]) -> Result<f32, String> {
                Ok(self.0.sample([p[0], p[1]])?.height)
            }
        }
        let mut ground = Ground(
            giants_game::terrain_queries::StaticTerrainQuery::new(
                &asset.terrain,
                1. / asset.terrain.spacing,
                giants_game::arithmetic::Precision::Double53,
            )
            .unwrap(),
        );
        let born = active
            .resident_states
            .advance_herd_offline(
                parent,
                giants_game::herd_generator::Tick {
                    step: 0,
                    global_delta: 0.25,
                    slow_areas: &[],
                },
                &npcs.factory,
                &npcs.globals,
                &mut active.variant_random,
                &mut ground,
            )
            .unwrap();
        assert!(born.len() > 1);
        // No bind pose is displayed for newly allocated cold actors.
        assert!(runtime.npc_draws(ticket).unwrap().is_empty());
        advance(&mut runtime, &born, 0, &asset.terrain);
        let before = runtime.active.as_ref().unwrap().variant_random.0;
        let packets = runtime.npc_draws(ticket).unwrap();
        assert_eq!(packets.len(), born.len());
        let first = packets[0].positions.clone();
        let second = packets[1].positions.clone();
        assert_ne!(first, second);
        assert_eq!(runtime.active.as_ref().unwrap().variant_random.0, before);
        let again = runtime.npc_draws(ticket).unwrap();
        assert_eq!(again[0].positions, first);
        assert!(Arc::ptr_eq(&packets[0].geometry, &again[0].geometry));
        runtime = crate::mission_npc_scene::check_render_assets(
            runtime,
            asset.clone(),
            |r| advance(r, &born, 1, &asset.terrain),
            ticket == 3512,
        );
        let active = runtime.active.as_mut().unwrap();
        let npcs = active.payload.npcs.as_mut().unwrap().as_mut().unwrap();
        active
            .resident_states
            .prepare_vimp_world_history(born[0], 2, &npcs.factory)
            .unwrap();
        assert!(runtime.npc_draws(ticket).is_err());
        let active = runtime.active.as_mut().unwrap();
        let npcs = active.payload.npcs.as_mut().unwrap().as_mut().unwrap();
        npcs.factory = Factory::load_registered(active.payload.plan.resources()).unwrap();
        assert!(
            runtime
                .npc_draws(ticket)
                .err()
                .unwrap()
                .contains("another constructor")
        );
        runtime.active.as_mut().unwrap().resident_states.clear();
        assert!(runtime.npc_draws(ticket).is_err());
        println!(
            "npc_render_source ticket={ticket} born={} model_slot=0 supplied_callback_admission=true cpu_assets=true gpu_verified=false",
            born.len()
        );
    }
}
