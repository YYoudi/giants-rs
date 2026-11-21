//! Scene-aware ordered Delphi entry component. This is not gameplay admission.
use super::*;
use giants_game::{
    camera_mode, camera_owner, camera_profile, camera_projection, mecc_camera_update,
    player_camera, player_shell,
};

pub(super) trait EntryCamera:
    player_shell::Camera + giants_game::delphi_player::ProjectionReset
{
}
impl<T: player_shell::Camera + giants_game::delphi_player::ProjectionReset> EntryCamera for T {}
pub(super) struct CameraRef<'a>(pub &'a mut dyn EntryCamera);
impl player_shell::Camera for CameraRef<'_> {
    fn change_view(
        &mut self,
        s: &mut giants_game::resident_states::Session,
        id: ActorId,
        request: i32,
    ) -> Result<(), String> {
        self.0.change_view(s, id, request)
    }
    fn update_camera(
        &mut self,
        s: &mut giants_game::resident_states::Session,
        id: ActorId,
    ) -> Result<(), String> {
        self.0.update_camera(s, id)
    }
}
impl giants_game::delphi_player::ProjectionReset for CameraRef<'_> {
    fn reset_projection(
        &mut self,
        s: &giants_game::resident_states::Session,
        id: ActorId,
    ) -> Result<(), String> {
        self.0.reset_projection(s, id)
    }
}

