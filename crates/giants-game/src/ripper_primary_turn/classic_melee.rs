//! Classic mode42 caller. Prototype PS2 `ripper_move` / `hurt_object` remain
//! unknown attributions. A finished channel requests actual kind3 injury;
//! entering this branch never supplies world readiness or successful damage.
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct ClassicMeleeContext {
    pub attack_range: f32,
    pub damage: f32,
    pub orbit: Parameters,
}
impl<K: Copy + Ord, E: Copy> Machine<K, E> {
    pub fn advance_classic_melee(
        &mut self,
        input: FrameInput<'_, K>,
        parameters: ClassicMeleeContext,
        random: &mut Random,
        services: &mut impl Services<K, E>,
    ) -> Result<Outcome, String> {
        if !parameters
            .orbit
            .radius
            .into_iter()
            .chain([
                parameters.orbit.seconds,
                parameters.attack_range,
                parameters.damage,
            ])
            .all(f32::is_finite)
        {
            return Err("classic melee needs actual finite class-global parameters".into());
        }
        self.advance_inner(
            input.context,
            input.frame,
            input.range,
            Branch::ClassicMelee(parameters),
            random,
            services,
        )
    }
}
pub(super) fn advance<K: Copy + Ord, E: Copy>(
    s: &mut State<K, E>,
    frame: &Frame<'_, K>,
    p: ClassicMeleeContext,
    random: &mut Random,
    services: &mut impl Services<K, E>,
) -> Result<(), String> {
    let target = s.retaliation.map(|id| frame.candidate(id)).transpose()?;
    let (lost, control) =
        crate::ripper_pursuit::target_control(target, s.entry.retained_timer, s.entry.control)?;
    s.entry.control = control;
    if lost {
        services.owner_sound(s, 1269, random)?;
        s.entry.mode = 43;
        s.retaliation = None;
        return Ok(());
    }
    let target = target.unwrap();
    s.steering = crate::ripper_steering::aim_at_point(s.steering, target.position)?;
    s.entry.flags = s.steering.flags;
    s.entry.packed = u32::from_le_bytes(s.steering.commands[..4].try_into().unwrap());
    if s.primary[8] & 2 != 0 {
        classic_orbit::initialise_orbit(s, target.position, 20., p.orbit, random);
        s.entry.mode = 35;
        let delta = std::array::from_fn::<_, 3, _>(|i| {
            f64::from(target.position[i]) - f64::from(s.entry.position[i])
        });
        let squared = ((delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]) as f32;
        if !squared.is_finite() {
            return Err("classic melee distance overflow".into());
        }
        if f64::from(squared) < f64::from(p.attack_range).powi(2) {
            services.melee(s, target.id, p.damage, random)?;
        }
    }
    Ok(())
}
