//! Explicit preview schedule for finite descendants of a hidden emitter.
//! Requests prepare their named random operands immediately and activate on the
//! next positive callback. This is not a claim about native global scheduling.
use crate::effect_emitter::{Configuration, Recipe};
#[derive(Clone, Copy, Debug)]
pub struct Presentation {
    pub attachment: Option<crate::effect_reference_emission::Attachment>,
    pub angles: Option<[f32; 3]>,
    pub position: [f32; 3],
    pub size: f32,
    pub transparency: f32,
    pub age: f32,
    pub serial: u64,
    pub expired: bool,
}
pub struct Particle {
    attachment: Option<crate::effect_reference_emission::Attachment>,
    configuration: Configuration,
    visual: crate::effect_visual::Visual,
    motion: crate::mecc_motion::Motion,
    velocity_initialized: bool,
    expired: bool,
    serial: u64,
    angles: Option<[f32; 3]>,
}
impl Particle {
    fn new(
        configuration: Configuration,
        position: [f32; 3],
        serial: u64,
    ) -> Result<Self, &'static str> {
        let visual = crate::effect_visual::Visual::new(configuration.visual)?;
        Ok(Self {
            attachment: None,
            configuration,
            visual,
            motion: crate::mecc_motion::Motion {
                position,
                velocity: [0.; 3],
                previous_velocity: [0.; 3],
                force: [0.; 3],
                mass: 1.,
                speed_limit: 0.,
                force_time_limit: 0.,
                stored_dt: 0.,
            },
            velocity_initialized: false,
            expired: false,
            serial,
            angles: None,
        })
    }
    fn advance(&mut self, dt: f32) -> Result<bool, &'static str> {
        if self.expired {
            return Ok(false);
        }
        match self.visual.advance(dt)? {
            crate::effect_visual::Step::FirstFrame => {}
            crate::effect_visual::Step::Expired => {
                self.expired = true;
                return Ok(true);
            }
            crate::effect_visual::Step::VisibleState => {
                if let Some(rates) = self.configuration.angular_velocity {
                    let initial = match self.angles {
                        Some(a) => a,
                        None => crate::effect_orientation::from_matrix(
                            crate::effect_orientation::matrix(
                                self.motion.position,
                                [0.; 3],
                                self.configuration.visual.start_size,
                            )?,
                        )?,
                    };
                    self.angles = Some(crate::effect_orientation::advance(initial, rates, dt)?);
                }
                if !self.velocity_initialized {
                    let matrix = crate::effect_orientation::matrix(
                        self.motion.position,
                        [0.; 3],
                        self.configuration.visual.start_size,
                    )?;
                    if self.configuration.side_speed.is_some() {
                        self.motion.velocity = crate::effect_emitter::initial_velocity(
                            matrix,
                            0.,
                            self.configuration.side_speed,
                        )?;
                    }
                    self.velocity_initialized = true;
                }
                if !self.configuration.physics_enabled {
                    return Ok(false);
                }
                self.motion = crate::mecc_motion::integrate(
                    self.motion,
                    dt,
                    false,
                    false,
                    crate::arithmetic::Precision::Double53,
                )?;
                let gravity = crate::mecc_gravity::apply(
                    crate::mecc_gravity::Input {
                        force: self.motion.force,
                        velocity: self.motion.velocity,
                        normal: [0.; 3],
                        gravity_mass: self.configuration.gravity_mass,
                        mass: 1.,
                        friction_limit: 0.,
                        damping_xy: 0.,
                        damping_z: 0.,
                        force_time_limit: self.motion.force_time_limit,
                        object_flags: 0,
                        physics_flags: 0,
                        project_contact: false,
                    },
                    crate::arithmetic::Precision::Double53,
                )?;
                self.motion.force = gravity.force;
                self.motion.force_time_limit = gravity.force_time_limit;
            }
        }
        Ok(false)
    }
    pub fn presentation(&self) -> Presentation {
        Presentation {
            attachment: self.attachment,
            angles: self.angles,
            position: self.motion.position,
            size: self.visual.size,
            transparency: self.visual.transparency,
            age: self.visual.age,
            serial: self.serial,
            expired: self.expired,
        }
    }
}
#[derive(Debug)]
pub struct Event {
    pub kind: &'static str,
    pub particle: usize,
    pub position: [f32; 3],
    pub serial: u64,
}
pub struct Runtime {
    pub recipe: std::sync::Arc<Recipe>,
    periodic: crate::effect_emission::Periodic,
    first: bool,
    slots: Vec<Vec<Option<Particle>>>,
    pending: Vec<Request>,
    serial: u64,
    growing: bool,
}
type Request = (
    usize,
    [f32; 3],
    Option<crate::effect_reference_emission::Attachment>,
);
impl Runtime {
    pub fn new(
        recipe: std::sync::Arc<Recipe>,
        reserve: usize,
        position: [f32; 3],
        seed: &mut u32,
    ) -> Result<Self, &'static str> {
        if !(1..=512).contains(&reserve) {
            return Err("invalid periodic slot reserve");
        }
        let periodic = recipe.create(seed, position)?;
        let slots = recipe
            .particles
            .iter()
            .map(|_| (0..reserve).map(|_| None).collect())
            .collect();
        Ok(Self {
            recipe,
            periodic,
            first: true,
            slots,
            pending: Vec::new(),
            serial: 0,
            growing: false,
        })
    }
    pub fn presentation(&self, particle: usize, slot: usize) -> Option<Presentation> {
        self.slots
            .get(particle)?
            .get(slot)?
            .as_ref()
            .map(Particle::presentation)
    }
    pub fn source_state(&self) -> crate::effect_emission::Periodic {
        self.periodic
    }
    pub fn slot_counts(&self) -> Vec<usize> {
        self.slots.iter().map(Vec::len).collect()
    }
    /// Allocate actual births, reusing expired slots. The realized interval is
    /// preserved; initial capacity does not impose a lifetime-based hard limit.
    pub fn sampled_growing_at(
        recipe: std::sync::Arc<Recipe>,
        interval: f32,
        constructor_position: [f32; 3],
    ) -> Result<Self, &'static str> {
        let mut runtime = Self::with_cache(recipe, 1, interval, constructor_position, false)?;
        runtime.growing = true;
        Ok(runtime)
    }
    /// Fixed command5b constructor profile: no interval RNG, cold position cache.
    pub fn fixed(recipe: std::sync::Arc<Recipe>, reserve: usize) -> Result<Self, &'static str> {
        if recipe.interval[0] != recipe.interval[1] {
            return Err("fixed emitter interval range");
        }
        let interval = recipe.interval[0];
        Self::with_cache(recipe, reserve, interval, [0.; 3], false)
    }
    /// The parent constructor already sampled the interval. Preserve its cache
    /// point without consuming RNG again; this is distinct from command5b cold0.
    pub fn sampled_at(
        recipe: std::sync::Arc<Recipe>,
        reserve: usize,
        interval: f32,
        constructor_position: [f32; 3],
    ) -> Result<Self, &'static str> {
        Self::with_cache(recipe, reserve, interval, constructor_position, true)
    }
    fn with_cache(
        recipe: std::sync::Arc<Recipe>,
        reserve: usize,
        interval: f32,
        constructor_position: [f32; 3],
        validate_position_reserve: bool,
    ) -> Result<Self, &'static str> {
        if !(1..=512).contains(&reserve)
            || recipe.interval.iter().any(|v| !v.is_finite() || *v < 0.)
            || !interval.is_finite()
            || interval <= 0.
            || interval < recipe.interval[0].min(recipe.interval[1])
            || interval > recipe.interval[0].max(recipe.interval[1])
            || constructor_position.iter().any(|v| !v.is_finite())
        {
            return Err("invalid sampled emitter reserve/interval/constructor position");
        }
        if validate_position_reserve {
            let mut realized = (*recipe).clone();
            realized.interval = [interval; 2];
            if realized.reserve(0.)? > reserve {
                return Err("sampled emitter requires more slots than prepared reserve");
            }
        }
        Ok(Self {
            periodic: crate::effect_emission::Periodic {
                interval,
                remaining: 0.,
                phase: 0.,
                previous_position: constructor_position,
            },
            first: true,
            slots: recipe
                .particles
                .iter()
                .map(|_| (0..reserve).map(|_| None).collect())
                .collect(),
            recipe,
            pending: Vec::new(),
            serial: 0,
            growing: false,
        })
    }
    pub fn advance(&mut self, dt: f32, seed: &mut u32) -> Result<Vec<Event>, &'static str> {
        self.advance_source(dt, self.periodic.previous_position, true, seed)
    }
    pub fn advance_source(
        &mut self,
        dt: f32,
        position: [f32; 3],
        emitting: bool,
        seed: &mut u32,
    ) -> Result<Vec<Event>, &'static str> {
        self.advance_requests(dt, seed, |periodic, _| {
            if !emitting {
                return Ok(Vec::new());
            }
            let step = periodic.advance(dt, position)?;
            *periodic = step.state;
            Ok(step.positions.into_iter().map(|p| (0, p, None)).collect())
        })
    }
    /// Reference-selected point requests use their own measured clock, never
    /// the position-only catch-up clock. Child preparation follows selection;
    /// activation/drainage retains the existing explicit preview schedule.
    pub fn advance_reference(
        &mut self,
        dt: f32,
        channel: &mut crate::effect_reference_emission::Channel,
        points: &std::collections::BTreeMap<u32, [f32; 3]>,
        emitting: bool,
        seed: &mut u32,
    ) -> Result<Vec<Event>, &'static str> {
        use crate::effect_reference_emission::{Mode, Selected};
        if !matches!(channel.mode, Mode::Single(_)) {
            return Err("paired emission context spatial backend pending");
        }
        self.advance_requests(dt, seed, |_, seed| {
            if !emitting {
                return Ok(Vec::new());
            }
            let step = channel.advance(dt, seed)?;
            *channel = step.state;
            match step.request {
                None => Ok(Vec::new()),
                Some(Selected::Single(index)) => {
                    let point = *points
                        .get(&index)
                        .ok_or("reference emission point absent")?;
                    if !point.into_iter().all(f32::is_finite) {
                        return Err("reference emission point nonfinite");
                    }
                    Ok(vec![(0, point, None)])
                }
                Some(Selected::Paired(_)) => Err("paired emission context spatial backend pending"),
            }
        })
    }
    /// Paired contexts retain a model-reference binding, not a detached point.
    /// Parent retirement propagates to attached children; simple channels keep
    /// their existing drainage path. Final matrices are resolved after the
    /// particle callback from current parent and child model poses.
    pub fn advance_paired(
        &mut self,
        dt: f32,
        channel: &mut crate::effect_reference_emission::Channel,
        parent_points: &std::collections::BTreeMap<u32, [f32; 3]>,
        parent_alive: bool,
        seed: &mut u32,
    ) -> Result<Vec<Event>, &'static str> {
        if !matches!(
            channel.mode,
            crate::effect_reference_emission::Mode::Paired(_)
        ) || self.recipe.particles.len() != 1
        {
            return Err("paired runtime requires one attached descendant");
        }
        self.recipe.particles[0].attachment_profile()?;
        if !dt.is_finite() || dt < 0. {
            return Err("invalid paired callback time");
        }
        if !parent_alive {
            self.pending.clear();
            let mut events = Vec::new();
            for (kind, slots) in self.slots.iter_mut().enumerate() {
                for particle in slots.iter_mut().flatten().filter(|p| !p.expired) {
                    particle.expired = true;
                    events.push(Event {
                        kind: "parent-retired",
                        particle: kind,
                        position: particle.motion.position,
                        serial: particle.serial,
                    });
                }
            }
            return Ok(events);
        }
        self.advance_requests(dt, seed, |_, seed| {
            let step = channel.advance(dt, seed)?;
            *channel = step.state;
            let Some(selected) = step.request else {
                return Ok(Vec::new());
            };
            let binding = selected.attachment()?;
            let point = *parent_points
                .get(&binding.parent_reference)
                .ok_or("paired parent reference point absent")?;
            if !point.into_iter().all(f32::is_finite) {
                return Err("paired parent reference point nonfinite");
            }
            Ok(vec![(0, point, Some(binding))])
        })
    }
    fn advance_requests(
        &mut self,
        dt: f32,
        seed: &mut u32,
        make_requests: impl FnOnce(
            &mut crate::effect_emission::Periodic,
            &mut u32,
        ) -> Result<Vec<Request>, &'static str>,
    ) -> Result<Vec<Event>, &'static str> {
        if !dt.is_finite() || dt < 0. {
            return Err("invalid periodic callback time");
        }
        if dt == 0. {
            return Ok(Vec::new());
        }
        if self.first {
            self.first = false;
            return Ok(Vec::new());
        }
        let mut events = Vec::new();
        for (kind, position, attachment) in std::mem::take(&mut self.pending) {
            let index = match self.slots[kind]
                .iter()
                .position(|s| s.as_ref().is_none_or(|p| p.expired))
            {
                Some(index) => index,
                None if self.growing => {
                    self.slots[kind]
                        .try_reserve(1)
                        .map_err(|_| "periodic particle allocation failed")?;
                    let index = self.slots[kind].len();
                    self.slots[kind].push(None);
                    index
                }
                None => {
                    return Err("periodic preview reserve exhausted; no request dropped silently");
                }
            };
            let slot = &mut self.slots[kind][index];
            let configuration = self.recipe.particles[kind].activate(seed)?;
            self.serial += 1;
            let mut particle = Particle::new(configuration, position, self.serial)?;
            particle.attachment = attachment;
            *slot = Some(particle);
            events.push(Event {
                kind: "activated",
                particle: kind,
                position,
                serial: self.serial,
            });
        }
        let mut retirements = Vec::new();
        for (kind, slots) in self.slots.iter_mut().enumerate() {
            for particle in slots.iter_mut().flatten() {
                if particle.advance(dt)? {
                    events.push(Event {
                        kind: "expired",
                        particle: kind,
                        position: particle.motion.position,
                        serial: particle.serial,
                    });
                    if self.recipe.particles[kind].retirement.is_some() {
                        let next = kind + 1;
                        if next >= self.recipe.particles.len() {
                            return Err("retirement projection absent");
                        }
                        retirements.push((next, particle.motion.position, None));
                    }
                }
            }
        }
        let mut requests = make_requests(&mut self.periodic, seed)?;
        requests.extend(retirements);
        if requests.len() + self.pending.len() > 4096 {
            return Err("periodic pending request budget");
        }
        for (kind, position, attachment) in requests {
            self.recipe.particles[kind].prepare(seed);
            events.push(Event {
                kind: "requested",
                particle: kind,
                position,
                serial: 0,
            });
            self.pending.push((kind, position, attachment));
        }
        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn growing_pool_crosses_fixed_capacity_and_reuses_expired_slots() {
        let mut seed = 1;
        let position = [10., 20., 30.];
        let mut leaf = (*recipe(0.001)).clone();
        leaf.particles.truncate(1);
        leaf.particles[0].retirement = None;
        let mut pool =
            super::Runtime::sampled_growing_at(std::sync::Arc::new(leaf), 0.001, position).unwrap();
        assert_eq!(pool.slot_counts(), vec![1]);
        assert!(
            pool.advance_source(0.01, position, true, &mut seed)
                .unwrap()
                .is_empty()
        );
        let requests = pool.advance_source(0.6, position, true, &mut seed).unwrap();
        let count = requests.len();
        assert!(count > 512);
        let born = pool
            .advance_source(0.01, position, false, &mut seed)
            .unwrap();
        assert_eq!(born.iter().filter(|e| e.kind == "activated").count(), count);
        assert_eq!(pool.slot_counts(), vec![count]);
        for (slot, event) in born.iter().filter(|e| e.kind == "activated").enumerate() {
            assert_eq!(event.serial, slot as u64 + 1);
            assert_eq!(pool.presentation(0, slot).unwrap().serial, event.serial);
        }
        for _ in 0..40 {
            pool.advance_source(0.01, position, false, &mut seed)
                .unwrap();
        }
        assert!((0..count).all(|slot| pool.presentation(0, slot).unwrap().expired));
        let requested = pool.advance_source(0.6, position, true, &mut seed).unwrap();
        let second = pool
            .advance_source(0.01, position, false, &mut seed)
            .unwrap();
        assert_eq!(
            second.iter().filter(|e| e.kind == "activated").count(),
            requested.len()
        );
        assert_eq!(pool.slot_counts(), vec![count.max(requested.len())]);
        assert_eq!(pool.source_state().interval, 0.001);
    }
    #[test]
    fn paired_parent_retirement_cancels_pending_and_live_children_without_rng() {
        use crate::effect_reference_emission::{Channel, Mode};
        let mut recipe = (*recipe(0.001)).clone();
        recipe.particles = vec![recipe.particles.last().unwrap().clone()];
        let mut runtime = super::Runtime::fixed(std::sync::Arc::new(recipe), 8).unwrap();
        let mut channel = Channel {
            interval: 0.001,
            remaining: 0.,
            mode: Mode::Paired([[0, 0], [9, 9]]),
        };
        let points = std::collections::BTreeMap::from([(9, [12., 8., 5.])]);
        let mut seed = 1;
        runtime
            .advance_paired(0.01, &mut channel, &points, true, &mut seed)
            .unwrap();
        let requested = runtime
            .advance_paired(0.01, &mut channel, &points, true, &mut seed)
            .unwrap();
        assert_eq!(requested[0].kind, "requested");
        let events = runtime
            .advance_paired(0.01, &mut channel, &points, true, &mut seed)
            .unwrap();
        assert_eq!(events[0].kind, "activated");
        assert_eq!(
            runtime
                .presentation(0, 0)
                .unwrap()
                .attachment
                .unwrap()
                .parent_reference,
            9
        );
        assert!(!runtime.pending.is_empty());
        let saved = seed;
        let events = runtime
            .advance_paired(0.01, &mut channel, &points, false, &mut seed)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, "parent-retired");
        assert!(runtime.presentation(0, 0).unwrap().expired);
        assert!(runtime.pending.is_empty());
        assert_eq!(seed, saved);
        assert!(
            runtime
                .advance_paired(0.01, &mut channel, &points, false, &mut seed)
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn angular_particle_turns_after_first_frame_without_enabling_physics() {
        let mut p = super::Particle::new(
            crate::effect_emitter::Configuration {
                class: 900,
                angular_velocity: Some([10., 20., 30.]),
                visual: crate::effect_visual::Configuration {
                    lifetime: 1.,
                    fade_in: 0.,
                    fade_out_start: 0.,
                    start_size: 2.,
                    end_size: 2.,
                },
                gravity_mass: 1.,
                side_speed: None,
                physics_enabled: false,
            },
            [4., 5., 6.],
            1,
        )
        .unwrap();
        p.advance(0.1).unwrap();
        assert!(p.presentation().angles.is_none());
        p.advance(0.25).unwrap();
        assert_eq!(p.presentation().angles, Some([2.5, 5., 7.5]));
        assert_eq!(p.presentation().position, [4., 5., 6.]);
        assert_eq!(p.presentation().age, 0.25);
    }
    fn recipe(interval: f32) -> std::sync::Arc<crate::effect_emitter::Recipe> {
        let mut root = vec![3];
        root.extend(900u32.to_le_bytes());
        root.extend([6, 0x21, 0x1e]);
        root.extend(b"child\0");
        root.extend(interval.to_le_bytes());
        root.extend(interval.to_le_bytes());
        root.extend([7, 2]);
        let body = |duration: f32, retirement: Option<&str>| {
            let mut b = vec![3];
            b.extend(901u32.to_le_bytes());
            b.extend([6, 4]);
            for value in [duration, 0., duration * 0.5] {
                b.extend(value.to_le_bytes());
            }
            if let Some(label) = retirement {
                b.push(0xd);
                b.extend(label.as_bytes());
                b.push(0);
            }
            b.extend([7, 2]);
            b
        };
        let programs = [
            ("root", root),
            ("child", body(0.02, Some("tail"))),
            ("tail", body(0.2, None)),
        ];
        let mut payload = Vec::new();
        let mut at = programs.iter().map(|(n, _)| n.len() + 5).sum::<usize>();
        for (name, body) in &programs {
            payload.extend(name.as_bytes());
            payload.push(0);
            payload.extend((at as u32).to_le_bytes());
            at += body.len();
        }
        for (_, body) in programs {
            payload.extend(body);
        }
        let mut bytes = 1u32.to_le_bytes().to_vec();
        bytes.extend((payload.len() as u32).to_le_bytes());
        bytes.extend(3u32.to_le_bytes());
        bytes.extend(payload);
        let image = giants_formats::effect_image::parse(&bytes).unwrap();
        std::sync::Arc::new(crate::effect_emitter::Recipe::project(&image, "root").unwrap())
    }
    #[test]
    fn first_callback_pause_and_single_retirement_are_preserved() {
        let mut seed = 1;
        let mut r = super::Runtime::new(recipe(1.), 2, [12.5, -40., 3.25], &mut seed).unwrap();
        let initial_seed = seed;
        assert!(r.advance(0., &mut seed).unwrap().is_empty());
        assert_eq!(seed, initial_seed);
        assert!(r.advance(0.01, &mut seed).unwrap().is_empty());
        assert_eq!(r.advance(0.01, &mut seed).unwrap()[0].kind, "requested");
        assert_eq!(r.advance(0.01, &mut seed).unwrap()[0].kind, "activated");
        let initial = r.presentation(0, 0).unwrap();
        assert_eq!(initial.age, 0.);
        assert_eq!(initial.position, [12.5, -40., 3.25]);
        assert!(r.advance(0.01, &mut seed).unwrap().is_empty());
        let events = r.advance(0.01, &mut seed).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, "expired");
        assert_eq!(events[1].particle, 1);
        let events = r.advance(0.01, &mut seed).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, "activated");
        assert!(r.presentation(0, 0).unwrap().expired);
        assert_eq!(r.presentation(1, 0).unwrap().age, 0.);
    }
    #[test]
    fn allocation_exhaustion_is_an_error() {
        let mut seed = 1;
        let mut r = super::Runtime::new(recipe(0.001), 1, [0.; 3], &mut seed).unwrap();
        r.advance(0.1, &mut seed).unwrap();
        r.advance(0.1, &mut seed).unwrap();
        assert!(
            r.advance(0.1, &mut seed)
                .unwrap_err()
                .contains("reserve exhausted")
        );
    }
    #[test]
    fn reference_requests_select_one_point_then_activate_and_drain() {
        use crate::effect_reference_emission::{Channel, Mode};
        let mut pool = super::Runtime::fixed(recipe(0.001), 4).unwrap();
        let points = std::collections::BTreeMap::from([(5, [10., 20., 30.])]);
        let mut channel = Channel {
            interval: 0.5,
            remaining: 0.,
            mode: Mode::Paired([[5, 5]; 2]),
        };
        let mut seed = 1;
        assert!(
            pool.advance_reference(0.01, &mut channel, &points, true, &mut seed)
                .is_err()
        );
        assert_eq!(seed, 1);
        channel.mode = Mode::Single([5, 5]);
        assert!(
            pool.advance_reference(0.01, &mut channel, &points, true, &mut seed)
                .unwrap()
                .is_empty()
        );
        assert_eq!(seed, 1);
        let requested = pool
            .advance_reference(2., &mut channel, &points, true, &mut seed)
            .unwrap();
        assert_eq!(requested.len(), 1);
        assert_eq!(requested[0].position, [10., 20., 30.]);
        assert_eq!(channel.remaining, 0.5);
        let saved = seed;
        let activated = pool
            .advance_reference(0.01, &mut channel, &points, false, &mut seed)
            .unwrap();
        assert_eq!(activated[0].kind, "activated");
        assert_eq!(activated[0].position, [10., 20., 30.]);
        assert_eq!(seed, saved);
        for _ in 0..30 {
            pool.advance_reference(0.01, &mut channel, &points, false, &mut seed)
                .unwrap();
        }
        assert!(pool.presentation(0, 0).unwrap().expired);
        assert_eq!(channel.remaining, 0.5);
    }
    #[test]
    fn fixed_moving_source_keeps_cold_cache_and_drains_after_parent_stops() {
        let mut seed = 1;
        let mut r = super::Runtime::fixed(recipe(0.05), 8).unwrap();
        assert_eq!(seed, 1);
        assert_eq!(r.source_state().previous_position, [0.; 3]);
        assert!(
            r.advance_source(0.01, [10., 20., 30.], true, &mut seed)
                .unwrap()
                .is_empty()
        );
        let events = r
            .advance_source(0.01, [10., 20., 30.], true, &mut seed)
            .unwrap();
        assert_eq!(events[0].kind, "requested");
        assert_eq!(events[0].position, [50., 100., 150.]);
        let events = r
            .advance_source(0.01, [12., 24., 36.], false, &mut seed)
            .unwrap();
        assert_eq!(events[0].kind, "activated");
        for _ in 0..6 {
            assert!(
                r.advance_source(0.01, [12., 24., 36.], false, &mut seed)
                    .unwrap()
                    .iter()
                    .all(|e| e.kind != "requested" || e.particle != 0)
            );
        }
    }
    #[test]
    fn sampled_interval_keeps_nonzero_constructor_cache_without_another_draw() {
        let mut data = (*recipe(0.05)).clone();
        data.interval = [0.1, 0.];
        let data = std::sync::Arc::new(data);
        let mut seed = 1;
        let position = [10., 20., 30.];
        let mut pool = super::Runtime::sampled_at(data.clone(), 8, 0.07, position).unwrap();
        assert_eq!(pool.source_state().previous_position, position);
        assert_eq!(pool.source_state().interval, 0.07);
        assert!(
            pool.advance_source(0.01, position, true, &mut seed)
                .unwrap()
                .is_empty()
        );
        assert_eq!(seed, 1);
        let requests = pool
            .advance_source(0.01, position, true, &mut seed)
            .unwrap();
        assert_eq!(requests[0].kind, "requested");
        assert_eq!(requests[0].position, position);
        let seed_after_preparation = seed;
        let births = pool
            .advance_source(0.01, position, false, &mut seed)
            .unwrap();
        assert_eq!(births[0].kind, "activated");
        assert_eq!(births[0].position, position);
        assert_eq!(seed, seed_after_preparation);
        for _ in 0..20 {
            pool.advance_source(0.01, position, false, &mut seed)
                .unwrap();
        }
        assert!(pool.presentation(0, 0).unwrap().expired);
        assert!(super::Runtime::fixed(data.clone(), 8).is_err());
        assert!(super::Runtime::sampled_at(data.clone(), 8, 0., position).is_err());
        assert!(super::Runtime::sampled_at(data.clone(), 8, 0.000001, position).is_err());
        assert!(super::Runtime::sampled_at(data.clone(), 8, 0.2, position).is_err());
        assert!(super::Runtime::sampled_at(data, 8, 0.07, [f32::NAN; 3]).is_err());
    }
}
