//! Ordinary actor finalization, with explicit owned footprint and environment
//! services. This does not supply a collision backend or acknowledge readiness.
use crate::{
    actor_affine::Cache,
    collider_copy::Fields,
    object_time::{Sample, SlowArea},
};

#[derive(Clone, Debug)]
pub struct State {
    pub fields: Fields,
    pub previous_physical_velocity: [f32; 3],
    pub friction: f32,
    pub parent_present: bool,
    pub footprint: Option<u32>,
    pub affine: Option<Cache>,
    /// Exact-D ABI quirk: on unparented physical actors the reused argument slot was not
    /// initialized with local time. Owned runtime callers must not invent it.
    pub physical_argument_word: Option<u32>,
}
/// A late error retains local mutations and records that external effects may
/// already have occurred. A consumed phase must never be replayed wholesale.
#[derive(Debug)]
pub struct Failure {
    pub state: Box<State>,
    pub phase_started: bool,
    pub message: String,
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}
impl std::error::Error for Failure {}
fn failure(state: State, phase_started: bool, message: impl Into<String>) -> Failure {
    Failure {
        state: Box::new(state),
        phase_started,
        message: message.into(),
    }
}
pub struct Time<'a> {
    pub global_seconds: f32,
    pub context: u32,
    pub areas: &'a [SlowArea],
}
pub trait Services {
    fn release_footprint(&mut self, identity: Option<u32>) -> Result<(), String>;
    fn environment(&mut self, state: &mut State) -> Result<(), String>;
}

