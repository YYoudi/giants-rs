//! Owned pre-heading camera callback; callers still supply actual world context.
use super::*;
impl Session {
    /// Ordinary third-person/direct-angle camera after a completed controlled
    /// frame. Profile, global time and world providers remain supplied boundaries;
    /// this does not execute the full camera dispatcher or acknowledge rendering.
    pub fn advance_delphi_ordinary_camera(
        &self,
        id: Id,
        frame: u64,
        seconds: f32,
        profile: crate::mecc_camera_update::Profile,
        state: &mut crate::mecc_camera_update::State,
        world: &mut impl crate::mecc_camera_update::World,
    ) -> Result<crate::mecc_camera_update::Output, String> {
        let (input, flags) = self.delphi_ordinary_pose_input(id, frame, seconds, profile)?;
        state.advance_delphi_third_person(input, flags, world)
    }

    /// Bound local ordinary mode-zero phase: profile selection, lens,
    /// projection, then pose. The caller excludes authored/alternate camera
    /// routes and supplies actual global input, viewport/depth and world data.
    /// This does not publish a device view or admit a mission player.
    pub fn advance_delphi_ordinary_view(
        &self,
        id: Id,
        frame: u64,
        controls: crate::delphi_ordinary_view::Controls,
        profiles: crate::delphi_camera::OrdinaryProfiles,
        state: &mut crate::delphi_ordinary_view::State,
        world: &mut impl crate::mecc_camera_update::World,
    ) -> Result<crate::delphi_ordinary_view::Output, String> {
        let (input, control_flags) = self.delphi_ordinary_pose_input(
            id,
            frame,
            controls.seconds,
            profiles.select(controls.scaled_input),
        )?;
        let fields = self.get(id).unwrap().construction_fields.as_ref().unwrap();
        if fields.configuration[20] != 3 || fields.object_flags & 0x8000 != 0 {
            return Err("Delphi ordinary view requires its acquired selected kind/profile".into());
        }
        state.advance(
            crate::delphi_ordinary_view::Input {
                number: input.number,
                position: input.position,
                yaw: input.yaw,
                pitch: input.pitch,
                look_height: input.look_height,
                scale: input.scale,
                skip_floor: input.skip_floor,
                control_flags,
                controls,
            },
            profiles,
            world,
        )
    }

    fn delphi_ordinary_pose_input(
        &self,
        id: Id,
        frame: u64,
        seconds: f32,
        profile: crate::mecc_camera_update::Profile,
    ) -> Result<(crate::mecc_camera_update::Input, u32), String> {
        let actor = self
            .get(id)
            .ok_or("Delphi camera owner absent or foreign")?;
        let cycle = actor
            .delphi_physical
            .as_ref()
            .ok_or("Delphi ordinary camera requires its controlled frame")?;
        if actor.class != 3
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 3
            || !cycle.resources.manual_rotation
            || cycle.view.number != frame
            || cycle.view.failure.is_some()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
        {
            return Err(
                "Delphi ordinary camera requires its completed ordinary controlled frame".into(),
            );
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Delphi camera fields absent")?;
        Ok((
            crate::mecc_camera_update::Input {
                number: frame,
                position: fields.position,
                yaw: fields.angles[0],
                pitch: self.camera_look_angle(id)?,
                look_height: f32::from_bits(fields.configuration[22]),
                scale: fields.scale,
                skip_floor: fields.object_flags & 0x10 != 0,
                seconds,
                profile,
            },
            fields.control_flags,
        ))
    }
    pub fn advance_delphi_camera(
        &self,
        id: Id,
        environment: crate::delphi_camera::Environment,
        state: &mut crate::delphi_camera::State,
        stamp: &mut crate::animation_epoch::Stamp,
    ) -> Result<(), String> {
        let actor = self
            .get(id)
            .ok_or("Delphi camera owner absent or foreign")?;
        if actor.class != 3 {
            return Err("Delphi camera requires its owned class3 actor".into());
        }
        if state.failure.is_some() {
            return Err("Delphi camera producer is quarantined".into());
        }
        if !environment.local_owner || environment.mode != -1 {
            return Ok(());
        }
        if environment.buttons & 0x8000 == 0 {
            state.clear_override()?;
            state.vertical_fov = None;
            return Ok(());
        }
        let mut produce = || -> Result<crate::authored_camera_reference::Output, String> {
            let source = environment.target.unwrap_or(id);
            let current = self
                .get(source)
                .and_then(|a| a.construction_fields.as_ref())
                .ok_or("Delphi camera target absent or foreign")?
                .channels[0][0];
            let current =
                u8::try_from(current).map_err(|_| "camera target clip outside source table")?;
            let output = if environment.target.is_some() {
                self.sample_reference_camera(source, current, 0, environment.aspect, stamp)?
            } else {
                match self.sample_reference_camera(id, current, 10, environment.aspect, stamp)? {
                    Some(value) => Some(value),
                    None => self.sample_reference_camera(id, 1, 10, environment.aspect, stamp)?,
                }
            };
            output.ok_or_else(|| "required Delphi camera reference producer absent".into())
        };
        match produce() {
            Ok(output) => {
                state.select_matrix(output.view)?;
                state.vertical_fov = output.vertical_fov;
                Ok(())
            }
            Err(error) => {
                // Preserve the last published view and any completed shared
                // epoch consumption. Refuse replay after this late failure.
                state.failure = Some(error.clone());
                Err(error)
            }
        }
    }
}
