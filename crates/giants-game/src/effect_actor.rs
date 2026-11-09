//! Shared acquired state78 resource and constructor preparation.
//! This owns the inline physical storage, not an admitted world actor or tick.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    attached_effect_context::{Context, Id},
    model_registry::StateBinding,
    procedural_aim::Matrix,
    scene_resources::SceneResources,
    state_storage::{Profile, Seed},
};
use std::sync::Arc;

pub struct Template {
    pub(crate) seed: Seed,
    pub(crate) fields: Fields,
    pub(crate) bindings: Vec<StateBinding>,
    pub(crate) specification: Arc<crate::specification::Prepared>,
    pub(crate) states: Arc<crate::state_resources::Prepared>,
}
#[derive(Clone, Copy, Debug)]
pub enum Placement<K> {
    Attached {
        recipient: K,
        reference: u32,
        tag: u32,
        opaque: u32,
    },
    Position([f32; 3]),
}
pub struct Initial<K> {
    pub origin: Id,
    pub placement: Placement<K>,
    pub seed: Seed,
    pub fields: Fields,
    pub render_flags: u32,
    pub matrix: Matrix,
}
impl<K> Initial<K> {
    /// A view of the actual owned payload, never a second physical allocation.
    pub fn physical_words(&self) -> Result<[u32; 15], String> {
        let data = physical_data(&self.seed)?;
        Ok(std::array::from_fn(|i| {
            u32::from_le_bytes(data[i * 4..i * 4 + 4].try_into().unwrap())
        }))
    }
}
fn physical_data(seed: &Seed) -> Result<&[u8], String> {
    if seed.storage.profile() != Profile::Debug || seed.storage.state() != 78 {
        return Err("effect physical view requires acquired D state78".into());
    }
    let data = seed.storage.data().ok_or("effect state payload absent")?;
    if data.len() != 268 {
        return Err("effect physical view has incompatible payload size".into());
    }
    Ok(&data[0xa0..0xdc])
}
fn initialize_physical(seed: &mut Seed) -> Result<(), String> {
    let data = physical_data(seed)?;
    let mut words =
        std::array::from_fn(|i| u32::from_le_bytes(data[i * 4..i * 4 + 4].try_into().unwrap()));
    crate::physical_initial::reset(&mut words);
    let data = seed.storage.data_mut().unwrap();
    // Presence only. The actual originating cursor identity lives in Initial.
    data[..4].copy_from_slice(&1u32.to_le_bytes());
    for (chunk, word) in data[0xa0..0xdc].as_chunks_mut::<4>().0.iter_mut().zip(words) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    Ok(())
}
impl Template {
    pub fn load(resources: &SceneResources, class: i32) -> Result<Self, String> {
        if class < 0 {
            return Err("effect constructor class outside acquired nonnegative profile".into());
        }
        let program = resources
            .classes
            .models
            .first_program(class)
            .ok_or("effect constructor class absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        if seed.storage.state() != 78
            || !seed.diagnostics.is_empty()
            || !seed.unresolved.is_empty()
            || seed
                .storage
                .data()
                .is_none_or(|d| d.len() != 268 || d.iter().any(|b| *b != 0))
        {
            return Err("effect constructor requires acquired zero D state78 template".into());
        }
        let specification = Arc::new(crate::specification::Prepared::load_registered(
            resources, class,
        )?);
        let states = Arc::new(crate::state_resources::Prepared::load(
            resources,
            class,
            Profile::Debug,
        )?);
        if !specification.needs.is_empty() || !states.needs().is_empty() {
            return Err("effect constructor resources incomplete".into());
        }
        Ok(Self {
            seed,
            fields: Fields::load(resources, class, &specification)?,
            bindings: resources.class_state_bindings_for(class, Profile::Debug)?,
            specification,
            states,
        })
    }
    fn validate<K>(&self, context: &Context<K>, opcode: u8, at: usize) -> Result<(), String> {
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let first = program
            .records
            .first()
            .ok_or("effect constructor program empty")?;
        if first.opcode != opcode
            || first.bytes.first() != Some(&opcode)
            || first
                .bytes
                .get(at..at + 4)
                .map(|b| i32::from_le_bytes(b.try_into().unwrap()))
                != Some(self.fields.class)
        {
            return Err("effect constructor class or origin differs from source program".into());
        }
        Ok(())
    }
    fn initial<K>(
        &self,
        origin: Id,
        placement: Placement<K>,
        flags: u32,
        attached: bool,
    ) -> Result<Initial<K>, String> {
        let mut seed = self.seed.instantiate()?;
        initialize_physical(&mut seed)?;
        let mut fields = self.fields.instantiate();
        fields.configuration[14] = (fields.configuration[14] & !0x20000) | 0x800;
        fields.configuration[22] = 50f32.to_bits();
        fields.object_flags |= if attached { 0x40410030 } else { 0x40010030 };
        fields.control_flags |= 0x20;
        let mut matrix = Matrix {
            flags: 0x10,
            values: [0.; 16],
        };
        if let Placement::Position(position) = &placement {
            fields.position = *position;
            fields.current = *position;
            matrix = Matrix::identity();
            matrix.flags = 0x10;
            matrix.values[3] = position[0];
            matrix.values[7] = position[1];
            matrix.values[11] = position[2];
        }
        Ok(Initial {
            origin,
            placement,
            seed,
            fields,
            matrix,
            render_flags: if flags & 0x8000 != 0 { 0x1000 } else { 0 },
        })
    }
    /// Before the subsequent size/lifetime/emitter VM commands. Resource-image
    /// scope, context consumption and ordinary-slot ownership belong to callers.
    pub fn attached<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
    ) -> Result<Initial<K>, String> {
        if !matches!(context.flags(), 4 | 0x8004) {
            return Err("attached constructor context inheritance outside acquired profile".into());
        }
        self.validate(context, 0x24, 5)?;
        let p = context.program.as_ref().map_err(Clone::clone)?;
        let first = &p.records[0].bytes;
        let word = |at| {
            first
                .get(at..at + 4)
                .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                .ok_or("attached constructor operand absent")
        };
        let placement = Placement::Attached {
            recipient: context.recipient().ok_or("effect parent absent")?,
            reference: word(9)?,
            opaque: word(1)?,
            tag: context.tag().ok_or("effect tag absent")?,
        };
        self.initial(origin, placement, context.flags(), true)
    }
    /// Acquired unparented positional profile, before subsequent VM commands.
    pub fn positioned<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
    ) -> Result<Initial<K>, String> {
        if !matches!(context.flags(), 0 | 0x8000) {
            return Err(
                "positioned constructor context inheritance outside acquired profile".into(),
            );
        }
        self.validate(context, 3, 1)?;
        let point = context
            .position()
            .ok_or("positioned effect binding absent")?;
        if !point.into_iter().all(f32::is_finite) {
            return Err("positioned effect point nonfinite".into());
        }
        self.initial(origin, Placement::Position(point), context.flags(), false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inline_physics_reset_preserves_opaque_storage_and_other_payload_fields() {
        let p = giants_formats::class_program::parse(&[0x1e, 78, 0xff]).unwrap();
        let mut seed = Seed::from_program_for(&p, Profile::Debug, Precision::Double53).unwrap();
        seed.storage.data_mut().unwrap().fill(0xa5);
        let before = seed.storage.data().unwrap().to_vec();
        initialize_physical(&mut seed).unwrap();
        let after = seed.storage.data().unwrap();
        assert_eq!(&after[4..0xa0], &before[4..0xa0]);
        assert_eq!(&after[0xdc..], &before[0xdc..]);
        assert_eq!(&after[0xb4..0xb8], &before[0xb4..0xb8]);
        let words = physical_data(&seed)
            .unwrap()
            .as_chunks::<4>().0.iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect::<Vec<_>>();
        assert_eq!(words[1], 1f32.to_bits());
        assert_eq!(words[2], 1f32.to_bits());
        assert_eq!(words[4], 1f32.to_bits());
        assert_eq!(u32::from_le_bytes(after[..4].try_into().unwrap()), 1);
        let p = giants_formats::class_program::parse(&[0xff]).unwrap();
        let mut other = Seed::from_program_for(&p, Profile::Debug, Precision::Double53).unwrap();
        assert!(initialize_physical(&mut other).is_err());
    }
}
