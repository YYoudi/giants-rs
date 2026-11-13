//! Acquired normal relative-scale one-shot object lock for immediate particles.
//! Prototype PS2 `object_lock_logic` remains an unknown PC attribution. Current
//! canonical references are required; no fallback pose or animation clock.
use super::*;

impl Session {
    pub fn timed_effect_immediate_binding(
        &self,
        id: Id,
    ) -> Option<crate::attached_effect_context::Immediate<Id>> {
        let binding = self.get(id)?.timed_effect.as_ref()?.immediate?;
        let Parent::Actor(parent) = binding.parent else {
            return None;
        };
        Some(crate::attached_effect_context::Immediate {
            parent,
            reference: binding.reference,
            own_reference: binding.own_reference,
        })
    }
    /// Shared acquired object-lock math, after the first callback/finalization.
    /// Consume only the real pending binding; leave launch to the next callback.
    /// This does not mark the ordinary preparation or vertex renderer ready.
    pub fn bind_immediate_timed(&mut self, id: Id) -> Result<(), String> {
        let binding = self
            .get(id)
            .and_then(|a| a.timed_effect.as_ref())
            .and_then(|m| m.immediate)
            .ok_or("immediate timed binding absent, foreign or already consumed")?;
        let result = (|| -> Result<(), String> {
            let Parent::Actor(parent) = binding.parent else {
                return Err("immediate particle requires a real actor parent".into());
            };
            let actor = self.get(id).unwrap();
            let metadata = actor.timed_effect.as_ref().unwrap();
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("immediate particle fields absent")?;
            if metadata.runtime.failure.is_some()
                || !metadata.runtime.idle()
                || fields.control_flags & 0x20 != 0
                || fields.response_flags & 0x1000 == 0
                || fields.response_flags & (0x2000000 | 0x8000) != 0
                || fields.object_flags & (0x20000000 | 0x8000) != 0
                || (binding.reference as u8 as i8) < 0
                || (binding.own_reference as u8 as i8) < 0
            {
                return Err(
                    "immediate lock outside acquired normal current-reference profile".into(),
                );
            }
            let parent_actor = self
                .get(parent)
                .ok_or("immediate timed parent retired or foreign")?;
            let parent_metadata = parent_actor
                .timed_effect
                .as_ref()
                .ok_or("immediate timed parent owner absent")?;
            if parent_metadata.runtime.failure.is_some() || !parent_metadata.runtime.idle() {
                return Err("immediate timed parent callback unavailable".into());
            }
            let parent_fields = parent_actor
                .construction_fields
                .as_ref()
                .ok_or("immediate parent fields absent")?;
            let parent_refs = self
                .specifications
                .get(&parent.serial)
                .ok_or("immediate parent specification absent")?
                .0
                .current_matrices(
                    parent_actor
                        .channels
                        .as_ref()
                        .ok_or("immediate parent channels absent")?,
                )?;
            let own_refs = self
                .specifications
                .get(&id.serial)
                .ok_or("immediate child specification absent")?
                .0
                .current_matrices(
                    actor
                        .channels
                        .as_ref()
                        .ok_or("immediate child channels absent")?,
                )?;
            let parent_ref = *parent_refs
                .get(binding.reference as usize)
                .ok_or("immediate parent reference out of range")?;
            let own_ref = *own_refs
                .get(binding.own_reference as usize)
                .ok_or("immediate child reference out of range")?;
            let mut matrix = crate::effect_attachment::initial_child_matrix(
                parent_metadata.matrix,
                parent_ref,
                own_ref,
                fields.scale,
                parent_fields.scale,
            )?;
            matrix.values[11] = (f64::from(matrix.values[11])
                + f64::from(f32::from_bits(fields.configuration[17])))
                as f32;
            let position = [matrix.values[3], matrix.values[7], matrix.values[11]];
            if position.iter().any(|v| !v.is_finite()) {
                return Err("immediate lock position nonfinite".into());
            }
            let actor = self.residents.get_mut(&id.serial).unwrap();
            let fields = actor.construction_fields.as_mut().unwrap();
            let metadata = actor.timed_effect.as_mut().unwrap();
            // Native normal one-shot lock clears the parent/mark before its
            // location/history store. No physical/world service is replaced.
            metadata.immediate = None;
            fields.response_flags &= !0x1000;
            fields.previous = fields.current;
            fields.current = position;
            fields.position = position;
            metadata.matrix = matrix;
            self.refresh_timed_cache(id)?;
            Ok(())
        })();
        if let Err(error) = &result
            && let Some(metadata) = self
                .residents
                .get_mut(&id.serial)
                .and_then(|a| a.timed_effect.as_mut())
        {
            metadata.runtime.failure = Some(error.clone());
        }
        result
    }
}
