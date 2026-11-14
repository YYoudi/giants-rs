//! Acquired classic modes35/36/45. Prototype PS2 `ripper_move` is an unknown
//! navigation relationship. This follows the PC helper's body-word tests;
//! it does not invent a countdown writer or force a creature position.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct ClassicOrbitParameters {
    pub orbit: crate::ripper_turn_completion::Parameters,
    pub decision_range: f32,
    pub close_percent: f32,
    pub fire_percent: f32,
    pub buried_speed_percent: f32,
}
impl ClassicOrbitParameters {
    pub fn from_globals(
        data: &giants_formats::global_data::GlobalData,
    ) -> Result<Self, giants_formats::Error> {
        Ok(Self {
            orbit: crate::ripper_turn_completion::Parameters::from_globals(data)?,
            decision_range: data.scalar(0xd4)?,
            close_percent: data.scalar(0xdc)?,
            fire_percent: data.scalar(0xe4)?,
            buried_speed_percent: data.scalar(0x288)?,
        })
    }
    fn validate(self) -> Result<(), String> {
        if !self
            .orbit
            .radius
            .into_iter()
            .chain([
                self.orbit.seconds,
                self.decision_range,
                self.close_percent,
                self.fire_percent,
                self.buried_speed_percent,
            ])
            .all(f32::is_finite)
        {
            return Err("classic orbit needs actual finite source parameters".into());
        }
        Ok(())
    }
}
impl<K: Copy + Ord, E: Copy> Machine<K, E> {
    pub fn advance_classic_orbit(
        &mut self,
        input: FrameInput<'_, K>,
        parameters: ClassicOrbitParameters,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        parameters.validate()?;
        self.advance_inner(
            input.context,
            input.frame,
            input.range,
            Branch::ClassicOrbit(parameters),
            random,
            services,
        )
    }
}
pub(super) fn initialise_orbit<K: Copy, E: Copy>(
    s: &mut State<K, E>,
    target: [f32; 3],
    span: f32,
    p: Parameters,
    random: &mut Random,
) {
    let bearing = crate::actor_volume::heading_wide(target, s.entry.position);
    let low = (bearing - f64::from(span)) as f32;
    let high = (bearing + f64::from(span)) as f32;
    let interval = |r: &mut Random, a: f32, b: f32| {
        (f64::from(r.next_15()) * (f64::from(b) - f64::from(a)) * (1. / 32768.) + f64::from(a))
            as f32
    };
    s.orbit.angle = crate::ripper_steering::normalize(interval(random, low, high));
    s.orbit.radius = interval(random, p.radius[0], p.radius[1]);
    s.timer = p.seconds;
}
pub(super) fn advance<K: Copy + Ord, E: Copy>(
    s: &mut State<K, E>,
    frame: &Frame<'_, K>,
    p: ClassicOrbitParameters,
    random: &mut Random,
    services: &mut impl Services<K, E>,
) -> Result<(), String> {
    if s.entry.mode == 45 {
        if s.effect.is_none() {
            s.effect = services.prepare_awakening_dirt(s, random)?;
        }
        if f32::from_bits(s.primary[1]) > 0.73333335_f32 {
            services.mark_surface_marker_retirement(s)?;
            s.steering.personality |= 0x40;
        }
        if s.primary[8] & 2 != 0 {
            // The pinned debug ground hook is a RET. No fabricated projection.
            if let Some(id) = s.retaliation {
                initialise_orbit(s, frame.candidate(id)?.position, 80., p.orbit, random);
            }
            s.entry.mode = 36;
            s.steering.personality |= 0x840;
            s.parameter_184 = 1.;
            services.owner_sound(s, 1256, random)?;
        }
        return Ok(());
    }
    let target = s.retaliation.map(|id| frame.candidate(id)).transpose()?;
    let (lost, control) =
        crate::ripper_pursuit::target_control(target, s.entry.retained_timer, s.entry.control)?;
    s.entry.control = control;
    if lost {
        if s.entry.mode == 36 {
            if let Some(effect) = s.effect {
                services.remove_effect(s, effect, random)?;
            }
            s.effect = None;
        } else {
            services.owner_sound(s, 1269, random)?;
        }
        s.entry.mode = 43;
        s.retaliation = None;
        return Ok(());
    }
    let target = target.unwrap();
    s.steering = crate::ripper_steering::pursue(s.steering, target.position, s.orbit)?;
    s.entry.flags = s.steering.flags;
    s.entry.packed = u32::from_le_bytes(s.steering.commands[..4].try_into().unwrap());
    let delta = |point: [f32; 3]| {
        let x = f64::from(point[0]) - f64::from(s.entry.position[0]);
        let y = f64::from(point[1]) - f64::from(s.entry.position[1]);
        x * x + y * y
    };
    let point_squared = delta(s.steering.target_position);
    if !point_squared.is_finite() {
        return Err("classic orbit point distance overflow".into());
    }
    if s.entry.mode == 36 {
        if point_squared < 5. || s.timer < 0. {
            let normal = services.ground_normal_z(s)?;
            if !normal.is_finite() {
                return Err("classic orbit ground normal nonfinite".into());
            }
            if normal >= 0.64278_f32 {
                s.entry.mode = 44;
                services.stop_owner_sound(s, 1256, random)?;
            }
        }
        return Ok(());
    }
    let target_squared = delta(target.position) as f32;
    if !target_squared.is_finite() {
        return Err("classic orbit target distance overflow".into());
    }
    if f64::from(target_squared) >= f64::from(p.decision_range).powi(2) {
        if point_squared >= 5. && s.timer >= 0. {
            return Ok(());
        }
        let roll = (random.next_15() * 100) >> 15;
        s.entry.mode = if f64::from(roll) < f64::from(p.fire_percent) {
            37
        } else {
            39
        };
    } else {
        let roll = (random.next_15() * 100) >> 15;
        if f64::from(roll) < f64::from(p.close_percent) {
            s.entry.mode = 41;
        } else {
            initialise_orbit(s, target.position, 20., p.orbit, random);
        }
    }
    Ok(())
}
