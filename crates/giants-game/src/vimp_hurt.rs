//! Offline hurt-state kernel. Shared movement, growth, physics, ANM selection
//! and whole-frame completion remain separate consumers.
use crate::{audio_parameters::Random, resident_states::Id};
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub mode: u32,
    pub delay: f32,
    pub speed_68: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Herd {
    pub word_0c: u32,
    pub timer_10: f32,
    pub flags_14: u32,
    pub timer_18: f32,
}
pub trait Sounds {
    fn stop(&mut self, actor: Id, selector: u32, random: &mut Random) -> Result<(), String>;
}
pub struct Tick<'a> {
    pub number: u64,
    pub global_delta: f32,
    pub slow_areas: &'a [crate::object_time::SlowArea],
}
fn interval(random: &mut Random, low: f32, high: f32) -> f64 {
    f64::from(random.next_15()) * f64::from(high - low) / 32768. + f64::from(low)
}
/// The caller supplies the actual completion flag and local float32 time.
/// A failed stop may have external effects; local/RNG progress is not undone.
pub fn advance(
    state: &mut State,
    herd: Option<&mut Herd>,
    seconds: f32,
    finished: bool,
    random: &mut Random,
    mut stop: impl FnMut(u32, &mut Random) -> Result<(), String>,
) -> Result<(), String> {
    if !matches!(state.mode, 2..=4)
        || ![state.delay, state.speed_68, seconds]
            .into_iter()
            .all(f32::is_finite)
        || seconds < 0.
        || herd
            .as_ref()
            .is_some_and(|h| !h.timer_10.is_finite() || !h.timer_18.is_finite())
    {
        return Err("Vimp hurt state outside finite modes2..4 profile".into());
    }
    match state.mode {
        4 => {
            if finished {
                stop(0x561, random)?;
                stop(0x562, random)?;
                state.mode = 2;
                state.delay = interval(random, 1.5, 5.) as f32;
            }
            if let Some(h) = herd
                && h.flags_14 & 2 == 0
            {
                h.flags_14 &= !0x20;
                h.word_0c = 0;
                h.timer_18 = interval(random, 2., 5.) as f32;
                let next = f64::from(h.timer_18) + interval(random, 1., 5.);
                h.flags_14 |= 2;
                h.timer_10 = next as f32;
            }
        }
        2 => {
            let next = f64::from(state.delay) - f64::from(seconds);
            if !(next as f32).is_finite() {
                return Err("Vimp hurt delay overflow".into());
            }
            state.delay = next as f32;
            if finished && next < 0. {
                state.mode = 3;
            }
        }
        3 if finished => {
            state.speed_68 = 0.;
            state.mode = 1;
        }
        _ => {}
    }
    Ok(())
}
