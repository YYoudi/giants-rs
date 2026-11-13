//! PC radial request traversal. Prototype PS2 search routes `hurt_object_do_multi`
//! and `hurt_object` are candidates, not confirmed routine identities. Original
//! PC aliases remain in the private identity-bound record. Actual actor/list,
//! alert and hit services are mandatory; this is not an empty-world provider.

#[derive(Clone, Copy, Debug)]
pub struct Input<I> {
    pub source: I,
    /// Encountering this eligible actor returns from the whole traversal.
    pub other: Option<I>,
    pub position: [f32; 3],
    /// Source payload order: inner radius, outer radius, inner amount, slope,
    /// intercept. The zero-slope guard precedes list and resource access.
    pub parameters: [f32; 5],
    pub context: u32,
    pub cross_context: bool,
    pub vimp_alert_range: f32,
}

/// A stronger, read-only proof: no actor can reach either radial side-effect
/// radius, regardless of the unbound cross-context word or health eligibility.
/// This does not supply a damage/alert consumer or acknowledge a hit. A possible
/// callback returns false; an unavailable consumed input remains an error.
pub fn no_effect_invariant<I>(
    parameters: [f32; 5],
    position: [f32; 3],
    vimp_alert_range: Option<f32>,
    actors: &[Actor<I>],
) -> Result<bool, String> {
    Ok(no_effect_obstacle(parameters, position, vimp_alert_range, actors)?.is_none())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PossibleEffect {
    DamageRadius,
    VimpAlertRadius,
}

/// The first obstacle to this conservative proof, not an eligible actor, an
/// executed side effect or a positive callback receipt. List order is retained.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoEffectObstacle {
    pub index: usize,
    pub reason: PossibleEffect,
    pub distance_squared: f32,
    pub bound_squared: f64,
}

