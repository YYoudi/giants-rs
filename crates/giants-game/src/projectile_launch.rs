//! D mode2 straight launch from an explicitly resolved current reference point.
//! Reference production, ballistic launch, homing, effects, contacts and the
//! firing/ammunition transaction remain separate acquired service boundaries.
use crate::{
    animation::{cross, normalize, simd_trig},
    procedural_aim::Matrix,
};

/// Bounds from the actual scene's independently loaded world-bounds table.
/// The historical cold launch control supplied zero auxiliary bounds; a real
/// scene may provide nonzero bounds for the same descriptor.
pub struct Bounds {
    pub(crate) specification: std::sync::Arc<crate::specification::Prepared>,
    pub(crate) models: crate::resident_environment::ResourceBounds,
}
impl Bounds {
    pub fn load(
        source: &crate::scene_resources::SceneResources,
        specification: std::sync::Arc<crate::specification::Prepared>,
    ) -> Result<Self, String> {
        if specification.class != 38 {
            return Err("launch bounds require class38 resources".into());
        }
        let resources = crate::resident_environment::Resources::load(source, &specification)?;
        let models = resources.bounds(1)?;
        Ok(Self {
            specification,
            models,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Straight {
    pub origin: [f32; 3],
    pub target: [f32; 3],
    pub matrix: Matrix,
    pub velocity: [f32; 3],
}

impl Straight {
    pub fn new(origin: [f32; 3], target: [f32; 3], speed: f32) -> Result<Self, String> {
        if !origin
            .into_iter()
            .chain(target)
            .chain([speed])
            .all(f32::is_finite)
            || speed <= 0.
        {
            return Err("straight launch requires finite points and positive speed".into());
        }
        let delta = std::array::from_fn(|i| (f64::from(target[i]) - f64::from(origin[i])) as f32);
        let forward = normalize(delta).map_err(|e| e.to_string())?;
        // The first cross consumes the unnormalized delta, not the stored
        // normalized forward; substituting it changes the binary32 rounding.
        let right = normalize(cross([0., 0., 1.], delta)).map_err(|e| e.to_string())?;
        let up = cross(forward, right);
        let basis = Matrix {
            flags: 0x11,
            values: [
                right[0], up[0], forward[0], origin[0], right[1], up[1], forward[1], origin[1],
                right[2], up[2], forward[2], origin[2], 0., 0., 0., 1.,
            ],
        };
        // The acquired D mode2 dispatch uses the shared measured approximation,
        // not scalar host trigonometry or an exact quarter turn. Its sine is
        // slightly below one, which affects both matrix and launch speed.
        let angle = -std::f32::consts::FRAC_PI_2;
        let (sine, cosine) = (simd_trig(angle, false), simd_trig(angle, true));
        let rotation = Matrix {
            flags: 1,
            values: [
                1., 0., 0., 0., 0., cosine, -sine, 0., 0., sine, cosine, 0., 0., 0., 0., 1.,
            ],
        };
        let matrix = basis.mul(rotation)?;
        let velocity = [matrix.values[1], matrix.values[5], matrix.values[9]]
            .map(|v| -(f64::from(v) * f64::from(speed)) as f32);
        if !matrix
            .values
            .into_iter()
            .chain(velocity)
            .all(f32::is_finite)
        {
            return Err("straight launch overflow outside acquired finite profile".into());
        }
        Ok(Self {
            origin,
            target,
            matrix,
            velocity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Straight;
    #[test]
    fn invalid_aim_never_publishes_a_direction() {
        assert!(Straight::new([0.; 3], [0.; 3], 600.).is_err());
        assert!(Straight::new([0.; 3], [0., 0., 1.], 600.).is_err());
        assert!(Straight::new([0.; 3], [f32::INFINITY, 1., 0.], 600.).is_err());
        assert!(Straight::new([0.; 3], [1., 0., 0.], 0.).is_err());
    }
}
