//! Shared camera mode, activation reset, lens transition and display admission.
//! Startup, input dispatch,
//! force/override ownership and the enclosing world schedule remain external.

#[derive(Clone, Copy, Debug)]
pub struct Profile {
    pub vertical_fov: f32,
    /// Historical API alias: the profile's signed camera pitch, copied to the
    /// owner's separate look-angle word. This is not its body/model scale.
    pub actor_scale: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Owner<K> {
    pub source: K,
    pub object_flags: u32,
}

/// Current command and input context, separate from camera-owner selection.
#[derive(Clone, Copy, Debug)]
pub struct Command<K> {
    pub request: i32,
    pub extended: bool,
    pub controlled: Option<K>,
    pub scaled_input: bool,
}

pub trait Services<K> {
    /// Resolve the current profile after the mode words have changed. The
    /// original can query twice; the second result must not reuse the first.
    fn profile(&mut self, mode: u32, view: u32) -> Result<Profile, String>;
    /// Historical alias: publish the acquired signed look-angle word.
    fn scale(&mut self, source: K, pitch: f32) -> Result<(), String>;
    /// Set the actual projection with the acquired mode-change flags (0, 1).
    fn projection(&mut self, vertical_fov: f32) -> Result<(), String>;
}

pub struct State {
    mode: u32,
    view: u32,
    transition: f32,
    failure: Option<String>,
}
impl State {
    /// All initial words belong to the enclosing camera lifecycle; no default
    /// startup state or selected first/third-person boolean is inferred.
    pub fn new(mode: u32, view: u32, transition: f32) -> Result<Self, String> {
        if !transition.is_finite() {
            return Err("camera transition outside finite profile".into());
        }
        Ok(Self {
            mode,
            view,
            transition,
            failure: None,
        })
    }
    pub fn mode(&self) -> u32 {
        self.mode
    }
    pub fn view(&self) -> u32 {
        self.view
    }
    pub fn transition(&self) -> f32 {
        self.transition
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    /// Acquired target-view predicate. The final word is a signed comparison,
    /// including supplied negative profiles; it is not a first-person boolean.
    pub fn target_uses_viewport(&self, forced: u32) -> Result<bool, String> {
        if self.failure.is_some() {
            return Err("camera target view cannot read a failed shared command".into());
        }
        Ok(forced == 0 && self.mode == 0 && (self.view as i32) <= 1)
    }
    /// Evaluate the acquired lens kernel against this same transition word.
    /// Profile/input production and projection publication remain external.
    pub fn advance_lens(&mut self, input: crate::camera_lens::Input) -> Result<f32, String> {
        if self.failure.is_some() {
            return Err("camera lens is quarantined after a failed shared command".into());
        }
        if input.mode as u32 != self.mode || input.profile_index != self.view {
            return Err("camera lens input differs from current shared mode/view".into());
        }
        let mut lens = crate::camera_lens::State::new(self.transition).map_err(str::to_owned)?;
        let result = lens.advance(input).map_err(str::to_owned);
        self.transition = lens.transition();
        result
    }
    /// Actual activation-time projection reset: resolve the current profile,
    /// clear the shared lens transition, then publish projection. This changes
    /// neither mode/view nor owner look angle and does not choose a camera owner.
    pub fn reset_projection<K>(&mut self, services: &mut impl Services<K>) -> Result<(), String> {
        if self.failure.is_some() {
            return Err("camera projection reset cannot replay a failed sequence".into());
        }
        let result = self.reset_projection_inner(services);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    /// Camera eligibility only. Actual owner/model preparation must still run
    /// before publishing display activity. The profile byte and force word are
    /// current shared inputs, independent of this command's transition scalar.
    pub fn display_eligible(
        &self,
        forced: u32,
        profile: u8,
        controlled_class: Option<i32>,
    ) -> Result<bool, String> {
        if self.failure.is_some() {
            return Err("camera mode is quarantined after a failed command".into());
        }
        Ok(forced == 0
            && (self.mode == 0 && self.view == 1
                || profile != 0 && matches!(controlled_class, Some(1..=3))))
    }
    /// Raw command -1 follows the acquired extended-mode policy; it is not a
    /// generic toggle. Unrecognized commands retain both words but still query
    /// the current profile, clear the transition and set projection.
    pub fn change<K: Copy>(
        &mut self,
        request: i32,
        extended: bool,
        owner: Option<Owner<K>>,
        services: &mut impl Services<K>,
    ) -> Result<(), String> {
        if self.failure.is_some() {
            return Err("camera mode command cannot replay a failed sequence".into());
        }
        let result = self.change_inner(request, extended, owner, services);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    fn change_inner<K: Copy>(
        &mut self,
        request: i32,
        extended: bool,
        owner: Option<Owner<K>>,
        services: &mut impl Services<K>,
    ) -> Result<(), String> {
        let request = if request == -1 {
            if extended {
                match self.mode {
                    1 => 5,
                    2 => 4,
                    _ if self.view == 2 => 1,
                    _ => self.view.wrapping_add(2),
                }
            } else {
                u32::from(self.view != 0) + 1
            }
        } else {
            request as u32
        };
        let scale = match request {
            1 => {
                self.mode = 0;
                self.view = 0;
                true
            }
            2 => {
                self.mode = 0;
                self.view = 1;
                true
            }
            3 => {
                self.mode = 2;
                self.view = 0;
                false
            }
            4 => {
                self.mode = 1;
                self.view = 0;
                false
            }
            5 => {
                self.mode = 0;
                self.view = 2;
                true
            }
            _ => false,
        };
        if scale && let Some(owner) = owner.filter(|o| o.object_flags & 0x8000 == 0) {
            let profile = services.profile(self.mode, self.view)?;
            if !profile.actor_scale.is_finite() {
                return Err("camera profile look angle outside finite profile".into());
            }
            services.scale(owner.source, profile.actor_scale)?;
        }
        self.reset_projection_inner(services)
    }
    fn reset_projection_inner<K>(&mut self, services: &mut impl Services<K>) -> Result<(), String> {
        let profile = services.profile(self.mode, self.view)?;
        self.transition = 0.;
        if !profile.vertical_fov.is_finite()
            || profile.vertical_fov <= 0.
            || profile.vertical_fov >= 180.
        {
            return Err("camera profile lens outside finite angular profile".into());
        }
        services.projection(profile.vertical_fov)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signed_profile_angle_is_published_without_a_positive_scale_constraint() {
        struct Pitch(Option<(u32, f32)>);
        impl Services<u32> for Pitch {
            fn profile(&mut self, _: u32, _: u32) -> Result<Profile, String> {
                Ok(Profile {
                    vertical_fov: 75.,
                    actor_scale: -37.5,
                })
            }
            fn scale(&mut self, owner: u32, value: f32) -> Result<(), String> {
                self.0 = Some((owner, value));
                Ok(())
            }
            fn projection(&mut self, _: f32) -> Result<(), String> {
                Ok(())
            }
        }
        let mut state = State::new(0, 1, 0.5).unwrap();
        let mut service = Pitch(None);
        state
            .change(
                1,
                false,
                Some(Owner {
                    source: 7,
                    object_flags: 0,
                }),
                &mut service,
            )
            .unwrap();
        assert_eq!(service.0, Some((7, -37.5)));
        assert_eq!(state.transition(), 0.);
    }
    struct Recorder {
        calls: Vec<&'static str>,
        fail: &'static str,
    }
    impl Services<u32> for Recorder {
        fn profile(&mut self, _: u32, _: u32) -> Result<Profile, String> {
            self.calls.push("profile");
            if self.fail == "profile" {
                return Err("profile failed".into());
            }
            Ok(Profile {
                vertical_fov: 62.,
                actor_scale: 0.6,
            })
        }
        fn scale(&mut self, _: u32, _: f32) -> Result<(), String> {
            self.calls.push("scale");
            if self.fail == "scale" {
                Err("scale failed".into())
            } else {
                Ok(())
            }
        }
        fn projection(&mut self, _: f32) -> Result<(), String> {
            self.calls.push("projection");
            if self.fail == "projection" {
                Err("projection failed".into())
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn lens_and_activation_reset_share_one_transition_word() {
        let mut state = State::new(0, 1, 0.5).unwrap();
        let input = crate::camera_lens::Input {
            profile_fov: 60.,
            override_fov: None,
            seconds: 0.25,
            inhibited: false,
            transition_input: true,
            scaled_input: false,
            mode: 0,
            profile_index: 1,
            selected_kind: None,
            input_scale: 1.,
        };
        assert_eq!(state.advance_lens(input).unwrap(), 33.75);
        assert_eq!(state.transition(), 0.75);
        state
            .reset_projection(&mut Recorder {
                calls: vec![],
                fail: "",
            })
            .unwrap();
        assert_eq!(state.advance_lens(input).unwrap(), 51.25);
        assert_eq!(state.transition(), 0.25);
        let mut stale = input;
        stale.profile_index = 2;
        assert!(state.advance_lens(stale).is_err());
        assert_eq!(state.transition(), 0.25);
        assert!(state.failure().is_none());
        state
            .reset_projection(&mut Recorder {
                calls: vec![],
                fail: "projection",
            })
            .unwrap_err();
        assert!(state.advance_lens(input).is_err());
        assert_eq!(state.transition(), 0.);
    }
    #[test]
    fn activation_reset_preserves_mode_and_scale_and_clears_transition() {
        let mut state = State::new(2, 0, 0.75).unwrap();
        let mut services = Recorder {
            calls: vec![],
            fail: "",
        };
        state.reset_projection(&mut services).unwrap();
        assert_eq!((state.mode(), state.view(), state.transition()), (2, 0, 0.));
        assert_eq!(services.calls, ["profile", "projection"]);
        state
            .change(
                2,
                false,
                Some(Owner {
                    source: 7,
                    object_flags: 0,
                }),
                &mut services,
            )
            .unwrap();
        assert_eq!((state.mode(), state.view(), state.transition()), (0, 1, 0.));
        assert_eq!(
            &services.calls[2..],
            ["profile", "scale", "profile", "projection"]
        );
    }
    #[test]
    fn activation_reset_failure_preserves_prefix_and_quarantines_mode_commands() {
        for failure in ["profile", "projection"] {
            let mut state = State::new(2, 0, 0.75).unwrap();
            let mut services = Recorder {
                calls: vec![],
                fail: failure,
            };
            assert!(state.reset_projection(&mut services).is_err());
            assert_eq!((state.mode(), state.view()), (2, 0));
            assert_eq!(
                state.transition(),
                if failure == "profile" { 0.75 } else { 0. }
            );
            let calls = services.calls.len();
            assert!(state.reset_projection(&mut services).is_err());
            assert!(state.change(2, false, None, &mut services).is_err());
            assert!(state.display_eligible(0, 0, None).is_err());
            assert_eq!(services.calls.len(), calls);
        }
    }
    #[test]
    fn provider_failure_preserves_completed_mode_stores_and_blocks_replay() {
        for failure in ["scale", "projection"] {
            let mut state = State::new(2, 0, 0.75).unwrap();
            let mut services = Recorder {
                calls: vec![],
                fail: failure,
            };
            let owner = Some(Owner {
                source: 7,
                object_flags: 0,
            });
            assert!(state.change(2, false, owner, &mut services).is_err());
            assert_eq!((state.mode(), state.view()), (0, 1));
            assert_eq!(
                state.transition(),
                if failure == "scale" { 0.75 } else { 0. }
            );
            assert_eq!(
                services.calls,
                if failure == "scale" {
                    vec!["profile", "scale"]
                } else {
                    vec!["profile", "scale", "profile", "projection"]
                }
            );
            let count = services.calls.len();
            assert!(state.change(1, false, owner, &mut services).is_err());
            assert!(state.display_eligible(0, 0, None).is_err());
            assert_eq!(services.calls.len(), count);
        }
    }
}