pub fn no_effect_obstacle<I>(
    parameters: [f32; 5],
    position: [f32; 3],
    vimp_alert_range: Option<f32>,
    actors: &[Actor<I>],
) -> Result<Option<NoEffectObstacle>, String> {
    if parameters[3] == 0. {
        return Ok(None);
    }
    if !parameters.into_iter().chain(position).all(f32::is_finite) {
        return Err("radial no-effect proof requires finite source arithmetic".into());
    }
    let outer_squared = (f64::from(parameters[1]) * f64::from(parameters[1])) as f32;
    if !outer_squared.is_finite() {
        return Err("radial outer radius square overflow".into());
    }
    for (index, actor) in actors.iter().enumerate() {
        if actor.active & 0x4000c000 != 0 {
            continue;
        }
        if !actor.position.into_iter().all(f32::is_finite) {
            return Err("radial no-effect proof needs the current actor pose".into());
        }
        let delta: [f64; 3] =
            std::array::from_fn(|i| f64::from(actor.position[i]) - f64::from(position[i]));
        let distance = ((delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]) as f32;
        if !distance.is_finite() {
            return Err("radial target distance overflow".into());
        }
        if distance <= outer_squared {
            return Ok(Some(NoEffectObstacle {
                index,
                reason: PossibleEffect::DamageRadius,
                distance_squared: distance,
                bound_squared: f64::from(outer_squared),
            }));
        }
        if actor.class == 8 {
            let range = vimp_alert_range
                .filter(|v| v.is_finite())
                .ok_or("radial no-effect proof requires the actual Vimp alert range")?;
            if f64::from(distance) < f64::from(range) * f64::from(range) {
                return Ok(Some(NoEffectObstacle {
                    index,
                    reason: PossibleEffect::VimpAlertRadius,
                    distance_squared: distance,
                    bound_squared: f64::from(range) * f64::from(range),
                }));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct World {
        actors: Vec<Actor<u32>>,
        events: Vec<(u32, u32)>,
        fail: bool,
        change_next: bool,
    }
    impl Services<u32> for World {
        fn first(&mut self) -> Result<Option<u32>, String> {
            self.events.push((0, 0));
            Ok(self.actors.first().map(|a| a.id))
        }
        fn next(&mut self, id: u32) -> Result<Option<u32>, String> {
            Ok(self
                .actors
                .iter()
                .position(|a| a.id == id)
                .and_then(|i| self.actors.get(i + 1))
                .map(|a| a.id))
        }
        fn actor(&mut self, id: u32) -> Result<Actor<u32>, String> {
            self.actors
                .iter()
                .find(|a| a.id == id)
                .copied()
                .ok_or("actor absent".into())
        }
        fn alert_vimp(&mut self, id: u32, _: u32) -> Result<(), String> {
            self.events.push((1, id));
            Ok(())
        }
        fn damage(&mut self, hit: Hit<u32>) -> Result<(), String> {
            assert_eq!((hit.kind, hit.tag), (2, 0));
            self.events.push((2, hit.target));
            if self.change_next
                && let Some(next) = self.actors.get_mut(1)
            {
                next.health = Some([10., 10.]);
            }
            if self.fail {
                Err("actual hit sink failed".into())
            } else {
                Ok(())
            }
        }
    }
    fn actor(id: u32, class: i32, x: f32) -> Actor<u32> {
        Actor {
            id,
            class,
            state: 23,
            context: 1,
            active: 0,
            personality: 0,
            position: [x, 0., 0.],
            health: Some([0., 10.]),
        }
    }
    fn input() -> Input<u32> {
        Input {
            source: 99,
            other: None,
            position: [0.; 3],
            parameters: [2., 5., 20., -2., 20.],
            context: 0,
            cross_context: false,
            vimp_alert_range: 10.,
        }
    }
    fn world(actors: Vec<Actor<u32>>) -> World {
        World {
            actors,
            events: vec![],
            fail: false,
            change_next: false,
        }
    }
    #[test]
    fn zero_slope_reads_no_world_or_other_parameters() {
        let mut services = world(vec![]);
        let mut i = input();
        i.parameters = [f32::NAN, f32::NAN, f32::NAN, -0., f32::NAN];
        i.position = [f32::NAN; 3];
        Machine::default().run(i, &mut services).unwrap();
        assert!(services.events.is_empty());
    }
    #[test]
    fn alert_precedes_hit_radius_and_eligible_other_ends_whole_traversal() {
        let mut services = world(vec![actor(1, 8, 6.), actor(2, 73, 100.), actor(3, 73, 0.)]);
        let mut i = input();
        i.other = Some(2);
        Machine::default().run(i, &mut services).unwrap();
        assert_eq!(services.events, [(0, 0), (1, 1)]);
        services.actors[1].active = 0x8000;
        services.events.clear();
        Machine::default().run(i, &mut services).unwrap();
        assert_eq!(services.events, [(0, 0), (1, 1), (2, 3)]);
    }
    #[test]
    fn callbacks_change_the_next_live_target_and_failure_cannot_replay() {
        let mut services = world(vec![actor(1, 8, 0.), actor(2, 73, 0.)]);
        services.change_next = true;
        Machine::default().run(input(), &mut services).unwrap();
        assert_eq!(services.events, [(0, 0), (1, 1), (2, 1)]);
        services.events.clear();
        services.fail = true;
        let mut machine = Machine::default();
        assert!(machine.run(input(), &mut services).is_err());
        assert_eq!(machine.phase(), Phase::Failed);
        assert_eq!(machine.failure(), Some("actual hit sink failed"));
        let before = services.events.clone();
        assert!(machine.run(input(), &mut services).is_err());
        assert_eq!(services.events, before);
    }
    #[test]
    fn missing_health_is_a_dependency_but_special_states_use_their_real_flag() {
        let mut a = actor(1, 73, 0.);
        a.health = None;
        let mut services = world(vec![a]);
        assert!(Machine::default().run(input(), &mut services).is_err());
        services.actors[0].state = 74;
        services.events.clear();
        Machine::default().run(input(), &mut services).unwrap();
        assert_eq!(services.events, [(0, 0)]);
        services.actors[0].personality = 0x20000;
        services.events.clear();
        Machine::default().run(input(), &mut services).unwrap();
        assert_eq!(services.events, [(0, 0), (2, 1)]);
    }
    #[test]
    fn no_effect_proof_keeps_inclusive_hit_and_strict_alert_boundaries() {
        let params = [0., 30., 50., -1., 50.];
        let a = actor(1, 73, 30.);
        assert!(!no_effect_invariant(params, [0.; 3], None, &[a]).unwrap());
        let a = Actor {
            position: [30.0001, 0., 0.],
            health: None,
            ..a
        };
        assert!(no_effect_invariant(params, [0.; 3], None, &[a]).unwrap());
        let vimp = Actor {
            class: 8,
            position: [100., 0., 0.],
            ..a
        };
        assert!(no_effect_invariant(params, [0.; 3], Some(100.), &[vimp]).unwrap());
        assert!(!no_effect_invariant(params, [0.; 3], Some(100.001), &[vimp]).unwrap());
        assert!(no_effect_invariant(params, [0.; 3], None, &[vimp]).is_err());
        assert!(
            no_effect_invariant(
                params,
                [0.; 3],
                None,
                &[Actor {
                    active: 0x8000,
                    ..vimp
                }]
            )
            .unwrap()
        );
        assert!(
            no_effect_invariant::<u32>(
                [f32::NAN, f32::NAN, f32::NAN, -0., f32::NAN],
                [f32::NAN; 3],
                None,
                &[]
            )
            .unwrap()
        );
    }

    #[test]
    fn obstacle_is_a_proof_failure_and_not_a_positive_eligibility_receipt() {
        let mut actors = vec![actor(1, 73, 1000.), actor(2, 8, 50.), actor(3, 73, 10.)];
        for a in &mut actors {
            a.context = 0;
        }
        actors[1].health = Some([10., 10.]);
        let mut i = input();
        i.context = 0;
        i.cross_context = false;
        let obstacle = no_effect_obstacle(i.parameters, i.position, Some(100.), &actors)
            .unwrap()
            .unwrap();
        assert_eq!(
            (obstacle.index, obstacle.reason),
            (1, PossibleEffect::VimpAlertRadius)
        );
        assert_eq!(
            (obstacle.distance_squared, obstacle.bound_squared),
            (2500., 10_000.)
        );
        assert!(!no_effect_invariant(i.parameters, i.position, Some(100.), &actors).unwrap());
        let mut actual = world(actors);
        Machine::default().run(i, &mut actual).unwrap();
        assert!(actual.events.iter().all(|event| event.0 == 0));
    }

    #[test]
    fn obstacle_preserves_mask_and_inclusive_damage_boundary_without_health_or_alert_input() {
        let masked = Actor {
            active: 0x4000_0000,
            position: [f32::NAN; 3],
            ..actor(1, 8, 0.)
        };
        let target = Actor {
            health: None,
            ..actor(2, 73, 30.)
        };
        let obstacle =
            no_effect_obstacle([0., 30., 50., -1., 50.], [0.; 3], None, &[masked, target])
                .unwrap()
                .unwrap();
        assert_eq!(
            (obstacle.index, obstacle.reason),
            (1, PossibleEffect::DamageRadius)
        );
        assert_eq!(
            (obstacle.distance_squared, obstacle.bound_squared),
            (900., 900.)
        );
        assert!(
            no_effect_obstacle::<u32>(
                [f32::NAN, f32::NAN, f32::NAN, -0., f32::NAN],
                [f32::NAN; 3],
                None,
                &[]
            )
            .unwrap()
            .is_none()
        );
    }
    #[test]
    #[ignore = "Requires GIANTS_RADIAL_NO_EFFECT_CONTROLS; actual PC traversal returns or stops before a real counterexample callback; supplied headers/list/range, not a native world or gameplay"]
    fn no_effect_proof_is_conservative_against_original_context_branches() {
        let text =
            std::fs::read_to_string(std::env::var_os("GIANTS_RADIAL_NO_EFFECT_CONTROLS").unwrap())
                .unwrap();
        let mut count = 0;
        for line in text.lines() {
            let row = line
                .split_whitespace()
                .map(|w| u32::from_str_radix(w, 16).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(row.len(), 6);
            let a = Actor {
                class: row[0] as i32,
                context: if row[2] != 0 { 100 } else { 0 },
                active: if row[4] != 0 { 0x8000 } else { 0 },
                ..actor(1, 73, f32::from_bits(row[1]))
            };
            let proof =
                no_effect_invariant([0., 30., 50., -1., 50.], [0.; 3], Some(100.), &[a]).unwrap();
            if proof {
                assert_eq!(row[5], 1, "native callback reached at row {count}");
            }
            if row[2] == 0 && row[3] == 1 {
                assert_eq!(proof, row[5] != 0);
            }
            count += 1;
        }
        assert_eq!(count, 80);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Actor<I> {
    pub id: I,
    pub class: i32,
    pub state: u32,
    pub context: u32,
    pub active: u32,
    pub personality: u32,
    pub position: [f32; 3],
    /// Actual accumulated damage and limit. Special states do not read health;
    /// a missing ordinary health producer cannot become an invented zero.
    pub health: Option<[f32; 2]>,
}

pub trait Services<I> {
    fn first(&mut self) -> Result<Option<I>, String>;
    /// Queried after callbacks, preserving their actual list/actor effects.
    fn next(&mut self, current: I) -> Result<Option<I>, String>;
    fn actor(&mut self, id: I) -> Result<Actor<I>, String>;
    fn alert_vimp(&mut self, target: I, source: I) -> Result<(), String>;
    fn damage(&mut self, request: Hit<I>) -> Result<(), String>;
}

#[derive(Clone, Copy, Debug)]
pub struct Hit<I> {
    pub target: I,
    pub source: I,
    pub kind: u32,
    pub amount: f32,
    pub point: [f32; 3],
    pub tag: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Fresh,
    Running,
    Complete,
    Failed,
}

#[derive(Debug)]
pub struct Machine {
    phase: Phase,
    failure: Option<String>,
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            phase: Phase::Fresh,
            failure: None,
        }
    }
}

impl Machine {
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    /// Execute once; a failed callback retains its prefix and forbids replay.
    /// Completion means all required sinks ran, not a computed amount alone.
    pub fn run<I: Copy + Eq>(
        &mut self,
        input: Input<I>,
        services: &mut impl Services<I>,
    ) -> Result<(), String> {
        if self.phase != Phase::Fresh {
            return Err("radial traversal already consumed or failed".into());
        }
        self.phase = Phase::Running;
        let result = Self::run_inner(input, services);
        match &result {
            Ok(()) => self.phase = Phase::Complete,
            Err(error) => {
                self.phase = Phase::Failed;
                self.failure = Some(error.clone());
            }
        }
        result
    }

    fn run_inner<I: Copy + Eq>(
        input: Input<I>,
        services: &mut impl Services<I>,
    ) -> Result<(), String> {
        let [inner, outer, amount, slope, intercept] = input.parameters;
        if slope == 0. {
            return Ok(());
        }
        if !input
            .parameters
            .into_iter()
            .chain(input.position)
            .chain([input.vimp_alert_range])
            .all(f32::is_finite)
        {
            return Err("radial arithmetic outside finite profile".into());
        }
        let outer_squared = (f64::from(outer) * f64::from(outer)) as f32;
        if !outer_squared.is_finite() {
            return Err("radial outer radius square overflow".into());
        }
        let mut cursor = services.first()?;
        let mut visited = Vec::new();
        while let Some(id) = cursor {
            if visited.len() >= 65536 || visited.contains(&id) {
                return Err("radial list cycle or traversal bound".into());
            }
            visited.push(id);
            let actor = services.actor(id)?;
            if actor.id != id {
                return Err("radial actor identity changed".into());
            }
            let context = actor.context != input.context || input.cross_context;
            if context && actor.active & 0x4000c000 == 0 {
                let eligible = if matches!(actor.state, 74 | 32 | 83 | 82) {
                    actor.personality & 0x20000 != 0
                } else {
                    let [damage, limit] = actor
                        .health
                        .ok_or("radial current health producer absent")?;
                    if ![damage, limit].into_iter().all(f32::is_finite) {
                        return Err("radial target health outside finite profile".into());
                    }
                    damage < limit
                };
                if eligible {
                    if !actor.position.into_iter().all(f32::is_finite) {
                        return Err("radial target pose outside finite profile".into());
                    }
                    let delta: [f64; 3] = std::array::from_fn(|i| {
                        f64::from(actor.position[i]) - f64::from(input.position[i])
                    });
                    let distance =
                        ((delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]) as f32;
                    if !distance.is_finite() {
                        return Err("radial target distance overflow".into());
                    }
                    if input.other == Some(id) {
                        return Ok(());
                    }
                    if actor.class == 8
                        && f64::from(distance)
                            < f64::from(input.vimp_alert_range) * f64::from(input.vimp_alert_range)
                    {
                        services.alert_vimp(id, input.source)?;
                    }
                    if distance <= outer_squared {
                        let amount = if f64::from(inner) * f64::from(inner) <= f64::from(distance) {
                            (f64::from(distance).sqrt() * f64::from(slope) + f64::from(intercept))
                                as f32
                        } else {
                            amount
                        };
                        if !amount.is_finite() {
                            return Err("radial amount overflow".into());
                        }
                        services.damage(Hit {
                            target: id,
                            source: input.source,
                            kind: 2,
                            amount,
                            point: input.position,
                            tag: 0,
                        })?;
                    }
                }
            }
            cursor = services.next(id)?;
        }
        Ok(())
    }
}
