//! Completed-frame terrain/candidate geometry, including the acquired two view
//! routes. Camera snapshots remain explicit; this does not publish readiness,
//! movement links or native world scheduling.
use crate::{
    delphi_target_owned::{Input, Output},
    first_person::Camera,
    resident_states::{Id, Session},
};

#[derive(Clone, Copy)]
pub struct View {
    /// Three actual selector words, in source order. Their broader roles are
    /// not inferred from the predicate that consumes them.
    pub selectors: [i32; 3],
    pub camera: Camera,
    /// Current viewport origin/basis, separate from the angle-derived terrain ray.
    pub viewport_origin: [f32; 3],
    pub viewport_forward: [f32; 3],
}
impl View {
    pub fn uses_viewport(self) -> bool {
        self.selectors[0] == 0 && self.selectors[1] == 0 && self.selectors[2] <= 1
    }
}
pub struct ScanInput {
    pub owner: Id,
    pub number: u64,
    pub normalization_threshold: f32,
    pub vertex_epoch: u32,
    pub force_vertices: bool,
    pub view: View,
}
pub struct HeldTerrainInput {
    pub owner: Id,
    pub number: u64,
    pub normalization_threshold: f32,
    pub range: f32,
}
pub struct ScanOutput {
    pub range: f32,
    pub terrain: crate::mecc_aim::Ray,
    pub query: Output,
    /// Final geometry point. This is distinct from the candidate ray's end and
    /// from the sampled/refined distance used for candidate clipping.
    pub aim_point: [f32; 3],
    pub viewport: bool,
}
impl Session {
    /// Actual static terrain provider for the running caller's held route.
    /// Point writes on a miss and the separate sampled distance are retained.
    /// Viewport selection, ripple and the no-sample native locals stay unbound.
    pub fn trace_delphi_prephysical_held_terrain(
        &self,
        input: HeldTerrainInput,
        terrain: &crate::terrain_queries::StaticTerrainQuery<'_>,
        point: &mut [f32; 3],
    ) -> Result<
        (
            crate::delphi_target_ray::Source,
            crate::delphi_local_target::Terrain,
        ),
        String,
    > {
        let sample = self.delphi_prephysical_target_ray(
            input.owner,
            input.number,
            input.normalization_threshold,
        )?;
        let trace = crate::mecc_aim::terrain_trace(
            terrain,
            sample.ray.origin,
            sample.ray.direction,
            input.range,
        )?;
        *point = trace
            .written_point
            .ok_or("Delphi terrain output without a native sample is unacquired")?;
        Ok((
            sample.source,
            crate::delphi_local_target::Terrain {
                hit: trace.ray.point.is_some(),
                distance: trace.ray.distance,
            },
        ))
    }
    pub fn scan_delphi_target(
        &mut self,
        resources: &crate::scene_resources::SceneResources,
        definitions: &crate::projectile_definition::DebugDefinitions,
        input: ScanInput,
        terrain: &crate::terrain_queries::StaticTerrainQuery<'_>,
    ) -> Result<ScanOutput, String> {
        self.check_delphi_target_frame(input.owner, input.number, input.vertex_epoch)?;
        let payload = self
            .get(input.owner)
            .and_then(|a| a.seed.storage.data())
            .ok_or("Delphi range payload absent")?;
        let word = |at| -> Result<i32, String> {
            Ok(i32::from_le_bytes(
                payload
                    .get(at..at + 4)
                    .ok_or("Delphi range payload truncated")?
                    .try_into()
                    .unwrap(),
            ))
        };
        let range = crate::delphi_target_range::resolve(
            word(0xdb4)?,
            [word(0xd78)?, word(0xd80)?, word(0xd88)?, word(0xd90)?],
            definitions,
        )?;
        let viewport = input.view.uses_viewport();
        let held = if viewport {
            None
        } else {
            Some(self.delphi_target_ray(
                input.owner,
                input.number,
                input.normalization_threshold,
            )?)
        };
        let (origin, direction) = match held {
            Some(ray) => (ray.origin, ray.direction),
            None => (
                input.view.camera.position,
                crate::mecc_aim::direction(input.view.camera.angles)?,
            ),
        };
        let terrain = crate::mecc_aim::terrain_ray(terrain, origin, direction, range)?;
        // The native caller uses the sampled/refined distance even on a miss,
        // including its half-cell overshoot. Do not clamp to declaration range.
        let ray = match held {
            Some(ray) => ray,
            None => crate::delphi_target_ray::Ray {
                origin: input.view.viewport_origin,
                direction: input.view.viewport_forward,
            },
        };
        let query = self.query_delphi_target_with_ray(
            resources,
            Input {
                owner: input.owner,
                number: input.number,
                normalization_threshold: input.normalization_threshold,
                range: terrain.distance,
                vertex_epoch: input.vertex_epoch,
                force_vertices: input.force_vertices,
            },
            ray,
        )?;
        let aim_point = if query.target.is_some() {
            query.point
        } else if let Some(point) = terrain.point {
            point
        } else {
            std::array::from_fn(|i| {
                (f64::from(input.view.viewport_forward[i]) * 1000.
                    + f64::from(input.view.camera.position[i])) as f32
            })
        };
        if !aim_point.into_iter().all(f32::is_finite) {
            return Err("Delphi target fallback outside finite source profile".into());
        }
        Ok(ScanOutput {
            range,
            terrain,
            query,
            aim_point,
            viewport,
        })
    }
}
