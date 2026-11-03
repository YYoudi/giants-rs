//! Source-backed herd parents, declared world initialization and offline primary services.
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
use giants_formats::world::{WorldPlacement, WorldStateWrite};
use std::sync::Arc;
pub struct Factory {
    seed: Seed,
    bindings: Vec<StateBinding>,
    flags: DirectFlags,
    spec: Arc<specification::Prepared>,
    states: Arc<state_resources::Prepared>,
    fields: Fields,
}
impl Factory {
    pub fn load(resources: &SceneResources) -> Result<Self, String> {
        let program = resources
            .classes
            .models
            .first_program(49)
            .ok_or("herd class49 absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        let spec = Arc::new(specification::Prepared::load_registered(resources, 49)?);
        let states = Arc::new(state_resources::Prepared::load(
            resources,
            49,
            Profile::Debug,
        )?);
        let fields = Fields::load(resources, 49, &spec)?;
        if seed.storage.state() != 17
            || seed.storage.data().map(<[u8]>::len) != Some(52)
            || !seed.diagnostics.is_empty()
            || !spec.needs.is_empty()
            || !states.needs().is_empty()
        {
            return Err("herd resources outside acquired state17 profile".into());
        }
        Ok(Self {
            seed,
            bindings: resources.class_state_bindings_for(49, Profile::Debug)?,
            flags: DirectFlags::project(49, program),
            spec,
            states,
            fields,
        })
    }
    pub fn place(&self, session: &mut Session, placement: &WorldPlacement) -> Result<Id, String> {
        if placement.object_type != 49
            || placement.word_174.is_some()
            || placement.values_f4.is_some()
        {
            return Err("herd placement outside acquired class/property profile".into());
        }
        let mut seed = self.seed.instantiate()?;
        let body = seed.storage.data_mut().unwrap();
        for write in &placement.state_writes {
            let words: Vec<(usize, u32)> = match *write {
                WorldStateWrite::Bytes8And28(v) => {
                    vec![(8, u32::from(v[0])), (0x28, u32::from(v[1]))]
                }
                WorldStateWrite::Word4(v) => vec![(4, v)],
                WorldStateWrite::Triple1c(v) => vec![(0x1c, v[0]), (0x20, v[1]), (0x24, v[2])],
            };
            for (at, v) in words {
                body[at..at + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        let mut fields = self.fields.placed(crate::collider_init::DeclaredPose {
            class: 49,
            position: placement.position,
            yaw: placement.yaw,
            pitch_roll: placement.pitch_roll,
        })?;
        if let Some(scale) = placement.scale {
            fields.set_declared_scale(scale)?;
        }
        if let Some(selector) = placement.selector {
            fields.action = u32::from(selector);
        }
        session.construct_herd(Construction {
            class: 49,
            seed: &seed,
            bindings: &self.bindings,
            flags: self.flags,
            specification: self.spec.clone(),
            state_resources: self.states.clone(),
            fields,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Owned {
    pub initialized: bool,
    pub markers: Option<Vec<Option<Id>>>,
    pub clock: Clock,
}

/// Owned E4/EC words and callback sequencing. No native address is retained.
#[derive(Clone, Debug, Default)]
pub struct Clock {
    pub mode: u32,
    pub delay: f32,
    pub steps: u64,
    pub failure: Option<String>,
}

pub struct Tick<'a> {
    pub step: u64,
    pub global_delta: f32,
    pub slow_areas: &'a [crate::object_time::SlowArea],
}

/// Native 4141b0 queries the parent's position, before the spatial offset is
/// applied. A provider must own the terrain service; no default floor exists.
pub trait Ground {
    fn height(&mut self, position: [f32; 3]) -> Result<f32, String>;
}
