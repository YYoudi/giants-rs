use super::*;

impl Session {
    /// Cold class0/state7 replacement only: retain the existing actor and list
    /// registrations while binding its acquired Delphi resources. Camera/world
    /// startup completion is a separate caller obligation.
    pub(crate) fn retype_player_shell_as_delphi(
        &mut self,
        owner: Id,
        input: Construction<'_>,
    ) -> Result<(), String> {
        self.check_player_shell(owner)?;
        let old = self.get(owner).unwrap();
        let fields = old.construction_fields.as_ref().unwrap();
        if old.constructor_mask != 0x11
            || old.environment.is_some()
            || old.ordinary_motion.is_some()
            || self.state_resources.contains_key(&owner.serial)
            || !matches!(fields.dispatch_context, 0 | 1)
            || fields.object_flags != 0
            || fields.response_flags != 0x80000000
            || fields.control_flags != 0
            || fields.action != 0
            || input.class != 3
            || input.seed.storage.profile() != Profile::Debug
            || input.seed.storage.state() != 3
            || !input.seed.diagnostics.is_empty()
            || input.fields.class != 3
            || input.fields.dispatch_context != fields.dispatch_context
            || input.fields.position != fields.position
            || input.fields.current != fields.current
            || input.fields.previous != fields.previous
            || input.fields.object_flags != input.flags.active
            || input.fields.configuration[14] != input.flags.personality
        {
            return Err("Delphi retyping requires its unchanged fresh generic player shell".into());
        }
        // Stage every fallible resource binding without allocating another live
        // actor or advancing the world's next identity. Existing registrations
        // stay in place through projection/history/special initialization.
        let mut staged = Self::new(self.ticket);
        staged.next = owner
            .serial
            .checked_sub(1)
            .ok_or("invalid shell identity")?;
        let id =
            staged.register_with_mask(3, input.seed, input.bindings, input.flags, 0x80000000)?;
        if id != owner {
            return Err("staged shell identity differs".into());
        }
        staged.bind_specification(id, input.specification)?;
        staged.bind_state_resources(id, input.state_resources)?;
        let channels = input.fields.channels;
        staged.bind_actor_fields(id, input.fields)?;
        let actor = staged.residents.get_mut(&id.serial).unwrap();
        actor.channels = Some(channels);
        actor.acquired_construction = true;
        actor.constructor_mask = 0x11;
        self.residents.insert(
            owner.serial,
            staged.residents.remove(&owner.serial).unwrap(),
        );
        self.specifications.insert(
            owner.serial,
            staged.specifications.remove(&owner.serial).unwrap(),
        );
        self.state_resources.insert(
            owner.serial,
            staged.state_resources.remove(&owner.serial).unwrap(),
        );
        Ok(())
    }
    pub(crate) fn construct_player_shell(
        &mut self,
        seed: &Seed,
        flags: DirectFlags,
        fields: crate::actor_fields::Fields,
    ) -> Result<Id, String> {
        if seed.storage.profile() != Profile::Debug
            || seed.storage.state() != 7
            || seed.storage.data().is_some()
            || !seed.unresolved.is_empty()
            || !seed.diagnostics.is_empty()
            || fields.class != 0
            || fields.configuration[20] != 7
            || fields.configuration[19] != 0
            || fields.configuration[29] != 0
            || fields.configuration[18] != 0
            || !fields.gunlist.is_empty()
            || fields.shadow.is_some()
            || fields.object_flags != flags.active
            || fields.configuration[14] != flags.personality
            || fields.response_flags & 0x80000000 == 0
        {
            return Err("player shell requires its acquired unmodeled class0 resources".into());
        }
        let mut staged = Self::new(self.ticket);
        staged.next = self.next;
        let owner = staged.register_with_mask(0, seed, &[], flags, 0x11)?;
        let actor = staged.residents.get_mut(&owner.serial).unwrap();
        actor.channels = Some(fields.channels);
        actor.construction_fields = Some(fields);
        actor.camera_look_angle = Some(0.);
        actor.acquired_construction = true;
        // This real source declaration has neither a body specification nor
        // state resources. Keep the ordinary publisher's resource requirements
        // intact; preflight this shell's registrations before publishing it.
        self.links
            .construct(owner.serial, 0, 7, flags, actor.constructor_mask)?;
        self.next = staged.next;
        self.residents.insert(
            owner.serial,
            staged.residents.remove(&owner.serial).unwrap(),
        );
        Ok(owner)
    }
    pub(crate) fn check_player_shell(&self, owner: Id) -> Result<(), String> {
        let actor = self
            .get(owner)
            .ok_or("player shell owner absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("player shell fields absent")?;
        if actor.class != 0
            || !actor.acquired_construction
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 7
            || actor.seed.storage.data().is_some()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || self.specification(owner).is_some()
            || actor.affine_cache.is_some()
            || actor.mecc_animation_stamp != 0
            || actor.channels != Some(fields.channels)
        {
            return Err("player shell outside acquired unmodeled pre-camera profile".into());
        }
        Ok(())
    }
    pub(crate) fn publish_player_shell_pose(
        &mut self,
        owner: Id,
        pose: crate::player_shell::WorldPose,
    ) -> Result<(), String> {
        self.check_player_shell(owner)?;
        let fields = self
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap();
        fields.dispatch_context = 1;
        fields.position = pose.position;
        fields.current = pose.position;
        fields.previous = pose.position;
        fields.angles[0] = pose.heading;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{camera_mode, camera_owner, camera_profile, player_shell};
    #[test]
    #[ignore = "Requires GIANTS_DELPHI_ROOT; acquired cold shell/type component, supplied pose and projection sink; no complete camera/startup or gameplay"]
    fn source_delphi_shell_retype_retains_identity_context_and_registration_nodes() {
        use crate::delphi_player::{ProjectionReset, ProjectionStatus};
        let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
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
        let shell = player_shell::Factory::load(&resources).unwrap();
        let factory = crate::delphi_special::Factory::load(&resources, &globals, &weapons).unwrap();
        struct Projection {
            owner: Id,
            next: u64,
            memberships: Vec<(u64, u32)>,
            calls: usize,
            fail: bool,
        }
        impl ProjectionReset for Projection {
            fn reset_projection(&mut self, session: &Session, owner: Id) -> Result<(), String> {
                self.calls += 1;
                assert_eq!(owner, self.owner);
                assert_eq!(session.next, self.next);
                assert_eq!(session.links.memberships(owner.serial), self.memberships);
                let actor = session.get(owner).unwrap();
                assert_eq!(actor.class, 3);
                assert_eq!(actor.seed.storage.state(), 3);
                let fields = actor.construction_fields.as_ref().unwrap();
                assert_eq!(fields.dispatch_context, 1);
                assert_eq!(fields.position, [7., 8., 9.]);
                assert_eq!(fields.angles, [32., 0., 0.]);
                assert!(!session.list(3).contains(&owner));
                assert!(session.delphi_special(owner).is_none());
                if self.fail {
                    Err("supplied projection sink failure".into())
                } else {
                    Ok(())
                }
            }
        }
        for fail in [false, true] {
            let mut session = Session::new(2140 + u64::from(fail));
            let prior = shell.construct(&mut session).unwrap().owner();
            let entry = shell.construct(&mut session).unwrap();
            let owner = entry.owner();
            // Explicit supplied initialized-shell component boundary. This does
            // not run or acknowledge Entry's required persistent camera services.
            session
                .publish_player_shell_pose(
                    owner,
                    player_shell::WorldPose {
                        position: [7., 8., 9.],
                        heading: 32.,
                    },
                )
                .unwrap();
            let tail = shell.construct(&mut session).unwrap().owner();
            let mut projection = Projection {
                owner,
                next: session.next,
                memberships: session.links.memberships(owner.serial),
                calls: 0,
                fail,
            };
            let attempt = factory.retype_player_shell_with_projection(
                &mut session,
                owner,
                crate::animation_channels::Blending::Enabled,
                &mut projection,
            );
            assert_eq!(attempt.owner, Some(owner));
            assert_eq!(projection.calls, 1);
            assert_eq!(entry.checkpoint(), player_shell::Checkpoint::Constructed);
            for &(node, list) in &projection.memberships {
                assert_eq!(session.links.node_object(list, node).unwrap(), owner.serial);
            }
            if fail {
                assert_eq!(attempt.projection, ProjectionStatus::Failed);
                assert!(
                    attempt
                        .result
                        .unwrap_err()
                        .contains("projection sink failure")
                );
                assert_eq!(session.counts().residents, 3);
                assert!(session.delphi_special(owner).is_none());
                assert!(!session.list(3).contains(&owner));
            } else {
                assert_eq!(attempt.projection, ProjectionStatus::Completed);
                let result = attempt.result.unwrap();
                assert_eq!(result.owner, owner);
                assert!(!result.camera_projection_reset_required);
                assert_eq!(session.list(3).first(), Some(&owner));
                assert_eq!(
                    session.counts().residents,
                    4 + 2 * result.special.copies.len()
                );
                assert_eq!(
                    session.next,
                    projection.next + 1 + 2 * result.special.copies.len() as u64
                );
                for copy in &result.special.copies {
                    assert_eq!(session.delphi_copy_source(copy.actor).unwrap().owner, owner);
                    assert_eq!(session.wield_binding(copy.held).unwrap().parent, copy.actor);
                }
                assert_eq!(
                    session.wield_binding(result.special.held).unwrap().parent,
                    owner
                );
                for id in session.list(0) {
                    if id != prior && id != tail {
                        assert_eq!(
                            session
                                .get(id)
                                .unwrap()
                                .construction_fields
                                .as_ref()
                                .unwrap()
                                .dispatch_context,
                            1
                        );
                    }
                }
                assert_eq!(
                    session.wield_binding(result.special.held).unwrap().context,
                    1
                );
            }
            let counts = session.counts();
            let retry = factory.retype_player_shell_with_projection(
                &mut session,
                owner,
                crate::animation_channels::Blending::Enabled,
                &mut projection,
            );
            assert!(retry.result.is_err());
            assert_eq!(retry.projection, ProjectionStatus::NotReached);
            assert_eq!(projection.calls, 1);
            assert_eq!(session.counts(), counts);
            let mut foreign = Session::new(9999);
            let rejected = factory.retype_player_shell_with_projection(
                &mut foreign,
                owner,
                crate::animation_channels::Blending::Enabled,
                &mut projection,
            );
            assert!(rejected.result.is_err());
            assert!(rejected.owner.is_none());
            assert_eq!(foreign.counts().residents, 0);
            session.clear();
            assert!(session.get(owner).is_none());
            assert!(
                factory
                    .retype_player_shell_with_projection(
                        &mut session,
                        owner,
                        crate::animation_channels::Blending::Enabled,
                        &mut projection,
                    )
                    .result
                    .is_err()
            );
        }
    }
    struct Camera {
        modes: camera_mode::State,
        owners: camera_owner::State<Id>,
        catalog: camera_profile::ExecutableCatalog,
        reference: i32,
        fail_command: bool,
        projections: Vec<f32>,
        calls: Vec<&'static str>,
        pose: player_shell::WorldPose,
    }
    impl player_shell::Camera for Camera {
        fn change_view(
            &mut self,
            session: &mut Session,
            controlled: Id,
            request: i32,
        ) -> Result<(), String> {
            self.calls.push("view");
            assert_eq!(request, 1);
            let fields = session
                .get(controlled)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap();
            assert_eq!(fields.dispatch_context, 0);
            assert_eq!(fields.position, [0.; 3]);
            assert_eq!(fields.current, [0.; 3]);
            assert_eq!(fields.previous, [0.; 3]);
            session.change_camera_mode(
                &mut self.modes,
                &self.owners,
                camera_mode::Command {
                    request,
                    extended: false,
                    controlled: Some(controlled),
                    scaled_input: false,
                },
                &self.catalog,
                &mut |fov| {
                    self.projections.push(fov);
                    Ok(())
                },
            )?;
            if self.fail_command {
                return Err("supplied projection transport failed after request".into());
            }
            Ok(())
        }
        fn update_camera(&mut self, session: &mut Session, controlled: Id) -> Result<(), String> {
            self.calls.push("camera");
            let fields = session
                .get(controlled)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap();
            assert_eq!(fields.dispatch_context, 1);
            assert_eq!(fields.position, self.pose.position);
            assert_eq!(fields.current, self.pose.position);
            assert_eq!(fields.previous, self.pose.position);
            assert_eq!(fields.angles, [self.pose.heading, 0., 0.]);
            self.owners.select_from_session(
                session,
                camera_owner::Input {
                    controlled: Some(controlled),
                    reference: self.reference,
                    override_enabled: 0,
                    override_owner: None,
                },
                &mut |_, _, _| Err("required reference provider boundary".into()),
            )?;
            Err("required lens/full-camera provider boundary".into())
        }
    }
    #[test]
    #[ignore = "Requires GIANTS_DELPHI_ROOT/EXECUTABLE; source class0 shell and camera-prefix controls, no ordinary gameplay"]
    fn source_delphi_player_shell_preserves_unmodeled_identity_and_camera_order() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
        let executable =
            std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_EXECUTABLE").unwrap());
        let resources =
            crate::scene_resources::SceneResources::load(&root, "w_Rstory1.bin", "English")
                .unwrap();
        let factory = player_shell::Factory::load(&resources).unwrap();
        for case in 0..3 {
            let mut session = Session::new(2040 + case);
            let prior = (case == 1).then(|| factory.construct(&mut session).unwrap().owner());
            if let Some(prior) = prior {
                // Supplied prior-camera input, matching the native control.
                session
                    .residents
                    .get_mut(&prior.serial)
                    .unwrap()
                    .camera_look_angle = Some(13.25);
            }
            let mut entry = factory.construct(&mut session).unwrap();
            let owner = entry.owner();
            assert_ne!(prior, Some(owner));
            assert!(session.specification(owner).is_none());
            assert!(session.get(owner).unwrap().seed.storage.data().is_none());
            assert_eq!(session.get(owner).unwrap().constructor_mask, 0x11);
            assert_eq!(session.camera_look_angle(owner).unwrap(), 0.);
            assert_eq!(session.position_buffer_count(), 0);
            let mut camera = Camera {
                modes: camera_mode::State::new(if case == 1 { 2 } else { 0 }, 1, 0.75).unwrap(),
                owners: camera_owner::State::new(prior, if prior.is_some() { 5 } else { 0 }),
                catalog: camera_profile::ExecutableCatalog::read(&executable).unwrap(),
                reference: if prior.is_some() { 7 } else { -1 },
                fail_command: case == 2,
                projections: Vec::new(),
                calls: Vec::new(),
                pose: player_shell::WorldPose {
                    position: [7., 8., 9.],
                    heading: 32.,
                },
            };
            let invalid = player_shell::WorldPose {
                position: [f32::NAN, 0., 0.],
                heading: 0.,
            };
            assert!(
                entry
                    .initialize(&mut session, invalid, &mut camera)
                    .is_err()
            );
            assert!(camera.calls.is_empty());
            assert!(entry.failure().is_none());
            let error = entry
                .initialize(&mut session, camera.pose, &mut camera)
                .unwrap_err();
            assert_eq!(entry.failure(), Some(error.as_str()));
            assert_eq!(camera.projections.len(), 1);
            assert_eq!(
                (
                    camera.modes.mode(),
                    camera.modes.view(),
                    camera.modes.transition()
                ),
                (0, 0, 0.)
            );
            assert_eq!(session.camera_look_angle(owner).unwrap(), 0.);
            if case == 2 {
                assert_eq!(entry.checkpoint(), player_shell::Checkpoint::Constructed);
                assert_eq!(camera.calls, ["view"]);
                assert_eq!(
                    session
                        .get(owner)
                        .unwrap()
                        .construction_fields
                        .as_ref()
                        .unwrap()
                        .position,
                    [0.; 3]
                );
            } else {
                assert_eq!(entry.checkpoint(), player_shell::Checkpoint::PoseApplied);
                assert_eq!(camera.calls, ["view", "camera"]);
                assert_eq!(camera.owners.owner(), prior.or(Some(owner)));
                assert_eq!(
                    camera.owners.reference_status(),
                    if prior.is_some() { 5 } else { 0 }
                );
                if let Some(prior) = prior {
                    assert_eq!(session.camera_look_angle(prior).unwrap(), 0.);
                }
            }
            let counts = session.counts();
            let calls = camera.calls.len();
            assert!(
                entry
                    .initialize(&mut session, camera.pose, &mut camera)
                    .is_err()
            );
            assert_eq!(session.counts(), counts);
            assert_eq!(camera.calls.len(), calls);
            assert!(session.specification(owner).is_none());
            assert_eq!(session.position_buffer_count(), 0);
            session.clear();
            assert!(session.get(owner).is_none());
            let replacement = factory.construct(&mut session).unwrap();
            assert_ne!(replacement.owner(), owner);
        }
    }
}
