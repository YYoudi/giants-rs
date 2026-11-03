//! Hurt-mode primary composition. World admission/history and post-actor
//! scheduling remain external; no ordinary mode1 AI is substituted here.
use crate::vimp_wander::Tick;

pub struct Primary<'a> {
    pub number: u64,
    pub tick: Tick<'a>,
}
pub struct Services<'a> {
    pub animation: crate::vimp_animation::Services<'a>,
    pub sounds: &'a mut dyn crate::vimp_hurt::Sounds,
    pub ground: &'a mut dyn FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
}
pub(crate) fn orient(
    fields: &mut crate::actor_fields::Fields,
    vimp: &mut crate::vimp_actor::Cold,
    tick: Tick<'_>,
) -> Result<(), String> {
    let p = |i| f32::from_bits(fields.configuration[i]);
    let oriented = crate::actor_common_motion::orient(
        crate::actor_common_motion::State {
            position: fields.position,
            heading: fields.angles[0],
            look_pitch: 0.,
            rates: [0., 0., 0., vimp.angular_speed],
            previous_rates: [0., 0., 0., vimp.previous_angular_speed],
            active: fields.object_flags,
            personality: fields.configuration[14],
            state: 21,
            context: vimp.context,
            parent_present: false,
            controls: std::array::from_fn(|i| vimp.commands[i] as i8),
            target_heading: 0.,
            target_position: [0.; 2],
        },
        &crate::actor_common_motion::Environment {
            parameters: crate::actor_common_motion::Parameters {
                acceleration: p(0),
                speed: p(1),
                braking: p(4),
                attenuation: p(3),
                turn_acceleration: p(5),
                turn_speed: p(6),
                turn_braking: p(7),
                gravity: p(10),
            },
            global_seconds: tick.global_delta,
            areas: tick.slow_areas,
            control_factor: tick.control_factor,
            running_factor: 1.,
            skip_heading: tick.skip_heading,
            controlled_actor: false,
        },
    )?;
    fields.angles[0] = oriented.heading;
    vimp.angular_speed = oriented.rates[3];
    vimp.previous_angular_speed = oriented.previous_rates[3];
    Ok(())
}
