//! Source-bounded detached particles. No contacts, nested emissions
//! or global dispatch cadence are invented by this visual consumer.
use crate::{
    arithmetic::Precision,
    effect_visual::{Configuration, SizeCurve, Step, Visual},
    mecc_motion::Motion,
};
#[derive(Clone, Debug)]
pub struct Recipe {
    pub class: i32,
    pub visual: Configuration,
    pub gravity_mass: f32,
    pub size_curve: SizeCurve,
    pub initial_offset: [f32; 3],
    pub random: Option<RandomRanges>,
    pub vertical_fade: Option<crate::vertical_fade::Configuration>,
}
#[derive(Clone, Debug)]
pub struct RandomRanges {
    ranges: [[f32; 2]; 5],
}

impl Recipe {
    pub fn project(p: &giants_formats::effect_program::Program) -> Result<Self, &'static str> {
        let ops = p.records.iter().map(|r| r.opcode).collect::<Vec<_>>();
        if ops == [3, 6, 0x37, 5, 0x46, 7, 2] {
            return Self::random_vertical(p);
        }
        let curved = ops == [3, 6, 4, 0x36, 7, 2] || ops == [3, 6, 4, 0x17, 0x36, 7, 2];
        if !curved && ops != [3, 6, 8, 4, 0x0e, 7, 2] {
            return Err("released effect program outside free-particle profile");
        }
        let word = |i: usize, at: usize| -> Result<u32, &'static str> {
            let b = p.records[i]
                .bytes
                .get(at..at + 4)
                .ok_or("truncated released effect operand")?;
            Ok(u32::from_le_bytes(
                b.try_into().map_err(|_| "invalid released effect word")?,
            ))
        };
        let size_record = if curved {
            if ops.len() == 7 { 4 } else { 3 }
        } else {
            2
        };
        let lifetime_record = if curved { 2 } else { 3 };
        let recipe = Self {
            class: word(0, 1)? as i32,
            visual: Configuration {
                start_size: f32::from_bits(word(size_record, 1)?),
                end_size: f32::from_bits(word(size_record, 5)?),
                lifetime: f32::from_bits(word(lifetime_record, 1)?),
                fade_in: f32::from_bits(word(lifetime_record, 5)?),
                fade_out_start: f32::from_bits(word(lifetime_record, 9)?),
            },
            gravity_mass: if curved {
                0.
            } else {
                f32::from_bits(word(4, 1)?)
            },
            size_curve: if curved {
                SizeCurve::EaseOut
            } else {
                SizeCurve::Linear
            },
            initial_offset: [
                0.,
                0.,
                if curved && ops.len() == 7 {
                    f32::from_bits(word(3, 1)?)
                } else {
                    0.
                },
            ],
            random: None,
            vertical_fade: None,
        };
        if recipe.class < 0
            || !recipe.gravity_mass.is_finite()
            || recipe.visual.start_size <= 0.
            || recipe.visual.end_size <= 0.
            || recipe.initial_offset.iter().any(|v| !v.is_finite())
        {
            return Err("invalid released effect parameters");
        }
        Visual::new(recipe.visual)?;
        Ok(recipe)
    }
    fn random_vertical(p: &giants_formats::effect_program::Program) -> Result<Self, &'static str> {
        let word = |i: usize, at: usize| -> Result<u32, &'static str> {
            Ok(u32::from_le_bytes(
                p.records[i]
                    .bytes
                    .get(at..at + 4)
                    .ok_or("truncated random vertical particle operand")?
                    .try_into()
                    .map_err(|_| "invalid particle word")?,
            ))
        };
        let value = |i, at| word(i, at).map(f32::from_bits);
        let ranges = [
            [value(2, 1)?, value(2, 5)?],
            [value(2, 9)?, value(2, 13)?],
            [value(3, 1)?, value(3, 5)?],
            [value(3, 9)?, value(3, 13)?],
            [value(3, 17)?, value(3, 21)?],
        ];
        if ranges
            .iter()
            .any(|r| r.iter().any(|v| !v.is_finite()) || r[0] < 0. || r[0] > r[1])
            || ranges[..3].iter().any(|r| r[0] <= 0.)
        {
            return Err("random particle ranges outside finite positive profile");
        }
        let class = word(0, 1)? as i32;
        if class < 0 {
            return Err("invalid random particle class");
        }
        let visual = Configuration {
            start_size: ranges[0][0],
            end_size: ranges[1][0],
            lifetime: ranges[2][0],
            fade_in: ranges[3][0],
            fade_out_start: ranges[4][0],
        };
        Visual::new(visual)?;
        let vertical_fade = crate::vertical_fade::Configuration {
            start: [value(4, 1)?, value(4, 5)?],
            end: [value(4, 9)?, value(4, 13)?],
            duration: value(4, 17)?,
        }
        .validate()?;
        Ok(Self {
            class,
            visual,
            gravity_mass: 0.,
            size_curve: SizeCurve::EaseOut,
            initial_offset: [0.; 3],
            random: Some(RandomRanges { ranges }),
            vertical_fade: Some(vertical_fade),
        })
    }
    fn configuration(&self, seed: &mut u32) -> Configuration {
        let Some(random) = &self.random else {
            return self.visual;
        };
        // Named skip traversal consumes the ranges before activation does.
        for range in random.ranges {
            crate::timed_effect::sample(seed, range);
        }
        self.activate_configuration(seed)
    }
    /// Active VM pass only: the context owner has already consumed skip draws.
    pub(crate) fn activate_configuration(&self, seed: &mut u32) -> Configuration {
        let Some(random) = &self.random else {
            return self.visual;
        };
        let values = random
            .ranges
            .map(|range| crate::timed_effect::sample(seed, range));
        Configuration {
            start_size: values[0],
            end_size: values[1],
            lifetime: values[2],
            fade_in: values[3],
            fade_out_start: values[4],
        }
    }
    pub fn facing(
        &self,
        p: &giants_formats::class_program::Program,
    ) -> Result<Option<crate::attachment_pose::InitialFacing>, &'static str> {
        state78_facing(self.class, p, false)
    }
}
/// Shared admission of initial single-loop state78 geometry, separate from an
/// effect object's creation/lifetime/movement recipe.
/// Flag52 is admitted only by the measured composed-object inspection path.
pub fn state78_facing(
    class: i32,
    p: &giants_formats::class_program::Program,
    allow_flag52: bool,
) -> Result<Option<crate::attachment_pose::InitialFacing>, &'static str> {
    geometry_facing(class, p, allow_flag52, false)
}
/// Detached burst geometry also has a source-declared static profile with no
/// loop command. Its fresh model positions remain intact; no loop is invented.
pub fn state78_burst_facing(
    class: i32,
    p: &giants_formats::class_program::Program,
) -> Result<Option<crate::attachment_pose::InitialFacing>, &'static str> {
    geometry_facing(class, p, true, true)
}
fn geometry_facing(
    class: i32,
    p: &giants_formats::class_program::Program,
    allow_flag52: bool,
    allow_static: bool,
) -> Result<Option<crate::attachment_pose::InitialFacing>, &'static str> {
    if p.records.iter().any(|r| {
        (!matches!(
            r.opcode,
            1 | 0x13 | 0x1d | 0x1b | 0x16 | 0x36 | 0x22 | 0x1e | 0xff
        ) && !(allow_flag52 && r.opcode == 0x52))
            || r.opcode == 0x13 && r.bytes.get(1) != Some(&1)
            || r.opcode == 0x1e && r.bytes.get(1) != Some(&78)
            || r.opcode == 0x36 && r.bytes.get(1) != Some(&6)
    }) || !p.records.iter().any(|r| r.opcode == 0x1e)
        || !allow_static && !p.records.iter().any(|r| r.opcode == 0x13)
    {
        return Err("released class outside loop1/state78 profile");
    }
    let flags = crate::object_lists::DirectFlags::project(class, p);
    if flags.personality & 0x84 == 0 {
        return Ok(None);
    }
    if flags.personality & 0x84 != 4 {
        return Err("released class outside ordinary billboard profile");
    }
    Ok(Some(crate::attachment_pose::InitialFacing {
        personality: flags.personality,
        scale: 1.,
    }))
}
pub struct Playback {
    pub recipe: Recipe,
    pub visual: Option<Visual>,
    pub motion: Motion,
    pub expired: bool,
    pending: bool,
    configuration: Option<Configuration>,
    pub vertical: Option<crate::vertical_fade::Playback>,
}
impl Playback {
    pub fn new(recipe: Recipe) -> Self {
        Self {
            recipe,
            visual: None,
            motion: Motion {
                position: [0.; 3],
                velocity: [0.; 3],
                previous_velocity: [0.; 3],
                force: [0.; 3],
                mass: 1.,
                speed_limit: 0.,
                force_time_limit: 0.,
                stored_dt: 0.,
            },
            expired: false,
            pending: false,
            configuration: None,
            vertical: None,
        }
    }
    pub fn reset(&mut self) {
        self.visual = None;
        self.expired = false;
        self.pending = false;
        self.configuration = None;
        self.vertical = None;
        self.motion.position = [0.; 3];
        self.motion.velocity = [0.; 3];
        self.motion.previous_velocity = [0.; 3];
        self.motion.force = [0.; 3];
        self.motion.stored_dt = 0.;
        self.motion.force_time_limit = 0.;
    }
    pub fn request(&mut self, position: [f32; 3]) -> Result<(), &'static str> {
        if self.recipe.random.is_some() {
            return Err("random particle requires explicit seed");
        }
        self.request_position(position)?;
        self.configuration = Some(self.recipe.visual);
        Ok(())
    }
    pub fn request_with_seed(
        &mut self,
        position: [f32; 3],
        seed: &mut u32,
    ) -> Result<(), &'static str> {
        self.request_position(position)?;
        self.configuration = Some(self.recipe.configuration(seed));
        Ok(())
    }
    fn request_position(&mut self, position: [f32; 3]) -> Result<(), &'static str> {
        if position.iter().any(|v| !v.is_finite()) || self.pending || self.visual.is_some() {
            return Err("invalid or repeated retirement particle request");
        }
        self.motion.position = std::array::from_fn(|i| {
            (f64::from(position[i]) + f64::from(self.recipe.initial_offset[i])) as f32
        });
        self.pending = true;
        Ok(())
    }
    pub fn advance(&mut self, dt: f32) -> Result<bool, &'static str> {
        if !dt.is_finite() || dt < 0. {
            return Err("invalid released effect delta");
        }
        if dt == 0. || self.expired {
            return Ok(false);
        }
        if self.pending {
            self.visual = Some(Visual::with_size_curve(
                self.configuration
                    .take()
                    .ok_or("particle configuration absent")?,
                self.recipe.size_curve,
            )?);
            self.vertical = self
                .recipe
                .vertical_fade
                .map(crate::vertical_fade::Playback::new)
                .transpose()?;
            self.pending = false;
        }
        let Some(visual) = &mut self.visual else {
            return Ok(false);
        };
        match visual.advance(dt)? {
            Step::FirstFrame => {}
            Step::Expired => {
                self.expired = true;
            }
            Step::VisibleState => {
                if let Some(vertical) = &mut self.vertical {
                    vertical.advance(dt)?;
                }
                self.motion = crate::mecc_motion::integrate(
                    self.motion,
                    dt,
                    false,
                    false,
                    Precision::Double53,
                )?;
                let gravity = crate::mecc_gravity::apply(
                    crate::mecc_gravity::Input {
                        force: self.motion.force,
                        velocity: self.motion.velocity,
                        normal: [0.; 3],
                        gravity_mass: self.recipe.gravity_mass,
                        mass: self.motion.mass,
                        friction_limit: 0.,
                        damping_xy: 0.,
                        damping_z: 0.,
                        force_time_limit: self.motion.force_time_limit,
                        object_flags: 0,
                        physics_flags: 0,
                        project_contact: true,
                    },
                    Precision::Double53,
                )?;
                self.motion.force = gravity.force;
                self.motion.force_time_limit = gravity.force_time_limit;
            }
        }
        Ok(true)
    }
    pub fn visible(&self) -> bool {
        self.visual.is_some() && !self.expired
    }
    pub fn requested(&self) -> bool {
        self.pending || self.visual.is_some()
    }
    pub fn size(&self) -> f32 {
        self.visual
            .as_ref()
            .map_or(self.recipe.visual.start_size, |v| v.size)
    }
    pub fn transparency(&self) -> f32 {
        self.visual.as_ref().map_or(0., |v| v.transparency)
    }
}

