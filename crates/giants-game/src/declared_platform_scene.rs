//! Read-only platform queries over actual declared rigid projections. These are
//! not registered world actors, complete footprints or native callback owners.
//! Prototype PS2 search routes: collgrid_make_grid, collgrid_forcemake_grid;
//! attribution remains unknown. See docs/SYMBOL_NAVIGATION.md.
use crate::{
    arithmetic::Precision,
    footprint_builder::{self, PlatformBits},
    footprint_policy::{self, Plan},
    platform_queries::{self, PlatformSample, PreparedPlatform, UnpreparedPlatform},
};

struct Surface {
    input: UnpreparedPlatform,
    bits: Option<PlatformBits>,
    triangles: Vec<[[f32; 3]; 3]>,
}
pub struct Prepared {
    inputs: Vec<UnpreparedPlatform>,
    surfaces: Vec<Surface>,
    unresolved: Vec<UnpreparedPlatform>,
}
impl Prepared {
    pub fn load(
        plan: &crate::instances::Plan,
        terrain: &giants_formats::gti::Terrain,
    ) -> Result<Self, String> {
        let source = crate::declared_mesh_scene::prepare_platform_candidates(plan, terrain);
        if let Some(need) = source.needs.first() {
            return Err(format!("declared platform {:?}: {}", need.key, need.reason));
        }
        let mut prepared = Self {
            inputs: Vec::new(),
            surfaces: Vec::new(),
            unresolved: Vec::new(),
        };
        for (mesh, record) in source.objects.into_iter().zip(source.records) {
            let fields = record.instance.fields;
            let input = || UnpreparedPlatform {
                id: mesh.fields.id,
                object_flags: fields.object_flags,
                collision_flags: fields.collision_flags,
                bounds_xy: [
                    [mesh.fields.bounds.min[0], mesh.fields.bounds.min[1]],
                    [mesh.fields.bounds.max[0], mesh.fields.bounds.max[1]],
                ],
            };
            prepared.inputs.push(input());
            if record.instance.footprint != 0 || matches!(record.instance.state, 74 | 83) {
                // Existing ownership and the separate building query have not
                // been supplied. Preserve these real source dependencies.
                prepared.unresolved.push(input());
                continue;
            }
            let selected = footprint_policy::select(
                footprint_policy::Input {
                    existing: false,
                    object_flags: fields.object_flags,
                    collision_flags: fields.collision_flags,
                    object_kind: record.instance.state,
                    // Not read by the acquired selector for these non-building states.
                    building_query: 0,
                    zero_gate: fields.plane[2],
                    z: fields.current[2],
                    floor: fields.floor,
                    ground_offset: fields.ground_offset,
                },
                Precision::Double53,
            )?;
            let bits = match selected {
                Plan::Raster3 => footprint_builder::platform_bits(
                    input().bounds_xy,
                    &mesh.triangles,
                    Precision::Double53,
                )?,
                Plan::Absent | Plan::Bounds1 | Plan::Pair2 | Plan::Packed4 | Plan::Circle5 => None,
                Plan::Existing => return Err("existing platform ownership required".into()),
            };
            prepared.surfaces.push(Surface {
                input: input(),
                bits,
                triangles: mesh.triangles,
            });
        }
        Ok(prepared)
    }

    /// Require matching ordered query-candidate fields. This cannot
    /// erase another world's unresolved candidates or supplied platform owners.
    pub fn matches_inputs(&self, inputs: &[UnpreparedPlatform]) -> bool {
        inputs.len() == self.inputs.len()
            && inputs.iter().zip(&self.inputs).all(|(a, b)| {
                a.id == b.id
                    && a.object_flags == b.object_flags
                    && a.collision_flags == b.collision_flags
                    && a.bounds_xy == b.bounds_xy
            })
    }
    pub fn sample(&self, point: [f32; 3]) -> Result<PlatformSample, &'static str> {
        let surfaces = self
            .surfaces
            .iter()
            .map(|surface| {
                Ok(PreparedPlatform {
                    id: surface.input.id,
                    object_flags: surface.input.object_flags,
                    collision_flags: surface.input.collision_flags,
                    bounds_xy: surface.input.bounds_xy,
                    footprint_hit: match &surface.bits {
                        Some(bits) => bits.contains([point[0], point[1]], Precision::Double53)?,
                        None => false,
                    },
                    triangles: &surface.triangles,
                })
            })
            .collect::<Result<Vec<_>, &'static str>>()?;
        platform_queries::sample_with_unprepared(
            point,
            &surfaces,
            &self.unresolved,
            Precision::Double53,
        )
    }
}
