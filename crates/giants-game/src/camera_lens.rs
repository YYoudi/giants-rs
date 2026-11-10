//! Acquired camera-lens transition and input scaling, before projection.
//! Profile selection, viewport/projection construction and renderer publication
//! remain separate. Inputs describe the actual camera invocation, not outcomes.

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub profile_fov: f32,
    pub override_fov: Option<f32>,
    pub seconds: f32,
    pub inhibited: bool,
    pub transition_input: bool,
    pub scaled_input: bool,
    pub mode: i32,
    pub profile_index: u32,
    /// Controlled actor's state-binding kind, separate from model class and
    /// from a camera owner selected by an override. Historical field name kept.
    pub selected_kind: Option<i32>,
    pub input_scale: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    transition: f32,
}
impl State {
    pub fn new(transition: f32) -> Result<Self, &'static str> {
        if !transition.is_finite() || !(0. ..=1.).contains(&transition) {
            return Err("Camera lens transition outside acquired finite profile");
        }
        Ok(Self { transition })
    }
    pub fn transition(&self) -> f32 {
        self.transition
    }
    /// Return the vertical FOV supplied to the projection producer.
    /// An explicit override retains the transition and consumes no timing or
    /// scale input. A refusal does not commit a partially evaluated transition.
    pub fn advance(&mut self, i: Input) -> Result<f32, &'static str> {
        let valid_angle = |v: f32| v.is_finite() && v > 0. && v < 180.;
        if let Some(value) = i.override_fov {
            return if valid_angle(value) {
                Ok(value)
            } else {
                Err("Camera lens override outside acquired projection profile")
            };
        }
        if !valid_angle(i.profile_fov) || !i.seconds.is_finite() || i.seconds < 0. {
            return Err("Camera lens requires a usable profile and nonnegative time");
        }
        let eligible = i.mode == 0 && i.profile_index == 1
            || i.scaled_input && matches!(i.selected_kind, Some(1..=3));
        let mut transition = self.transition;
        if i.transition_input && !i.inhibited && eligible {
            let next = f64::from(transition) + f64::from(i.seconds);
            transition = if next > 1. { 1. } else { next as f32 };
        } else if transition > 0. {
            let next = f64::from(transition) - f64::from(i.seconds);
            transition = if next < 0. { 0. } else { next as f32 };
        }
        let mut fov = i.profile_fov;
        if transition != 0. {
            let t = f64::from(transition);
            fov = ((1. - t) * f64::from(fov) + t * 25.) as f32;
        }
        if !i.inhibited && eligible && i.scaled_input {
            if !i.input_scale.is_finite() || i.input_scale <= 0. {
                return Err("Camera lens input scale outside acquired positive profile");
            }
            fov *= i.input_scale;
        }
        if !valid_angle(fov) {
            return Err("Camera lens result outside acquired projection profile");
        }
        self.transition = transition;
        Ok(fov)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> Input {
        Input {
            profile_fov: 60.,
            override_fov: None,
            seconds: 0.25,
            inhibited: false,
            transition_input: true,
            scaled_input: true,
            mode: 0,
            profile_index: 0,
            selected_kind: Some(3),
            input_scale: 1.,
        }
    }
    #[test]
    fn override_preserves_transition_and_does_not_consume_other_scalars() {
        let mut state = State::new(0.5).unwrap();
        assert_eq!(
            state
                .advance(Input {
                    override_fov: Some(50.),
                    profile_fov: f32::NAN,
                    seconds: f32::NAN,
                    input_scale: f32::NAN,
                    ..input()
                })
                .unwrap(),
            50.
        );
        assert_eq!(state.transition(), 0.5);
    }
    #[test]
    fn invalid_scale_cannot_commit_the_transition() {
        let mut state = State::new(0.5).unwrap();
        assert!(
            state
                .advance(Input {
                    input_scale: f32::NAN,
                    ..input()
                })
                .is_err()
        );
        assert_eq!(state.transition(), 0.5);
        // Inhibited input releases an existing transition and does not read scale.
        assert_eq!(
            state
                .advance(Input {
                    inhibited: true,
                    input_scale: f32::NAN,
                    ..input()
                })
                .unwrap(),
            51.25
        );
        assert_eq!(state.transition(), 0.25);
    }
}
