//! Owned ordinary-frame continuation. Behavior/animation and world services
//! remain explicit phases; this is not a complete native actor scheduler.
use crate::{actor_fields::Fields, actor_finalization::State};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    History,
    MotionStarted,
    MotionFinished,
    Failed,
}
#[derive(Clone, Debug)]
pub struct Owned {
    pub(crate) state: State,
    pub(crate) phase: Phase,
    pub(crate) failure: Option<String>,
}
impl Owned {
    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub(crate) fn fresh(
        header: &Fields,
        environment: &crate::resident_environment::Prepared,
        affine: crate::actor_affine::Cache,
    ) -> Result<Self, String> {
        if !matches!(header.class, 73 | 230)
            || header.configuration[20] != 23
            || header.response_flags & 0x80000000 != 0
            || header.object_flags & 0x10000 != 0
        {
            return Err("finalization requires fresh ordinary initialized D Ripper".into());
        }
        let fields = environment.initialized_fields();
        if fields.physical || fields.velocity != [0.; 3] || fields.model_vertical_speed != 0. {
            return Err("fresh finalization physical history unavailable".into());
        }
        Ok(Self {
            state: State {
                fields,
                previous_physical_velocity: [0.; 3],
                friction: f32::from_bits(header.configuration[28]),
                parent_present: false,
                footprint: None,
                affine: Some(affine),
                physical_argument_word: None,
            },
            phase: Phase::Idle,
            failure: None,
        })
    }
    pub(crate) fn begin(&mut self, header: &mut Fields) -> Result<(), String> {
        if self.phase != Phase::Idle
            || header.object_flags & 0x1c000 != 0
            || header.position.map(f32::to_bits) != self.state.fields.current.map(f32::to_bits)
            || header.current.map(f32::to_bits) != self.state.fields.previous.map(f32::to_bits)
        {
            return Err("frame history requires completed current ordinary actor".into());
        }
        header.advance_position_history();
        self.phase = Phase::History;
        Ok(())
    }
    pub(crate) fn input(&self, header: &Fields, vertical_speed: f32) -> Result<State, String> {
        if self.phase != Phase::MotionFinished {
            return Err("finalization requires completed owned motion phase".into());
        }
        let f = self.state.fields;
        let c = header.configuration;
        if header.class as u32 != f.class
            || header.scale.to_bits() != f.scale.to_bits()
            || c[20] != 23
            || c[14] & 0x81008 != f.collision_flags & 0x81008
            || header.object_flags & 0x10000 != 0
            || c[28] != self.state.friction.to_bits()
            || [c[17], c[22], c[23], c[25], c[26], c[27]]
                != [
                    f.ground_offset.to_bits(),
                    f.center_offset.to_bits(),
                    f.height.to_bits(),
                    f.packed_parameters,
                    f.radius_limit.to_bits(),
                    f.radius_override.to_bits(),
                ]
        {
            return Err("finalization dimensions/profile changed without acquired producer".into());
        }
        let mut state = self.state.clone();
        state.fields.current = header.position;
        state.fields.previous = header.current;
        state.fields.angles = header.angles;
        state.fields.object_flags = header.object_flags;
        state.fields.response_flags = header.response_flags;
        state.fields.collision_flags = c[14];
        state.fields.model_vertical_speed = vertical_speed;
        Ok(state)
    }
}
