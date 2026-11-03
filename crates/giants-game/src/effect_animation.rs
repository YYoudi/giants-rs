//! Declared, unblended class-loop inspection. Cue ownership and the viewer's
//! elapsed time are caller inputs; native callback order/cadence stay separate.
use crate::{actor_context::Prepared, animation_loop::Clock, arithmetic::Precision};

pub struct Playback {
    actor: Prepared,
    pub clock: Clock,
    pub duration: f32,
    pub rate: f64,
    pub updates: u64,
}
impl Playback {
    /// Share immutable class resources, allocate independent initial channels,
    /// pose buffers and clock for another effect object.
    pub fn fresh_instance(&self) -> Result<Self, String> {
        Self::new(Prepared::class_initial(self.actor.class.clone()))?
            .ok_or("fresh effect loop absent".into())
    }
    pub fn new(actor: Prepared) -> Result<Option<Self>, String> {
        let channel = actor.channels[0];
        if channel[0] == 254 {
            return Ok(None);
        }
        if actor.inline.is_some()
            || actor.director_slot.is_some()
            || channel[8] != 0x101
            || channel[1..4] != [0; 3]
            || actor.channels[1..].iter().any(|c| c[0] != 254)
        {
            return Err("effect animation outside constructed single-loop profile".into());
        }
        let reference = f32::from_bits(channel[5]);
        if reference != 0. {
            return Err("effect animation requires live model-speed inputs".into());
        }
        let (_, template) = actor
            .descriptor(u8::try_from(channel[0]).map_err(|e| e.to_string())?)
            .ok_or("effect loop descriptor absent")?;
        let duration = template.duration;
        if !duration.is_finite() || duration <= 0. {
            return Err("effect animation duration outside looping profile".into());
        }
        let rate = crate::animation_loop::playback_rate(
            reference,
            channel[4] as i32,
            [f32::NAN; 3], // Unused with zero reference speed, as in the native prefix.
            Precision::Double53,
        )
        .map_err(str::to_owned)?;
        Ok(Some(Self {
            actor,
            clock: Clock::default(),
            duration,
            rate,
            updates: 0,
        }))
    }
    pub fn reset(&mut self) {
        self.clock = Clock::default();
        self.updates = 0;
    }
    pub fn advance(&mut self, dt: f32) -> Result<bool, &'static str> {
        if !dt.is_finite() || dt < 0. {
            return Err("invalid effect animation elapsed time");
        }
        if dt == 0. {
            return Ok(false);
        }
        self.clock
            .advance_with_rate(self.duration, dt, self.rate, Precision::Double53)?;
        self.updates += 1;
        Ok(true)
    }
    pub fn reference(&self, index: usize) -> Result<crate::procedural_aim::Matrix, String> {
        let spec = self
            .actor
            .class
            .specification
            .as_ref()
            .ok_or("effect reference model absent")?;
        let name = spec
            .metadata
            .references
            .names
            .get(index)
            .and_then(Option::as_deref)
            .ok_or("effect reference slot undeclared")?;
        let slot = u8::try_from(self.actor.channels[0][0]).map_err(|e| e.to_string())?;
        let (animation, _) = self
            .actor
            .descriptor(slot)
            .ok_or("effect reference descriptor absent")?;
        if name.is_empty() || !name.is_ascii() {
            return Err("invalid ANM lookup name".into());
        }
        let tick = crate::model_pose::seconds_tick(animation.timeline, self.clock.seconds)
            .map_err(str::to_owned)?;
        spec.metadata
            .references
            .bind(animation)
            .and_then(|mut binding| binding.evaluate(index, tick))
            .and_then(crate::animation_references::EvaluatedReference::initial_object_matrix)
            .map_err(|e| e.to_string())
    }
    /// Named link handle from this playback's selected descriptor and own clock.
    /// Unlike a metadata reference slot, a missing link name remains an error.
    pub fn named_reference(&self, name: &str) -> Result<crate::procedural_aim::Matrix, String> {
        let slot = u8::try_from(self.actor.channels[0][0]).map_err(|e| e.to_string())?;
        let (animation, _) = self
            .actor
            .descriptor(slot)
            .ok_or("link descriptor absent")?;
        let node = animation
            .default_node(name)
            .map_err(|e| e.to_string())?
            .offset;
        let tick = crate::model_pose::seconds_tick(animation.timeline, self.clock.seconds)
            .map_err(str::to_owned)?;
        crate::animation_references::node_matrix(animation, node, tick).map_err(|e| e.to_string())
    }
    pub fn positions(&self) -> Result<crate::actor_context::PositionUpdate, String> {
        self.actor.position_update(0, self.clock.seconds)
    }
}
