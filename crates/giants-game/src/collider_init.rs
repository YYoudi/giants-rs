//! Late collider initialization after template/active copies. Ground and owned
//! footprint ownership remain external; no AI/state callback or list insertion.
use crate::{arithmetic::Precision, collider_copy::Fields, collision_radius};
#[derive(Clone, Copy, Debug)]
pub struct Instance {
    pub fields: Fields,
    pub state: u32,
    pub usage: [u32; 2],
    /// Independent owned-footprint identity, never a native pointer.
    pub footprint: u32,
}
pub struct Output {
    pub instance: Instance,
    pub initialized: bool,
}
/// Collision-field projections, not owned AI payloads or a ready world object.
pub struct ColdProjection {
    pub template: Instance,
    pub initial: Instance,
}
#[derive(Clone, Copy, Debug)]
pub struct DeclaredPose {
    pub class: u32,
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch_roll: Option<[f32; 2]>,
}
impl ColdProjection {
    /// Fresh field copy followed by the declared pose stores. Scale modifiers,
    /// world registration and callbacks remain separate stages.
    pub fn placed(&self, pose: DeclaredPose) -> Result<Instance, &'static str> {
        if pose.class != self.template.fields.class {
            return Err("placement class does not match cold projection");
        }
        if !pose
            .position
            .into_iter()
            .chain([pose.yaw])
            .chain(pose.pitch_roll.into_iter().flatten())
            .all(f32::is_finite)
        {
            return Err("non-finite declared collider pose");
        }
        let mut fields = Fields::zeroed();
        fields.copy_from(&self.template.fields);
        fields.current = pose.position;
        fields.previous = pose.position;
        fields.angles[0] = pose.yaw;
        if let Some([pitch, roll]) = pose.pitch_roll {
            fields.angles[1] = pitch;
            fields.angles[2] = roll;
        }
        Ok(Instance {
            fields,
            state: self.template.state,
            usage: self.template.usage,
            footprint: 0,
        })
    }
}
pub fn cold_projection(
    resources: &crate::scene_resources::SceneResources,
    class: i32,
) -> Result<ColdProjection, Box<dyn std::error::Error>> {
    let fields = Fields::cold_declaration(resources, class)?;
    let definition = resources
        .classes
        .models
        .first_program(class)
        .ok_or("collider class absent")?;
    let program = definition.program.as_ref().map_err(Clone::clone)?;
    let state = u32::from(
        crate::state_storage::Seed::from_program(program)?
            .storage
            .state(),
    );
    let mut usage = [0; 2];
    for command in &program.records {
        match command.opcode {
            0x4f => usage[1] |= 0xf,
            0x50 => usage[1] |= 0xf00,
            _ => {}
        }
    }
    let template = Instance {
        fields,
        state,
        usage,
        footprint: 0,
    };
    let mut copied = Fields::zeroed();
    copied.copy_from(&fields);
    Ok(ColdProjection {
        template,
        initial: Instance {
            fields: copied,
            state,
            usage,
            footprint: 0,
        },
    })
}
pub struct GeometryResources {
    /// Radius calculation reads animation slot 1, independently of the active slot.
    pub radius: giants_formats::world_bounds::Resolved,
    pub current_slot: u32,
    pub current_auxiliary: Option<[f32; 6]>,
}
#[derive(Clone, Copy, Debug)]
pub enum SourceTransform {
    Cached([f32; 12]),
    Uncached,
}

