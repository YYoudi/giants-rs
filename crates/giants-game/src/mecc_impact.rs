//! Ordinary Mecc impact classification at the acquired secondary boundary.
//! Authority, damage execution, positional context preparation and channels are
//! separate ordered consumers. A classification is not a completed reaction.
use giants_formats::global_data::GlobalData;

/// Current offline damage gates supplied by their actual mission owner. This
/// does not authorize a network profile or stand in for the death service.
#[derive(Clone, Copy)]
pub struct Context {
    pub offline: bool,
    pub damage: crate::resident_states::MeccDamageContext,
}

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub threshold: f32,
    pub heavy_threshold: f32,
    pub damage_scale: f32,
    pub damage_base: f32,
}
impl Parameters {
    pub fn from_globals(globals: &GlobalData) -> Result<Self, String> {
        let get = |at| globals.scalar(at).map_err(|e| e.to_string());
        Ok(Self {
            threshold: get(0x66c)?,
            heavy_threshold: get(0x670)?,
            damage_scale: get(0xc00)?,
            damage_base: get(0xc04)?,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Classification {
    /// Stored single-precision delta length, distinct from its initial wide test.
    pub delta: f32,
    /// Number of acquired inclusive threshold tests that passed: zero, one, two.
    pub severity: u8,
    /// Requested kind-16 amount. Authority/parent and damage gates still apply.
    pub amount: Option<f32>,
}
pub fn classify(
    current: [f32; 3],
    previous: [f32; 3],
    parameters: Parameters,
) -> Result<Classification, String> {
    if !current
        .into_iter()
        .chain(previous)
        .chain([
            parameters.threshold,
            parameters.heavy_threshold,
            parameters.damage_scale,
            parameters.damage_base,
        ])
        .all(f32::is_finite)
    {
        return Err("impact classification requires finite velocities and parameters".into());
    }
    let difference =
        std::array::from_fn::<_, 3, _>(|i| (f64::from(current[i]) - f64::from(previous[i])) as f32);
    if !difference.into_iter().all(f32::is_finite) {
        return Err("impact velocity difference exceeds acquired finite profile".into());
    }
    let length = |v: [f32; 3]| {
        let [x, y, z] = v.map(f64::from);
        ((x * x + y * y) + z * z).sqrt()
    };
    let wide_delta = length(difference);
    let delta = wide_delta as f32;
    let mut severity = 0;
    // The previous speed is explicitly stored before the current wide return
    // is compared. Replacing both operands with wide lengths changes boundaries.
    if wide_delta >= f64::from(parameters.threshold)
        && length(current) < f64::from(length(previous) as f32)
    {
        severity =
            u8::from(delta >= parameters.heavy_threshold) + u8::from(delta >= parameters.threshold);
    }
    let amount = (severity != 0).then(|| {
        ((f64::from(delta) - f64::from(parameters.threshold)) * f64::from(parameters.damage_scale)
            + f64::from(parameters.damage_base)) as f32
    });
    if !delta.is_finite() || amount.is_some_and(|v| !v.is_finite()) {
        return Err("impact result exceeds acquired finite profile".into());
    }
    Ok(Classification {
        delta,
        severity,
        amount,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parameters() -> Parameters {
        Parameters {
            threshold: 10.,
            heavy_threshold: 20.,
            damage_scale: 0.5,
            damage_base: 1.,
        }
    }
    #[test]
    fn inclusive_thresholds_require_reduced_stored_speed() {
        for (previous, severity, amount) in [(9., 0, None), (10., 1, Some(1.)), (20., 2, Some(6.))]
        {
            let result = classify([0.; 3], [previous, 0., 0.], parameters()).unwrap();
            assert_eq!(result.severity, severity);
            assert_eq!(result.amount, amount);
        }
        // Equal mathematical speeds can still pass the original comparison:
        // sqrt(35) rounds upward when the previous length is stored as f32.
        assert_eq!(
            classify([-5., 3., 1.], [5., 3., 1.], parameters())
                .unwrap()
                .severity,
            1
        );
        assert_eq!(
            classify([12., 0., 0.], [0.; 3], parameters())
                .unwrap()
                .severity,
            0
        );
        assert!(classify([f32::NAN, 0., 0.], [0.; 3], parameters()).is_err());
        assert!(classify([f32::MAX, 0., 0.], [-f32::MAX, 0., 0.], parameters()).is_err());
    }
}
