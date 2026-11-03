//! Bounded timed attachment program. Retirement requests retain their child
//! name; allocation, child motion and native callback cadence are not simulated.
use crate::effect_visual::{Configuration, Step, Visual};
#[derive(Clone, Debug)]
pub struct Recipe {
    pub class: i32,
    pub opaque: u32,
    pub range: [f32; 2],
    /// False only for source objects with no size-range command.
    pub sample_size: bool,
    pub lifetime: f32,
    pub fade_in: f32,
    pub fade_out: f32,
    pub child: String,
}
impl Recipe {
    pub fn project(p: &giants_formats::effect_program::Program) -> Result<Self, &'static str> {
        if p.records.iter().map(|r| r.opcode).collect::<Vec<_>>() != [0x24, 6, 0x1c, 4, 0x0d, 7, 2]
        {
            return Err("effect program outside timed attached profile");
        }
        let word = |r: usize, at: usize| -> Result<u32, &'static str> {
            let b = p.records[r]
                .bytes
                .get(at..at + 4)
                .ok_or("truncated timed effect operand")?;
            Ok(u32::from_le_bytes(
                b.try_into().map_err(|_| "invalid timed effect word")?,
            ))
        };
        if word(0, 9)? != 0 {
            return Err("timed attachment reference operand unsupported");
        }
        let range = [f32::from_bits(word(2, 1)?), f32::from_bits(word(2, 5)?)];
        let recipe = Self {
            class: word(0, 5)? as i32,
            opaque: word(0, 1)?,
            range,
            sample_size: true,
            lifetime: f32::from_bits(word(3, 1)?),
            fade_in: f32::from_bits(word(3, 5)?),
            fade_out: f32::from_bits(word(3, 9)?),
            child: String::from_utf8(
                p.records[4]
                    .bytes
                    .get(1..p.records[4].bytes.len() - 1)
                    .ok_or("missing child name")?
                    .to_vec(),
            )
            .map_err(|_| "invalid child name")?,
        };
        if recipe.class < 0
            || range.iter().any(|v| !v.is_finite() || *v <= 0.)
            || recipe.child.is_empty()
        {
            return Err("invalid timed attachment profile");
        }
        Visual::new(recipe.configuration(range[0]))?;
        Ok(recipe)
    }
    pub fn validate_class(
        &self,
        p: &giants_formats::class_program::Program,
    ) -> Result<(), &'static str> {
        if p.records.iter().any(|r| {
            !matches!(
                r.opcode,
                1 | 0x13 | 0x1d | 0x1b | 0x36 | 0x22 | 0x1e | 0x52 | 0xff
            ) || r.opcode == 0x13 && r.bytes.get(1) != Some(&1)
                || r.opcode == 0x1e && r.bytes.get(1) != Some(&78)
                || r.opcode == 0x36 && r.bytes.get(1) != Some(&6)
        }) || !p.records.iter().any(|r| r.opcode == 0x1e)
            || !p.records.iter().any(|r| r.opcode == 0x13)
            || crate::object_lists::DirectFlags::project(self.class, p).personality & 0x84 != 0
        {
            return Err("timed effect class outside checked normal state78 profile");
        }
        Ok(())
    }
    fn configuration(&self, size: f32) -> Configuration {
        Configuration {
            lifetime: self.lifetime,
            fade_in: self.fade_in,
            fade_out_start: self.fade_out,
            start_size: size,
            end_size: size,
        }
    }
    pub fn create(&self, seed: &mut u32) -> Result<Visual, &'static str> {
        if !self.sample_size {
            return Visual::new(self.configuration(self.range[0]));
        }
        // Named preparation traverses the range opcode even in skip mode.
        sample(seed, self.range);
        let size = sample(seed, self.range);
        Visual::new(self.configuration(size))
    }
}
pub fn sample(seed: &mut u32, range: [f32; 2]) -> f32 {
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    let n = (*seed >> 16) & 32767;
    ((f64::from(n) * (f64::from(range[1]) - f64::from(range[0]))) * (1. / 32768.)
        + f64::from(range[0])) as f32
}
pub struct Playback {
    pub recipe: Recipe,
    pub visual: Option<Visual>,
    pub expired: bool,
}
impl Playback {
    pub fn new(recipe: Recipe) -> Self {
        Self {
            recipe,
            visual: None,
            expired: false,
        }
    }
    pub fn reset(&mut self) {
        self.visual = None;
        self.expired = false;
    }
    /// First reached update creates at age zero. Subsequent positive viewer
    /// deltas consume the first native frame and then age the object.
    pub fn advance(
        &mut self,
        reached: bool,
        dt: f32,
        seed: &mut u32,
    ) -> Result<bool, &'static str> {
        if !dt.is_finite() || dt < 0. {
            return Err("invalid timed effect delta");
        }
        if !reached || self.expired {
            return Ok(false);
        }
        if self.visual.is_none() {
            let mut visual = self.recipe.create(seed)?;
            if dt > 0. {
                visual.advance(dt)?;
            }
            self.visual = Some(visual);
            return Ok(true);
        }
        if dt == 0. {
            return Ok(false);
        }
        if self.visual.as_mut().unwrap().advance(dt)? == Step::Expired {
            self.expired = true;
        }
        Ok(true)
    }
    pub fn visible(&self) -> bool {
        self.visual.is_some() && !self.expired
    }
    pub fn size(&self) -> f32 {
        self.visual.as_ref().map_or(1., |v| v.size)
    }
    pub fn transparency(&self) -> f32 {
        self.visual.as_ref().map_or(0., |v| v.transparency)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn loading_creates_without_consuming_the_first_positive_callback() {
        let recipe = super::Recipe {
            class: 12345,
            opaque: 987,
            range: [1., 2.],
            sample_size: true,
            lifetime: 0.2,
            fade_in: 0.,
            fade_out: 0.,
            child: "synthetic_child".into(),
        };
        let mut p = super::Playback::new(recipe);
        let mut seed = 1;
        p.advance(true, 0., &mut seed).unwrap();
        let created_seed = seed;
        p.advance(true, 0., &mut seed).unwrap();
        assert_eq!(seed, created_seed);
        p.advance(true, 0.02, &mut seed).unwrap();
        assert_eq!(p.visual.as_ref().unwrap().age, 0.);
        p.advance(true, 0.02, &mut seed).unwrap();
        assert_eq!(p.visual.as_ref().unwrap().age, 0.02);
    }
    #[test]
    fn timed_callbacks_require_the_declared_state_and_loop() {
        let recipe = super::Recipe {
            class: 12345,
            opaque: 987,
            range: [1., 2.],
            sample_size: true,
            lifetime: 0.2,
            fade_in: 0.,
            fade_out: 0.,
            child: "synthetic_child".into(),
        };
        let class = |state, loop_slot| {
            giants_formats::class_program::parse(&[
                1, 4, 0, 0, 0, 0x13, loop_slot, 0x1d, 0x22, 0x1e, state, 255,
            ])
            .unwrap()
        };
        assert!(recipe.validate_class(&class(78, 1)).is_ok());
        assert!(recipe.validate_class(&class(77, 1)).is_err());
        assert!(recipe.validate_class(&class(78, 2)).is_err());
        assert!(
            recipe
                .validate_class(
                    &giants_formats::class_program::parse(&[1, 4, 0, 0, 0, 255]).unwrap()
                )
                .is_err()
        );
    }
}