/// Explicit composition of late initialization followed by the bounds writer.
/// This API supplies that ordering; it does not establish native list scheduling.
pub fn initialize_geometry(
    instance: Instance,
    resources: GeometryResources,
    transform: SourceTransform,
    precision: Precision,
    ground: impl FnMut([f32; 3]) -> Result<(f32, [f32; 4]), &'static str>,
) -> Result<Output, &'static str> {
    if resources.current_slot != instance.fields.animation_slot {
        return Err("collider bounds slot does not match instance");
    }
    if (instance.fields.object_flags & 0x20 != 0) != matches!(transform, SourceTransform::Cached(_))
    {
        return Err("collider transform does not match source cache flag");
    }
    let primary = resources.radius.primary;
    let mut output = initialize(instance, resources.radius, precision, ground)?;
    let fields = &mut output.instance.fields;
    let models = (fields.model != 0).then_some(crate::collision_bounds::Models {
        primary,
        auxiliary: resources.current_auxiliary,
        slot: resources.current_slot,
    });
    fields.bounds = match transform {
        SourceTransform::Cached(matrix) => {
            crate::collision_bounds::cached(models, matrix, precision)?
        }
        SourceTransform::Uncached => crate::collision_bounds::uncached(
            models,
            crate::uncached_pose::Pose {
                position: fields.current,
                angles: fields.angles,
                scale: fields.scale,
            },
            precision,
        )?,
    };
    Ok(output)
}
pub fn initialize(
    mut instance: Instance,
    bounds: giants_formats::world_bounds::Resolved,
    precision: Precision,
    mut ground: impl FnMut([f32; 3]) -> Result<(f32, [f32; 4]), &'static str>,
) -> Result<Output, &'static str> {
    let fields = &mut instance.fields;
    if fields.response_flags & 0x8000_0000 == 0 || fields.model == 0 {
        return Ok(Output {
            instance,
            initialized: false,
        });
    }
    let class = fields.class;
    let state = instance.state;
    if state != 78 {
        let masks = if matches!(class, 2 | 3) {
            [0x3fc0, 0xff]
        } else if matches!(class, 11 | 742 | 584 | 215 | 8 | 229 | 73 | 230 | 1080) {
            [0x3c03fc0, 0xff]
        } else if class == 677 {
            [1, 0xf0]
        } else if matches!(state, 61 | 98)
            || matches!(
                class,
                585 | 1061
                    | 1062
                    | 1185
                    | 1186
                    | 1188
                    | 1187
                    | 60
                    | 562
                    | 571
                    | 730
                    | 735
                    | 736
                    | 1148
                    | 1149
                    | 1150
                    | 1151
                    | 1152
                    | 1154
                    | 1156
                    | 1158
                    | 989
                    | 1181
                    | 1189
            )
        {
            [0, 0xf0f]
        } else if class == 745 {
            [0, 0xf00]
        } else if matches!(class, 102 | 101) {
            [0x3ffc, 0]
        } else if matches!(
            class,
            78 | 91
                | 546
                | 549
                | 520
                | 507
                | 509
                | 513
                | 514
                | 515
                | 516
                | 680
                | 681
                | 687
                | 531
                | 544
        ) {
            [0x3c03c00, 0]
        } else {
            [0, 0]
        };
        for (value, mask) in instance.usage.iter_mut().zip(masks) {
            *value |= mask;
        }
        if matches!(state, 60 | 61) && fields.collision_flags & 0x40 == 0 {
            fields.collision_flags |= 0x8000_0000;
        }
    }
    instance.footprint = 0;
    fields.radius = collision_radius::calculate(
        collision_radius::Input {
            primary: bounds.primary,
            auxiliary: bounds.animation,
            scale: fields.scale,
            radius_override: fields.radius_override,
            limit: fields.radius_limit,
        },
        precision,
    )?;
    let (floor, plane) = ground(fields.current)?;
    fields.floor = floor;
    fields.plane = plane;
    fields.response_flags &= 0x7fff_ffff;
    Ok(Output {
        instance,
        initialized: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_bounds_and_contradictory_cache_are_rejected_before_ground_callbacks() {
        let mut fields = Fields::zeroed();
        fields.model = 7;
        fields.animation_slot = 1;
        fields.object_flags = 0x20;
        let instance = Instance {
            fields,
            state: 0,
            usage: [0; 2],
            footprint: 0,
        };
        let resources = |slot| GeometryResources {
            radius: giants_formats::world_bounds::Resolved {
                primary: [0.; 6],
                animation: None,
            },
            current_slot: slot,
            current_auxiliary: None,
        };
        let no_ground = |_| -> Result<(f32, [f32; 4]), &'static str> {
            panic!("invalid preparation must not query ground")
        };
        assert!(
            initialize_geometry(
                instance,
                resources(0),
                SourceTransform::Cached([0.; 12]),
                Precision::Double53,
                no_ground
            )
            .is_err()
        );
        assert!(
            initialize_geometry(
                instance,
                resources(1),
                SourceTransform::Uncached,
                Precision::Double53,
                no_ground
            )
            .is_err()
        );
        let mut uncached = instance;
        uncached.fields.object_flags = 0;
        assert!(
            initialize_geometry(
                uncached,
                resources(1),
                SourceTransform::Cached([0.; 12]),
                Precision::Double53,
                no_ground
            )
            .is_err()
        );
    }
}
