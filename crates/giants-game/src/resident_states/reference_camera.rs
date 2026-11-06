//! Reference-camera sampling over the current owned actor and its real cache.
use super::*;
impl Session {
    /// A reference query only: this neither chooses Delphi's target/fallback
    /// nor publishes a renderer camera or admits a character update.
    pub fn sample_reference_camera(
        &self,
        id: Id,
        clip: u8,
        reference: usize,
        aspect: f32,
        stamp: &mut crate::animation_epoch::Stamp,
    ) -> Result<Option<crate::authored_camera_reference::Output>, String> {
        let actor = self
            .get(id)
            .ok_or("camera reference owner absent or foreign")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("camera owner fields absent")?;
        let current_clip = u8::try_from(fields.channels[0][0])
            .map_err(|_| "camera owner clip outside source table")?;
        crate::reference_camera::sample(
            &crate::reference_camera::Source {
                specification: self
                    .specification(id)
                    .ok_or("camera specification absent")?,
                current_clip,
                current_seconds: f32::from_bits(fields.channels[0][1]),
                // The source getter consumes the existing cache, even when its
                // sampled pose is older than the actor's current body angles.
                world: actor
                    .affine_cache
                    .as_ref()
                    .map(|cache| crate::procedural_aim::Matrix {
                        flags: cache.value.flags,
                        values: cache.value.matrix,
                    }),
                references: self.reference_cache_storage(id)?,
            },
            clip,
            reference,
            aspect,
            stamp,
        )
    }
}
