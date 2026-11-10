//! Common living-primary scalar arithmetic. State dispatch, controls, growth,
//! physics and ANM remain separate; these helpers never complete a frame.
use giants_formats::global_data::GlobalData;

pub struct Tick<'a> {
    pub number: u64,
    pub global_delta: f32,
    pub slow_areas: &'a [crate::object_time::SlowArea],
}
#[derive(Clone, Copy)]
pub enum Phase {
    Regeneration,
    Movement,
}
impl Phase {
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Regeneration => 0,
            Self::Movement => 1,
        }
    }
}

/// Entry-mode policy runs before state dispatch. The special mode6 primary
/// exits before regeneration, and modes2..5 preserve accumulated damage.
pub fn regenerate(mode: u32, damage: f32, seconds: f64, rate: f32) -> Result<f32, String> {
    if mode > 9 || !damage.is_finite() || !seconds.is_finite() || seconds < 0. || !rate.is_finite()
    {
        return Err("Vimp regeneration outside finite acquired profile".into());
    }
    if damage <= 0. || matches!(mode, 2..=6) {
        return Ok(damage);
    }
    let next = f64::from(damage) - seconds * f64::from(rate);
    let stored = next as f32;
    if !stored.is_finite() {
        return Err("Vimp regeneration overflow".into());
    }
    Ok(if next > 0. { stored } else { 0. })
}

#[derive(Clone, Copy, Debug)]
pub struct Movement {
    pub speed_68: f32,
    pub parameter_6c: f32,
    pub parameter_70: f32,
    pub desired_speed: f32,
}
/// Nonzero damage uses one common parameter set, independently of gait. With
/// zero damage the three acquired gait selections have distinct parameters.
pub fn movement(
    globals: &GlobalData,
    damage: f32,
    scale: f32,
    gait: u32,
    old_speed: f32,
    seconds: f64,
) -> Result<Movement, String> {
    if ![damage, scale, old_speed].into_iter().all(f32::is_finite)
        || !seconds.is_finite()
        || seconds < 0.
    {
        return Err("Vimp movement scalars outside finite acquired profile".into());
    }
    let parameters = movement_parameters(globals, damage, scale, gait)?;
    Ok(Movement {
        speed_68: approach_speed(
            old_speed,
            parameters.desired_speed,
            seconds,
            parameters.acceleration,
        )?,
        parameter_6c: parameters.parameter_6c,
        parameter_70: parameters.parameter_70,
        desired_speed: parameters.desired_speed,
    })
}
#[derive(Clone, Copy)]
pub(crate) struct MovementParameters {
    pub desired_speed: f32,
    pub parameter_6c: f32,
    pub parameter_70: f32,
    pub acceleration: f32,
}
pub(crate) fn movement_parameters(
    globals: &GlobalData,
    damage: f32,
    scale: f32,
    gait: u32,
) -> Result<MovementParameters, String> {
    if ![damage, scale].into_iter().all(f32::is_finite) {
        return Err("Vimp movement parameters outside finite acquired profile".into());
    }
    let base = if damage != 0. {
        0x934
    } else {
        match gait {
            0 => 0x940,
            1 => 0x94c,
            2 => 0x958,
            _ => return Err("Vimp zero-damage gait has no acquired parameter producer".into()),
        }
    };
    let scalar = |at| globals.scalar(at).map_err(|e| e.to_string());
    let g = [base, base + 4, base + 8, 0x994, 0x6dc]
        .map(scalar)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    if g.iter().any(|v| !v.is_finite()) {
        return Err("Vimp movement parameters are nonfinite".into());
    }
    let factor = (1. - f64::from(scale)) * f64::from(g[3]);
    let scaled = |base: f32| (factor * f64::from(base) + f64::from(base)) as f32;
    let desired_speed = scaled(g[0]);
    let result = MovementParameters {
        desired_speed,
        parameter_6c: scaled(g[1]),
        parameter_70: scaled(g[2]),
        acceleration: g[4],
    };
    if ![
        result.desired_speed,
        result.parameter_6c,
        result.parameter_70,
    ]
    .into_iter()
    .all(f32::is_finite)
    {
        return Err("Vimp movement scalar overflow".into());
    }
    Ok(result)
}
pub(crate) fn approach_speed(
    old_speed: f32,
    desired_speed: f32,
    seconds: f64,
    acceleration: f32,
) -> Result<f32, String> {
    if !old_speed.is_finite() || !seconds.is_finite() || seconds < 0. {
        return Err("Vimp movement scalars outside finite acquired profile".into());
    }
    let speed = if desired_speed > old_speed {
        (f64::from(old_speed) + seconds * f64::from(acceleration)).min(f64::from(desired_speed))
    } else {
        (f64::from(old_speed) - seconds * f64::from(acceleration)).max(f64::from(desired_speed))
    } as f32;
    if !speed.is_finite() {
        return Err("Vimp movement scalar overflow".into());
    }
    Ok(speed)
}
