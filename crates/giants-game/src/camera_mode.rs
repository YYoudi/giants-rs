//! Shared camera-mode command and display admission. Startup, input dispatch,
//! force/override ownership and the enclosing world schedule remain external.

#[derive(Clone, Copy, Debug)]
pub struct Profile {
    pub vertical_fov: f32,
    pub actor_scale: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Owner<K> {
    pub source: K,
    pub object_flags: u32,
}

pub trait Services<K> {
    /// Resolve the current profile after the mode words have changed. The
    /// original can query twice; the second result must not reuse the first.
    fn profile(&mut self, mode: u32, view: u32) -> Result<Profile, String>;
    fn scale(&mut self, source: K, scale: f32) -> Result<(), String>;
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
            if !profile.actor_scale.is_finite() || profile.actor_scale <= 0. {
                return Err("camera profile scale outside finite positive profile".into());
            }
            services.scale(owner.source, profile.actor_scale)?;
        }
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
    struct Recorder {
        calls: Vec<&'static str>,
        fail: &'static str,
    }
    impl Services<u32> for Recorder {
        fn profile(&mut self, _: u32, _: u32) -> Result<Profile, String> {
            self.calls.push("profile");
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
