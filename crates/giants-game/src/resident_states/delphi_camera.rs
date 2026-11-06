//! Owned pre-heading camera callback; callers still supply actual world context.
use super::*;
impl Session {
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
            state.enabled = false;
            state.descriptor = None;
            state.matrix_selected = false;
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
                state.descriptor = None;
                state.matrix_selected = true;
                state.retained_view = Some(output.view);
                state.enabled = true;
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
