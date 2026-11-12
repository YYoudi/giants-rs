//! Class230 mode66 countdown/decision inputs from the acquired PC callback.
//! Prototype PS2 `ripper_move` remains an unknown attribution. This composes
//! current perception and object time, not a new clock or a PS2 layout.
use super::*;

#[derive(Clone, Copy)]
pub struct VariantCooldownContext<'a> {
    pub close_range: f32,
    pub retreat_timer: [f32; 2],
    pub global_seconds: f32,
    pub areas: &'a [crate::object_time::SlowArea],
}
impl VariantCooldownContext<'_> {
    fn validate(&self, timer: f32) -> Result<(), String> {
        if ![timer, self.close_range, self.global_seconds]
            .into_iter()
            .chain(self.retreat_timer)
            .all(f32::is_finite)
            || self.global_seconds < 0.
            || self.areas.iter().any(|a| {
                !a.center
                    .into_iter()
                    .chain([a.factor, a.radius_squared])
                    .all(f32::is_finite)
                    || a.factor < 0.
                    || a.radius_squared < 0.
            })
        {
            return Err("variant cooldown requires actual finite timer/radii/object clock".into());
        }
        Ok(())
    }
}
pub(super) struct Plan {
    pub remaining: f32,
    pub expired: bool,
    pub close: bool,
}
pub(super) fn prepare<K: Copy + Ord, E>(
    state: &State<K, E>,
    frame: &Frame<'_, K>,
    p: VariantCooldownContext<'_>,
) -> Result<Plan, String> {
    let seconds = crate::object_time::seconds(
        p.global_seconds,
        Some(crate::object_time::Sample {
            owner: state.entry.context,
            position: state.entry.position,
        }),
        None,
        p.areas,
    );
    let remaining = f64::from(state.timer) - seconds;
    if !seconds.is_finite() || seconds < 0. || !(remaining as f32).is_finite() {
        return Err("variant cooldown object-time result outside finite profile".into());
    }
    let distance = if let Some(target) = state.retaliation {
        let position = frame.candidate(target)?.position;
        if !position.into_iter().all(f32::is_finite) {
            return Err("variant cooldown target position nonfinite".into());
        }
        let x = f64::from(position[0]) - f64::from(state.entry.position[0]);
        let y = f64::from(position[1]) - f64::from(state.entry.position[1]);
        (x * x + y * y) as f32
    } else {
        // Actual PC no-target squared-distance sentinel, not a distant actor.
        999_998_000_000.0_f32
    };
    if !distance.is_finite() {
        return Err("variant cooldown target distance overflow".into());
    }
    Ok(Plan {
        remaining: remaining as f32,
        expired: remaining < 0.,
        close: f64::from(distance) < f64::from(p.close_range) * f64::from(p.close_range),
    })
}
impl<K: Copy + Ord, E: Copy> Machine<K, E> {
    pub fn advance_variant_cooldown(
        &mut self,
        input: FrameInput<'_, K>,
        p: VariantCooldownContext<'_>,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        p.validate(self.state.timer)?;
        self.advance_inner(
            input.context,
            input.frame,
            input.range,
            Branch::VariantCooldown(p),
            random,
            services,
        )
    }
}
