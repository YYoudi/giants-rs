//! Acquired post-query filtering and retained-reference transition. Actor
//! lifetime, actual bounds, clock ownership and marker publication are services;
//! this does not complete a Kabuto primary or admit a live player.
//! Prototype PS2 `kabuto_move` is an unknown parent-primary navigation route,
//! not an attributed name or signature for this PC component.
use crate::kabuto_reference_query::Selection;

#[derive(Clone, Copy, Debug)]
pub struct State<K> {
    pub raw: Option<K>,
    pub accepted: Option<K>,
    pub retained: Option<K>,
    pub remaining: f32,
    pub score: f32,
}
pub struct Bounds {
    pub lower: [f32; 3],
    pub upper: [f32; 3],
}
pub struct Pose {
    pub position: [f32; 3],
    pub scale: f32,
    pub height: f32,
    /// Three acquired source orientation coefficients. Their exact combination
    /// below is retained; this is not replaced by a conventional dot product.
    pub axis: [f32; 3],
}
/// Successful filter/retained-reference handoff. The caller still owns its
/// invocation/frame guards; this sample is not selector or primary admission.
#[derive(Clone, Copy, Debug)]
pub struct View<K> {
    owner: K,
    clock_word: u32,
}
impl<K: Copy> View<K> {
    pub fn owner(&self) -> K {
        self.owner
    }
    /// The actual single-precision clock store used by this invocation.
    pub fn clock_word(&self) -> u32 {
        self.clock_word
    }
}
pub trait Services<K> {
    fn bounds(&mut self, target: K) -> Result<Bounds, String>;
    fn seconds(&mut self, published: &State<K>) -> Result<f64, String>;
    /// Publish the acquired global selection indication before marker reset.
    fn announce(&mut self, published: &State<K>) -> Result<(), String>;
    fn reset_marker(&mut self, published: &State<K>) -> Result<(), String>;
    fn parent(&mut self, target: K) -> Result<Option<K>, String>;
    fn pose(&mut self) -> Result<Pose, String>;
}
fn check_bounds(bounds: &Bounds) -> Result<(), String> {
    if !bounds
        .lower
        .into_iter()
        .chain(bounds.upper)
        .all(f32::is_finite)
    {
        return Err("Kabuto retained-reference bounds outside finite profile".into());
    }
    Ok(())
}

/// Errors retain completed state writes and external service effects. These
/// owned refusals do not claim native exception behavior or rollback parity.
pub fn advance<K: Copy + Eq>(
    state: &mut State<K>,
    owner: K,
    source_reference_present: bool,
    selection: Selection<K>,
    services: &mut impl Services<K>,
) -> Result<(), String> {
    advance_with_clock(state, owner, source_reference_present, selection, services).map(|_| ())
}

