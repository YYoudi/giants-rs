//! Initial lifetime, source size curves and transparency of a simple effect object.
//! The caller supplies elapsed time. Creation, movement, animation, nested
//! programs, visibility and drawing remain separate consumers.

#[derive(Clone, Copy, Debug)]
pub struct Configuration {
    pub lifetime: f32,
    pub fade_in: f32,
    pub fade_out_start: f32,
    pub start_size: f32,
    pub end_size: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The original first update requests animation advancement, without aging.
    FirstFrame,
    VisibleState,
    /// A cleanup request; this API does not free an actor or its resources.
    Expired,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SizeCurve {
    #[default]
    Linear,
    EaseIn,
    EaseOut,
}

#[derive(Clone, Debug)]
pub struct Visual {
    configuration: Configuration,
    pub age: f32,
    pub size: f32,
    /// Zero is opaque; one is fully transparent.
    pub transparency: f32,
    first_frame: bool,
    expired: bool,
    fading_in: bool,
    fade_in_rate: f32,
    fading_out: bool,
    fade_out_rate: f32,
    size_curve: SizeCurve,
    size_root: f32,
}

impl Visual {
    pub(crate) fn first_frame_consumed(&self) -> bool {
        !self.first_frame
    }
    pub(crate) fn control_flags(&self) -> u32 {
        2 | if self.fading_in { 0x800000 } else { 0 }
            | if self.fading_out { 4 } else { 0 }
            | if self.configuration.start_size == self.configuration.end_size {
                0
            } else {
                match self.size_curve {
                    SizeCurve::Linear => 8,
                    SizeCurve::EaseIn => 0x80000,
                    SizeCurve::EaseOut => 0x100000,
                }
            }
    }
    /// A deferred effect resumes lifetime commands after its existing owner
    /// may already have consumed the source's one first-frame animation pass.
    pub(crate) fn resumed(
        configuration: Configuration,
        first_frame_pending: bool,
    ) -> Result<Self, &'static str> {
        let mut visual = Self::new(configuration)?;
        visual.first_frame = first_frame_pending;
        Ok(visual)
    }
    pub(crate) fn expired(&self) -> bool {
        self.expired
    }
    pub(crate) fn size_curve(&self) -> SizeCurve {
        self.size_curve
    }
    /// A newly configured timed effect, with no inherited flags or delayed time.
    /// Negative/zero lifetimes and nonfinite inputs are outside this profile.
    pub fn new(configuration: Configuration) -> Result<Self, &'static str> {
        Self::with_size_curve(configuration, SizeCurve::Linear)
    }

    pub fn with_size_curve(
        configuration: Configuration,
        size_curve: SizeCurve,
    ) -> Result<Self, &'static str> {
        let c = configuration;
        if [
            c.lifetime,
            c.fade_in,
            c.fade_out_start,
            c.start_size,
            c.end_size,
        ]
        .iter()
        .any(|v| !v.is_finite())
            || c.lifetime <= 0.
            || c.fade_in < 0.
            || c.fade_out_start < 0.
        {
            return Err("effect visual configuration outside finite timed profile");
        }
        if size_curve == SizeCurve::EaseIn && c.end_size < c.start_size {
            return Err("decreasing accelerated size produces a nonfinite source root");
        }
        let fading_in = c.fade_in > 0.;
        let fading_out = c.fade_out_start > 0. && c.fade_out_start < c.lifetime;
        let fade_in_rate = if fading_in {
            (1. / f64::from(c.fade_in)) as f32
        } else {
            0.
        };
        let fade_out_rate = if fading_out {
            (1. / (f64::from(c.lifetime) - f64::from(c.fade_out_start))) as f32
        } else {
            0.
        };
        if !fade_in_rate.is_finite() || !fade_out_rate.is_finite() {
            return Err("effect fade reciprocal outside finite profile");
        }
        Ok(Self {
            configuration,
            age: 0.,
            size: c.start_size,
            transparency: if fading_in { 1. } else { 0. },
            first_frame: true,
            expired: false,
            fading_in,
            fade_in_rate,
            fading_out,
            fade_out_rate,
            size_curve,
            // The source stores the square root once in binary32, before the
            // callback divides by lifetime. Squaring end-start directly loses
            // this constructor rounding. Decreasing source34 sizes are refused
            // above: their source root and subsequent size become nonfinite.
            size_root: if size_curve == SizeCurve::EaseIn && c.end_size > c.start_size {
                (f64::from(c.end_size) - f64::from(c.start_size)).sqrt() as f32
            } else {
                0.
            },
        })
    }

    pub fn advance(&mut self, elapsed: f32) -> Result<Step, &'static str> {
        if !elapsed.is_finite() || elapsed < 0. || self.expired {
            return Err("invalid effect step or already expired object");
        }
        if self.first_frame {
            self.first_frame = false;
            return Ok(Step::FirstFrame);
        }
        let age = (f64::from(self.age) + f64::from(elapsed)) as f32;
        if !age.is_finite() {
            return Err("effect age overflow");
        }
        self.age = age;
        let c = self.configuration;
        if age >= c.lifetime {
            self.expired = true;
            return Ok(Step::Expired);
        }
        if self.fading_in {
            if c.fade_in > age {
                self.transparency =
                    ((f64::from(c.fade_in) - f64::from(age)) * f64::from(self.fade_in_rate)) as f32;
            } else {
                self.fading_in = false;
                self.transparency = 0.;
            }
        }
        // The fade-out write follows the fade-in write, including overlaps.
        if self.fading_out && c.fade_out_start < age {
            self.transparency = ((f64::from(age) - f64::from(c.fade_out_start))
                * f64::from(self.fade_out_rate)) as f32;
        }
        if c.start_size != c.end_size {
            self.size = match self.size_curve {
                SizeCurve::Linear => {
                    ((f64::from(c.end_size) - f64::from(c.start_size)) / f64::from(c.lifetime)
                        * f64::from(age)
                        + f64::from(c.start_size)) as f32
                }
                SizeCurve::EaseIn => {
                    let growth = f64::from(self.size_root) / f64::from(c.lifetime) * f64::from(age);
                    (growth * growth + f64::from(c.start_size)) as f32
                }
                SizeCurve::EaseOut => {
                    let inverse = 1. / f64::from(c.lifetime);
                    let growth = (f64::from(c.end_size) - f64::from(c.start_size))
                        * inverse
                        * f64::from(age);
                    (growth * (1. - inverse * f64::from(age)) + f64::from(c.start_size) + growth)
                        as f32
                }
            };
        }
        Ok(Step::VisibleState)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accelerated_size_matches_native_words_and_refuses_nonfinite_direction() {
        let c = Configuration {
            lifetime: 2.,
            fade_in: 0.,
            fade_out_start: 1.,
            start_size: 4.,
            end_size: 10.,
        };
        let mut v = Visual::with_size_curve(c, SizeCurve::EaseIn).unwrap();
        // Exact-debug supplied callbacks, size34-GP. Constructor root is f32.
        for (dt, word) in [
            (0.1, 1082130432),
            (0.1, 1082161889),
            (0.25, 1082515784),
            (0.25, 1083262894),
            (0.5, 1085936763),
        ] {
            v.advance(dt).unwrap();
            assert_eq!(v.size.to_bits(), word);
        }
        assert!(
            Visual::with_size_curve(
                Configuration {
                    start_size: 10.,
                    end_size: 4.,
                    ..c
                },
                SizeCurve::EaseIn
            )
            .is_err()
        );
    }
    fn configuration() -> Configuration {
        Configuration {
            lifetime: 1.,
            fade_in: 0.25,
            fade_out_start: 0.75,
            start_size: 2.,
            end_size: 6.,
        }
    }
    #[test]
    fn first_update_does_not_consume_lifetime() {
        let mut v = Visual::new(configuration()).unwrap();
        assert_eq!(v.advance(2.).unwrap(), Step::FirstFrame);
        assert_eq!((v.age, v.size, v.transparency), (0., 2., 1.));
        assert_eq!(v.advance(0.125).unwrap(), Step::VisibleState);
        assert_eq!((v.age, v.size, v.transparency), (0.125, 2.5, 0.5));
    }
    #[test]
    fn fade_out_and_expiry_keep_the_last_visual_state_separate() {
        let mut v = Visual::new(configuration()).unwrap();
        v.advance(0.).unwrap();
        v.advance(0.875).unwrap();
        assert_eq!((v.size, v.transparency), (5.5, 0.5));
        assert_eq!(v.advance(0.125).unwrap(), Step::Expired);
        assert_eq!((v.age, v.size, v.transparency), (1., 5.5, 0.5));
        assert!(v.advance(0.).is_err());
    }
    #[test]
    fn overlapping_fades_follow_the_original_write_order() {
        let mut c = configuration();
        c.fade_in = 0.75;
        c.fade_out_start = 0.25;
        let mut v = Visual::new(c).unwrap();
        v.advance(0.).unwrap();
        v.advance(0.5).unwrap();
        assert_eq!(v.transparency, (0.25_f64 * f64::from(1_f32 / 0.75)) as f32);
    }
    #[test]
    fn invalid_steps_leave_the_object_unchanged() {
        let mut v = Visual::new(configuration()).unwrap();
        let before = format!("{v:?}");
        assert!(v.advance(f32::NAN).is_err());
        assert_eq!(before, format!("{v:?}"));
        let mut c = configuration();
        c.lifetime = 0.;
        assert!(Visual::new(c).is_err());
    }
}
