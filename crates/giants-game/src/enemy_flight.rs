//! Acquired D state6 ordinary flight arithmetic, before world/contact dispatch.
//! Audio, frame history, bounds, authority and retirement remain caller phases.
use crate::object_time::{Sample, SlowArea};
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub position: [f32; 3],
    pub angles: [f32; 3],
    pub rates: [f32; 4],
    pub previous_rates: [f32; 4],
    pub response: u32,
    pub personality: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub braking: f32,
    pub gravity: f32,
    pub pitch_rate: f32,
}
pub struct Time<'a> {
    pub seconds: f32,
    pub context: u32,
    pub areas: &'a [SlowArea],
}
impl Time<'_> {
    fn validate(&self) -> Result<(), &'static str> {
        if !self.seconds.is_finite()
            || !(0. ..=1.).contains(&self.seconds)
            || self.areas.len() > 4096
            || self.areas.iter().any(|a| {
                !a.center
                    .into_iter()
                    .chain([a.radius_squared, a.factor])
                    .all(f32::is_finite)
                    || a.radius_squared < 0.
                    || !(0. ..=100.).contains(&a.factor)
            })
        {
            return Err("enemy flight clock outside bounded nonnegative profile");
        }
        Ok(())
    }
    fn at(&self, position: [f32; 3]) -> f64 {
        crate::object_time::seconds(
            self.seconds,
            Some(Sample {
                owner: self.context,
                position,
            }),
            None,
            self.areas,
        )
    }
}
pub fn advance(mut s: State, p: Parameters, time: &Time<'_>) -> Result<State, &'static str> {
    time.validate()?;
    if !s
        .position
        .into_iter()
        .chain(s.angles)
        .chain(s.rates)
        .chain(s.previous_rates)
        .chain([p.braking, p.gravity, p.pitch_rate])
        .all(f32::is_finite)
        || s.angles[0].abs() > 1_000_000.
    {
        return Err("enemy flight fields outside finite ordinary profile");
    }
    s.personality |= 0x600;
    if s.response & 0x1800 != 0 {
        return Ok(s);
    }
    if s.response & 0x4000 != 0 {
        if s.personality & 4 == 0 {
            return Err("enemy flight requires acquired affine resynchronization");
        }
        s.response &= !0x4000;
    }
    let dt = time.at(s.position);
    if s.rates[1] > 0. {
        let next = f64::from(s.rates[1]) - dt * f64::from(p.braking);
        s.rates[1] = if next < 0. { 0. } else { next as f32 };
    }
    let forward = ((f64::from(s.previous_rates[1]) + f64::from(s.rates[1])) * dt * 0.5) as f32;
    s.rates[0] = (f64::from(s.rates[0]) - dt * f64::from(p.gravity)) as f32;
    let vertical = ((f64::from(s.previous_rates[0]) + f64::from(s.rates[0])) * dt * 0.5) as f32;
    let (sin, cos) = crate::object_transform::sincos(s.angles[0]);
    let delta = [
        (f64::from(forward) * sin) as f32,
        (-f64::from(forward) * cos) as f32,
        vertical,
    ];
    for (value, delta) in s.position.iter_mut().zip(delta) {
        *value = (f64::from(*value) + f64::from(delta)) as f32;
    }
    // Unlike velocity integration, pitch samples object time after translation.
    s.angles[1] = (f64::from(s.angles[1]) + time.at(s.position) * f64::from(p.pitch_rate)) as f32;
    if !s
        .position
        .into_iter()
        .chain(s.angles)
        .chain(s.rates)
        .all(f32::is_finite)
    {
        return Err("enemy flight result overflow");
    }
    Ok(s)
}
/// Execute only after the world/contact continuation has supplied its actual
/// final position. Expiry is strict and compared before the f32 store is popped.
pub fn countdown(
    remaining: f32,
    position: [f32; 3],
    time: &Time<'_>,
) -> Result<(f32, bool), &'static str> {
    time.validate()?;
    if !remaining.is_finite() || !position.into_iter().all(f32::is_finite) {
        return Err("enemy lifetime outside finite profile");
    }
    let next = f64::from(remaining) - time.at(position);
    let stored = next as f32;
    if !stored.is_finite() {
        return Err("enemy lifetime result overflow");
    }
    Ok((stored, next < 0.))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_lifetime_is_not_yet_expired_and_missing_resynchronization_is_refused() {
        let time = Time {
            seconds: 0.,
            context: 1,
            areas: &[],
        };
        assert_eq!(countdown(0., [0.; 3], &time), Ok((0., false)));
        let state = State {
            position: [0.; 3],
            angles: [0.; 3],
            rates: [0.; 4],
            previous_rates: [0.; 4],
            response: 0x4000,
            personality: 0,
        };
        let parameters = Parameters {
            braking: 0.,
            gravity: 0.,
            pitch_rate: 0.,
        };
        assert!(advance(state, parameters, &time).is_err());
        let skipped = advance(
            State {
                response: 0x5800,
                ..state
            },
            parameters,
            &time,
        )
        .unwrap();
        assert_eq!(skipped.response, 0x5800);
        assert_eq!(skipped.personality, 0x600);
    }
    #[test]
    fn invalid_clock_is_refused_before_publishing_a_step() {
        assert!(
            countdown(
                1.,
                [0.; 3],
                &Time {
                    seconds: f32::NAN,
                    context: 1,
                    areas: &[]
                }
            )
            .is_err()
        );
        assert!(
            countdown(
                1.,
                [f32::INFINITY; 3],
                &Time {
                    seconds: 0.1,
                    context: 1,
                    areas: &[]
                }
            )
            .is_err()
        );
    }
}
