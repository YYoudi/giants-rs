//! Acquired cold Mecc special children/equipment. This is not the complete
//! player type-change, live child callbacks, renderer or mission readiness.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    model_registry::StateBinding,
    object_lists::DirectFlags,
    resident_states::{Construction, Id, Session},
    scene_resources::SceneResources,
    specification, state_resources,
    state_storage::{Profile, Seed},
};
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Child {
    pub parent: Id,
    pub reference_word: i32,
    pub binding_word: u32,
    pub context: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Initialized {
    pub children: [Id; 3],
    pub controlled: bool,
    /// CPU fields of the cold class0 -> Mecc path, not graphics completion.
    pub player_fields: bool,
}

/// Read-only attachment projection from current owned parent/child references.
/// It does not advance the child's ANM, callbacks, history or world phases.
pub struct ChildPose {
    pub id: Id,
    pub class: i32,
    pub hidden: bool,
    pub world: crate::procedural_aim::Matrix,
}

pub fn child_pose(session: &Session, id: Id) -> Result<ChildPose, String> {
    use crate::procedural_aim::Matrix;
    let actor = session.get(id).ok_or("jet child absent or foreign")?;
    let binding = session
        .mecc_special_child(id)
        .ok_or("jet child binding absent")?;
    let fields = actor
        .construction_fields
        .as_ref()
        .ok_or("jet child fields absent")?;
    if !matches!(actor.class, 84 | 85 | 781)
        || actor.seed.storage.state() != 0
        || binding.binding_word != 2
        || binding.context != 0
        || fields.object_flags & !(0x40000000 | 0x20 | 0x40 | 0x800) != 0x00400010
        || fields.response_flags & (0x02000000 | 0x9000) != 0
    {
        return Err("jet child outside acquired ordinary attachment profile".into());
    }
    let parent = session
        .get(binding.parent)
        .ok_or("jet parent absent or foreign")?;
    let parent_fields = parent
        .construction_fields
        .as_ref()
        .ok_or("jet parent fields absent")?;
    if parent.class != 2 {
        return Err("jet child belongs to another parent class".into());
    }
    let reference = |owner, index: i32, parent| -> Result<Matrix, String> {
        if (index as u8 as i8) < 0 {
            return Ok(Matrix::identity());
        }
        let matrices = session.current_reference_matrices(owner)?;
        if parent && matrices.is_empty() {
            return Ok(Matrix::identity());
        }
        matrices
            .get(index as usize)
            .copied()
            .ok_or("jet reference outside acquired array".into())
    };
    let cached = session.actor_current_affine(binding.parent)?;
    let anchor = Matrix {
        flags: cached.flags,
        values: cached.matrix,
    }
    .mul(reference(
        binding.parent,
        binding.binding_word as i32,
        true,
    )?)
    .map_err(str::to_owned)?;
    let mut world = crate::effect_attachment::normal_matrix(
        anchor,
        fields.scale,
        parent_fields.scale,
        reference(id, binding.reference_word, false)?,
    )
    .map_err(str::to_owned)?;
    world.values[11] =
        (f64::from(world.values[11]) + f64::from(f32::from_bits(fields.configuration[17]))) as f32;
    if !world.values.iter().all(|v| v.is_finite()) {
        return Err("jet attachment matrix nonfinite".into());
    }
    Ok(ChildPose {
        id,
        class: actor.class,
        hidden: fields.object_flags & 0x40000000 != 0,
        world,
    })
}
struct Template {
    class: i32,
    seed: Seed,
    bindings: Vec<StateBinding>,
    flags: DirectFlags,
    specification: Arc<specification::Prepared>,
    states: Arc<state_resources::Prepared>,
    fields: Fields,
}
impl Template {
    fn load(resources: &SceneResources, class: i32) -> Result<Self, String> {
        let program = resources
            .classes
            .models
            .first_program(class)
            .ok_or("Mecc child class absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        let specification = Arc::new(specification::Prepared::load_registered(resources, class)?);
        let states = Arc::new(state_resources::Prepared::load(
            resources,
            class,
            Profile::Debug,
        )?);
        let fields = Fields::load(resources, class, &specification)?;
        if seed.storage.state() != 0
            || seed.storage.data().is_some()
            || !seed.diagnostics.is_empty()
            || !specification.needs.is_empty()
            || !states.needs().is_empty()
        {
            return Err("Mecc special child outside acquired state0 resources".into());
        }
        Ok(Self {
            class,
            seed,
            bindings: resources.class_state_bindings_for(class, Profile::Debug)?,
            flags: DirectFlags::project(class, program),
            specification,
            states,
            fields,
        })
    }
    fn input(&self) -> Construction<'_> {
        Construction {
            class: self.class,
            seed: &self.seed,
            bindings: &self.bindings,
            flags: self.flags,
            specification: self.specification.clone(),
            state_resources: self.states.clone(),
            fields: self.fields.instantiate(),
        }
    }
}
struct Prepared {
    owner: Seed,
    owner_scale: f32,
    children: [Template; 3],
}
impl Prepared {
    fn load(resources: &SceneResources) -> Result<Self, String> {
        let program = resources
            .classes
            .models
            .first_program(2)
            .ok_or("Mecc owner class absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        Ok(Self {
            owner: Seed::from_program_for(program, Profile::Debug, Precision::Double53)?,
            owner_scale: Fields::load(
                resources,
                2,
                &specification::Prepared::load_registered(resources, 2)?,
            )?
            .scale,
            children: [
                Template::load(resources, 84)?,
                Template::load(resources, 85)?,
                Template::load(resources, 781)?,
            ],
        })
    }
}
pub struct Factory<'a> {
    resources: &'a SceneResources,
    prepared: Option<Result<Prepared, String>>,
}
impl<'a> Factory<'a> {
    pub fn new(resources: &'a SceneResources) -> Self {
        Self {
            resources,
            prepared: None,
        }
    }
    pub fn initialize(
        &mut self,
        session: &mut Session,
        owner: Id,
        local_player: Option<Id>,
    ) -> Result<Initialized, String> {
        let prepared = self
            .prepared
            .get_or_insert_with(|| Prepared::load(self.resources))
            .as_ref()
            .map_err(Clone::clone)?;
        session.initialize_mecc_special(
            owner,
            &prepared.owner,
            prepared.children.each_ref().map(Template::input),
            local_player,
        )
    }
    /// Collapse the acquired fresh generic-to-Mecc path into the owned Mecc
    /// constructor. Camera projection reset remains an explicit pending request;
    /// this does not support changing a live actor's type or complete readiness.
    pub fn initialize_player_fields(
        &mut self,
        session: &mut Session,
        owner: Id,
        blending: crate::animation_channels::Blending,
    ) -> Result<Initialized, String> {
        let prepared = self
            .prepared
            .get_or_insert_with(|| Prepared::load(self.resources))
            .as_ref()
            .map_err(Clone::clone)?;
        session.initialize_cold_mecc_player_fields(
            owner,
            &prepared.owner,
            prepared.children.each_ref().map(Template::input),
            prepared.owner_scale,
            blending,
        )
    }
}