#[cfg(test)]
mod static_burst_tests {
    #[test]
    fn absent_loop_is_static_and_nonstandard_loop_is_refused() {
        let static_program =
            giants_formats::class_program::parse(&[0x1e, 78, 0x36, 6, 0xff]).unwrap();
        assert!(super::state78_facing(900, &static_program, true).is_err());
        assert!(super::state78_burst_facing(900, &static_program).is_ok());
        let wrong_loop =
            giants_formats::class_program::parse(&[0x13, 2, 0x1e, 78, 0x36, 6, 0xff]).unwrap();
        assert!(super::state78_burst_facing(900, &wrong_loop).is_err());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn curved_particle_offset_and_reset_do_not_follow_the_carrier() {
        let mut bytes = vec![3];
        bytes.extend(915_u32.to_le_bytes());
        bytes.extend([6, 4]);
        for f in [0.5_f32, 0., 0.001] {
            bytes.extend(f.to_le_bytes());
        }
        bytes.push(0x17);
        bytes.extend(0.3_f32.to_le_bytes());
        bytes.push(0x36);
        for f in [10_f32, 40.] {
            bytes.extend(f.to_le_bytes());
        }
        bytes.extend([7, 2]);
        let program = giants_formats::effect_program::parse(&bytes).unwrap();
        let recipe = Recipe::project(&program).unwrap();
        let mut p = Playback::new(recipe);
        p.request([1.25, -2.5, 3.75]).unwrap();
        p.advance(0.05).unwrap();
        assert_eq!(
            p.motion.position.map(f32::to_bits),
            [0x3fa00000, 0xc0200000, 0x4081999a]
        );
        assert!(p.request([99.; 3]).is_err());
        p.advance(0.05).unwrap();
        // Original control: eased growth differs from a linear size16 here.
        assert_eq!(p.size().to_bits(), 1098593075);
        p.reset();
        p.request([0.; 3]).unwrap();
        p.advance(0.05).unwrap();
        assert_eq!(p.motion.position, [0., 0., 0.3]);
        let mut malformed = program.clone();
        malformed.records.insert(2, program.records[2].clone());
        assert!(Recipe::project(&malformed).is_err());
    }
    #[test]
    fn detached_particle_pauses_expires_and_requires_reset_before_reuse() {
        let mut p = Playback::new(Recipe {
            class: 123,
            gravity_mass: -0.001,
            size_curve: SizeCurve::Linear,
            initial_offset: [0.; 3],
            random: None,
            vertical_fade: None,
            visual: Configuration {
                start_size: 1.,
                end_size: 3.,
                lifetime: 0.1,
                fade_in: 0.,
                fade_out_start: 0.04,
            },
        });
        let origin = [1.25, -2.5, 3.75];
        p.request(origin).unwrap();
        assert!(p.request([0.; 3]).is_err());
        assert!(!p.advance(0.).unwrap());
        assert!(!p.visible());
        p.advance(0.02).unwrap();
        assert!(p.visible());
        assert_eq!(p.motion.position, origin);
        assert_eq!(p.visual.as_ref().unwrap().age, 0.);
        p.advance(0.02).unwrap();
        p.advance(0.02).unwrap();
        assert!(p.motion.position[2] > origin[2]);
        let paused = (
            p.motion.position,
            p.visual.as_ref().unwrap().age,
            p.size(),
            p.transparency(),
        );
        p.advance(0.).unwrap();
        assert_eq!(
            paused,
            (
                p.motion.position,
                p.visual.as_ref().unwrap().age,
                p.size(),
                p.transparency()
            )
        );
        for _ in 0..10 {
            p.advance(0.02).unwrap();
        }
        assert!(p.expired && !p.visible());
        assert!(p.request(origin).is_err());
        p.reset();
        assert!(!p.visible() && !p.expired);
        p.request([4., 5., 6.]).unwrap();
        p.advance(0.01).unwrap();
        assert_eq!(p.motion.position, [4., 5., 6.]);
        assert_eq!(p.motion.velocity, [0.; 3]);
    }
}
