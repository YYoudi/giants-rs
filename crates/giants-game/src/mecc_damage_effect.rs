//! Acquired damage-smoke context policy. Display reset and context services are
//! required owners; preparation does not execute an active effect or world turn.
use giants_formats::global_data::GlobalData;

/// Actual current pre-world display snapshot, with actor/frame ownership.
#[derive(Clone, Copy)]
pub struct Display {
    pub source: crate::resident_states::Id,
    pub number: u64,
    pub display: crate::first_person::Display<crate::resident_states::Id>,
}
impl Display {
    pub(crate) fn validate(
        &self,
        session: &crate::resident_states::Session,
        source: crate::resident_states::Id,
        number: u64,
    ) -> Result<(), String> {
        if self.source != source
            || self.number != number
            || session.get(source).is_none()
            || self
                .display
                .camera_owner
                .is_some_and(|owner| session.get(owner).is_none())
        {
            return Err("damage smoke requires the current owned display/frame snapshot".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub damage: f32,
    pub limit: f32,
    /// Light, medium and heavy percentage thresholds, in source priority order.
    pub thresholds: [f32; 3],
}
impl Input {
    pub fn from_globals(damage: f32, limit: f32, globals: &GlobalData) -> Result<Self, String> {
        let scalar = |at| globals.scalar(at).map_err(|e| e.to_string());
        Ok(Self {
            damage,
            limit,
            thresholds: [scalar(0x1074)?, scalar(0x106c)?, scalar(0x1070)?],
        })
    }
    fn level(self) -> Result<u32, String> {
        if ![self.damage, self.limit]
            .into_iter()
            .chain(self.thresholds)
            .all(f32::is_finite)
            || self.limit <= 0.
            || self.damage < 0.
        {
            return Err("damage smoke outside finite nonnegative health profile".into());
        }
        let percent = f64::from(self.damage) / f64::from(self.limit) * 100.;
        Ok(if percent >= f64::from(self.thresholds[2]) {
            3
        } else if percent >= f64::from(self.thresholds[1]) {
            2
        } else if percent >= f64::from(self.thresholds[0]) {
            1
        } else {
            0
        })
    }
}

pub trait Services<E> {
    /// Check even a retained binding; no release/create call may occur this turn.
    fn validate_binding(&mut self, binding: Option<E>) -> Result<(), String>;
    /// Actual display-owner decision. True releases first, then selection still
    /// runs; this is not an instruction to suppress creation for the whole call.
    fn reset_before_selection(&mut self) -> Result<bool, String>;
    fn release(&mut self, effect: E) -> Result<(), String>;
    fn prepare(&mut self, level: u32) -> Result<Option<E>, String>;
}

#[derive(Clone, Debug)]
pub struct State<E> {
    pub level: u32,
    pub binding: Option<E>,
}
impl<E> Default for State<E> {
    fn default() -> Self {
        Self {
            level: 0,
            binding: None,
        }
    }
}
impl<E: Copy> State<E> {
    /// Completed stores/releases survive later provider failures. Providers must
    /// validate their handles and quarantine a failed enclosing turn.
    pub fn advance(&mut self, input: Input, services: &mut impl Services<E>) -> Result<(), String> {
        let level = input.level()?;
        if self.level > 3 {
            return Err("damage smoke level outside acquired profile".into());
        }
        services.validate_binding(self.binding)?;
        if services.reset_before_selection()? {
            self.level = 0;
            if let Some(effect) = self.binding {
                services.release(effect)?;
                self.binding = None;
            }
        }
        if self.level != level
            && let Some(effect) = self.binding
        {
            services.release(effect)?;
            self.binding = None;
        }
        self.level = level;
        if self.binding.is_none() && level != 0 {
            self.binding = services.prepare(level)?;
        }
        Ok(())
    }
}

/// Prepared-only adapter. The caller supplies the actual display decision; no
/// camera mode, native pointer, active interpreter or device is invented here.
pub struct Prepared<'a, K> {
    pub reset: bool,
    pub pool: &'a mut crate::attached_effect_context::Pool<K>,
    pub source: K,
    pub source_flags: u32,
    pub random: &'a mut crate::audio_parameters::Random,
}
/// Normal primary ownership with explicit access to canonical active emitters.
/// Creation still only prepares; the acquired world activation phase calls the
/// pool separately. This accepts no presentation-only or foreign actor handle.
pub struct Bound<'a> {
    pub session: &'a mut crate::resident_states::Session,
    pub prepared: Prepared<'a, crate::resident_states::Id>,
}
impl Services<crate::attached_effect_context::Id> for Bound<'_> {
    fn validate_binding(
        &mut self,
        binding: Option<crate::attached_effect_context::Id>,
    ) -> Result<(), String> {
        if let Some(id) = binding
            && self.prepared.pool.damage_smoke_actor(id).is_some()
        {
            let context = self.prepared.pool.get(id).unwrap();
            if context.recipient() != Some(self.prepared.source)
                || context.tag() != Some(3)
                || context.label != *b"FX_mecc_smoking_"
            {
                return Err("active damage smoke belongs to another source/profile".into());
            }
            return self.prepared.pool.validate_damage_smoke(id, self.session);
        }
        self.prepared.validate_binding(binding)
    }
    fn reset_before_selection(&mut self) -> Result<bool, String> {
        self.prepared.reset_before_selection()
    }
    fn release(&mut self, effect: crate::attached_effect_context::Id) -> Result<(), String> {
        self.validate_binding(Some(effect))?;
        if self.prepared.pool.damage_smoke_actor(effect).is_some() {
            self.prepared
                .pool
                .release_damage_smoke(effect, self.session)
                .map(|_| ())
        } else {
            self.prepared.release(effect)
        }
    }
    fn prepare(
        &mut self,
        level: u32,
    ) -> Result<Option<crate::attached_effect_context::Id>, String> {
        self.prepared.prepare(level)
    }
}
impl<K: Copy + Eq> Services<crate::attached_effect_context::Id> for Prepared<'_, K> {
    fn validate_binding(
        &mut self,
        binding: Option<crate::attached_effect_context::Id>,
    ) -> Result<(), String> {
        if let Some(effect) = binding {
            let context = self
                .pool
                .get(effect)
                .ok_or("damage smoke context absent, foreign or stale")?;
            if context.recipient() != Some(self.source)
                || context.tag() != Some(3)
                || context.flags() != 0x20004
                || context.label != *b"FX_mecc_smoking_"
            {
                return Err("damage smoke context belongs to another source/profile".into());
            }
        }
        Ok(())
    }
    fn reset_before_selection(&mut self) -> Result<bool, String> {
        Ok(self.reset)
    }
    fn release(&mut self, effect: crate::attached_effect_context::Id) -> Result<(), String> {
        let context = self
            .pool
            .get(effect)
            .ok_or("damage smoke context absent, foreign or stale")?;
        if context.recipient() != Some(self.source)
            || context.tag() != Some(3)
            || context.flags() != 0x20004
        {
            return Err("damage smoke context belongs to another source/profile".into());
        }
        self.pool.release_prepared(effect).map(|_| ())
    }
    fn prepare(
        &mut self,
        level: u32,
    ) -> Result<Option<crate::attached_effect_context::Id>, String> {
        let name = match level {
            1 => "FX_mecc_smoking_light",
            2 => "FX_mecc_smoking_medium",
            3 => "FX_mecc_smoking_heavy",
            _ => return Err("damage smoke creation requires a nonzero acquired level".into()),
        };
        let (creation, _) = self.pool.prepare_extended_invariant(
            name,
            self.source,
            self.source_flags,
            3,
            self.random,
        )?;
        Ok(match creation {
            crate::attached_effect_context::Creation::Absent => None,
            crate::attached_effect_context::Creation::Prepared(id) => Some(id),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Provider {
        calls: Vec<&'static str>,
        reset: bool,
        absent: bool,
        fail: Option<&'static str>,
    }
    impl Provider {
        fn enter(&mut self, name: &'static str) -> Result<(), String> {
            self.calls.push(name);
            if self.fail == Some(name) {
                Err(format!("injected {name} failure"))
            } else {
                Ok(())
            }
        }
    }
    impl Services<u32> for Provider {
        fn validate_binding(&mut self, _: Option<u32>) -> Result<(), String> {
            self.enter("validate")
        }
        fn reset_before_selection(&mut self) -> Result<bool, String> {
            self.enter("gate")?;
            Ok(self.reset)
        }
        fn release(&mut self, _: u32) -> Result<(), String> {
            self.enter("release")
        }
        fn prepare(&mut self, _: u32) -> Result<Option<u32>, String> {
            self.enter("prepare")?;
            Ok((!self.absent).then_some(7))
        }
    }
    fn input(damage: f32) -> Input {
        Input {
            damage,
            limit: 100.,
            thresholds: [40., 70., 90.],
        }
    }
    #[test]
    fn threshold_neighbors_priority_and_wide_percentage_are_preserved() {
        for (damage, expected) in [
            (0., 0),
            (39.999996, 0),
            (40., 1),
            (40.000004, 1),
            (69.99999, 1),
            (70., 2),
            (70.00001, 2),
            (89.99999, 2),
            (90., 3),
            (90.00001, 3),
        ] {
            assert_eq!(input(damage).level().unwrap(), expected);
        }
        // These finite values distinguish wide arithmetic from a stored f32 ratio.
        let wide = Input {
            damage: 28.,
            limit: 1000.,
            thresholds: [2.8, 70., 90.],
        };
        assert_eq!(wide.level().unwrap(), 1);
        let unsorted = Input {
            thresholds: [90., 70., 40.],
            ..input(55.)
        };
        assert_eq!(unsorted.level().unwrap(), 3);
    }
    #[test]
    fn reuse_reset_absence_and_late_failures_retain_native_order() {
        let mut state = State {
            level: 1,
            binding: Some(7),
        };
        let mut p = Provider::default();
        state.advance(input(55.), &mut p).unwrap();
        assert_eq!(p.calls, ["validate", "gate"]);
        p.calls.clear();
        p.reset = true;
        state.advance(input(55.), &mut p).unwrap();
        assert_eq!(p.calls, ["validate", "gate", "release", "prepare"]);
        for reset in [false, true] {
            let mut state = State {
                level: 1,
                binding: Some(7),
            };
            let mut p = Provider {
                reset,
                fail: Some("release"),
                ..Default::default()
            };
            assert!(state.advance(input(75.), &mut p).is_err());
            assert_eq!(
                (state.level, state.binding),
                (if reset { 0 } else { 1 }, Some(7))
            );
            let mut state = State {
                level: 1,
                binding: Some(7),
            };
            let mut p = Provider {
                reset,
                fail: Some("prepare"),
                ..Default::default()
            };
            assert!(state.advance(input(75.), &mut p).is_err());
            assert_eq!((state.level, state.binding), (2, None));
        }
        let mut state = State::default();
        let mut p = Provider {
            absent: true,
            ..Default::default()
        };
        for _ in 0..2 {
            state.advance(input(55.), &mut p).unwrap();
        }
        assert_eq!(state.level, 1);
        assert_eq!(state.binding, None);
        assert_eq!(p.calls.iter().filter(|&&c| c == "prepare").count(), 2);
        state.binding = Some(7);
        p.calls.clear();
        state.advance(input(0.), &mut p).unwrap();
        assert_eq!((state.level, state.binding), (0, None));
        assert_eq!(p.calls, ["validate", "gate", "release"]);
    }
    #[test]
    fn invalid_health_and_foreign_bindings_do_not_reach_the_gate() {
        let mut state = State {
            level: 1,
            binding: Some(7),
        };
        let mut p = Provider::default();
        for bad in [
            Input {
                damage: f32::NAN,
                ..input(1.)
            },
            Input {
                limit: 0.,
                ..input(1.)
            },
            input(-1.),
        ] {
            assert!(state.advance(bad, &mut p).is_err());
            assert!(p.calls.is_empty());
        }
        p.fail = Some("validate");
        assert!(state.advance(input(55.), &mut p).is_err());
        assert_eq!(p.calls, ["validate"]);
        assert_eq!((state.level, state.binding), (1, Some(7)));
    }
}
