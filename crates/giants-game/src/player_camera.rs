//! Ordered local normal-camera binding for the generic player and its Delphi
//! replacement. The caller must establish the direct-angle, ordinary-world
//! branch without authored overrides, shake, forced view or scaled input.
//! Global initialization and ordinary player admission remain separate.
//! Reuses the confirmed `SetupCameraDataNormal` / `chase_value_time` kernels
//! and the probable `camera_get_curcamdef` selector relationship; this binding
//! establishes no additional prototype PS2 routine identity.
use crate::{
    camera_mode, camera_owner, camera_profile, camera_projection, delphi_player,
    mecc_camera_update, player_shell,
    resident_states::{Id, Session},
};

/// Borrow the actual persistent owners; retyping must not replace these states.
/// Their initialization/reset provenance belongs to the enclosing lifecycle.
pub struct Components<'a> {
    pub modes: &'a mut camera_mode::State,
    pub owners: &'a mut camera_owner::State<Id>,
    pub pose: &'a mut mecc_camera_update::State,
    pub catalog: &'a camera_profile::ExecutableCatalog,
}
#[derive(Clone, Copy)]
pub struct Controls {
    pub seconds: f32,
    pub transition_input: bool,
    pub skip_floor: bool,
    /// Only the acquired negative-reference local branch is admitted here.
    pub reference: i32,
}
/// Required consumers. A completed CPU matrix is not device publication.
pub trait Sink {
    /// Read current viewport/depth caches on every submission. The preceding
    /// view may have changed them before type activation or the next frame.
    fn projection_parameters(&mut self) -> Result<camera_projection::Parameters, String>;
    fn projection(&mut self, output: camera_projection::Output) -> Result<(), String>;
    fn view(&mut self, owner: Id, output: &mecc_camera_update::Output) -> Result<(), String>;
}
pub struct Normal<'a, W, S> {
    parts: Components<'a>,
    world: &'a mut W,
    sink: &'a mut S,
    controls: Controls,
    last_view_owner: Option<Id>,
    activated: Option<Id>,
    projection: Option<camera_projection::Output>,
    failure: Option<String>,
}
impl<'a, W: mecc_camera_update::World, S: Sink> Normal<'a, W, S> {
    pub fn new(
        parts: Components<'a>,
        world: &'a mut W,
        sink: &'a mut S,
        controls: Controls,
    ) -> Self {
        Self {
            parts,
            world,
            sink,
            controls,
            last_view_owner: None,
            activated: None,
            projection: None,
            failure: None,
        }
    }
    pub fn set_controls(&mut self, controls: Controls) {
        self.controls = controls;
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    /// Retained even when the required sink or a later world service fails.
    pub fn projection(&self) -> Option<&camera_projection::Output> {
        self.projection.as_ref()
    }
    pub fn output(&self) -> Option<&mecc_camera_update::Output> {
        self.parts.pose.output()
    }
    /// Identity of the last successfully consumed view, including after a later
    /// failure. The caller must retain the ticket/lifetime and failure checks.
    pub fn view_owner(&self) -> Option<Id> {
        self.last_view_owner
    }
    fn check(&self) -> Result<(), String> {
        if self.failure.is_some()
            || self.parts.modes.failure().is_some()
            || self.parts.pose.failure().is_some()
        {
            return Err("normal player camera is quarantined".into());
        }
        if self.controls.reference >= 0 {
            return Err("normal player camera requires its negative-reference branch".into());
        }
        Ok(())
    }
    fn retain<T>(&mut self, result: Result<T, String>) -> Result<T, String> {
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    fn fields<'s>(
        &self,
        session: &'s Session,
        owner: Id,
    ) -> Result<&'s crate::actor_fields::Fields, String> {
        let actor = session
            .get(owner)
            .ok_or("normal camera actor absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("normal camera fields absent")?;
        if !matches!((actor.class, actor.seed.storage.state()), (0, 7) | (3, 3))
            || fields.dispatch_context != 1
            || fields.object_flags & 0x8000 != 0
            || crate::camera_profile::World::parent(session, owner)?.is_some()
        {
            return Err("normal camera requires its local generic/Delphi owner".into());
        }
        if actor.class == 0 {
            session.check_player_shell(owner)?;
        }
        Ok(fields)
    }
    /// Run a following normal camera invocation on these same persistent states.
    /// Real floor/mesh providers and projection/view consumers are mandatory.
    pub fn advance(&mut self, session: &mut Session, controlled: Id) -> Result<(), String> {
        self.check()?;
        let result = self.advance_inner(session, controlled);
        self.retain(result)
    }
    fn advance_inner(&mut self, session: &mut Session, controlled: Id) -> Result<(), String> {
        let fields = self.fields(session, controlled)?;
        let class = fields.class;
        if self.parts.modes.mode() != 0
            || self.parts.modes.view() != 0
            || (class == 3
                && (self.activated != Some(controlled)
                    || session.delphi_special(controlled).is_none()
                    || fields.configuration[14] & 1 == 0))
        {
            return Err(
                "normal camera requires its acquired mode and ordered type projection".into(),
            );
        }
        self.parts.owners.select_from_session(
            session,
            camera_owner::Input {
                controlled: Some(controlled),
                reference: self.controls.reference,
                override_enabled: 0,
                override_owner: None,
            },
            &mut |_, _, _| Err("normal camera reference provider outside acquired branch".into()),
        )?;
        let selected = camera_profile::from_session(
            session,
            self.parts.owners,
            self.parts.modes,
            Some(controlled),
            false,
        )?;
        let record = self.parts.catalog.record(selected)?;
        let fov = self.parts.modes.advance_lens(crate::camera_lens::Input {
            profile_fov: record.camera.vertical_fov,
            override_fov: None,
            seconds: self.controls.seconds,
            inhibited: false,
            transition_input: self.controls.transition_input,
            scaled_input: false,
            mode: self.parts.modes.mode() as i32,
            profile_index: self.parts.modes.view(),
            selected_kind: Some(session.get(controlled).unwrap().seed.storage.state() as i32),
            input_scale: 1.,
        })?;
        let projection = camera_projection::build(fov, self.sink.projection_parameters()?)
            .map_err(str::to_owned)?;
        self.projection = Some(projection);
        self.sink.projection(projection)?;
        let fields = self.fields(session, controlled)?;
        let number = self
            .parts
            .pose
            .output()
            .map_or(Some(1), |v| v.number.checked_add(1))
            .ok_or("normal camera invocation exhausted")?;
        let input = mecc_camera_update::Input {
            number,
            position: fields.position,
            yaw: fields.angles[0],
            pitch: session.camera_look_angle(controlled)?,
            look_height: f32::from_bits(fields.configuration[22]),
            scale: fields.scale,
            skip_floor: self.controls.skip_floor || fields.object_flags & 0x10 != 0,
            seconds: self.controls.seconds,
            profile: record.camera,
        };
        let flags = fields.control_flags;
        let output = if class == 3 {
            self.parts
                .pose
                .advance_delphi_third_person(input, flags, self.world)?
        } else {
            self.parts.pose.advance(input, self.world)?
        };
        self.sink.view(controlled, &output)?;
        self.last_view_owner = Some(controlled);
        Ok(())
    }
}
impl<W: mecc_camera_update::World, S: Sink> player_shell::Camera for Normal<'_, W, S> {
    fn change_view(
        &mut self,
        session: &mut Session,
        controlled: Id,
        request: i32,
    ) -> Result<(), String> {
        self.check()?;
        let result = session.change_camera_mode(
            self.parts.modes,
            self.parts.owners,
            camera_mode::Command {
                request,
                extended: false,
                controlled: Some(controlled),
                scaled_input: false,
            },
            self.parts.catalog,
            &mut |fov| {
                let output = camera_projection::build(fov, self.sink.projection_parameters()?)
                    .map_err(str::to_owned)?;
                self.projection = Some(output);
                self.sink.projection(output)
            },
        );
        self.retain(result)
    }
    fn update_camera(&mut self, session: &mut Session, controlled: Id) -> Result<(), String> {
        self.advance(session, controlled)
    }
}
impl<W: mecc_camera_update::World, S: Sink> delphi_player::ProjectionReset for Normal<'_, W, S> {
    fn reset_projection(&mut self, session: &Session, controlled: Id) -> Result<(), String> {
        self.check()?;
        let result = (|| {
            self.fields(session, controlled)?;
            if session.get(controlled).unwrap().class != 3
                || self.last_view_owner != Some(controlled)
                || self.activated.is_some()
                || self.parts.owners.owner_from_session(session)? != Some(controlled)
            {
                return Err(
                    "type projection requires the same completed generic camera owner".into(),
                );
            }
            struct Reset<'a, S> {
                session: &'a Session,
                controlled: Id,
                owner: Id,
                catalog: &'a camera_profile::ExecutableCatalog,
                retained: &'a mut Option<camera_projection::Output>,
                sink: &'a mut S,
            }
            impl<S: Sink> camera_mode::Services<()> for Reset<'_, S> {
                fn profile(&mut self, _: u32, view: u32) -> Result<camera_mode::Profile, String> {
                    camera_profile::select(
                        camera_profile::Input {
                            owner: Some(self.owner),
                            controlled: Some(self.controlled),
                            view,
                            scaled_input: false,
                        },
                        self.session,
                    )?
                    .resolve(self.catalog)
                }
                fn scale(&mut self, _: (), _: f32) -> Result<(), String> {
                    Err("type projection cannot write owner look".into())
                }
                fn projection(&mut self, fov: f32) -> Result<(), String> {
                    let output = camera_projection::build(fov, self.sink.projection_parameters()?)
                        .map_err(str::to_owned)?;
                    *self.retained = Some(output);
                    self.sink.projection(output)
                }
            }
            self.parts.modes.reset_projection(&mut Reset {
                session,
                controlled,
                owner: controlled,
                catalog: self.parts.catalog,
                retained: &mut self.projection,
                sink: self.sink,
            })?;
            self.activated = Some(controlled);
            Ok(())
        })();
        self.retain(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct World {
        fail_height: bool,
        height_calls: usize,
        clip_calls: usize,
    }
    impl mecc_camera_update::World for World {
        fn height(&mut self, _: [f32; 2]) -> Result<f32, String> {
            self.height_calls += 1;
            if self.fail_height {
                Err("supplied terrain failure".into())
            } else {
                Ok(0.)
            }
        }
        fn clip(&mut self, _: &mut [f32; 3], _: [f32; 3]) -> Result<(), String> {
            self.clip_calls += 1;
            Ok(()) // Explicit empty-mesh component fixture.
        }
    }
    struct Capture {
        owner: Id,
        parameters: camera_projection::Parameters,
        projections: Vec<camera_projection::Output>,
        views: Vec<mecc_camera_update::Output>,
        parameter_reads: Vec<camera_projection::Parameters>,
        fail_projection: Option<usize>,
        fail_view: bool,
    }
    impl Sink for Capture {
        fn projection_parameters(&mut self) -> Result<camera_projection::Parameters, String> {
            self.parameter_reads.push(self.parameters);
            Ok(self.parameters)
        }
        fn projection(&mut self, output: camera_projection::Output) -> Result<(), String> {
            self.projections.push(output);
            if self.fail_projection == Some(self.projections.len()) {
                Err("supplied projection transport failure".into())
            } else {
                Ok(())
            }
        }
        fn view(&mut self, owner: Id, output: &mecc_camera_update::Output) -> Result<(), String> {
            assert_eq!(owner, self.owner);
            self.views.push(output.clone());
            self.parameters.depth_reference = output.viewport_origin;
            self.parameters.depth_axis = output.viewport_forward;
            if self.fail_view {
                Err("supplied view transport failure".into())
            } else {
                Ok(())
            }
        }
    }
    #[test]
    #[ignore = "Requires GIANTS_DELPHI_ROOT/EXECUTABLE; real source resources with supplied normal context, pose, clock, flat/empty world and matrix sinks; no ordinary input/GPU"]
    fn source_delphi_camera_keeps_generic_owner_history_and_live_projection_caches() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
        let executable =
            std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_EXECUTABLE").unwrap());
        let resources =
            crate::scene_resources::SceneResources::load(&root, "w_Rstory1.bin", "English")
                .unwrap();
        let globals =
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
                .unwrap();
        let weapons = giants_formats::weapons::parse_with_profile(
            &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
            giants_formats::weapons::Profile::Debug,
        )
        .unwrap();
        let factory = crate::delphi_special::Factory::load(&resources, &globals, &weapons).unwrap();
        let shell = player_shell::Factory::load(&resources).unwrap();
        let catalog = camera_profile::ExecutableCatalog::read(&executable).unwrap();
        for case in 0..4 {
            let mut session = Session::new(3540 + case);
            let mut entry = shell.construct(&mut session).unwrap();
            let owner = entry.owner();
            let mut modes = camera_mode::State::new(0, 1, 0.75).unwrap();
            let mut owners = camera_owner::State::new(None, 0);
            // Supplied acquired fresh-pose-cache boundary; the enclosing global
            // reset/initialization writer is not provided by this fixture.
            let mut pose = mecc_camera_update::State::default();
            let mut world = World {
                fail_height: case == 3,
                height_calls: 0,
                clip_calls: 0,
            };
            let mut sink = Capture {
                owner,
                parameters: camera_projection::Parameters {
                    aspect: None,
                    viewport: [960., 540.],
                    near: 1.,
                    far: 10_000.,
                    limit_far: true,
                    depth_reference: [0.; 3],
                    depth_axis: [0.; 3],
                },
                projections: Vec::new(),
                views: Vec::new(),
                parameter_reads: Vec::new(),
                fail_projection: (case == 1).then_some(3),
                fail_view: case == 2,
            };
            {
                let mut camera = Normal::new(
                    Components {
                        modes: &mut modes,
                        owners: &mut owners,
                        pose: &mut pose,
                        catalog: &catalog,
                    },
                    &mut world,
                    &mut sink,
                    Controls {
                        seconds: 1. / 60.,
                        transition_input: false,
                        skip_floor: false,
                        reference: -1,
                    },
                );
                let entered = entry.initialize(
                    &mut session,
                    player_shell::WorldPose {
                        position: [7., 8., 9.],
                        heading: 32.,
                    },
                    &mut camera,
                );
                if case >= 2 {
                    assert!(entered.is_err());
                    assert_eq!(entry.checkpoint(), player_shell::Checkpoint::PoseApplied);
                    assert!(camera.projection().is_some());
                    assert!(camera.failure().is_some());
                    assert_eq!(camera.output().is_some(), case == 2);
                    assert!(camera.advance(&mut session, owner).is_err());
                    assert!(
                        entry
                            .initialize(
                                &mut session,
                                player_shell::WorldPose {
                                    position: [7., 8., 9.],
                                    heading: 32.,
                                },
                                &mut camera
                            )
                            .is_err()
                    );
                } else {
                    entered.unwrap();
                    assert_eq!(entry.checkpoint(), player_shell::Checkpoint::CameraApplied);
                    let generic = camera.output().unwrap().clone();
                    assert_eq!(generic.number, 1);
                    assert_eq!(camera.view_owner(), Some(owner));
                    let attempt = factory.retype_player_shell_with_projection(
                        &mut session,
                        owner,
                        crate::animation_channels::Blending::Disabled,
                        &mut camera,
                    );
                    assert_eq!(attempt.owner, Some(owner));
                    if case == 1 {
                        assert_eq!(attempt.projection, delphi_player::ProjectionStatus::Failed);
                        assert!(
                            attempt
                                .result
                                .unwrap_err()
                                .contains("projection transport failure")
                        );
                        assert!(session.delphi_special(owner).is_none());
                        assert!(camera.failure().is_some());
                        assert_eq!(camera.output().unwrap().number, 1);
                        assert!(camera.advance(&mut session, owner).is_err());
                    } else {
                        let initialized = attempt.result.unwrap();
                        assert_eq!(initialized.owner, owner);
                        assert!(!initialized.camera_projection_reset_required);
                        assert_eq!(camera.output().unwrap().number, 1);
                        camera.advance(&mut session, owner).unwrap();
                        let next = camera.output().unwrap();
                        assert_eq!(next.number, 2);
                        let selected = camera_profile::from_session(
                            &session,
                            camera.parts.owners,
                            camera.parts.modes,
                            Some(owner),
                            false,
                        )
                        .unwrap();
                        let target = catalog.record(selected).unwrap().camera.distance;
                        assert_ne!(generic.chase[0], target);
                        assert_ne!(
                            next.chase[0], target,
                            "the generic chase must not reset on retype"
                        );
                        assert!(camera.failure().is_none());
                        let counts = session.counts();
                        assert!(
                            factory
                                .retype_player_shell_with_projection(
                                    &mut session,
                                    owner,
                                    crate::animation_channels::Blending::Disabled,
                                    &mut camera,
                                )
                                .result
                                .is_err()
                        );
                        assert_eq!(session.counts(), counts);
                        assert_eq!(camera.output().unwrap().number, 2);
                        session.clear();
                        assert!(camera.advance(&mut session, owner).is_err());
                        assert_eq!(camera.output().unwrap().number, 2);
                    }
                }
            }
            assert_eq!(owners.owner(), Some(owner));
            assert_eq!(
                sink.projections.len(),
                if case < 2 {
                    if case == 0 { 4 } else { 3 }
                } else {
                    2
                }
            );
            assert_eq!(sink.parameter_reads.len(), sink.projections.len());
            if case < 2 {
                assert_eq!(
                    sink.parameter_reads[2].depth_reference,
                    sink.views[0].viewport_origin
                );
                assert_eq!(
                    sink.parameter_reads[2].depth_axis,
                    sink.views[0].viewport_forward
                );
            }
            assert_eq!(
                sink.views.len(),
                match case {
                    0 => 2,
                    1 | 2 => 1,
                    _ => 0,
                }
            );
            assert!(world.height_calls > 0);
            session.clear();
            assert!(session.get(owner).is_none());
            let fresh = shell.construct(&mut session).unwrap();
            assert_ne!(fresh.owner(), owner);
        }
    }
}
