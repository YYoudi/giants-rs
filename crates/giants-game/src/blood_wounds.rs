//! Attached impact wounds, separate from the shared blood-particle pool.
//! Current owner clocks, animation joints and transforms are scene providers.
use crate::{audio_parameters::Random, blood_particles, contact_pose::Cached, skinned_hit_point};
use giants_formats::global_data::GlobalData;

pub const CAPACITY: usize = 50;
#[derive(Clone, Copy, Debug)]
pub struct Configuration {
    pub life: f32,
    pub count_rate: [i32; 2],
    pub spread: f32,
    pub speed: [f32; 2],
    pub vertical_speed: [f32; 2],
    pub emitting_seconds: [f32; 2],
    pub paused_seconds: [f32; 2],
}
impl Configuration {
    pub fn from_globals(g: &GlobalData) -> Result<Self, String> {
        let f = |at| g.scalar(at).map_err(|e| e.to_string());
        let word = |at: usize| {
            g.words
                .get(at / 4)
                .copied()
                .map(|v| v as i32)
                .ok_or("wound rate outside global table")
        };
        let configuration = Self {
            life: f(0x560)?,
            count_rate: [word(0x564)?, word(0x568)?],
            spread: f(0x56c)?,
            speed: [f(0x570)?, f(0x574)?],
            vertical_speed: [f(0x578)?, f(0x57c)?],
            emitting_seconds: [f(0x580)?, f(0x584)?],
            paused_seconds: [f(0x588)?, f(0x58c)?],
        };
        configuration.validate()?;
        Ok(configuration)
    }
    fn validate(self) -> Result<(), String> {
        if ![self.life, self.spread]
            .into_iter()
            .chain(self.speed)
            .chain(self.vertical_speed)
            .chain(self.emitting_seconds)
            .chain(self.paused_seconds)
            .all(f32::is_finite)
            || self.life < 0.
            || self.spread < 0.
            || [self.speed, self.vertical_speed]
                .into_iter()
                .any(|[low, high]| low > high)
            || self.count_rate[0] < 0
            || self.count_rate[1] < self.count_rate[0]
            || self
                .emitting_seconds
                .into_iter()
                .chain(self.paused_seconds)
                .any(|time| time < 0.)
        {
            return Err("wound configuration outside finite acquired profile".into());
        }
        Ok(())
    }
}
pub struct Owner<J> {
    pub seconds: f32,
    pub retired: bool,
    pub position: [f32; 3],
    pub pose: Cached,
    /// None represents an actual empty animation-frame selection.
    pub joints: Option<Vec<skinned_hit_point::Joint<J>>>,
}
pub trait Scene<K, J> {
    fn owner(&mut self, id: K) -> Result<Owner<J>, String>;
}
pub struct Wound<K, J> {
    pub owner: K,
    pub binding: skinned_hit_point::Binding<J>,
    pub paused: bool,
    pub clock: f32,
    pub life: f32,
}
pub struct Pool<K, J> {
    configuration: Configuration,
    nodes: Vec<Option<Wound<K, J>>>,
    active: Vec<usize>,
    free: Vec<usize>,
}
impl<K: Copy, J: Clone + Eq> Pool<K, J> {
    pub fn new(configuration: Configuration) -> Result<Self, String> {
        configuration.validate()?;
        Ok(Self {
            configuration,
            nodes: (0..CAPACITY).map(|_| None).collect(),
            active: Vec::new(),
            free: (0..CAPACITY).rev().collect(),
        })
    }
    pub fn active(&self) -> impl Iterator<Item = (usize, &Wound<K, J>)> {
        self.active
            .iter()
            .map(|&index| (index, self.nodes[index].as_ref().unwrap()))
    }
    pub fn attach(&mut self, owner: K, point: [f32; 3], pose: Cached) -> Result<usize, String> {
        let binding = skinned_hit_point::Binding::new(pose.to_local(point)?)?;
        let index = self
            .free
            .pop()
            .or_else(|| self.active.pop())
            .ok_or("wound pool ownership lost")?;
        self.nodes[index] = Some(Wound {
            owner,
            binding,
            paused: false,
            clock: 0.,
            life: self.configuration.life,
        });
        self.active.insert(0, index);
        Ok(index)
    }
    pub fn step(
        &mut self,
        global_seconds: f32,
        scene: &mut impl Scene<K, J>,
        particles: &mut blood_particles::Pool,
        random: &mut Random,
    ) -> Result<usize, String> {
        if !global_seconds.is_finite() || global_seconds < 0. {
            return Err("wound global clock outside finite nonnegative profile".into());
        }
        let c = self.configuration;
        let mut emitted = 0;
        for &index in &self.active {
            let wound = self.nodes[index].as_mut().unwrap();
            let owner = scene.owner(wound.owner)?;
            if !owner.seconds.is_finite()
                || owner.seconds < 0.
                || !owner.position.into_iter().all(f32::is_finite)
            {
                return Err("wound owner clock/position outside acquired profile".into());
            }
            wound.life = (f64::from(wound.life) - f64::from(owner.seconds)) as f32;
            if owner.retired {
                wound.life = 0.;
                continue;
            }
            let clock = f64::from(wound.clock) - f64::from(owner.seconds);
            wound.clock = clock as f32;
            if !wound.paused && clock < 0. {
                wound.clock = interval(random, c.paused_seconds);
                wound.paused = true;
                continue;
            }
            if wound.paused {
                if clock > 0. {
                    continue;
                }
                wound.clock = interval(random, c.emitting_seconds);
                wound.paused = false;
            }
            let Some(joints) = owner.joints else {
                continue;
            };
            wound.binding.advance(&joints)?;
            let position = owner.pose.to_world(wound.binding.position())?;
            let heading = crate::actor_volume::heading_wide(owner.position, position);
            let rate = c
                .count_rate
                .map(|rate| f64::from(rate) * f64::from(global_seconds));
            if rate.into_iter().any(|count| count < 0. || count > 4096.) {
                return Err("wound count outside bounded particle emission profile".into());
            }
            emitted += particles.emit_directional(
                blood_particles::Directional {
                    position,
                    count: [(rate[0] as i32).max(1), rate[1] as i32],
                    heading: [
                        (heading - f64::from(c.spread)) as f32,
                        (heading + f64::from(c.spread)) as f32,
                    ],
                    speed: c.speed,
                    vertical_speed: c.vertical_speed,
                },
                random,
            )?;
        }
        self.purge();
        Ok(emitted)
    }
    /// Reference invalidation is a separate lifecycle phase, before owners
    /// disappear. It does not silently acknowledge a missing scene provider.
    pub fn retire_owner(&mut self, mut retired: impl FnMut(K) -> bool) {
        for &index in &self.active {
            let wound = self.nodes[index].as_mut().unwrap();
            if retired(wound.owner) {
                wound.life = 0.;
            }
        }
        self.purge();
    }
    fn purge(&mut self) {
        self.active.retain(|&index| {
            if self.nodes[index].as_ref().unwrap().life <= 0. {
                self.nodes[index] = None;
                self.free.push(index);
                false
            } else {
                true
            }
        });
    }
}
fn interval(random: &mut Random, [low, high]: [f32; 2]) -> f32 {
    ((f64::from(random.next_15()) * (f64::from(high) - f64::from(low))) * (1. / 32768.)
        + f64::from(low)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Configuration {
        Configuration {
            life: 1.,
            count_rate: [90, 150],
            spread: 10.,
            speed: [8., 12.],
            vertical_speed: [9., 13.],
            emitting_seconds: [0.1, 0.5],
            paused_seconds: [0.1, 0.],
        }
    }
    fn pose() -> Cached {
        Cached::new(0x20, 16, std::array::from_fn(|i| f32::from(i / 4 == i % 4))).unwrap()
    }
    #[test]
    fn separate_fifty_wound_budget_reuses_oldest_and_releases_owner_references() {
        let mut pool = Pool::<u32, String>::new(config()).unwrap();
        for owner in 0..51 {
            pool.attach(owner, [0.; 3], pose()).unwrap();
        }
        assert_eq!(pool.active().count(), 50);
        assert_eq!(pool.active().next().unwrap().1.owner, 50);
        assert!(pool.active().all(|(_, w)| w.owner != 0));
        pool.retire_owner(|id| id % 2 == 0);
        assert_eq!(pool.active().count(), 25);
        assert!(pool.active().all(|(_, w)| w.owner % 2 == 1));
        pool.attach(99, [1.; 3], pose()).unwrap();
        assert_eq!(pool.active().count(), 26);
        pool.retire_owner(|_| true);
        assert_eq!(pool.active().count(), 0);
    }
    struct EmptyFrames;
    impl Scene<u32, String> for EmptyFrames {
        fn owner(&mut self, _: u32) -> Result<Owner<String>, String> {
            Ok(Owner {
                seconds: 0.,
                retired: false,
                position: [0.; 3],
                pose: pose(),
                joints: None,
            })
        }
    }
    #[test]
    fn actual_empty_animation_frame_does_not_invent_a_joint_or_emit_particles() {
        let mut pool = Pool::<u32, String>::new(config()).unwrap();
        pool.attach(1, [1.; 3], pose()).unwrap();
        let mut particles = blood_particles::Pool::new(blood_particles::Configuration {
            initial_delay: 0.1,
            fade_seconds: 1.,
            gravity: 10.,
            friction: 2.,
            size: [0.5, 1.],
            vertical_speed: [2., 3.],
            count: [1, 1],
            spread: 1.,
        })
        .unwrap();
        let mut random = Random(123);
        assert_eq!(
            pool.step(0., &mut EmptyFrames, &mut particles, &mut random)
                .unwrap(),
            0
        );
        assert_eq!(random.0, 123);
        assert_eq!(particles.active().count(), 0);
        assert!(pool.active().next().unwrap().1.binding.key().is_none());
    }
}
