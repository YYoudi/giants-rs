//! Owned numeric history of the source tracer, not a point light.
//! Global slot allocation, owner callbacks and ribbon submission stay upstream.
pub const CAPACITY: usize = 150;

#[derive(Clone, Debug)]
pub struct Configuration {
    pub lifetime: f32,
    pub fade_start: f32,
    pub colors: [[f32; 3]; 2],
    pub min_interval: f32,
    pub half_width: f32,
}
impl Configuration {
    pub fn configured_words(&self) -> Vec<u32> {
        let mut values = vec![self.lifetime, self.fade_start, self.inverse_fade()];
        values.extend(self.colors.into_iter().flatten());
        values.extend([self.min_interval, self.half_width]);
        values.into_iter().map(f32::to_bits).collect()
    }
    fn inverse_fade(&self) -> f32 {
        if self.fade_start < self.lifetime {
            (1. / (f64::from(self.lifetime) - f64::from(self.fade_start))) as f32
        } else {
            0.
        }
    }
    fn validate(&self) -> Result<(), &'static str> {
        if self
            .configured_words()
            .iter()
            .any(|w| !f32::from_bits(*w).is_finite())
            || self.lifetime <= 0.
            || self.fade_start < 0.
            || self.fade_start >= self.lifetime
            || self.min_interval < 0.
            || self.half_width < 0.
            || self
                .colors
                .iter()
                .flatten()
                .any(|c| !(0. ..=1.).contains(c))
        {
            return Err("tracer configuration outside finite source profile");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub fade: f32,
    pub age: f32,
}
impl Point {
    pub fn words(self) -> [u32; 8] {
        [
            self.position[0],
            self.position[1],
            self.position[2],
            self.color[0],
            self.color[1],
            self.color[2],
            self.fade,
            self.age,
        ]
        .map(f32::to_bits)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Segment {
    pub previous: usize,
    pub current: usize,
    pub opacity: f32,
    pub half_width: f32,
}

#[derive(Clone, Debug)]
pub struct Track {
    configuration: Configuration,
    points: [Point; CAPACITY],
    written: [bool; CAPACITY],
    count: usize,
    cursor: usize,
    opacity: f32,
}
impl Track {
    pub fn new(configuration: Configuration) -> Result<Self, &'static str> {
        configuration.validate()?;
        Ok(Self {
            configuration,
            points: [Point::default(); CAPACITY],
            written: [false; CAPACITY],
            count: 0,
            cursor: 0,
            opacity: 1.,
        })
    }
    pub(crate) fn reset(&mut self, configuration: Configuration) -> Result<(), &'static str> {
        configuration.validate()?;
        self.configuration = configuration;
        self.count = 0;
        self.cursor = 0;
        self.opacity = 1.;
        Ok(())
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn opacity(&self) -> f32 {
        self.opacity
    }
    pub fn defined_points(&self) -> impl Iterator<Item = (usize, Point)> + '_ {
        self.points
            .iter()
            .copied()
            .enumerate()
            .filter(|(i, _)| self.written[*i])
    }
    fn initialize(&mut self, position: [f32; 3]) {
        self.points[self.cursor] = Point {
            position,
            color: self.configuration.colors[0],
            fade: 0.,
            age: 0.,
        };
        self.written[self.cursor] = true;
    }
    pub fn sample(&mut self, position: [f32; 3]) -> Result<(), &'static str> {
        if position.iter().any(|p| !p.is_finite()) {
            return Err("nonfinite tracer position");
        }
        if self.count == 0 {
            self.initialize(position);
            self.count = 1;
        } else if self.points[self.cursor].age <= self.configuration.min_interval {
            self.points[self.cursor].position = position;
            return Ok(());
        }
        self.cursor = (self.cursor + 1) % CAPACITY;
        self.initialize(position);
        self.count = (self.count + 1).min(CAPACITY);
        Ok(())
    }
    pub fn set_opacity(&mut self, opacity: f32) -> Result<(), &'static str> {
        if !opacity.is_finite() || !(0. ..=1.).contains(&opacity) {
            return Err("tracer opacity outside unit profile");
        }
        self.opacity = opacity;
        Ok(())
    }
    /// A supplied global delta; this does not choose a director/world clock.
    pub fn advance(&mut self, delta: f32) -> Result<Vec<Segment>, &'static str> {
        if !delta.is_finite() || delta < 0. {
            return Err("invalid tracer delta");
        }
        if self.count <= 1 {
            return Ok(Vec::new());
        }
        let inverse = self.configuration.inverse_fade();
        for visited in 0..self.count {
            let index = (self.cursor + CAPACITY - visited) % CAPACITY;
            let p = &mut self.points[index];
            p.age = (f64::from(p.age) + f64::from(delta)) as f32;
            if p.age >= self.configuration.lifetime {
                self.count = visited;
                break;
            }
            if p.age > self.configuration.fade_start {
                p.fade = ((f64::from(p.age) - f64::from(self.configuration.fade_start))
                    * f64::from(inverse)) as f32;
            }
            for (i, color) in p.color.iter_mut().enumerate() {
                *color = ((f64::from(self.configuration.colors[1][i])
                    - f64::from(self.configuration.colors[0][i]))
                    / f64::from(self.configuration.lifetime)
                    * f64::from(delta)
                    + f64::from(*color)) as f32;
            }
        }
        Ok((0..self.count.saturating_sub(1))
            .map(|offset| {
                let current = (self.cursor + CAPACITY - offset) % CAPACITY;
                Segment {
                    previous: (current + CAPACITY - 1) % CAPACITY,
                    current,
                    opacity: self.opacity,
                    half_width: self.configuration.half_width,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_inputs_leave_a_prepared_history_unchanged() {
        let mut track = Track::new(Configuration {
            lifetime: 1.,
            fade_start: 0.1,
            colors: [[1., 0., 0.], [0., 1., 0.]],
            min_interval: 0.01,
            half_width: 0.5,
        })
        .unwrap();
        track.sample([2., 3., 4.]).unwrap();
        track.advance(0.02).unwrap();
        let before = format!("{track:?}");
        assert!(track.sample([f32::NAN, 0., 0.]).is_err());
        assert!(track.advance(-1.).is_err());
        assert!(track.advance(f32::INFINITY).is_err());
        assert!(track.set_opacity(1.1).is_err());
        assert_eq!(format!("{track:?}"), before);
    }
}
