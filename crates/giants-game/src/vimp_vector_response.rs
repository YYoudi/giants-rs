//! Vector response and marker retargeting. Session/route snapshots, optional
//! hint actors and action execution remain explicit owned dependencies.
use crate::{actor_session_predicates, audio_parameters::Random, vimp_response};

#[derive(Clone, Copy, Debug)]
pub struct Marker {
    pub position: [f32; 3],
    pub radius: f32,
}
#[derive(Clone, Debug)]
pub struct Route<K> {
    pub owner: K,
    pub selected: u32,
    pub position: [f32; 3],
    pub markers: Vec<Option<Marker>>,
}
#[derive(Clone, Debug)]
pub struct State<K> {
    pub actor: vimp_response::State<K>,
    pub parent: Option<K>,
    pub heading: f32,
    pub selected_heading: f32,
    pub marker: u32,
    pub visits: u32,
    pub counted: u32,
    pub offsets: [f32; 2],
    pub wander_target: [f32; 3],
    pub hint: Option<K>,
}
#[derive(Clone, Copy)]
pub struct Parameters {
    pub turn_low: f32,
    pub turn_high: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Retarget {
        actor_xy: [i16; 2],
        target_xy: [i16; 2],
    },
    Heading(f32),
}
pub trait Services<K> {
    fn session(
        &mut self,
        state: &State<K>,
        query: vimp_response::Predicate,
    ) -> Result<actor_session_predicates::Snapshot<'_>, String>;
    /// Must publish the current prefix and resolve the actual hint actor,
    /// geometry and affine services. Callback writes/retirement must survive.
    fn hint(&mut self, state: &mut State<K>, route: &mut Route<K>, hint: K) -> Result<(), String>;
    fn action(
        &mut self,
        state: &mut State<K>,
        route: &mut Route<K>,
        action: Action,
    ) -> Result<(), String>;
}
fn predicate<K>(
    s: &State<K>,
    services: &mut impl Services<K>,
    query: vimp_response::Predicate,
) -> Result<bool, String> {
    let snapshot = services.session(s, query)?;
    match query {
        vimp_response::Predicate::First => {
            actor_session_predicates::first(&snapshot, s.actor.owner_word)
        }
        vimp_response::Predicate::Second => {
            actor_session_predicates::second(&snapshot, s.actor.owner_word)
        }
    }
}
fn finite(v: f32) -> Result<(), String> {
    if v.is_finite() {
        Ok(())
    } else {
        Err("Vimp vector input outside finite profile".into())
    }
}
fn wide_interval(random: &mut Random, low: f32, high: f32) -> Result<f64, String> {
    finite(low)?;
    finite(high)?;
    Ok(f64::from(random.next_15()) * (f64::from(high) - f64::from(low)) / 32768. + f64::from(low))
}
fn marker<K>(route: &Route<K>, index: u32) -> Result<Marker, String> {
    route
        .markers
        .get(index as usize)
        .copied()
        .flatten()
        .ok_or_else(|| "Vimp route marker absent or unowned".into())
}
fn next_marker<K>(state: &mut State<K>, route: &mut Route<K>) -> Result<(), String> {
    let move_parent = state.marker != route.selected;
    if route.markers.is_empty() {
        return Err("Vimp route has no marker slots".into());
    }
    let count = i32::try_from(route.markers.len())
        .map_err(|_| "Vimp route count exceeds acquired index profile")?;
    for _ in 0..count {
        state.marker = state.marker.wrapping_add(1);
        if state.marker as i32 >= count {
            state.marker = 0;
        }
        if route
            .markers
            .get(state.marker as usize)
            .is_some_and(Option::is_some)
        {
            if move_parent {
                route.selected = state.marker;
            }
            route.position = marker(route, route.selected)?.position;
            return Ok(());
        }
    }
    Err("Vimp route has no eligible marker".into())
}
/// Actual retarget phase. An error retains earlier random draws, offsets and
/// stores; the owning frame must not replay already consumed providers.
pub fn retarget<K: Copy + Eq>(
    state: &mut State<K>,
    route: &mut Route<K>,
    random: &mut Random,
    services: &mut impl Services<K>,
) -> Result<(), String> {
    if state.parent != Some(route.owner) {
        return Err("Vimp retarget route owner mismatch".into());
    }
    let angle = wide_interval(random, 0., 360.)? as f32;
    let (sin, cos) = crate::object_transform::sincos(angle);
    let radius = marker(route, state.marker)?.radius;
    state.offsets[0] = (wide_interval(random, 0., radius)? * cos) as f32;
    let radius = marker(route, state.marker)?.radius;
    state.offsets[1] = (wide_interval(random, 0., radius)? * sin) as f32;
    for i in 0..2 {
        let position = marker(route, state.marker)?.position[i];
        finite(position)?;
        state.wander_target[i] = (f64::from(position) + f64::from(state.offsets[i])) as f32;
    }
    if predicate(state, services, vimp_response::Predicate::Second)? {
        for coordinate in &mut state.actor.position[..2] {
            finite(*coordinate)?;
            *coordinate = coordinate.clamp(-8000., 8000.);
        }
        for coordinate in &mut state.wander_target[..2] {
            finite(*coordinate)?;
            *coordinate = coordinate.clamp(-8000., 8000.);
        }
        let actor_xy = std::array::from_fn(|i| (f64::from(state.actor.position[i]) * 2.) as i16);
        let target_xy = std::array::from_fn(|i| (f64::from(state.wander_target[i]) * 2.) as i16);
        services.action(
            state,
            route,
            Action::Retarget {
                actor_xy,
                target_xy,
            },
        )?;
    }
    if let Some(hint) = state.hint {
        services.hint(state, route, hint)?;
    }
    Ok(())
}
/// Complete vector helper with mandatory route, optional-hint and action
/// dependencies. It mutates owned projections in native order, not atomically.
pub fn advance<K: Copy + Eq>(
    state: &mut State<K>,
    route: Option<&mut Route<K>>,
    origin: [f32; 3],
    fallback: f32,
    parameters: Parameters,
    random: &mut Random,
    services: &mut impl Services<K>,
) -> Result<(), String> {
    if predicate(state, services, vimp_response::Predicate::First)? {
        let route = route.ok_or("Vimp vector route owner absent")?;
        if state.parent != Some(route.owner) {
            return Err("Vimp vector route owner mismatch".into());
        }
        let turn = wide_interval(random, parameters.turn_low, parameters.turn_high)? as f32;
        let backwards =
            vimp_response::backwards(state.actor.forward, state.actor.position, origin)?;
        finite(state.heading)?;
        state.selected_heading = if backwards == 0 {
            (f64::from(state.heading) + f64::from(turn)) as f32
        } else {
            (f64::from(state.heading) - f64::from(turn)) as f32
        };
        if state.selected_heading > 360. {
            state.selected_heading = (f64::from(state.selected_heading) - 360.) as f32;
        } else if state.selected_heading < 0. {
            state.selected_heading = (f64::from(state.selected_heading) + 360.) as f32;
        }
        let current = marker(route, state.marker)?;
        for v in [
            state.actor.position[0],
            state.actor.position[1],
            current.position[0],
            current.position[1],
            current.radius,
        ] {
            finite(v)?;
        }
        if crate::vimp_wander::squared(current.position, state.actor.position)
            <= f64::from(current.radius) * f64::from(current.radius)
        {
            if state.counted == 0 {
                state.visits = state.visits.wrapping_add(1);
            } else {
                state.counted = 0;
            }
            next_marker(state, route)?;
        }
        retarget(state, route, random, services)?;
        if let Some(hint) = state.hint {
            services.hint(state, route, hint)?;
        }
        if predicate(state, services, vimp_response::Predicate::Second)? {
            services.action(state, route, Action::Heading(state.selected_heading))?;
        }
    } else {
        state.selected_heading = fallback;
    }
    state.actor.response_kind = 2;
    state.actor.mode = 1;
    state.actor.binding_flags |= 0x400;
    state.actor.response_stage = 0;
    state.actor.response_active = 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Provider {
        mode: i32,
        hints: Vec<u32>,
        fail: bool,
        retire_hint: bool,
    }
    impl Services<u32> for Provider {
        fn session(
            &mut self,
            _: &State<u32>,
            _: vimp_response::Predicate,
        ) -> Result<actor_session_predicates::Snapshot<'_>, String> {
            Ok(actor_session_predicates::Snapshot {
                mode: self.mode,
                declared_count: 1,
                current_key: 17,
                slots: &[17],
            })
        }
        fn hint(
            &mut self,
            s: &mut State<u32>,
            r: &mut Route<u32>,
            hint: u32,
        ) -> Result<(), String> {
            self.hints.push(hint);
            if self.retire_hint {
                s.hint = None;
            }
            if self.fail {
                s.actor.flags = 123;
                r.position[2] = 42.;
                return Err("hint failed after writes".into());
            }
            Ok(())
        }
        fn action(
            &mut self,
            s: &mut State<u32>,
            _: &mut Route<u32>,
            _: Action,
        ) -> Result<(), String> {
            s.actor.flags = 456;
            Err("action unbound".into())
        }
    }
    fn fixture() -> (State<u32>, Route<u32>, Provider) {
        let actor = vimp_response::State {
            owner_word: 0,
            position: [0.; 3],
            forward: [1., 0., 0.],
            physical_flags: 0,
            channel_flags: 2,
            priority: 0,
            flags: 0,
            binding_flags: 0,
            remaining_signal: 0.,
            signal_origin: [0.; 3],
            linked_seconds: 0.,
            linked: None,
            target: None,
            target_position: [0.; 3],
            backwards: 0,
            mode: 7,
            delay: 0.,
            response_kind: 123,
            response_active: 234,
            response_stage: 345,
        };
        (
            State {
                actor,
                parent: Some(1),
                heading: 360.,
                selected_heading: 17.,
                marker: 0,
                visits: u32::MAX,
                counted: 0,
                offsets: [0.; 2],
                wander_target: [0., 0., 55.],
                hint: None,
            },
            Route {
                owner: 1,
                selected: 0,
                position: [10., 20., 30.],
                markers: vec![
                    Some(Marker {
                        position: [0.; 3],
                        radius: 10.,
                    }),
                    None,
                    Some(Marker {
                        position: [100., 0., 7.],
                        radius: 20.,
                    }),
                ],
            },
            Provider {
                mode: 0,
                hints: Vec::new(),
                fail: false,
                retire_hint: false,
            },
        )
    }
    fn run(
        s: &mut State<u32>,
        r: Option<&mut Route<u32>>,
        rng: &mut Random,
        p: &mut Provider,
    ) -> Result<(), String> {
        advance(
            s,
            r,
            [1., 0., 0.],
            -1.,
            Parameters {
                turn_low: 0.,
                turn_high: 0.,
            },
            rng,
            p,
        )
    }
    #[test]
    fn sparse_route_preserves_parent_selection_and_exact_360_with_wrapping_visits() {
        let (mut s, mut r, mut p) = fixture();
        let mut rng = Random(1);
        run(&mut s, Some(&mut r), &mut rng, &mut p).unwrap();
        assert_eq!((s.marker, s.visits, r.selected), (2, 0, 0));
        assert_eq!(r.position, [0.; 3]);
        assert_eq!(s.selected_heading, 360.);
        assert_eq!(s.wander_target[2], 55.);
        assert_eq!(
            (s.actor.mode, s.actor.response_kind, s.actor.response_stage),
            (1, 2, 0)
        );
        let mut expected = Random(1);
        for _ in 0..4 {
            expected.next_15();
        }
        assert_eq!(rng.0, expected.0);
    }
    #[test]
    fn parent_follows_only_a_previously_different_marker_and_skips_null_on_wrap() {
        let (mut s, mut r, mut p) = fixture();
        s.marker = 2;
        s.actor.position = [100., 0., 7.];
        s.counted = 1;
        run(&mut s, Some(&mut r), &mut Random(1), &mut p).unwrap();
        assert_eq!(
            (s.marker, s.counted, s.visits, r.selected),
            (0, 0, u32::MAX, 0)
        );
        assert_eq!(r.position, [0.; 3]);
    }
    #[test]
    fn predicate_skip_needs_no_route_or_draw_but_missing_admitted_owner_refuses() {
        let (mut s, _, mut p) = fixture();
        let mut rng = Random(1);
        p.mode = 4;
        s.actor.owner_word = 1;
        run(&mut s, None, &mut rng, &mut p).unwrap();
        assert_eq!(s.selected_heading, -1.);
        assert_eq!(rng.0, 1);
        s.actor.owner_word = 0;
        assert!(run(&mut s, None, &mut rng, &mut p).is_err());
        assert_eq!(rng.0, 1);
    }
    #[test]
    fn required_hint_error_retains_callback_writes_and_does_not_finalize() {
        let (mut s, mut r, mut p) = fixture();
        s.hint = Some(2);
        p.fail = true;
        let mut rng = Random(1);
        assert!(run(&mut s, Some(&mut r), &mut rng, &mut p).is_err());
        assert_eq!(p.hints, [2]);
        assert_eq!(s.actor.flags, 123);
        assert_eq!(r.position[2], 42.);
        assert_eq!(
            (s.actor.mode, s.actor.response_kind, s.actor.response_stage),
            (7, 123, 345)
        );
        assert_ne!(rng.0, 1);
    }
    #[test]
    fn hint_identity_is_reread_after_retarget_and_action_errors_are_not_successes() {
        let (mut s, mut r, mut p) = fixture();
        s.hint = Some(2);
        p.retire_hint = true;
        run(&mut s, Some(&mut r), &mut Random(1), &mut p).unwrap();
        assert_eq!(p.hints, [2]);
        assert_eq!(s.hint, None);
        let (mut s, mut r, mut p) = fixture();
        p.mode = 4;
        assert!(run(&mut s, Some(&mut r), &mut Random(1), &mut p).is_err());
        assert_eq!(s.actor.flags, 456);
        assert_eq!(s.actor.mode, 7);
        assert_eq!(s.actor.response_kind, 123);
    }
}