pub fn finalize(
    mut s: State,
    time: Time<'_>,
    services: &mut impl Services,
) -> Result<State, Failure> {
    let f = &s.fields;
    if !f
        .current
        .into_iter()
        .chain(f.previous)
        .chain(f.velocity)
        .chain(s.previous_physical_velocity)
        .chain([
            s.friction,
            time.global_seconds,
            f.scale,
            f.ground_offset,
            f.height,
            f.center_offset,
            f.floor,
            f.model_vertical_speed,
        ])
        .all(f32::is_finite)
        || (f.object_flags & 0x10000 == 0 && !f.angles.into_iter().all(f32::is_finite))
        || time.areas.iter().any(|a| {
            !a.center
                .into_iter()
                .chain([a.radius_squared, a.factor])
                .all(f32::is_finite)
        })
    {
        return Err(failure(
            s,
            false,
            "actor finalization outside finite profile",
        ));
    }
    let mut delta = std::array::from_fn::<_, 3, _>(|i| {
        (f64::from(f.current[i]) - f64::from(f.previous[i])) as f32
    });
    let threshold = if f.physical && s.parent_present {
        None
    } else if f.physical {
        let Some(word) = s.physical_argument_word else {
            return Err(failure(
                s,
                false,
                "physical finalization needs acquired caller-slot semantics",
            ));
        };
        let v = f32::from_bits(word);
        if !v.is_finite() {
            return Err(failure(
                s,
                false,
                "physical caller-slot threshold outside finite profile",
            ));
        }
        Some(v)
    } else {
        let dt = crate::object_time::seconds(
            time.global_seconds,
            Some(Sample {
                owner: time.context,
                position: f.current,
            }),
            None,
            time.areas,
        ) as f32;
        for (i, d) in delta.iter_mut().enumerate() {
            *d = ((f64::from(f.velocity[i]) + f64::from(s.previous_physical_velocity[i]))
                * f64::from(dt)
                * 0.5
                + f64::from(*d)) as f32;
        }
        s.previous_physical_velocity = s.fields.velocity;
        for v in &mut s.fields.velocity {
            if *v > 0. {
                let wide = f64::from(*v) - f64::from(dt) * f64::from(s.friction);
                *v = if wide < 0. { 0. } else { wide as f32 };
            }
            if *v < 0. {
                let wide = f64::from(*v) + f64::from(dt) * f64::from(s.friction);
                *v = if wide > 0. { 0. } else { wide as f32 };
            }
        }
        Some(dt)
    };
    if delta[0] != 0. || delta[1] != 0. {
        if let Err(message) = services.release_footprint(s.footprint) {
            return Err(failure(s, true, message));
        }
        s.footprint = None;
    }
    let previous = s.fields.previous;
    let z = f64::from(previous[2]) + f64::from(delta[2]);
    for (i, d) in delta.into_iter().enumerate() {
        s.fields.current[i] = (f64::from(previous[i]) + f64::from(d)) as f32;
    }
    if s.parent_present {
        // The native parented path only marks the cache flag. It does not
        // produce a world affine cache; retaining None is deliberate.
        s.fields.object_flags |= 0x20;
        return Ok(s);
    }
    let threshold = threshold.expect("unparented threshold prevalidated");
    if matches!(s.fields.class, 6 | 7) {
        s.fields.floor = z as f32;
    } else if s.fields.object_flags & 0x10 != 0 {
        s.fields.floor = (z - f64::from(s.fields.ground_offset)) as f32;
        s.fields.model_vertical_speed = 0.;
    } else {
        if let Err(message) = services.environment(&mut s) {
            return Err(failure(s, true, message));
        }
    }
    let f = &mut s.fields;
    if f.current[2] <= 0. {
        f.object_flags |= 0x100000;
        if -(f64::from(f.center_offset) * f64::from(f.scale)) >= f64::from(f.current[2]) {
            f.object_flags |= 0x08000000;
        } else {
            f.object_flags &= !0x08000000;
        }
    } else {
        f.object_flags &= !0x08100000;
    }
    let actual = [
        (f64::from(f.current[0]) - f64::from(previous[0])) as f32,
        (f64::from(f.current[1]) - f64::from(previous[1])) as f32,
    ];
    let ax = delta[0].abs();
    let ay = delta[1].abs();
    let (dominant, minor) = if ax > ay { (0, 1) } else { (1, 0) };
    let abs = [ax, ay];
    let blocked = if abs[dominant] < threshold {
        false
    } else {
        let ratio = f64::from(abs[minor]) / f64::from(abs[dominant]);
        // Unordered comparisons occur for zero motion at a zero threshold.
        // Preserve their native flags rather than turning 0/0 into an error.
        let limited = |i: usize| {
            let v = f64::from(actual[i]) / f64::from(delta[i]);
            v <= 0.5 || v.is_nan()
        };
        limited(dominant) || ((ratio >= f64::from(0.35f32) || ratio.is_nan()) && limited(minor))
    };
    if blocked {
        f.object_flags |= 0x40000;
    } else {
        f.object_flags &= !0x40000;
    }
    let affine_result = crate::actor_affine::update(
        crate::uncached_pose::Pose {
            position: f.current,
            angles: f.angles,
            scale: f.scale,
        },
        f.object_flags,
        s.affine,
        crate::arithmetic::Precision::Double53,
    );
    let (active, cache) = match affine_result {
        Ok(result) => result,
        Err(message) => return Err(failure(s, true, message)),
    };
    let f = &mut s.fields;
    f.object_flags = active;
    s.affine = Some(cache);
    Ok(s)
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        releases: Vec<Option<u32>>,
        environment_calls: u32,
        fail: bool,
        change_previous: bool,
    }
    impl Services for Fixture {
        fn release_footprint(&mut self, id: Option<u32>) -> Result<(), String> {
            self.releases.push(id);
            Ok(())
        }
        fn environment(&mut self, s: &mut State) -> Result<(), String> {
            self.environment_calls += 1;
            if self.change_previous {
                s.fields.previous[0] = 100.;
            }
            if self.fail {
                s.fields.floor = 123.;
                return Err("environment unavailable after partial work".into());
            }
            Ok(())
        }
    }
    fn fixture() -> (State, Fixture) {
        let mut f = Fields::zeroed();
        f.class = 6;
        f.scale = 1.;
        f.center_offset = 1.;
        (
            State {
                fields: f,
                previous_physical_velocity: [0.; 3],
                friction: 10.,
                parent_present: false,
                footprint: Some(7),
                affine: None,
                physical_argument_word: None,
            },
            Fixture {
                releases: vec![],
                environment_calls: 0,
                fail: false,
                change_previous: false,
            },
        )
    }
    fn time(dt: f32) -> Time<'static> {
        Time {
            global_seconds: dt,
            context: 1,
            areas: &[],
        }
    }
    #[test]
    fn zero_motion_zero_time_preserves_unordered_progress_test() {
        let (s, mut svc) = fixture();
        let zero = finalize(s.clone(), time(0.), &mut svc).unwrap();
        assert_ne!(zero.fields.object_flags & 0x40000, 0);
        let advancing = finalize(s, time(0.1), &mut svc).unwrap();
        assert_eq!(advancing.fields.object_flags & 0x40000, 0);
        assert!(svc.releases.is_empty());
        assert_eq!(svc.environment_calls, 0);
    }
    #[test]
    fn parented_path_keeps_missing_affine_after_velocity_history_and_release() {
        let (mut s, mut svc) = fixture();
        s.parent_present = true;
        s.fields.current = [1., 2., 3.];
        s.fields.velocity = [2., 0., -2.];
        let s = finalize(s, time(0.1), &mut svc).unwrap();
        assert_eq!(s.previous_physical_velocity, [2., 0., -2.]);
        assert_eq!(s.fields.velocity, [1., 0., -1.]);
        assert_eq!(svc.releases, [Some(7)]);
        assert_eq!(s.footprint, None);
        assert!(s.affine.is_none());
        assert_ne!(s.fields.object_flags & 0x20, 0);
        assert_eq!(svc.environment_calls, 0);
        let (mut physical, mut services) = fixture();
        physical.parent_present = true;
        physical.fields.physical = true;
        let physical = finalize(physical, time(0.1), &mut services).unwrap();
        assert!(physical.affine.is_none());
        assert!(services.releases.is_empty());
        assert_eq!(services.environment_calls, 0);
    }
    #[test]
    fn late_failure_retains_partial_state_and_unknown_physical_abi_is_refused_before_services() {
        let (mut s, mut svc) = fixture();
        s.fields.class = 73;
        s.fields.current[0] = 2.;
        svc.fail = true;
        let e = finalize(s, time(0.1), &mut svc).unwrap_err();
        assert!(e.phase_started);
        assert_eq!(e.state.footprint, None);
        assert_eq!(e.state.fields.floor, 123.);
        assert!(e.state.affine.is_none());
        let (mut s, mut svc) = fixture();
        s.fields.physical = true;
        let e = finalize(s, time(0.1), &mut svc).unwrap_err();
        assert!(!e.phase_started);
        assert_eq!(e.state.footprint, Some(7));
        assert!(svc.releases.is_empty());
        assert_eq!(svc.environment_calls, 0);
    }
    #[test]
    fn progress_uses_reference_position_saved_before_environment() {
        let (mut s, mut svc) = fixture();
        s.fields.class = 73;
        s.fields.current = [2., 0., 1.];
        svc.change_previous = true;
        let s = finalize(s, time(0.1), &mut svc).unwrap();
        assert_eq!(s.fields.previous[0], 100.);
        assert_eq!(s.fields.object_flags & 0x40000, 0);
        assert_eq!(s.affine.unwrap().matrix[3], 2.);
    }
    #[test]
    fn depth_flags_use_center_offset_not_height() {
        let (mut s, mut svc) = fixture();
        s.fields.current[2] = -1.;
        s.fields.previous[2] = -1.;
        s.fields.height = 0.5;
        s.fields.center_offset = 2.;
        s.fields.object_flags = 0x08000000;
        let s = finalize(s, time(0.1), &mut svc).unwrap();
        assert_ne!(s.fields.object_flags & 0x100000, 0);
        assert_eq!(s.fields.object_flags & 0x08000000, 0);
    }
}