pub(super) struct State {
    catalog: camera_profile::ExecutableCatalog,
    objects: Vec<giants_game::player_world_collision::Object>,
    modes: camera_mode::State,
    owners: camera_owner::State<ActorId>,
    pose: mecc_camera_update::State,
    sequence: Option<player_camera::Sequence>,
    pub pending: bool,
    failure: Option<String>,
}
impl State {
    pub fn prepare(
        root: &std::path::Path,
        plan: &Plan,
        packet: &PreparedMission,
    ) -> Result<Self, String> {
        let executable = std::env::var_os("GIANTS_DELPHI_EXECUTABLE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| root.join("Giants.exe"));
        let catalog = camera_profile::ExecutableCatalog::read(&executable)?;
        let terrain = packet.terrain.as_ref().ok_or("Delphi terrain absent")?;
        let prepared = giants_game::declared_mesh_scene::prepare_camera(plan, terrain);
        if !prepared.needs.is_empty() {
            return Err(format!(
                "Delphi camera mesh preparation has {} unresolved declarations: {}",
                prepared.needs.len(),
                prepared
                    .needs
                    .iter()
                    .take(3)
                    .map(|n| format!("{:?}: {}", n.key, n.reason))
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        if prepared
            .player_objects
            .iter()
            .any(|o| o.fields.id == 1 || o.fields.parent == 1)
        {
            return Err("Delphi reserved local camera key overlaps prepared geometry".into());
        }
        Ok(Self {
            catalog,
            objects: prepared.player_objects,
            // Explicit normal-profile inspection lifecycle. This does not
            // certify the original enclosing global reset or authored startup.
            modes: camera_mode::State::new(0, 1, 0.)?,
            owners: camera_owner::State::new(None, 0),
            pose: Default::default(),
            sequence: Some(Default::default()),
            pending: true,
            failure: None,
        })
    }
}

struct Sink<'a> {
    scene: &'a mut crate::world_scene::Scene,
    view: &'a crate::source_view::SourceView,
    buffers: &'a mut Assets<bevy::render::storage::ShaderBuffer>,
    parameters: camera_projection::Parameters,
}
impl Sink<'_> {
    fn publish(&mut self) -> Result<(), String> {
        let camera = self
            .scene
            .validation_camera
            .as_ref()
            .ok_or("Delphi render camera absent")?;
        let clip = camera.clip_from_bevy();
        if !clip.is_finite() {
            return Err("Delphi render clip is nonfinite".into());
        }
        if !self.buffers.contains(self.view.buffer.id())
            || !self.buffers.contains(self.view.source_buffer.id())
        {
            return Err("Delphi source-view upload resources absent".into());
        }
        {
            let mut buffer = self.buffers.get_mut(&self.view.buffer).unwrap();
            buffer.clear();
            buffer.extend_from_slice(&[clip.to_cols_array()]);
        }
        let mut source = self.buffers.get_mut(&self.view.source_buffer).unwrap();
        source.clear();
        source.extend_from_slice(&[crate::source_view::source_clip(clip).to_cols_array()]);
        Ok(())
    }
}
impl player_camera::Sink for Sink<'_> {
    fn projection_parameters(&mut self) -> Result<camera_projection::Parameters, String> {
        let camera = self
            .scene
            .validation_camera
            .as_ref()
            .ok_or("Delphi live viewport absent")?;
        self.parameters.viewport = [camera.viewport.z as f32, camera.viewport.w as f32];
        Ok(self.parameters)
    }
    fn projection(&mut self, output: camera_projection::Output) -> Result<(), String> {
        self.scene
            .validation_camera
            .as_mut()
            .ok_or("Delphi projection consumer absent")?
            .projection = output.matrix;
        self.publish()
    }
    fn view(&mut self, _: ActorId, output: &mecc_camera_update::Output) -> Result<(), String> {
        self.scene
            .validation_camera
            .as_mut()
            .ok_or("Delphi view consumer absent")?
            .view = output.view;
        self.scene.camera_origin = Some(output.viewport_origin);
        self.parameters.depth_reference = output.viewport_origin;
        self.parameters.depth_axis = output.viewport_forward;
        self.publish()
    }
}
impl Runtime {
    /// Consume an actual scene and source-view buffers. Projection/view requests
    /// reach Bevy's upload resources; GPU/render acceptance remains separate.
    pub(crate) fn enter_delphi_scene(
        &mut self,
        scene: &mut crate::world_scene::Scene,
        view: &crate::source_view::SourceView,
        buffers: &mut Assets<bevy::render::storage::ShaderBuffer>,
    ) -> Result<bool, String> {
        let Some(active) = self.active.as_mut() else {
            return Ok(false);
        };
        if !active
            .payload
            .delphi_entry
            .as_ref()
            .is_some_and(|state| state.pending)
        {
            return Ok(false);
        }
        let world = active.payload.world.as_ref().map_err(Clone::clone)?;
        if !Arc::ptr_eq(world, &scene.asset)
            || scene.validation_camera.is_none()
            || scene.camera_origin.is_none()
            || !buffers.contains(view.buffer.id())
            || !buffers.contains(view.source_buffer.id())
        {
            return Ok(false);
        }
        let Some(mut state) = active.payload.delphi_entry.take() else {
            return Ok(false);
        };
        if !state.pending {
            active.payload.delphi_entry = Some(state);
            return Ok(false);
        }
        let result = (|| -> Result<bool, String> {
            let camera = scene
                .validation_camera
                .as_ref()
                .ok_or("Delphi initial render camera unavailable")?;
            let origin = scene
                .camera_origin
                .ok_or("Delphi initial render origin unavailable")?;
            let parameters = camera_projection::Parameters {
                aspect: None,
                viewport: [camera.viewport.z as f32, camera.viewport.w as f32],
                // The common renderer's existing inspection clipping policy.
                near: 1.,
                far: 25000.,
                limit_far: true,
                depth_reference: origin,
                depth_axis: [camera.view[2], camera.view[6], camera.view[10]],
            };
            let world = active.payload.world.as_ref().map_err(Clone::clone)?.clone();
            let query = giants_game::terrain_queries::StaticTerrainQuery::new(
                &world.terrain,
                1. / world.terrain.spacing,
                giants_game::arithmetic::Precision::Double53,
            )
            .map_err(str::to_owned)?;
            let mut camera_world =
                giants_game::mecc_camera_wall::World::new(&query, 1, &state.objects);
            let mut sink = Sink {
                scene,
                view,
                buffers,
                parameters,
            };
            let mut camera = player_camera::Normal::resume(
                player_camera::Components {
                    modes: &mut state.modes,
                    owners: &mut state.owners,
                    pose: &mut state.pose,
                    catalog: &state.catalog,
                },
                &mut camera_world,
                &mut sink,
                player_camera::Controls {
                    // Initial component invocation, no simulated world time step.
                    seconds: 0.,
                    transition_input: false,
                    skip_floor: false,
                    reference: -1,
                },
                state
                    .sequence
                    .take()
                    .ok_or("Delphi camera sequence ownership absent")?,
            );
            state.pending = false;
            let mut blocked = Vec::new();
            let (mut session, bindings) = construct_residents(
                active.ticket,
                &mut active.payload,
                &mut blocked,
                Some(&mut camera),
            );
            let result = (|| -> Result<(), String> {
                if bindings.delphi_projection
                    != Some(giants_game::delphi_player::ProjectionStatus::Completed)
                    || active.payload.delphi_body.is_none()
                {
                    return Err(blocked
                        .last()
                        .cloned()
                        .unwrap_or_else(|| "Delphi ordered entry incomplete".into()));
                }
                camera.advance(
                    &mut session,
                    bindings.player.ok_or("Delphi source owner absent")?,
                )?;
                Ok(())
            })();
            state.sequence = Some(camera.into_sequence());
            active.resident_states = session;
            active.bindings = bindings;
            active.reported = false;
            active
                .blocked
                .retain(|b| b != "Delphi entry is waiting for the scene camera");
            active.blocked.extend(blocked);
            result?;
            sink.scene.camera_source = "delphi_source_entry_inspection";
            println!(
                "mission_delphi_scene_entry ticket={} source_view_upload=true source_terrain=true prepared_meshes={} same_owner=true primary=false controls=false ready=false",
                active.ticket,
                state.objects.len()
            );
            Ok(true)
        })();
        if let Err(error) = &result {
            state.pending = false;
            state.failure = Some(error.clone());
            active.blocked.push(format!("Delphi scene entry: {error}"));
        }
        active.payload.delphi_entry = Some(state);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{ecs::world::CommandQueue, render::storage::ShaderBuffer};

    #[test]
    #[ignore = "Requires GIANTS_DELPHI_ROOT and GIANTS_DELPHI_EXECUTABLE; source menu/environment/upload integration, no GPU/input claim"]
    fn source_delphi_scene_entry_preserves_owner_and_reenters() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_DELPHI_ROOT").unwrap());
        let source = giants_game::world_source::resolve(&root, "Rstory1").unwrap();
        let menu = giants_game::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
            u32::MAX,
            true,
        )
        .unwrap();
        let asset = Arc::new(giants_game::load_world(&root, "Rstory1").unwrap());
        let mut ecs = World::new();
        ecs.init_resource::<Assets<ShaderBuffer>>();
        let view = crate::source_view::SourceView::from_world(&mut ecs);
        let mut buffers = ecs.remove_resource::<Assets<ShaderBuffer>>().unwrap();
        let mut scene = crate::world_scene::Scene {
            asset: asset.clone(),
            oblique: false,
            validation_camera: Some(crate::validation_camera::ValidationCamera {
                view: Mat4::IDENTITY.to_cols_array(),
                projection: Mat4::IDENTITY.to_cols_array(),
                viewport: UVec4::new(0, 0, 960, 540),
            }),
            camera_source: "test_initial_inspection",
            camera_origin: Some([0.; 3]),
            cinematic_capture_ready: false,
            no_fog: false,
            render_terrain: true,
            render_sky: true,
            render_underside: false,
            reflection_profile: false,
            water: None,
            render_water: false,
        };
        let mut runtime = Runtime::default();
        let baseline = ecs.query::<Entity>().iter(&ecs).count();
        assert!(
            !runtime
                .enter_delphi_scene(&mut scene, &view, &mut buffers)
                .unwrap()
        );
        for ticket in [1201, 1202, 1203] {
            let packet = Arc::new(
                giants_game::mission_loading::prepare(&root, menu.request("Rstory1", 0).unwrap())
                    .unwrap(),
            );
            let plan = packet.instance_plan(&root, "English").unwrap();
            let factory = prepare_delphi(&root, &plan, &packet).unwrap().unwrap();
            let start = factory.declared_start().clone();
            let entry = State::prepare(&root, &plan, &packet).unwrap();
            let mut payload = super::super::tests::payload();
            payload.world = Ok(asset.clone());
            payload.packet = packet;
            payload.plan = plan;
            payload.delphi = Ok(Some(factory));
            payload.start_candidate = Ok(start);
            payload.delphi_entry = Some(entry);
            let mut queue = CommandQueue::default();
            let (scene_root, actors, blocked, residents, bindings) =
                spawn(&mut Commands::new(&mut queue, &ecs), ticket, &mut payload);
            queue.apply(&mut ecs);
            assert_eq!(residents.counts().residents, 0);
            assert!(bindings.player.is_none());
            runtime.active = Some(Active {
                ticket,
                root: scene_root,
                actors,
                blocked,
                payload,
                resident_states: residents,
                start: StartState {
                    intent: LaunchIntent::SavedMission,
                    progress: Progress::load(None, None).unwrap(),
                },
                reported: false,
                pickup: None,
                variant_random: giants_game::audio_parameters::Random(1),
                cinematic_epoch: Default::default(),
                audio_registry: None,
                bindings,
                controlled_mecc_owner: None,
                owned_mecc: None,
                landing: None,
                campaign: None,
            });
            let candidate = runtime.environment_candidate().unwrap().unwrap();
            assert!(Arc::ptr_eq(&candidate.asset, &scene.asset));
            scene.validation_camera = Some(candidate.camera);
            scene.camera_origin = None;
            assert!(
                !runtime
                    .enter_delphi_scene(&mut scene, &view, &mut buffers)
                    .unwrap()
            );
            assert!(
                runtime
                    .active
                    .as_ref()
                    .unwrap()
                    .payload
                    .delphi_entry
                    .as_ref()
                    .unwrap()
                    .pending
            );
            scene.camera_origin = Some(candidate.eye);
            assert!(
                !runtime
                    .enter_delphi_scene(&mut scene, &view, &mut Assets::default())
                    .unwrap()
            );
            if ticket == 1203 {
                scene.validation_camera.as_mut().unwrap().viewport.z = 0;
                assert!(
                    runtime
                        .enter_delphi_scene(&mut scene, &view, &mut buffers)
                        .is_err()
                );
                assert!(
                    runtime
                        .active
                        .as_ref()
                        .unwrap()
                        .payload
                        .delphi_entry
                        .as_ref()
                        .unwrap()
                        .failure
                        .is_some()
                );
                assert!(
                    !runtime
                        .enter_delphi_scene(&mut scene, &view, &mut buffers)
                        .unwrap()
                );
                let mut queue = CommandQueue::default();
                clear(&mut runtime, &mut Commands::new(&mut queue, &ecs));
                queue.apply(&mut ecs);
                assert_eq!(ecs.query::<Entity>().iter(&ecs).count(), baseline);
                continue;
            }
            assert!(
                runtime
                    .enter_delphi_scene(&mut scene, &view, &mut buffers)
                    .unwrap()
            );
            let active = runtime.active.as_ref().unwrap();
            let owner = active.player_owner().unwrap();
            let actor = active.resident_states.get(owner).unwrap();
            assert_eq!(actor.class, 3);
            assert_eq!(
                actor.construction_fields.as_ref().unwrap().dispatch_context,
                1
            );
            assert_eq!(
                active.bindings.delphi_projection,
                Some(giants_game::delphi_player::ProjectionStatus::Completed)
            );
            assert!(
                active
                    .payload
                    .delphi_body
                    .as_ref()
                    .unwrap()
                    .current(&active.resident_states)
            );
            assert_eq!(scene.camera_source, "delphi_source_entry_inspection");
            let clip = scene.validation_camera.as_ref().unwrap().clip_from_bevy();
            for (handle, expected) in [
                (&view.buffer, clip),
                (&view.source_buffer, crate::source_view::source_clip(clip)),
            ] {
                let bevy::render::storage::ShaderBufferData::Initialized { data, .. } =
                    &buffers.get(handle).unwrap().data
                else {
                    panic!("camera upload absent")
                };
                let expected: Vec<u8> = expected
                    .to_cols_array()
                    .into_iter()
                    .flat_map(f32::to_ne_bytes)
                    .collect();
                assert_eq!(data.as_slice(), expected);
            }
            assert_ne!(
                scene.validation_camera.as_ref().unwrap().view,
                Mat4::IDENTITY.to_cols_array()
            );
            assert!(
                !runtime
                    .enter_delphi_scene(&mut scene, &view, &mut buffers)
                    .unwrap()
            );
            assert!(runtime.startup().is_err());
            let mut queue = CommandQueue::default();
            clear(&mut runtime, &mut Commands::new(&mut queue, &ecs));
            queue.apply(&mut ecs);
            assert!(runtime.active.is_none());
            assert_eq!(ecs.query::<Entity>().iter(&ecs).count(), baseline);
        }
    }
}
