//! State23 enemy-shot launch calculation. This does not construct an actor,
//! resolve a muzzle reference, submit FX/audio, advance state6 or apply damage.
use crate::audio_parameters::Random;

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub muzzle: [f32; 3],
    pub target: [f32; 3],
    /// Source projectile class speed, not the callback's third argument.
    pub forward_speed: f32,
    pub gravity: f32,
    /// The callback argument scales vertical-rate variation as a percentage.
    pub vertical_spread_percent: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Launch {
    pub flight_seconds: f32,
    pub vertical_rate: f32,
    pub forward_rate: f32,
    /// Copied to both state6 lifetime fields by the native launch writer.
    pub lifetime: f32,
}
/// The acquired helper selects the large family for class230, small for73.
/// Other callers require their own acquisition, not an implicit default.
pub fn resource_class(actor_class: i32) -> Result<i32, &'static str> {
    match actor_class {
        73 => Ok(90),
        230 => Ok(232),
        _ => Err("enemy shot caller outside acquired state23 class profile"),
    }
}
pub fn launch(p: Parameters, random: &mut Random) -> Result<Launch, &'static str> {
    if !p
        .muzzle
        .into_iter()
        .chain(p.target)
        .chain([p.forward_speed, p.gravity, p.vertical_spread_percent])
        .all(f32::is_finite)
        || p.forward_speed <= 0.
    {
        return Err("enemy ballistic launch requires finite fields and positive source speed");
    }
    let dx = f64::from(p.target[0]) - f64::from(p.muzzle[0]);
    let dy = f64::from(p.target[1]) - f64::from(p.muzzle[1]);
    // Native uses horizontal distance, then spills this quotient before gravity.
    let seconds = ((dx * dx + dy * dy).sqrt() / f64::from(p.forward_speed)) as f32;
    if !seconds.is_finite() || seconds <= 0. {
        return Err("enemy ballistic flight time outside finite nonzero profile");
    }
    let time = f64::from(seconds);
    let height = f64::from(p.target[2]) - f64::from(p.muzzle[2]);
    let vertical = (height + ((time * f64::from(p.gravity)) * time) * 0.5) / time;
    // Keep the source f32 percentage constant and independent endpoint spills.
    let variation = f64::from(p.vertical_spread_percent) * f64::from(0.01_f32);
    let high = ((1. + variation) * vertical) as f32;
    let low = ((1. - variation) * vertical) as f32;
    let lifetime = (time + 1.) as f32;
    if ![high, low, lifetime].into_iter().all(f32::is_finite) {
        return Err("enemy ballistic interval exceeds finite launch profile");
    }
    // Unlike other interval helpers, the endpoint difference is not spilled.
    // Negative vertical rates can reverse the endpoints; preserve their order.
    let vertical_rate = ((f64::from(random.next_15()) * (f64::from(high) - f64::from(low)))
        * (1. / 32768.)
        + f64::from(low)) as f32;
    Ok(Launch {
        flight_seconds: seconds,
        vertical_rate,
        forward_rate: p.forward_speed,
        lifetime,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parameters() -> Parameters {
        Parameters {
            muzzle: [0., 0., 0.],
            target: [30., 40., 20.],
            forward_speed: 50.,
            gravity: 10.,
            vertical_spread_percent: 0.,
        }
    }
    #[test]
    fn horizontal_time_gravity_and_lifetime_have_separate_roles() {
        let mut random = Random(1);
        let a = launch(parameters(), &mut random).unwrap();
        assert_eq!(
            [
                a.flight_seconds,
                a.vertical_rate,
                a.forward_rate,
                a.lifetime
            ],
            [1., 25., 50., 2.]
        );
        assert_eq!(random.0, 2745024); // A zero-width interval still consumes RNG.
    }
    #[test]
    fn spread_argument_changes_variation_not_forward_speed() {
        let mut p = parameters();
        p.vertical_spread_percent = 10.;
        let a = launch(p, &mut Random(1)).unwrap();
        assert_eq!(a.forward_rate, 50.);
        assert!(a.vertical_rate >= 22.5 && a.vertical_rate <= 27.5);
    }
    #[test]
    fn negative_vertical_intervals_retain_native_endpoint_order() {
        let mut p = parameters();
        p.target[2] = -20.;
        p.gravity = 0.;
        p.vertical_spread_percent = 50.;
        let a = launch(p, &mut Random(1)).unwrap();
        // First random value is near zero, so native interpolation starts near
        // the lower multiplier (-10), not a numerically sorted bound (-30).
        assert!(a.vertical_rate < -10. && a.vertical_rate > -11.);
    }
    #[test]
    fn invalid_flight_refuses_before_random_is_consumed() {
        let mut random = Random(17);
        let mut p = parameters();
        p.target = [0., 0., 100.];
        assert!(launch(p, &mut random).is_err());
        p = parameters();
        p.forward_speed = 0.;
        assert!(launch(p, &mut random).is_err());
        p = parameters();
        p.vertical_spread_percent = f32::NAN;
        assert!(launch(p, &mut random).is_err());
        assert_eq!(random.0, 17);
    }
}
