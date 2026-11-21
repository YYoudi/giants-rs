//! PC state37 health-only motion/timer prefix. Prototype PS2 item/pickup names
//! remain unknown navigation candidates. Respawn/inventory variants are outside
//! this source birth; collection is a separately retained continuation.
use super::*;

#[derive(Clone)]
pub(crate) struct State {
    pub fields: Fields,
    pub owned: Owned,
}
pub(crate) trait Services {
    fn ground(&mut self, state: &State) -> Result<f32, String>;
    fn blocked(&mut self, state: &State, from: [f32; 3]) -> Result<bool, String>;
    fn expired(&mut self, state: &State) -> Result<(), String>;
}
fn seconds(s: &State, input: &PrimaryInput<'_>) -> f64 {
    crate::object_time::seconds(
        input.global_seconds,
        Some(crate::object_time::Sample {
            owner: s.fields.dispatch_context,
            position: s.fields.position,
        }),
        None,
        input.areas,
    )
}
pub(crate) fn advance(
    s: &mut State,
    input: &PrimaryInput<'_>,
    services: &mut impl Services,
) -> Result<(), String> {
    if input.paused {
        return Ok(());
    }
    if f64::from(s.owned.floor) + 3. < f64::from(s.fields.position[2]) || input.force_ground {
        let ground = services.ground(s)?;
        if !ground.is_finite() {
            return Err("health primary ground is nonfinite".into());
        }
        s.owned.floor = ground;
    }
    if f64::from(s.owned.floor) + 3. < f64::from(s.fields.position[2]) {
        s.fields.current = s.fields.position;
        s.fields.position[2] = (f64::from(s.fields.position[2])
            + seconds(s, input) * f64::from(s.owned.velocity[2]))
            as f32;
        s.owned.velocity[2] = (f64::from(s.owned.velocity[2]) - seconds(s, input) * 40.) as f32;
        s.fields.position[0] = (f64::from(s.fields.position[0])
            + seconds(s, input) * f64::from(s.owned.velocity[0]))
            as f32;
        s.fields.position[1] = (f64::from(s.fields.position[1])
            + seconds(s, input) * f64::from(s.owned.velocity[1]))
            as f32;
        if services.blocked(s, s.fields.current)? {
            s.fields.position = s.fields.current;
        }
    }
    let floor = (f64::from(s.owned.floor) + 3.) as f32;
    if s.fields.position[2] <= floor {
        s.fields.position[2] = floor;
        s.owned.velocity = [0.; 3];
    }
    if f32::from_bits(s.fields.configuration[17]) == 0. {
        s.fields.configuration[17] = 2f32.to_bits();
    }
    let mut heading = seconds(s, input) * 90. + f64::from(s.fields.angles[0]);
    s.fields.angles[0] = heading as f32;
    if heading > 360. {
        while heading > 360. {
            heading -= 360.;
        }
        s.fields.angles[0] = heading as f32;
    }
    if s.fields.action == 0 && s.owned.lifetime > 0. {
        let remaining = f64::from(s.owned.lifetime) - seconds(s, input);
        s.owned.lifetime = remaining as f32;
        if remaining <= 0. {
            s.fields.object_flags |= 0x8000;
            services.expired(s)?;
        }
    }
    let lock = f64::from(s.owned.owner_delay) - seconds(s, input);
    s.owned.owner_delay = if lock < 0. { 0. } else { lock as f32 };
    Ok(())
}
