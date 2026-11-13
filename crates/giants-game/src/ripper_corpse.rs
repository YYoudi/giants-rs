//! Acquired PC corpse helper, separate from the lethal hit and actor cleanup.
//! Prototype PS2 `ripper_death_logic__FP6OBJECT` is a confirmed routine identity
//! for debug-1.1 0x004d92a0. This Rust entry admits class73/230 only; no prototype
//! signature, layout or single-precision arithmetic is imported.
use crate::{audio_parameters::Random, state23_damage::State};

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    /// Actual class-global values, not a drop probability or Vimp meat count.
    pub large_mana: f32,
    pub medium_mana: f32,
}

pub trait Services<I> {
    /// Execute the actual nearby-recipient lookup and conditional creation.
    /// A genuine no-recipient result may return false. Missing ownership must
    /// return Err; neither invocation nor a guessed recipient is success.
    fn create_mana(
        &mut self,
        corpse: &State<I>,
        amount: f32,
        random: &mut Random,
    ) -> Result<bool, String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Retained,
    /// The original helper marks the actor; this does not release its owners.
    Marked,
}

/// Continue a genuine completed class73/230 lethal callback. This helper uses
/// the global clock (null object/position), unlike living object-local timers.
/// `corpse.timer` is the lethal callback's header countdown, distinct from the
/// living AI payload timer. The creation gate reads `control`, independently
/// of the personality bits changed by the lethal callback.
/// It stores the rounded timer but branches on the wide subtraction. A zero
/// mana parameter retires immediately, including before the timer expires.
///
/// A service error retains the consumed timer and external RNG/side effects.
/// The owning primary must latch that failure rather than retry this callback.
/// This is not a whole primary, a lethal producer or a retirement receipt.
#[doc(alias = "ripper_death_logic__FP6OBJECT")]
pub fn ripper_death_logic<I: Copy>(
    corpse: &mut State<I>,
    global_seconds: f32,
    parameters: Parameters,
    random: &mut Random,
    services: &mut impl Services<I>,
) -> Result<Outcome, String> {
    if !matches!(corpse.class, 73 | 230)
        || corpse.mode != 94
        || corpse.flags & 0x4000_0010 != 0x4000_0010
        || corpse.limit <= 0.
        || corpse.damage < corpse.limit
        || ![
            corpse.timer,
            corpse.limit,
            corpse.damage,
            global_seconds,
            parameters.large_mana,
            parameters.medium_mana,
        ]
        .into_iter()
        .all(f32::is_finite)
    {
        return Err("corpse continuation requires actual finite completed Ripper death".into());
    }
    let remaining = f64::from(corpse.timer) - f64::from(global_seconds);
    if !(remaining as f32).is_finite() {
        return Err("corpse countdown outside finite profile".into());
    }
    corpse.timer = remaining as f32;
    if remaining >= 0. {
        let amount = if corpse.class == 230 {
            parameters.large_mana
        } else {
            parameters.medium_mana
        };
        if amount != 0.
            && (corpse.control & 8 != 0 || !services.create_mana(corpse, amount, random)?)
        {
            return Ok(Outcome::Retained);
        }
    }
    corpse.flags |= 0x8000;
    Ok(Outcome::Marked)
}

#[cfg(test)]
mod tests;
