//! State23 secondary callback: animation choice and ordered attack requests.
//! Clocks, behavior mode and global parameters must come from their producers.
use crate::{
    animation_channels::{self, Blending, Channels, Request},
    audio_parameters::Random,
};

#[derive(Clone, Debug)]
pub struct State {
    pub class: i32,
    pub mode: u32,
    pub timer: f32,
    pub counter: i32,
    pub forward_speed: f32,
    pub channels: Channels,
}
#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub parent_state: Option<u32>,
    pub target_position: Option<[f32; 3]>,
    /// Source parameter times actor scale, supplied for the mode20 leap.
    pub leap_reference_speed: f32,
    /// Source payload resource value, not a movement estimate.
    pub leap_model_speed: f32,
    /// Passed to the enemy launch writer; not the projectile's forward speed.
    pub vertical_spread_percent: f32,
    pub cooldown: [f32; 2],
    pub repeat_timer: f32,
    pub blending: Blending,
}
/// Distinguish actual creation from the original absent-resource return.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireOutcome {
    Spawned,
    ResourceUnavailable,
}
/// Execute the real attack service, or return an error. A resource-unavailable
/// result must come from a real lookup. The service can consume shared RNG
/// before the callback's cooldown draw; a backend is never supplied implicitly.
pub trait Services {
    fn fire(
        &mut self,
        state: &State,
        target: [f32; 3],
        vertical_spread_percent: f32,
        random: &mut Random,
    ) -> Result<FireOutcome, String>;
}
fn variant(random: &mut Random, low: u8, high: u8) -> u8 {
    if (random.next_15() * 100) >> 15 <= 50 {
        high
    } else {
        low
    }
}
pub fn update(
    mut state: State,
    context: Context,
    random: &mut Random,
    services: &mut impl Services,
) -> Result<State, String> {
    if !matches!(state.class, 73 | 230) {
        return Err("state23 animation requires acquired Ripper classes".into());
    }
    if matches!(context.parent_state, Some(1 | 76)) {
        return Ok(state);
    }
    let channel = state.channels[0];
    let now = f32::from_bits(channel[1]);
    let previous = f32::from_bits(channel[2]);
    if ![state.timer, state.forward_speed, now, previous]
        .into_iter()
        .chain([
            context.leap_reference_speed,
            context.leap_model_speed,
            context.vertical_spread_percent,
            context.repeat_timer,
        ])
        .chain(context.cooldown)
        .chain(context.target_position.into_iter().flatten())
        .all(f32::is_finite)
    {
        return Err("state23 animation outside finite callback profile".into());
    }
    let current = channel[0];
    let wrapped = channel[8] & 2 != 0;
    let mut looping = true;
    let slot = match state.mode {
        5..=9 | 33 | 35 | 40 | 41 | 63 | 64 | 67 | 80 | 100 | 101 | 103 | 104 | 150 | 151 => 2,
        10 => 51,
        69 | 70 => {
            looping = false;
            51
        }
        11 => 9,
        20 => {
            if current == 2 {
                return Ok(state);
            }
            state.forward_speed = context.leap_model_speed;
            animation_channels::select_primary(
                &mut state.channels,
                Request {
                    slot: 2,
                    flags: 0x21,
                    speed_source: 1,
                    reference_speed: context.leap_reference_speed,
                    blend_seconds: f32::from_bits(0x3e2a_aaab),
                },
                context.blending,
            )?;
            return Ok(state);
        }
        21 | 106 => {
            if current == 1 && wrapped {
                50
            } else if current != 1 && wrapped {
                1
            } else {
                if previous < f32::from_bits(0x3edd_ddde) && now >= f32::from_bits(0x3edd_ddde) {
                    if let Some(target) = context.target_position {
                        services.fire(&state, target, context.vertical_spread_percent, random)?;
                        state.timer = context.repeat_timer;
                        state.counter = state.counter.wrapping_sub(1);
                    } else {
                        state.counter = 0;
                    }
                }
                50
            }
        }
        31 | 34 | 36 | 44 | 61 => 42,
        32 | 62 => {
            looping = false;
            if matches!(current, 40 | 41) {
                current as u8
            } else {
                variant(random, 40, 41)
            }
        }
        37 | 38 => {
            looping = false;
            1
        }
        39 | 65 => {
            looping = false;
            50
        }
        42 | 68 => {
            looping = false;
            10
        }
        43 | 45 | 71 => {
            looping = false;
            43
        }
        85 => {
            if !wrapped {
                return Ok(state);
            }
            if state.timer < 0. && context.target_position.is_some() {
                state.mode = 86;
                50
            } else {
                1
            }
        }
        86 => {
            if previous < f32::from_bits(0x3edd_ddde)
                && now >= f32::from_bits(0x3edd_ddde)
                && let Some(target) = context.target_position
            {
                services.fire(&state, target, context.vertical_spread_percent, random)?;
            }
            if wrapped {
                state.timer = (f64::from(random.next_15())
                    * (f64::from(context.cooldown[1]) - f64::from(context.cooldown[0]))
                    * (1. / 32768.)
                    + f64::from(context.cooldown[0])) as f32;
                state.mode = 85;
                1
            } else {
                50
            }
        }
        90 | 108 => {
            looping = false;
            if matches!(current, 49 | 52) {
                current as u8
            } else {
                variant(random, 49, 52)
            }
        }
        91 | 94 => {
            looping = state.mode == 94;
            44
        }
        92 => {
            if matches!(current, 45 | 48) {
                return Ok(state);
            }
            variant(random, 45, 48)
        }
        93 => {
            looping = false;
            46
        }
        152 => 57,
        _ => 1,
    };
    if current != u32::from(slot) {
        animation_channels::select_primary(
            &mut state.channels,
            Request {
                slot,
                flags: if looping { 0x21 } else { 0x20 },
                speed_source: i32::from(slot == 2),
                reference_speed: if slot == 2 {
                    context.leap_reference_speed
                } else {
                    0.
                },
                blend_seconds: f32::from_bits(0x3e2a_aaab),
            },
            context.blending,
        )?;
    }
    Ok(state)
}
#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn fixture() -> (State, Context) {
        let mut channels = [[0; 9]; 4];
        channels[0][0] = 50;
        channels[0][1] = f32::from_bits(0x3edd_ddde).to_bits();
        channels[0][2] = f32::from_bits(0x3edd_dddd).to_bits();
        (
            State {
                class: 73,
                mode: 86,
                timer: 0.,
                counter: 1,
                forward_speed: 0.,
                channels,
            },
            Context {
                parent_state: None,
                target_position: Some([1., 2., 3.]),
                leap_reference_speed: 1.,
                leap_model_speed: 2.,
                vertical_spread_percent: 100.,
                cooldown: [1., 3.],
                repeat_timer: 2.,
                blending: Blending::Enabled,
            },
        )
    }
    struct Fire {
        calls: u32,
        refuse: bool,
        consume: bool,
    }
    impl Services for Fire {
        fn fire(
            &mut self,
            _: &State,
            _: [f32; 3],
            _: f32,
            random: &mut Random,
        ) -> Result<FireOutcome, String> {
            self.calls += 1;
            if self.consume {
                random.next_15();
            }
            if self.refuse {
                Err("real projectile service unavailable".into())
            } else {
                Ok(FireOutcome::ResourceUnavailable)
            }
        }
    }
    #[test]
    fn event_crossing_is_strict_before_and_inclusive_after() {
        let (s, c) = fixture();
        let mut random = Random(1);
        let mut fire = Fire {
            calls: 0,
            refuse: false,
            consume: false,
        };
        update(s.clone(), c, &mut random, &mut fire).unwrap();
        assert_eq!(fire.calls, 1);
        let mut s = s;
        s.channels[0][2] = 0x3edd_ddde;
        update(s, c, &mut random, &mut fire).unwrap();
        assert_eq!(fire.calls, 1);
    }
    #[test]
    fn firing_consumes_random_before_cooldown_draw_and_failure_does_not_continue() {
        let (mut s, c) = fixture();
        s.channels[0][8] = 2;
        let mut random = Random(7);
        let mut expected = random;
        expected.next_15();
        let draw = expected.next_15();
        let mut fire = Fire {
            calls: 0,
            refuse: false,
            consume: true,
        };
        let result = update(s.clone(), c, &mut random, &mut fire).unwrap();
        assert_eq!(result.mode, 85);
        assert_eq!(result.timer, ((f64::from(draw) * 2. / 32768.) + 1.) as f32);
        assert_eq!(random.0, expected.0);
        let mut random = Random(7);
        let mut fire = Fire {
            calls: 0,
            refuse: true,
            consume: true,
        };
        assert!(update(s, c, &mut random, &mut fire).is_err());
        let mut one = Random(7);
        one.next_15();
        assert_eq!(random.0, one.0);
    }
    #[test]
    fn zero_timer_does_not_start_attack_and_retained_variant_does_not_draw() {
        let (mut s, c) = fixture();
        s.mode = 85;
        s.channels[0][8] = 2;
        s.channels[0][0] = 1;
        let mut random = Random(7);
        let mut fire = Fire {
            calls: 0,
            refuse: false,
            consume: false,
        };
        assert_eq!(
            update(s.clone(), c, &mut random, &mut fire).unwrap().mode,
            85
        );
        s.timer = -f32::MIN_POSITIVE;
        assert_eq!(
            update(s.clone(), c, &mut random, &mut fire).unwrap().mode,
            86
        );
        s.mode = 90;
        s.channels[0][0] = 52;
        let before = random.0;
        let channels = s.channels;
        assert_eq!(
            update(s, c, &mut random, &mut fire).unwrap().channels,
            channels
        );
        assert_eq!(random.0, before);
        assert_eq!(fire.calls, 0);
    }
}
