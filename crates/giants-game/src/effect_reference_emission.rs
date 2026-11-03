//! Unbounded reference-selected channel clock and inclusive selection only.
//! A request is not a constructed FX context, an attachment or a rendered object.
//! Callers must construct each child before advancing another channel sharing RNG.
use crate::effect_scene::EmissionConfiguration;

#[derive(Clone, Copy, Debug)]
pub enum Mode {
    Single([u32; 2]),
    Paired([[u32; 2]; 2]),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selected {
    Single(u32),
    Paired([u32; 2]),
}
/// The paired factory selects the child's reference first, then the parent's.
/// Keeping their scopes explicit prevents interpreting both as guide points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attachment {
    pub child_reference: u32,
    pub parent_reference: u32,
}
impl Selected {
    pub fn attachment(self) -> Result<Attachment, &'static str> {
        match self {
            Self::Paired([child_reference, parent_reference])
                if child_reference <= 65535 && parent_reference <= 65535 =>
            {
                Ok(Attachment {
                    child_reference,
                    parent_reference,
                })
            }
            _ => Err("selected request is not a bounded paired attachment"),
        }
    }
}
impl Attachment {
    /// Resolve the two models independently at their supplied clocks. This is
    /// the normal attached-matrix profile only, not child creation, state78,
    /// owner retirement, or a service acknowledgement. Callers must refuse
    /// unresolved parent transforms before supplying a cached matrix here.
    pub fn matrix(
        self,
        parent_matrix: crate::procedural_aim::Matrix,
        parent_size: f32,
        child_size: f32,
        parent: &crate::effect_animation::Playback,
        child: &crate::effect_animation::Playback,
    ) -> Result<crate::procedural_aim::Matrix, String> {
        // Validate even publicly constructed bindings before model access.
        Selected::Paired([self.child_reference, self.parent_reference])
            .attachment()
            .map_err(str::to_owned)?;
        let parent_reference = parent.reference(self.parent_reference as usize)?;
        let child_reference = child.reference(self.child_reference as usize)?;
        crate::effect_attachment::initial_child_matrix(
            parent_matrix,
            parent_reference,
            child_reference,
            child_size,
            parent_size,
        )
        .map_err(str::to_owned)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Channel {
    pub interval: f32,
    pub remaining: f32,
    pub mode: Mode,
}
#[derive(Clone, Copy, Debug)]
pub struct Step {
    pub state: Channel,
    pub phase: f32,
    pub request: Option<Selected>,
}
impl Mode {
    /// Only this range belongs to the guide's model. In paired mode the first
    /// range belongs to the child and must not be resolved on the parent.
    pub fn parent_range(self) -> [u32; 2] {
        match self {
            Self::Single(range) | Self::Paired([_, range]) => range,
        }
    }
    fn validate(self) -> Result<(), &'static str> {
        let ranges = match self {
            Self::Single(r) => [r, r],
            Self::Paired(r) => r,
        };
        if ranges.iter().any(|r| r[0] > r[1] || r[1] > 65535) {
            return Err("reference selector outside bounded inclusive range");
        }
        Ok(())
    }
    fn select(self, seed: &mut u32) -> Selected {
        let pick = |range: [u32; 2], seed: &mut u32| {
            *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
            let sample = (*seed >> 16) & 32767;
            range[0] + ((sample * (range[1] - range[0] + 1)) >> 15)
        };
        match self {
            Self::Single(r) => Selected::Single(pick(r, seed)),
            Self::Paired(r) => Selected::Paired([pick(r[0], seed), pick(r[1], seed)]),
        }
    }
}
impl Channel {
    /// The checked source factory initializes an unbounded channel at zero.
    /// Position-only selectors and expiry/ownership variants stay separate.
    pub fn new(config: &EmissionConfiguration) -> Result<Self, &'static str> {
        if config.source_flag9 {
            return Err("reference channel ownership variant pending");
        }
        let mode = if config.source_flag8 {
            Mode::Paired(config.selectors)
        } else if config.selectors[1] == [u32::MAX; 2] {
            Mode::Single(config.selectors[0])
        } else {
            return Err("reference channel secondary selector mode pending");
        };
        mode.validate()?;
        if !config.interval.is_finite() || config.interval <= 0. {
            return Err("reference channel interval outside positive profile");
        }
        Ok(Self {
            interval: config.interval,
            remaining: 0.,
            mode,
        })
    }
    pub fn advance(self, dt: f32, seed: &mut u32) -> Result<Step, &'static str> {
        self.mode.validate()?;
        if !dt.is_finite()
            || dt < 0.
            || !self.remaining.is_finite()
            || !self.interval.is_finite()
            || self.interval <= 0.
        {
            return Err("reference channel outside finite nonnegative-time profile");
        }
        let mut state = self;
        state.remaining = (f64::from(self.remaining) - f64::from(dt)) as f32;
        if !state.remaining.is_finite() {
            return Err("reference channel clock overflow");
        }
        let emitted = state.remaining < 0.;
        if emitted {
            // This path discards overdue time; do not reuse Periodic catch-up.
            state.remaining = self.interval;
        }
        let phase = (f64::from(self.interval) - f64::from(state.remaining)) as f32;
        if !phase.is_finite() {
            return Err("reference channel phase overflow");
        }
        let request = emitted.then(|| self.mode.select(seed));
        Ok(Step {
            state,
            phase,
            request,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paired_references_keep_distinct_model_scopes() {
        let binding = Selected::Paired([7, 19]).attachment().unwrap();
        assert_eq!(binding.child_reference, 7);
        assert_eq!(binding.parent_reference, 19);
        assert_eq!(Mode::Paired([[7, 7], [19, 23]]).parent_range(), [19, 23]);
        assert!(Selected::Single(7).attachment().is_err());
        assert!(Selected::Paired([u32::MAX, 19]).attachment().is_err());
    }
    #[test]
    fn overdue_reference_channel_emits_once_and_discards_debt() {
        let mut seed = 1;
        let step = Channel {
            interval: 0.125,
            remaining: 0.,
            mode: Mode::Single([5, 15]),
        }
        .advance(2., &mut seed)
        .unwrap();
        assert_eq!(step.state.remaining, 0.125);
        assert_eq!(step.phase, 0.);
        assert_eq!(step.request, Some(Selected::Single(5)));
        let saved = seed;
        let boundary = step.state.advance(0.125, &mut seed).unwrap();
        assert_eq!(boundary.request, None);
        assert_eq!(seed, saved);
    }
    #[test]
    fn equal_endpoints_still_consume_two_ordered_samples() {
        let mut seed = 1;
        let step = Channel {
            interval: 0.1,
            remaining: 0.,
            mode: Mode::Paired([[7, 7], [9, 9]]),
        }
        .advance(0.01, &mut seed)
        .unwrap();
        assert_eq!(step.request, Some(Selected::Paired([7, 9])));
        assert_eq!(
            seed,
            1u32.wrapping_mul(214013)
                .wrapping_add(2531011)
                .wrapping_mul(214013)
                .wrapping_add(2531011)
        );
        let saved = seed;
        assert!(
            Channel {
                mode: Mode::Single([u32::MAX; 2]),
                ..step.state
            }
            .advance(0.1, &mut seed)
            .is_err()
        );
        assert_eq!(seed, saved);
    }
}
