//! Source declaration coverage, not ordinary encounter acceptance.
use super::*;

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; ordinary supplied-input frame/pose control, no GPU or OS input"]
fn source_wife_rippers_complete_current_frames_without_replaying_history() {
    current_frames(false, false);
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; explicit real marker-constructor fixture in ordinary frame/render consumers, no GPU or natural awakening proof"]
fn source_wife_marker_children_use_current_frames_and_common_render_assets() {
    current_frames(true, false);
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; supplied source motion for explicit two-marker retirement fixture, headless assets only, not natural AI or OS input"]
fn source_wife_markers_retire_through_the_ordinary_component_and_render_consumer() {
    current_frames(true, true);
}

fn current_frames(marker_fixture: bool, retirement_fixture: bool) {
    use crate::mission_runtime as runtime;
    use bevy::{ecs::world::CommandQueue, prelude::*};
    use std::sync::Arc;
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let source = giants_game::world_source::resolve(&root, "Story2").unwrap();
    let menu = giants_game::mission_selection::MissionMenu::parse(
        &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
        u32::MAX,
        true,
    )
    .unwrap();
    let packet = Arc::new(
        giants_game::mission_loading::prepare(&root, menu.request("Story2", 1).unwrap()).unwrap(),
    );
    let asset = Arc::new(packet.world_asset(&root).unwrap());
    for ticket in [9401, 9402] {
        let plan = packet.instance_plan(&root, "English").unwrap();
        let mut payload = runtime::tests::payload();
        payload.rippers = super::State::prepare(&root, &plan);
        payload.player = giants_game::player_initial::Prepared::load(
            &root,
            &plan,
            packet.world.as_ref().unwrap(),
            packet.mission.as_ref().unwrap(),
        )
        .map(Arc::new)
        .map_err(|e| e.to_string());
        payload.world = Ok(asset.clone());
        let mut viewer = crate::mecc_dev_scene::State::load_mission(
            &root,
            &asset,
            &packet,
            payload.player.as_ref().unwrap(),
        )
        .unwrap();
        viewer.sample_pose().unwrap();
        payload.plan = plan;
        payload.packet = packet.clone();
        let mut world = World::new();
        let mut queue = CommandQueue::default();
        let (scene_root, actors, blocked, residents, bindings) =
            runtime::spawn(&mut Commands::new(&mut queue, &world), ticket, &mut payload);
        queue.apply(&mut world);
        let mut runtime = runtime::Runtime {
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
                campaign: None,
                variant_random: giants_game::audio_parameters::Random(1),
                audio_registry: None,
                bindings,
                controlled_mecc_owner: None,
                owned_mecc: None,
            }),
            ..Default::default()
        };
        assert!(runtime.npc_draws(ticket).unwrap().is_empty());
        assert!(runtime.ripper_contact_snapshots(ticket).unwrap().is_empty());
        {
            let active = runtime.active.as_mut().unwrap();
            let id = active
                .payload
                .rippers
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .bound
                .as_ref()
                .unwrap()
                .1[0]
                .id;
            assert!(
                active
                    .resident_states
                    .ripper_contact_snapshot(id, 1)
                    .is_err()
            );
        }
        for number in 1..=180u64 {
            runtime
                .sample_world_clock(ticket, 1000 + number as u32 * 16)
                .unwrap();
            let motion = viewer
                .step(0.016, Vec2::ZERO, &asset.terrain, false, false)
                .unwrap();
            runtime
                .publish_controlled_mecc_motion(ticket, number, motion)
                .unwrap();
            runtime
                .advance_normal_rippers(
                    ticket,
                    number,
                    viewer.camera().1,
                    viewer.mission_environment_colliders(),
                )
                .unwrap_or_else(|e| panic!("frame {number}: {e}"));
            let draws = runtime.npc_draws(ticket).unwrap();
            assert_eq!(
                draws.len(),
                5,
                "five completed source Ripper poses must reach the common draw consumer"
            );
            assert!(
                draws
                    .iter()
                    .all(|d| matches!(d.class, 73 | 230) && !d.positions.is_empty())
            );
            if number.is_multiple_of(30) {
                let contacts = runtime.ripper_contact_snapshots(ticket).unwrap();
                assert_eq!(contacts.len(), 5);
                assert_eq!(contacts.iter().filter(|v| v.actor.class == 73).count(), 4);
                assert_eq!(contacts.iter().filter(|v| v.actor.class == 230).count(), 1);
                for snapshot in &contacts {
                    assert_eq!(snapshot.frame, number);
                    assert_eq!(snapshot.actor.state, 23);
                    assert!(!snapshot.triangles.is_empty());
                    // A supplied segment through an actual current source face
                    // must be consumable by the shared projectile narrow phase.
                    let triangle = snapshot
                        .triangles
                        .iter()
                        .find(|t| {
                            let a: [f32; 3] =
                                std::array::from_fn(|i| t.vertices[1][i] - t.vertices[0][i]);
                            let b: [f32; 3] =
                                std::array::from_fn(|i| t.vertices[2][i] - t.vertices[0][i]);
                            Vec3::from(a).cross(Vec3::from(b)).length_squared() > 0.01
                        })
                        .unwrap();
                    let center = (Vec3::from(triangle.vertices[0])
                        + Vec3::from(triangle.vertices[1])
                        + Vec3::from(triangle.vertices[2]))
                        / 3.;
                    let normal = (Vec3::from(triangle.vertices[1])
                        - Vec3::from(triangle.vertices[0]))
                    .cross(Vec3::from(triangle.vertices[2]) - Vec3::from(triangle.vertices[0]))
                    .normalize();
                    assert!(
                        snapshot
                            .mesh
                            .hit(
                                (center + normal * 100.).to_array(),
                                (center - normal * 100.).to_array()
                            )
                            .unwrap()
                            .is_some()
                    );
                }
                viewer.set_mission_contacts(ticket, contacts).unwrap();
                let active = runtime.active.as_mut().unwrap();
                let id = active
                    .payload
                    .rippers
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .bound
                    .as_ref()
                    .unwrap()
                    .1[0]
                    .id;
                assert!(
                    active
                        .resident_states
                        .ripper_contact_snapshot(id, 0)
                        .is_err()
                );
                assert!(
                    Session::new(ticket + 100)
                        .ripper_contact_snapshot(id, number)
                        .is_err()
                );
            }
            let active = runtime.active.as_mut().unwrap();
            let rippers = active.payload.rippers.as_mut().unwrap().as_mut().unwrap();
            assert_eq!(
                rippers
                    .bound
                    .as_ref()
                    .unwrap()
                    .1
                    .iter()
                    .filter(|owner| rippers.driver.completed(owner.id).is_some())
                    .count(),
                5
            );
            for owner in &rippers.bound.as_ref().unwrap().1 {
                assert_eq!(
                    active
                        .resident_states
                        .actor_finalization(owner.id)
                        .unwrap()
                        .phase(),
                    giants_game::resident_finalization::Phase::Idle
                );
                assert!(
                    active
                        .resident_states
                        .advance_ripper_animation(
                            owner.id,
                            0.016,
                            &mut giants_game::animation_epoch::Stamp::default(),
                            rippers.contexts.as_mut().unwrap(),
                            &mut active.variant_random
                        )
                        .is_err()
                );
            }
            let before = active.variant_random.0;
            let duplicate = runtime
                .advance_normal_rippers(
                    ticket,
                    number,
                    viewer.camera().1,
                    viewer.mission_environment_colliders(),
                )
                .unwrap();
            assert!(duplicate.is_empty());
            assert_eq!(runtime.active.as_ref().unwrap().variant_random.0, before);
        }
        if marker_fixture {
            let active = runtime.active.as_mut().unwrap();
            let state = active.payload.rippers.as_mut().unwrap().as_mut().unwrap();
            let parents = state
                .bound
                .as_ref()
                .unwrap()
                .1
                .iter()
                .take(2)
                .map(|owner| owner.id)
                .collect::<Vec<_>>();
            for parent in parents {
                state
                    .marker_factory
                    .construct(&mut active.resident_states, parent)
                    .unwrap();
            }
            runtime.sample_world_clock(ticket, 1000 + 181 * 16).unwrap();
            let motion = viewer
                .step(0.016, Vec2::ZERO, &asset.terrain, false, false)
                .unwrap();
            runtime
                .publish_controlled_mecc_motion(ticket, 181, motion)
                .unwrap();
            runtime
                .advance_normal_rippers(
                    ticket,
                    181,
                    viewer.camera().1,
                    viewer.mission_environment_colliders(),
                )
                .unwrap();
            assert_eq!(
                runtime
                    .npc_draws(ticket)
                    .unwrap()
                    .iter()
                    .filter(|draw| draw.class == 219)
                    .count(),
                2
            );
        }
        runtime = crate::mission_npc_scene::check_render_assets(
            runtime,
            asset.clone(),
            |runtime| {
                let mut markers = Vec::new();
                if retirement_fixture {
                    let session = &mut runtime.active.as_mut().unwrap().resident_states;
                    markers = session
                        .list(0)
                        .into_iter()
                        .filter(|&id| session.get(id).unwrap().class == 219)
                        .collect();
                    // Explicit behavior-output fixture through the real common-motion
                    // consumer. It neither supplies a pose nor proves natural awakening.
                    let parents: Vec<_> = markers
                        .iter()
                        .map(|&id| match session.surface_link(id).unwrap() {
                            giants_game::resident_states::SurfaceLink::Marker { owner, .. } => {
                                owner
                            }
                            _ => panic!(),
                        })
                        .collect();
                    for parent in parents {
                        session.begin_actor_frame_history(parent).unwrap();
                        session.begin_actor_motion_history(parent, [0; 8]).unwrap();
                        session
                            .advance_actor_common_motion(
                                parent,
                                giants_game::resident_motion::Commands {
                                    bytes: [0, 0, 32, 0, 0, 0, 0, 0],
                                    context: 99,
                                    target_heading: 0.,
                                    target_position: [0.; 2],
                                },
                                giants_game::resident_motion::Time {
                                    seconds: 1.,
                                    areas: &[],
                                    control_factor: 1.,
                                    running_factor: 1.,
                                    skip_heading: false,
                                    controlled_actor: false,
                                },
                            )
                            .unwrap();
                        let fields = session
                            .get(parent)
                            .unwrap()
                            .construction_fields
                            .as_ref()
                            .unwrap();
                        let context = fields.dispatch_context;
                        let query = giants_game::terrain_queries::StaticTerrainQuery::new(
                            &asset.terrain,
                            1. / asset.terrain.spacing,
                            giants_game::arithmetic::Precision::Double53,
                        )
                        .unwrap();
                        let mut finalization =
                            giants_game::actor_environment::FinalizationServices {
                                actor: session.actor_collision_identity(parent).unwrap(),
                                state: 23,
                                slope_parameter: f32::from_bits(fields.configuration[11]),
                                physics: None,
                                skip_object_responses: false,
                                objects: viewer.mission_environment_colliders(),
                                ground: |point| {
                                    giants_game::physical_ground::Ground::from_sample(
                                        query.sample(point)?,
                                        asset.terrain.width,
                                    )
                                },
                                platform: |point| {
                                    giants_game::platform_queries::sample_prepared(
                                        point,
                                        &[],
                                        giants_game::arithmetic::Precision::Double53,
                                    )
                                },
                                seconds: |_| 1.,
                                release: |footprint: Option<u32>| {
                                    if footprint.is_none() {
                                        Ok(())
                                    } else {
                                        Err("fixture footprint registry absent".into())
                                    }
                                },
                                statistics: None,
                            };
                        session
                            .finalize_actor(
                                parent,
                                giants_game::actor_finalization::Time {
                                    global_seconds: 1.,
                                    context,
                                    areas: &[],
                                },
                                &mut finalization,
                            )
                            .unwrap();
                        session.refresh_actor_environment_geometry(parent).unwrap();
                    }
                }
                struct Release {
                    markers: Vec<Id>,
                    released: Vec<Id>,
                }
                impl giants_game::actor_retirement::Services for Release {
                    fn release(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
                        assert!(self.markers.contains(&id));
                        assert!(!self.released.contains(&id));
                        // Both simultaneous closing children lose incoming references
                        // before the first external release, in current-list order.
                        for &marker in &self.markers {
                            if session.get(marker).is_some() {
                                assert!(session.incoming_actor_references(marker)?.is_empty());
                            }
                        }
                        self.released.push(id);
                        Ok(())
                    }
                }
                let mut release = Release {
                    markers: markers.clone(),
                    released: vec![],
                };
                for number in if retirement_fixture {
                    182..=681u64
                } else if marker_fixture {
                    182..=211u64
                } else {
                    181..=210u64
                } {
                    runtime
                        .sample_world_clock(ticket, 1000 + number as u32 * 16)
                        .unwrap();
                    let motion = viewer
                        .step(0.016, Vec2::ZERO, &asset.terrain, false, false)
                        .unwrap();
                    runtime
                        .publish_controlled_mecc_motion(ticket, number, motion)
                        .unwrap();
                    runtime
                        .advance_normal_rippers_with_retirement(
                            ticket,
                            number,
                            viewer.camera().1,
                            viewer.mission_environment_colliders(),
                            &mut release,
                        )
                        .unwrap();
                }
                if retirement_fixture {
                    assert_eq!(release.released, markers);
                    let active = runtime.active.as_ref().unwrap();
                    for marker in &markers {
                        assert!(active.resident_states.get(*marker).is_none());
                    }
                    let state = active.payload.rippers.as_ref().unwrap().as_ref().unwrap();
                    assert!(state.driver.markers().next().is_none());
                    assert_eq!(runtime.npc_draws(ticket).unwrap().len(), 5);
                }
            },
            false,
        );
        // A new history phase invalidates both render-time and contact-time
        // use of the old completed pose; do not substitute a constructor mesh.
        {
            let active = runtime.active.as_mut().unwrap();
            let id = active
                .payload
                .rippers
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .bound
                .as_ref()
                .unwrap()
                .1[0]
                .id;
            active
                .resident_states
                .begin_actor_frame_history(id)
                .unwrap();
            assert!(runtime.ripper_contact_snapshots(ticket).is_err());
        }
        let active = runtime.active.as_mut().unwrap();
        active.resident_states.clear();
        assert!(
            active
                .payload
                .rippers
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .view(&active.resident_states, ticket)
                .is_err()
        );
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; declaration admission across authored Story2 scenarios, no live AI"]
fn source_ripper_declarations_follow_each_authored_scenario() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let world = giants_game::world_source::resolve(&root, "Story2").unwrap();
    let menu = giants_game::mission_selection::MissionMenu::parse(
        &std::fs::read(root.join("Bin").join(world.catalog)).unwrap(),
        u32::MAX,
        true,
    )
    .unwrap();
    for (index, expected) in [
        (0, (7, 4)),
        (1, (4, 1)),
        (2, (7, 1)),
        (3, (2, 0)),
        (4, (2, 1)),
    ] {
        let request = menu.request("Story2", index).unwrap();
        let packet = giants_game::mission_loading::prepare(&root, request).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let ticket = 9200 + index as u64;
        let mut session = Session::new(ticket);
        let report = giants_game::declared_residents::construct_acquired(&plan, &mut session);
        let mut component = State::prepare(&root, &plan).unwrap().unwrap();
        let view = component.bind(ticket, &report, &session).unwrap();
        assert_eq!((view.class73, view.class230), expected);
        assert_eq!(
            session
                .list(0)
                .iter()
                .filter(|id| {
                    session
                        .get(**id)
                        .is_some_and(|actor| matches!(actor.class, 73 | 230))
                })
                .count(),
            expected.0 + expected.1
        );
        session.clear();
        assert!(component.view(&session, ticket).is_err());
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; exact source mission declarations and constructor identity only"]
fn source_wife_ripper_bindings_use_declared_owners_and_survive_no_ticket_reuse() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let world = giants_game::world_source::resolve(&root, "Story2").unwrap();
    let menu = giants_game::mission_selection::MissionMenu::parse(
        &std::fs::read(root.join("Bin").join(world.catalog)).unwrap(),
        u32::MAX,
        true,
    )
    .unwrap();
    let request = menu.request("Story2", 1).unwrap();
    assert_eq!(request.source_id, 1);
    assert_eq!(request.mission_name, "2b_1_savewife");
    let packet = giants_game::mission_loading::prepare(&root, request).unwrap();
    let plan = packet.instance_plan(&root, "English").unwrap();
    let world = packet.world_asset(&root).unwrap();
    let mut old: Option<State> = None;
    for ticket in [9101, 9102] {
        let mut session = Session::new(ticket);
        let mut report = giants_game::declared_residents::construct_acquired(&plan, &mut session);
        let mut state = State::prepare(&root, &plan).unwrap().unwrap();
        assert_eq!(state.expected.len(), 5);
        assert!(state.expected.iter().all(|d| d.key.scope == Scope::Mission));
        let view = state.bind(ticket, &report, &session).unwrap();
        assert_eq!(
            view,
            View {
                class73: 4,
                class230: 1
            }
        );
        assert!(state.bind(ticket, &report, &session).is_err());
        assert!(state.view(&session, ticket + 1).is_err());
        if let Some(previous) = &old {
            assert!(previous.view(&session, ticket).is_err());
        }
        state
            .initialize(&mut session, ticket, plan.resources(), &world)
            .unwrap();
        assert!(
            state
                .initialize(&mut session, ticket, plan.resources(), &world)
                .is_err()
        );
        for owner in &state.bound.as_ref().unwrap().1 {
            let fields = session
                .get(owner.id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap();
            let declaration = plan.entries[owner.declaration.key.index]
                .source
                .as_ref()
                .unwrap();
            assert_eq!(
                fields.position.map(f32::to_bits),
                declaration.position.map(f32::to_bits)
            );
            assert_eq!(fields.angles[0].to_bits(), declaration.yaw.to_bits());
            assert!(session.ripper_primary(owner.id).is_err());
            assert_eq!(
                session.actor_finalization(owner.id).unwrap().phase(),
                giants_game::resident_finalization::Phase::Idle
            );
            assert!(!session.actor_motion(owner.id).unwrap().awaiting_motion());
        }
        // Replacing a report ID by another real Ripper must not be accepted.
        let first = state.expected[0].key;
        let other = state.bound.as_ref().unwrap().1[1].id;
        report
            .records
            .iter_mut()
            .find(|r| r.key == first)
            .unwrap()
            .result = Ok(other);
        let mut bad = State::prepare(&root, &plan).unwrap().unwrap();
        assert!(bad.bind(ticket, &report, &session).is_err());
        assert!(bad.bound.is_none());
        session.clear();
        assert!(state.view(&session, ticket).is_err());
        old = Some(state);
    }
}
