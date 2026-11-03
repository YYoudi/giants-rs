//! Measured generic radius writer. Bounds and object overrides remain explicit.
use crate::arithmetic::Precision;
pub struct Input {
    pub primary: [f32; 6],
    pub auxiliary: Option<[f32; 6]>,
    pub scale: f32,
    pub radius_override: f32,
    pub limit: f32,
}
pub fn calculate(input: Input, precision: Precision) -> Result<f32, &'static str> {
    let r = |v| precision.round(v);
    if ![input.scale, input.radius_override, input.limit]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err("non-finite radius input");
    }
    let mut radius = if input.radius_override != 0. {
        r(f64::from(input.radius_override) * f64::from(input.scale)) as f32
    } else {
        let (b, factor) = input
            .auxiliary
            .map_or((input.primary, 0.5_f32), |b| (b, 0.85_f32));
        if !b.iter().all(|v| v.is_finite()) {
            return Err("non-finite radius bounds");
        }
        let mut x = -b[0];
        if x < b[3] {
            x = b[3];
        }
        let mut y = -b[1];
        if y < b[4] {
            y = b[4];
        }
        let xy = r(r(f64::from(y) * f64::from(y)) + r(f64::from(x) * f64::from(x)));
        r(r(r(xy.sqrt()) * f64::from(factor)) * f64::from(input.scale)) as f32
    };
    if input.limit != 0. {
        let mut limit = r(r(f64::from(input.limit) * f64::from(input.scale)) - 1.);
        if limit < 0.5 {
            limit = 0.5;
        }
        if limit < f64::from(radius) {
            radius = limit as f32;
        }
    }
    if !radius.is_finite() {
        return Err("radius overflow");
    }
    Ok(radius)
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fields {
    pub radius_override: f32,
    pub limit: f32,
    pub byte_1f7: u8,
}
impl Fields {
    /// Generic template initialization, before class commands and live callbacks.
    pub fn cold() -> Self {
        Self {
            radius_override: 0.,
            limit: 0.,
            byte_1f7: 0,
        }
    }
}
/// Prepared world resource bounds; live object initialization remains separate.
pub struct ColdProfile {
    pub primary_name: String,
    pub auxiliary_name: Option<String>,
    pub primary: [f32; 6],
    pub auxiliary: Option<[f32; 6]>,
    pub auxiliary_bounds_resolved: bool,
    pub scale: f32,
    pub fields: Fields,
    pub radius: f32,
}
impl ColdProfile {
    pub fn load(
        resources: &crate::scene_resources::SceneResources,
        class: i32,
        precision: Precision,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let definition = resources
            .classes
            .models
            .first_program(class)
            .ok_or("radius class absent")?;
        let program = definition.program.as_ref().map_err(Clone::clone)?;
        let fields = project(Fields::cold(), program)?;
        let scale = program.declared_scale()?;
        let (primary_name, _) = resources
            .model_mesh_slot(class, 0)?
            .ok_or("primary radius model absent")?;
        // Animation slot 1 and visual mesh slot 1 are different namespaces.
        // World ABX overrides apply before degenerate animation bounds inherit
        // the final primary. Live object initialization/callback order is separate.
        let binding = resources
            .classes
            .models
            .first_selection(class)
            .ok_or("radius model selection absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        let auxiliary_name = binding
            .selection
            .animations
            .iter()
            .find(|a| a.slot == 1)
            .map(|a| a.name.clone());
        let resolved = resources.collision_resource_bounds(class, Some(1))?;
        let primary = resolved.primary;
        let auxiliary = resolved.animation;
        let auxiliary_bounds_resolved = true;
        let radius = calculate(
            Input {
                primary,
                auxiliary,
                scale,
                radius_override: fields.radius_override,
                limit: fields.limit,
            },
            precision,
        )?;
        Ok(Self {
            primary_name,
            auxiliary_name,
            primary,
            auxiliary,
            auxiliary_bounds_resolved,
            scale,
            fields,
            radius,
        })
    }
}
/// Project only the measured direct radius commands onto a supplied baseline.
/// Absence of a command does not establish a zero initial field.
pub fn project(
    mut fields: Fields,
    program: &giants_formats::class_program::Program,
) -> Result<Fields, &'static str> {
    for record in &program.records {
        let scalar = |at| -> Result<f32, &'static str> {
            let bytes = record
                .bytes
                .get(at..at + 4)
                .ok_or("truncated radius command")?;
            let value = f32::from_le_bytes(bytes.try_into().unwrap());
            if !value.is_finite() {
                return Err("non-finite radius declaration");
            }
            Ok(value)
        };
        match record.opcode {
            0x25 => fields.radius_override = scalar(1)?,
            0x26 => {
                fields.byte_1f7 = *record.bytes.get(1).ok_or("truncated radius byte")?;
                fields.limit = scalar(5)?;
            }
            _ => {}
        }
    }
    Ok(fields)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_radius_ignores_missing_geometry_and_limit_uses_wide_comparison() {
        let value = calculate(
            Input {
                primary: [f32::NAN; 6],
                auxiliary: None,
                scale: 2.,
                radius_override: 3.,
                limit: 2.,
            },
            Precision::Double53,
        )
        .unwrap();
        assert_eq!(value, 3.);
        assert!(
            calculate(
                Input {
                    primary: [0.; 6],
                    auxiliary: None,
                    scale: f32::INFINITY,
                    radius_override: 1.,
                    limit: 0.
                },
                Precision::Double53
            )
            .is_err()
        );
    }
}
