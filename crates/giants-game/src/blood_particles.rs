//! Debug-reference shared blood/trail pool. Source clock and ground height are
//! supplied by the scene. This is not the unrelated Flick free-particle state.
use crate::audio_parameters::Random;
use giants_formats::global_data::GlobalData;
pub const CAPACITY: usize = 500;
pub const TEXTURE: &str = "bloodblob.tga";
#[derive(Clone, Copy, Debug)]
pub struct Configuration {
    pub initial_delay: f32,
    pub fade_seconds: f32,
    pub gravity: f32,
    pub friction: f32,
    pub size: [f32; 2],
    pub vertical_speed: [f32; 2],
    pub count: [u32; 2],
    pub spread: f32,
}
impl Configuration {
    pub fn from_globals(g: &GlobalData) -> Result<Self, String> {
        let scalar = |at| g.scalar(at).map_err(|e| e.to_string());
        let word = |at: usize| {
            g.words
                .get(at / 4)
                .copied()
                .ok_or("blood count outside global table".to_string())
        };
        let config = Self {
            initial_delay: scalar(0x2a4)?,
            fade_seconds: scalar(0x2a8)?,
            gravity: scalar(0x2ac)?,
            friction: scalar(0x2b0)?,
            size: [scalar(0x2b4)?, scalar(0x2b8)?],
            vertical_speed: [scalar(0x2bc)?, scalar(0x2c0)?],
            count: [word(0x2c4)?, word(0x2c8)?],
            spread: scalar(0x3d0)?,
        };
        config.validate()?;
        Ok(config)
    }
    fn validate(self) -> Result<(), String> {
        if ![
            self.initial_delay,
            self.fade_seconds,
            self.gravity,
            self.friction,
            self.size[0],
            self.size[1],
            self.vertical_speed[0],
            self.vertical_speed[1],
            self.spread,
        ]
        .into_iter()
        .all(f32::is_finite)
            || self.initial_delay < 0.
            || self.fade_seconds <= 0.
            || self.gravity < 0.
            || self.friction < 0.
            || self.spread < 0.
            || self.size[0] < 0.
            || self.size[0] > self.size[1]
            || self.vertical_speed[0] > self.vertical_speed[1]
            || self.count[0] > self.count[1]
            || self.count[1] > CAPACITY as u32
        {
            return Err("blood parameters outside finite bounded debug profile".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Particle {
    pub next: Option<usize>,
    pub textured: bool,
    pub position: [f32; 3],
    pub size: f32,
    pub phase: f32,
    pub velocity: [f32; 3],
    pub flags: u32,
    pub life: f32,
    pub trail_clock: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Actor {
    pub position: [f32; 3],
    pub scale: f32,
    pub center: f32,
    pub height: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Quad {
    pub position: [f32; 3],
    pub textured: bool,
    pub half_width: f32,
    pub half_height: f32,
    pub alpha: u8,
}
pub struct Pool {
    configuration: Configuration,
    nodes: Vec<Particle>,
    active: Option<usize>,
    free: Option<usize>,
}
fn interval(random: &mut Random, low: f32, high: f32) -> f64 {
    (f64::from(random.next_15()) * (f64::from(high) - f64::from(low))) * (1. / 32768.)
        + f64::from(low)
}
fn uniform(random: &mut Random, maximum: f32) -> f32 {
    ((f64::from(random.next_15()) * f64::from(maximum)) * (1. / 32768.)) as f32
}
impl Pool {
    pub fn new(configuration: Configuration) -> Result<Self, String> {
        configuration.validate()?;
        let nodes = (0..CAPACITY)
            .map(|i| Particle {
                next: (i + 1 < CAPACITY).then_some(i + 1),
                ..Default::default()
            })
            .collect();
        Ok(Self {
            configuration,
            nodes,
            active: None,
            free: Some(0),
        })
    }
    pub fn active(&self) -> impl Iterator<Item = (usize, &Particle)> {
        std::iter::successors(self.active, |i| self.nodes[*i].next).map(|i| (i, &self.nodes[i]))
    }
    pub fn nodes(&self) -> &[Particle] {
        &self.nodes
    }
    pub fn heads(&self) -> (Option<usize>, Option<usize>) {
        (self.active, self.free)
    }
    fn allocate(&mut self) -> Result<usize, String> {
        let index = if let Some(index) = self.free {
            self.free = self.nodes[index].next;
            index
        } else {
            let mut previous = self
                .active
                .ok_or("blood pool has neither active nor free nodes")?;
            loop {
                let next = self.nodes[previous]
                    .next
                    .ok_or("blood reuse requires at least two active nodes")?;
                if self.nodes[next].next.is_none() {
                    self.nodes[previous].next = None;
                    break next;
                }
                previous = next;
            }
        };
        self.nodes[index] = Particle {
            next: self.active,
            size: 1.,
            ..Default::default()
        };
        self.active = Some(index);
        Ok(index)
    }
    pub fn emit(&mut self, actor: Actor, random: &mut Random) -> Result<usize, String> {
        if !actor
            .position
            .into_iter()
            .chain([actor.scale, actor.center, actor.height])
            .all(f32::is_finite)
            || actor.scale <= 0.
            || actor.height < 0.
        {
            return Err("blood actor outside finite positive-scale profile".into());
        }
        let c = self.configuration;
        let count = c.count[0] + ((random.next_15() * (c.count[1] - c.count[0] + 1)) >> 15);
        let base = (f64::from(actor.scale) * f64::from(actor.height) * f64::from(0.1_f32)) as f32;
        for _ in 0..count {
            let i = self.allocate()?;
            self.nodes[i].position = actor.position;
            self.nodes[i].textured = true;
            self.nodes[i].position[2] = (f64::from(actor.position[2])
                + f64::from(actor.scale) * f64::from(actor.center))
                as f32;
            self.nodes[i].size = (interval(random, c.size[0], c.size[1]) * f64::from(base)) as f32;
            self.nodes[i].velocity[0] =
                (interval(random, -c.spread, c.spread) * f64::from(base)) as f32;
            self.nodes[i].velocity[1] =
                (interval(random, -c.spread, c.spread) * f64::from(base)) as f32;
            self.nodes[i].velocity[2] = (interval(random, c.vertical_speed[0], c.vertical_speed[1])
                * f64::from(base)) as f32;
            self.nodes[i].life = (f64::from(c.initial_delay) + f64::from(c.fade_seconds)) as f32;
            self.nodes[i].phase = uniform(random, 1.);
            self.nodes[i].flags = 4;
        }
        Ok(count as usize)
    }
    /// `clock` is evaluated twice for an emitting particle: once before motion,
    /// once at its updated position for the trail timer, as in the native loop.
    pub fn update(
        &mut self,
        random: &mut Random,
        mut clock: impl FnMut([f32; 3]) -> Result<f32, String>,
        mut ground: impl FnMut([f32; 3]) -> Result<f32, String>,
    ) -> Result<(), String> {
        let c = self.configuration;
        let mut cursor = self.active;
        let mut visited = 0;
        while let Some(i) = cursor {
            visited += 1;
            if visited > CAPACITY * CAPACITY {
                return Err("blood linked traversal budget".into());
            }
            let dt = clock(self.nodes[i].position)?;
            if !dt.is_finite() || dt < 0. {
                return Err("blood clock outside finite nonnegative profile".into());
            }
            let p = &mut self.nodes[i];
            p.velocity[2] =
                (f64::from(p.velocity[2]) - f64::from(c.gravity) * f64::from(dt)) as f32;
            for v in &mut p.velocity[..2] {
                if *v != 0. {
                    let next = if *v > 0. {
                        f64::from(*v) - f64::from(c.friction) * f64::from(dt)
                    } else {
                        f64::from(*v) + f64::from(c.friction) * f64::from(dt)
                    };
                    *v = if *v > 0. && next <= 0. || *v < 0. && next >= 0. {
                        0.
                    } else {
                        next as f32
                    };
                }
            }
            for axis in 0..3 {
                p.position[axis] = (f64::from(p.position[axis])
                    + f64::from(dt) * f64::from(p.velocity[axis]))
                    as f32;
            }
            let phase = f64::from(p.phase) + f64::from(dt) * f64::from(0.25_f32);
            p.phase = if phase >= 1. {
                (phase - 1.) as f32
            } else {
                phase as f32
            };
            if p.phase >= 1. {
                p.phase = (f64::from(p.phase) - 1.) as f32;
            }
            if self.nodes[i].flags & 4 != 0 {
                let trail_dt = clock(self.nodes[i].position)?;
                if !trail_dt.is_finite() || trail_dt < 0. {
                    return Err("blood trail clock outside profile".into());
                }
                let left = f64::from(self.nodes[i].trail_clock) - f64::from(trail_dt);
                self.nodes[i].trail_clock = left as f32;
                if left <= 0. {
                    self.nodes[i].trail_clock = uniform(random, 0.25);
                    let new = self.allocate()?;
                    // Read after allocation: saturated reuse may alias a node
                    // currently being traversed; do not replace by cloned data.
                    self.nodes[new].position = self.nodes[i].position;
                    self.nodes[new].textured = self.nodes[i].textured;
                    self.nodes[new].size =
                        (f64::from(self.nodes[i].size) * f64::from(0.75_f32)) as f32;
                    self.nodes[new].velocity[0] =
                        (f64::from(self.nodes[i].velocity[0]) * f64::from(0.9_f32)) as f32;
                    self.nodes[new].velocity[1] =
                        (f64::from(self.nodes[i].velocity[1]) * f64::from(0.9_f32)) as f32;
                    self.nodes[new].velocity[2] =
                        (f64::from(self.nodes[i].velocity[2]) * f64::from(0.8_f32)) as f32;
                    self.nodes[new].life = self.nodes[i].life;
                    self.nodes[new].phase = uniform(random, 1.);
                }
            }
            self.nodes[i].life = (f64::from(self.nodes[i].life) - f64::from(dt)) as f32;
            let floor = ground(self.nodes[i].position)?;
            if !floor.is_finite() {
                return Err("blood ground outside finite profile".into());
            }
            if floor >= self.nodes[i].position[2] || self.nodes[i].life <= 0. {
                self.nodes[i].flags |= 1;
            }
            cursor = self.nodes[i].next;
        }
        Ok(())
    }
    pub fn purge(&mut self) {
        let mut previous: Option<usize> = None;
        let mut cursor = self.active;
        while let Some(i) = cursor {
            let next = self.nodes[i].next;
            if self.nodes[i].flags & 1 != 0 {
                if let Some(p) = previous {
                    self.nodes[p].next = next;
                } else {
                    self.active = next;
                }
                self.nodes[i].next = self.free;
                self.free = Some(i);
            } else {
                previous = Some(i);
            }
            cursor = next;
        }
    }
    pub fn quads(&self) -> impl Iterator<Item = Quad> {
        self.active().filter_map(|(_, p)| {
            let fade =
                (1. - f64::from(p.life) / f64::from(self.configuration.fade_seconds)).max(0.);
            if fade >= 1. {
                return None;
            }
            let mix = (f64::from(p.phase) + 1.) * f64::from(0.8_f32);
            let mix = if mix > f64::from(1.2_f32) {
                f64::from(2.4_f32) - mix
            } else {
                mix
            };
            let other = (2. - mix) as f32;
            Some(Quad {
                position: p.position,
                textured: p.textured,
                half_width: (mix * f64::from(p.size)) as f32,
                half_height: (f64::from(other) * f64::from(p.size)) as f32,
                alpha: ((1. - fade) * 255.) as u8,
            })
        })
    }
}