/// The same acquired filter, retaining its actual caller clock store for the
/// next component. This does not recompute time or complete caller admission.
pub fn advance_with_clock<K: Copy + Eq>(
    state: &mut State<K>,
    owner: K,
    source_reference_present: bool,
    selection: Selection<K>,
    services: &mut impl Services<K>,
) -> Result<View<K>, String> {
    state.raw = selection.target;
    state.accepted = selection.target;
    state.score = selection.score;
    if state.score.is_nan() || state.score == f32::INFINITY || !state.remaining.is_finite() {
        return Err("Kabuto retained-reference state outside acquired profile".into());
    }
    if let Some(target) = state.accepted.filter(|_| state.score > 0.) {
        let b = services.bounds(target)?;
        check_bounds(&b)?;
        let half: [f64; 3] =
            std::array::from_fn(|i| (f64::from(b.upper[i]) - f64::from(b.lower[i])) * 0.5);
        let radius_squared = (half[0] * half[0] + half[1] * half[1]) + half[2] * half[2];
        if f64::from(state.score) > radius_squared {
            state.accepted = None;
        }
    }
    // This caller stores the shared clock return before subtracting it; the
    // following timer sign test still uses the wide subtraction result.
    let seconds = services.seconds(state)?;
    if !seconds.is_finite() || seconds < 0. || !(seconds as f32).is_finite() {
        return Err("Kabuto retained-reference clock outside finite nonnegative profile".into());
    }
    let remaining = f64::from(state.remaining) - f64::from(seconds as f32);
    state.remaining = remaining as f32;
    if remaining < 0. {
        state.remaining = 0.;
        state.retained = None;
    }
    if let Some(target) = state.accepted {
        services.announce(state)?;
        if state.retained != Some(target) {
            services.reset_marker(state)?;
        }
        state.remaining = 1.5;
        state.retained = Some(target);
    }
    if !source_reference_present {
        state.retained = None;
    }
    if let Some(target) = state.retained
        && services.parent(target)? == Some(owner)
    {
        state.retained = None;
    }
    if let Some(target) = state.retained {
        let p = services.pose()?;
        if !p
            .position
            .into_iter()
            .chain([p.scale, p.height])
            .chain(p.axis)
            .all(f32::is_finite)
            || p.scale == 0.
        {
            return Err(
                "Kabuto retained-reference pose outside finite nonzero-scale profile".into(),
            );
        }
        let inverse_scale = (1. / f64::from(p.scale)) as f32;
        let origin_z = (f64::from(p.height) * f64::from(p.scale) + f64::from(p.position[2])) as f32;
        let b = services.bounds(target)?;
        check_bounds(&b)?;
        let center: [f64; 3] =
            std::array::from_fn(|i| (f64::from(b.upper[i]) + f64::from(b.lower[i])) * 0.5);
        // Only X's centre stays wide until subtraction. Y/Z's centres and the
        // source height-adjusted Z have their own earlier float32 stores.
        let delta = [
            (center[0] - f64::from(p.position[0])) as f32,
            (f64::from(center[1] as f32) - f64::from(p.position[1])) as f32,
            (f64::from(center[2] as f32) - f64::from(origin_z)) as f32,
        ];
        if !delta.into_iter().chain([inverse_scale]).all(f32::is_finite) {
            return Err("Kabuto retained-reference directional input overflow".into());
        }
        let normal = crate::sun_glare::normalize(delta);
        let expression = -(((f64::from(p.axis[2]) + f64::from(p.axis[0])) * f64::from(normal[0])
            + f64::from(normal[1]) * f64::from(p.axis[1]))
            * f64::from(inverse_scale));
        if expression < f64::from(0.707_f32) {
            state.retained = None;
        }
    }
    Ok(View {
        owner,
        clock_word: (seconds as f32).to_bits(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct ClockOnly {
        seconds: f64,
        calls: usize,
    }
    impl Services<u32> for ClockOnly {
        fn seconds(&mut self, _: &State<u32>) -> Result<f64, String> {
            self.calls += 1;
            Ok(self.seconds)
        }
        fn bounds(&mut self, _: u32) -> Result<Bounds, String> {
            panic!("no selected target")
        }
        fn announce(&mut self, _: &State<u32>) -> Result<(), String> {
            panic!("no selected target")
        }
        fn reset_marker(&mut self, _: &State<u32>) -> Result<(), String> {
            panic!("no selected target")
        }
        fn parent(&mut self, _: u32) -> Result<Option<u32>, String> {
            panic!("no retained target")
        }
        fn pose(&mut self) -> Result<Pose, String> {
            panic!("no retained target")
        }
    }
    #[test]
    fn successful_query_carries_its_clock_without_requery_or_foreign_owner_publication() {
        for seconds in [-0., 0., 1. / 60., 0.123456789] {
            let mut state = State {
                raw: None,
                accepted: None,
                retained: None,
                remaining: 10.,
                score: 0.,
            };
            let mut world = ClockOnly { seconds, calls: 0 };
            let sample = advance_with_clock(
                &mut state,
                7,
                false,
                Selection {
                    target: None,
                    score: 0.,
                },
                &mut world,
            )
            .unwrap();
            assert_eq!(sample.owner(), 7);
            assert_eq!(world.calls, 1);
            let mut scratch = crate::kabuto_action_select::Scratch {
                packet_word: 3,
                slot: 4,
                retained_time: 0xdeadbeef,
                flags: 5,
                clear_flags: 6,
                behavior: 8,
                definition: 9,
                blend_seconds: 0.25,
                reference_speed: 12.,
                speed_source: -1,
            };
            assert!(scratch.retain_reference_clock(8, &sample).is_err());
            assert_eq!(scratch.retained_time, 0xdeadbeef);
            scratch.retain_reference_clock(7, &sample).unwrap();
            assert_eq!(scratch.retained_time, (seconds as f32).to_bits());
            assert_eq!(
                (
                    scratch.packet_word,
                    scratch.slot,
                    scratch.flags,
                    scratch.clear_flags,
                    scratch.behavior,
                    scratch.definition,
                    scratch.blend_seconds.to_bits(),
                    scratch.reference_speed.to_bits(),
                    scratch.speed_source
                ),
                (3, 4, 5, 6, 8, 9, 0.25f32.to_bits(), 12f32.to_bits(), -1)
            );
            assert_eq!(world.calls, 1);
        }
    }
    struct LateFailure {
        marker: bool,
        announced: bool,
    }
    impl Services<u32> for LateFailure {
        fn bounds(&mut self, _: u32) -> Result<Bounds, String> {
            panic!("zero score requested radial bounds")
        }
        fn seconds(&mut self, s: &State<u32>) -> Result<f64, String> {
            assert_eq!(s.raw, Some(3));
            assert_eq!(s.accepted, Some(3));
            assert_eq!(s.retained, Some(2));
            assert_eq!(s.remaining, 10.);
            Ok(0.25)
        }
        fn announce(&mut self, s: &State<u32>) -> Result<(), String> {
            assert_eq!(s.remaining, 9.75);
            self.announced = true;
            Ok(())
        }
        fn reset_marker(&mut self, s: &State<u32>) -> Result<(), String> {
            assert!(self.announced);
            assert_eq!(s.retained, Some(2));
            self.marker = true;
            Err("marker publication failed after its effect".into())
        }
        fn parent(&mut self, _: u32) -> Result<Option<u32>, String> {
            panic!("failed marker entered parent lookup")
        }
        fn pose(&mut self) -> Result<Pose, String> {
            panic!("failed marker entered directional geometry")
        }
    }
    #[test]
    fn late_marker_failure_retains_selection_clock_and_external_effects() {
        let mut state = State {
            raw: None,
            accepted: None,
            retained: Some(2),
            remaining: 10.,
            score: 0.,
        };
        let mut services = LateFailure {
            marker: false,
            announced: false,
        };
        assert!(
            advance(
                &mut state,
                1,
                true,
                Selection {
                    target: Some(3),
                    score: 0.,
                },
                &mut services,
            )
            .is_err()
        );
        assert_eq!(state.raw, Some(3));
        assert_eq!(state.accepted, Some(3));
        assert_eq!(state.retained, Some(2));
        assert_eq!(state.remaining, 9.75);
        assert!(services.marker && services.announced);
    }
}
