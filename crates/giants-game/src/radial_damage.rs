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
