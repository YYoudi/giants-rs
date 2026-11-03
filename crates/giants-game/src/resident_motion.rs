//! Owned ordinary motion/history. Behavior outputs remain explicit inputs;
//! this phase consumer is not a primary callback or a mission scheduler.
use crate::{actor_common_motion as motion, actor_fields::Fields, object_time::SlowArea};

#[derive(Clone, Copy, Debug)]
pub struct Commands {
    /// All eight bytes participate in the primary wrapper's idle gate.
    pub bytes: [u8; 8],
    pub context: u32,
    pub target_heading: f32,
    pub target_position: [f32; 2],
}
pub struct Time<'a> {
    pub seconds: f32,
    pub areas: &'a [SlowArea],
    pub control_factor: f32,
    pub running_factor: f32,
    pub skip_heading: bool,
    pub controlled_actor: bool,
}
#[derive(Clone, Debug)]
pub struct Owned {
    rates: [f32; 4],
    previous_rates: [f32; 4],
    look_pitch: f32,
    commands: [u8; 8],
    awaiting_motion: bool,
    surface_reset: bool,
}
impl Owned {
    /// Only the acquired fresh class73/230 constructors establish these zeros.
    pub(crate) fn fresh(header: &Fields) -> Result<Self, String> {
        if !matches!(header.class, 73 | 230) || header.configuration[20] != 23 {
            return Err("ordinary motion requires acquired fresh Ripper construction".into());
        }
        Ok(Self {
            rates: [0.; 4],
            previous_rates: [0.; 4],
            look_pitch: 0.,
            commands: [0; 8],
            awaiting_motion: false,
            surface_reset: false,
        })
    }
    pub fn rates(&self) -> [f32; 4] {
        self.rates
    }
    pub fn previous_rates(&self) -> [f32; 4] {
        self.previous_rates
    }
    pub fn look_pitch(&self) -> f32 {
        self.look_pitch
    }
    pub fn commands(&self) -> [u8; 8] {
        self.commands
    }
    pub fn awaiting_motion(&self) -> bool {
        self.awaiting_motion
    }
    pub(crate) fn publish_finalized_vertical_speed(&mut self, speed: f32) {
        self.rates[0] = speed;
    }
    /// D 49fb10 resets both rate vectors without consuming the primary phase,
    /// changing commands, or touching look pitch.
    pub(crate) fn surface_reset(&self) -> bool {
        self.surface_reset
    }
    pub(crate) fn reset_surface_rates(&mut self, state: &crate::actor_surface_reset::State) {
        self.rates = state.rates;
        self.previous_rates = state.previous_rates;
        self.surface_reset = true;
    }
    /// Prefix only, ending at the native primary dispatcher boundary. No
    /// callback is acknowledged. Do not run again before consuming the phase.
    pub(crate) fn begin(&mut self, header: &mut Fields, commands: [u8; 8]) -> Result<bool, String> {
        if self.awaiting_motion {
            return Err("ordinary primary history already consumed; pending motion phase".into());
        }
        self.commands = commands;
        let entry = crate::actor_primary_gate::enter(
            header.object_flags,
            header.configuration[14],
            commands,
            self.rates.map(f32::to_bits),
            self.previous_rates.map(f32::to_bits),
        );
        if !entry.invoke {
            return Ok(false);
        }
        header.configuration[14] = entry.binding_flags;
        self.previous_rates = entry.previous_rates.map(f32::from_bits);
        self.surface_reset = false;
        self.awaiting_motion = true;
        Ok(true)
    }
    /// Caller supplies the still-unimplemented behavior's outputs, never
    /// velocities or history. The primary body itself is not executed here.
    pub(crate) fn advance(
        &mut self,
        header: &mut Fields,
        commands: Commands,
        time: Time<'_>,
    ) -> Result<(), String> {
        if !self.awaiting_motion {
            return Err("ordinary motion requires its owned primary history phase".into());
        }
        let p = |i| f32::from_bits(header.configuration[i]);
        let state = motion::State {
            position: header.position,
            heading: header.angles[0],
            look_pitch: self.look_pitch,
            rates: self.rates,
            previous_rates: self.previous_rates,
            active: header.object_flags,
            personality: header.configuration[14],
            state: header.configuration[20],
            context: commands.context,
            parent_present: false,
            controls: std::array::from_fn(|i| commands.bytes[i] as i8),
            target_heading: commands.target_heading,
            target_position: commands.target_position,
        };
        let next = motion::step(
            state,
            &motion::Environment {
                parameters: motion::Parameters {
                    acceleration: p(0),
                    speed: p(1),
                    braking: p(4),
                    attenuation: p(3),
                    turn_acceleration: p(5),
                    turn_speed: p(6),
                    turn_braking: p(7),
                    gravity: p(10),
                },
                global_seconds: time.seconds,
                areas: time.areas,
                control_factor: time.control_factor,
                running_factor: time.running_factor,
                skip_heading: time.skip_heading,
                controlled_actor: time.controlled_actor,
            },
        )?;
        self.rates = next.rates;
        self.previous_rates = next.previous_rates;
        self.look_pitch = next.look_pitch;
        self.commands = commands.bytes;
        self.awaiting_motion = false;
        header.position = next.position;
        header.angles[0] = next.heading;
        // The D position helper is a RET. Leave flags/cache and AC/B8 alone;
        // existing consumers must detect the resulting stale pose.
        Ok(())
    }
}
