//! Acquired size pursuit within the primary. Target decisions and complete
//! primary/world scheduling are separate producers.
use crate::object_time_precision;

pub struct Frame<'a> {
    /// Output of an upstream growth decision, not a keyboard or feeding rule.
    pub target: f32,
    pub global_seconds: f32,
    pub areas: &'a [object_time_precision::Area],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub current: f32,
    /// The source invokes its size refresh whenever the incoming sizes differ,
    /// including a zero timestep that leaves the current size unchanged.
    pub refreshed: bool,
}

/// Preserve the wide actor-local timestep through the acquired Double53 step.
/// Equal sizes bypass time consumption and preserve the current word's sign.
pub fn approach(current: f32, target: f32, seconds: f64) -> Result<View, &'static str> {
    if !current.is_finite() || !target.is_finite() {
        return Err("Kabuto growth requires finite current and target sizes");
    }
    if current == target {
        return Ok(View {
            current,
            refreshed: false,
        });
    }
    if !seconds.is_finite() || seconds < 0. {
        return Err("Kabuto growth requires a finite nonnegative timestep");
    }
    let step = seconds * f64::from(0.4_f32);
    let value = if current < target {
        f64::from(current) + step
    } else {
        f64::from(current) - step
    };
    let stored = value as f32;
    let current = if (current < target && value > f64::from(target))
        || (current > target && value < f64::from(target))
    {
        target
    } else {
        stored
    };
    Ok(View {
        current,
        refreshed: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pursuit_clamps_both_directions_and_refreshes_a_zero_step() {
        assert_eq!(approach(0., 1., 10.).unwrap().current, 1.);
        assert_eq!(approach(3., 1., 10.).unwrap().current, 1.);
        assert_eq!(
            approach(3., 1., 0.).unwrap(),
            View {
                current: 3.,
                refreshed: true
            }
        );
        assert!(approach(0., 1., -1.).is_err());
    }
    #[test]
    fn equal_sizes_bypass_time_and_keep_signed_zero() {
        let result = approach(-0., 0., f64::NAN).unwrap();
        assert!(!result.refreshed);
        assert_eq!(result.current.to_bits(), (-0f32).to_bits());
        assert!(approach(f32::NAN, 0., 0.).is_err());
    }
}
