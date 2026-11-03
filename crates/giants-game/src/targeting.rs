//! Measured initial direction and no-enemy circular target rule.
//! Target selection, player controls and the state callback envelope are separate.
pub fn initial_direction(angles: [f32; 2]) -> Result<[f32; 4], &'static str> {
    if angles
        .into_iter()
        .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
    {
        return Err("initial direction angles outside measured profile");
    }
    let (sine, cosine) = crate::object_transform::sincos(angles[0]);
    let (elevation, level) = crate::object_transform::sincos(angles[1]);
    Ok([
        (sine * level) as f32,
        (-cosine * level) as f32,
        elevation as f32,
        0.,
    ])
}
/// Explicit simulation time and already-selected local delta; this assigns no
/// normal/benchmark clock policy. The native rule caps only the upper factor.
pub fn idle(
    previous: [f32; 4],
    position: [f32; 3],
    phase_seconds: f32,
    local_delta: f64,
    response: f32,
) -> Result<[f32; 4], &'static str> {
    if !previous
        .into_iter()
        .chain(position)
        .chain([phase_seconds, response])
        .all(f32::is_finite)
        || !local_delta.is_finite()
        || phase_seconds.abs() > 1_000_000.
    {
        return Err("target input outside finite measured profile");
    }
    let phase = f64::from(phase_seconds) * 2.;
    let desired = [
        (phase.sin() * 50. + f64::from(position[0])) as f32,
        (phase.cos() * 50. + f64::from(position[1])) as f32,
        position[2],
    ];
    let factor = (local_delta * f64::from(response)).min(1.);
    let mut out = [0.; 4];
    for i in 0..3 {
        out[i] = ((f64::from(desired[i]) - f64::from(previous[i])) * factor
            + f64::from(previous[i])) as f32;
    }
    out[3] = 1.;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_response_preserves_xyz_and_sets_point_weight() {
        let previous = [1., 2., 3., 0.];
        assert_eq!(
            idle(previous, [100.; 3], 0., 1. / 60., 0.).unwrap(),
            [1., 2., 3., 1.]
        );
    }
    #[test]
    fn invalid_inputs_are_not_silently_converted_to_a_direction() {
        assert!(initial_direction([f32::NAN, 0.]).is_err());
        assert!(initial_direction([0., 1e8]).is_err());
        assert!(idle([0.; 4], [0.; 3], f32::INFINITY, 0., 1.).is_err());
        assert!(idle([0.; 4], [0.; 3], 0., f64::NAN, 1.).is_err());
    }
}
