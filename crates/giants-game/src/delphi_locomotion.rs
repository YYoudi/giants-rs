//! Acquired living dry sword locomotion secondary, after physical history.
//! Water, equipment changes, attacks, damage and landing effects remain separate.
use crate::{animation_channels::Request, audio_parameters::Random};

/// Animation identifiers supplied by the installation's character definitions.
/// No original executable table is embedded in this service.
#[derive(Clone, Copy, Debug)]
pub struct Clips {
    pub idle: [u8; 3],
    pub side: [u8; 2],
    pub forward: [u8; 2],
    pub turn: u8,
    pub jump: [u8; 2],
    pub fall: [u8; 2],
}
impl Clips {
    pub(crate) fn slot(self, mode: i32) -> Result<u8, String> {
        Ok(match mode {
            0..=2 => self.idle[mode as usize],
            3..=4 => self.side[(mode - 3) as usize],
            5 => self.forward[0],
            8 => self.turn,
            21 => self.jump[0],
            25 => self.jump[1],
            28..=29 => self.fall[(mode - 28) as usize],
            35 => self.forward[1],
            _ => return Err("Delphi secondary mode requires an unacquired action consumer".into()),
        })
    }
    pub(crate) fn slots(self) -> impl Iterator<Item = u8> {
        self.idle
            .into_iter()
            .chain(self.side)
            .chain(self.forward)
            .chain([self.turn])
            .chain(self.jump)
            .chain(self.fall)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub impact: f32,
    pub fall_speed: f32,
    pub fall_height: f32,
    pub slide_speed: f32,
    pub slide_fall_speed: f32,
}
impl Parameters {
    pub fn from_globals(g: &giants_formats::global_data::GlobalData) -> Result<Self, String> {
        let scalar = |at| g.scalar(at).map_err(|e| e.to_string());
        Ok(Self {
            impact: scalar(0xbc8)?,
            fall_speed: scalar(0xc10)?,
            fall_height: scalar(0x117c)?,
            slide_speed: scalar(0x1174)?,
            slide_fall_speed: scalar(0x1178)?,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub mode: i32,
    pub internal: u32,
    pub active: u32,
    pub controls: u32,
    pub physics: u32,
    pub clip: u32,
    pub wrapped: bool,
    pub model: [f32; 4],
    pub velocity: [f32; 3],
    pub previous_velocity: [f32; 3],
    pub position_z: f32,
    pub floor: f32,
    pub offset: f32,
    pub scale: f32,
    pub damage: f32,
    pub maximum_damage: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Selection {
    pub mode: i32,
    pub internal: u32,
    pub controls: u32,
    pub request: Option<Request>,
}
pub fn select(
    s: State,
    p: Parameters,
    clips: Clips,
    random: &mut Random,
) -> Result<Selection, String> {
    clips.slot(s.mode)?;
    if !s
        .model
        .into_iter()
        .chain(s.velocity)
        .chain(s.previous_velocity)
        .chain([
            s.position_z,
            s.floor,
            s.offset,
            s.scale,
            s.damage,
            s.maximum_damage,
            p.impact,
            p.fall_speed,
            p.fall_height,
            p.slide_speed,
            p.slide_fall_speed,
        ])
        .all(f32::is_finite)
        || s.scale <= 0.
        || s.damage < 0.
        || s.damage >= s.maximum_damage
        || s.physics & 0x40 != 0
        || s.internal != 0
        || s.active & 0x800 != 0
        || s.controls & !(0x4 | 0x800 | 0x4000000) != 0
        || clips.slots().any(|slot| slot >= 254)
    {
        return Err("Delphi secondary outside the acquired living dry sword profile".into());
    }
    let mut next_random = *random;
    // The original range helper uses the shared thread's actual 15-bit draw.
    let mut draw = || (next_random.next_15() * 100) >> 15;
    let mut mode = s.mode;
    let mut controls = s.controls & !0x4000000;
    let mut internal = s.internal;
    let mut looping = false;
    let mut speed_source = 0;
    let mut reference = 0.0f32;
    let mut fallback = false;
    if s.active & 0x100 != 0 {
        if !matches!(mode, 21 | 25) {
            mode = if s.model[1] == 0. { 21 } else { 25 };
            looping = true;
        }
    } else if s.velocity[2] < 0. {
        if controls & 0x800 == 0 {
            let gap = (f64::from(s.position_z) - f64::from(s.floor))
                - f64::from(s.offset) * f64::from(s.scale);
            if s.velocity[2] < -p.fall_speed && f64::from(p.fall_height) < gap {
                controls |= 0x4000000;
                if mode == 28 {
                    if s.wrapped {
                        mode = 29;
                        looping = true;
                    }
                } else if mode != 29 {
                    mode = 28;
                }
            } else {
                fallback = true;
            }
        } else {
            fallback = s.active & 0x80000 != 0;
        }
    } else if s.active & 0x80000 == 0 {
        fallback = true;
    } else if s.model[1] != 0. {
        if mode == 35 && !s.wrapped { /* Retain the current variant. */
        } else {
            mode = if mode == 5 && s.wrapped && draw() < 10 {
                35
            } else {
                5
            };
            reference = 30.;
            speed_source = 1;
        }
        looping = true;
    } else if s.model[2] != 0. {
        mode = if s.model[2] > 0. { 3 } else { 4 };
        reference = if s.model[2] > 0. { 30. } else { -30. };
        speed_source = 2;
        looping = true;
    } else if s.model[3] != 0. {
        mode = 8;
        reference = 200.;
        speed_source = 3;
        looping = true;
    } else if matches!(mode, 1 | 2) && !s.wrapped { /* Preserve the idle until it wraps. */
    } else if mode == 0 && s.wrapped && draw() >= 96 {
        mode = if draw() >= 50 { 2 } else { 1 };
    } else {
        fallback = true;
    }

    if fallback {
        let delta: [f32; 3] = std::array::from_fn(|i| {
            (f64::from(s.velocity[i]) - f64::from(s.previous_velocity[i])) as f32
        });
        let length = |v: [f32; 3]| {
            v.into_iter()
                .map(|x| f64::from(x) * f64::from(x))
                .sum::<f64>()
                .sqrt()
        };
        if length(delta) >= f64::from(p.impact)
            && length(s.velocity) < f64::from(length(s.previous_velocity) as f32)
        {
            return Err("Delphi impact requires its landing effect/damage producer".into());
        }
        if (s.active & 0x80000 != 0 || s.physics & 2 != 0)
            && (s.velocity[2] < -p.slide_speed
                || (s.mode == 29 && s.velocity[2] < -p.slide_fall_speed))
        {
            return Err("Delphi slide requires its acquired effect producer".into());
        }
        controls &= !0x800;
        if s.model[1] != 0. {
            mode = 5;
            reference = 30.;
            speed_source = 1;
            looping = true;
        } else if s.model[2] != 0. {
            mode = if s.model[2] > 0. { 3 } else { 4 };
            reference = if s.model[2] > 0. { 30. } else { -30. };
            speed_source = 2;
            looping = true;
        } else if s.model[3] != 0. {
            mode = 8;
            reference = 200.;
            speed_source = 3;
            looping = true;
        } else {
            mode = 0;
            looping = true;
        }
        internal &= !0x10;
    }
    internal &= !0x110;
    let slot = clips.slot(mode)?;
    // A shared mapped clip can retain the old mode; compare clips, not mode IDs.
    let request = (u32::from(slot) != s.clip).then_some(Request {
        slot,
        flags: if looping { 0x21 } else { 0x20 },
        speed_source,
        reference_speed: (f64::from(reference) * f64::from(s.scale)) as f32,
        blend_seconds: f32::from_bits(0x3e2aaaab),
    });
    *random = next_random;
    Ok(Selection {
        mode: if request.is_some() { mode } else { s.mode },
        internal,
        controls,
        request,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (State, Parameters, Clips) {
        (
            State {
                mode: 0,
                internal: 0,
                active: 0x80000,
                controls: 0,
                physics: 2,
                clip: 11,
                wrapped: false,
                model: [0.; 4],
                velocity: [0.; 3],
                previous_velocity: [0.; 3],
                position_z: 0.,
                floor: 0.,
                offset: 0.,
                scale: 1.,
                damage: 0.,
                maximum_damage: 250.,
            },
            Parameters {
                impact: 100.,
                fall_speed: 100.,
                fall_height: 10.,
                slide_speed: 100.,
                slide_fall_speed: 100.,
            },
            Clips {
                idle: [11, 12, 13],
                side: [14, 15],
                forward: [16, 17],
                turn: 18,
                jump: [19, 20],
                fall: [21, 22],
            },
        )
    }
    #[test]
    fn jump_latches_through_command_changes_and_release_until_contact() {
        let (mut s, p, c) = fixture();
        let mut random = Random(1);
        s.active = 0x100;
        s.model[1] = 20.;
        let a = select(s, p, c, &mut random).unwrap();
        assert_eq!(a.mode, 25);
        s.mode = a.mode;
        s.clip = u32::from(a.request.unwrap().slot);
        s.model[1] = 0.;
        s.model[2] = -20.;
        assert!(select(s, p, c, &mut random).unwrap().request.is_none());
        s.active = 0;
        s.controls = 0x800;
        s.velocity[2] = -5.;
        assert_eq!(select(s, p, c, &mut random).unwrap().mode, 25);
        s.active = 0x80000;
        s.velocity[2] = 0.;
        s.model = [0.; 4];
        let landed = select(s, p, c, &mut random).unwrap();
        assert_eq!(landed.mode, 0);
        assert_eq!(landed.controls & 0x800, 0);
        assert_eq!(random.0, 1);
    }
    #[test]
    fn idle_variants_consume_two_shared_draws_only_at_the_idle_wrap() {
        for (seed, mode) in [(9634, 1), (9787, 2)] {
            let (mut s, p, c) = fixture();
            let mut random = Random(seed);
            assert!(select(s, p, c, &mut random).unwrap().request.is_none());
            assert_eq!(random.0, seed);
            s.wrapped = true;
            let idle = select(s, p, c, &mut random).unwrap();
            assert_eq!(idle.mode, mode);
            let after = random.0;
            s.mode = mode;
            s.clip = u32::from(idle.request.unwrap().slot);
            s.wrapped = false;
            assert!(select(s, p, c, &mut random).unwrap().request.is_none());
            assert_eq!(random.0, after);
            s.wrapped = true;
            assert_eq!(select(s, p, c, &mut random).unwrap().mode, 0);
            assert_eq!(random.0, after);
        }
    }
    #[test]
    fn a_shared_mapped_clip_retains_the_actual_mode_and_channel_state() {
        let (mut s, p, c) = fixture();
        s.model[1] = 20.;
        s.clip = u32::from(c.forward[0]);
        s.controls = 0x4000000;
        let result = select(s, p, c, &mut Random(1)).unwrap();
        assert_eq!(result.mode, 0);
        assert!(result.request.is_none());
        assert_eq!(result.controls & 0x4000000, 0);
    }
    #[test]
    fn unacquired_action_and_impact_refusals_preserve_random_state() {
        let (mut s, p, c) = fixture();
        let mut random = Random(9634);
        s.controls = 0x200;
        s.wrapped = true;
        assert!(select(s, p, c, &mut random).is_err());
        assert_eq!(random.0, 9634);
        s.controls = 0;
        s.wrapped = false;
        s.previous_velocity[2] = -200.;
        assert!(select(s, p, c, &mut random).is_err());
        assert_eq!(random.0, 9634);
    }
}
