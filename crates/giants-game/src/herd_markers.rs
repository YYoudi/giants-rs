//! Source-owned basic herd markers. State96 markers and their behavior remain separate.
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
#[derive(Clone, Copy, Debug)]
pub struct Marker {
    pub index: u32,
    pub values: [u32; 3],
}
pub struct Factory {
    seed: Seed,
    bindings: Vec<StateBinding>,
    flags: DirectFlags,
    spec: Arc<specification::Prepared>,
    states: Arc<state_resources::Prepared>,
    fields: Fields,
}
impl Factory {
    /// Acquired mission5/6/7 stores target the same marker header fields as
    /// world59/90/82. No state payload or locked-child construction is implied.
    pub fn place_mission(
        &self,
        session: &mut Session,
        p: &giants_formats::mission::Placement,
        selector: Option<u8>,
        index: Option<u32>,
        values: Option<[u32; 3]>,
    ) -> Result<Id, String> {
        if p.scale
            .is_some_and(|scale| scale.to_bits() != self.fields.scale.to_bits())
        {
            return Err("mission marker scale replacement requires its later cache writer".into());
        }
        self.place_profile(
            session,
            &giants_formats::world::WorldPlacement {
                source_offset: p.source_offset,
                object_type: p.object_type,
                position: p.position,
                yaw: p.yaw,
                pitch_roll: p.pitch_roll,
                scale: p.scale,
                selector,
                word_174: index,
                values_f4: values,
                state_writes: Vec::new(),
                cinematic_exclusions: Vec::new(),
            },
            true,
        )
    }
    pub fn load(resources: &SceneResources) -> Result<Self, String> {
        let p = resources
            .classes
            .models
            .first_program(679)
            .ok_or("herd marker class679 absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(p, Profile::Debug, Precision::Double53)?;
        let spec = Arc::new(specification::Prepared::load_registered(resources, 679)?);
        let states = Arc::new(state_resources::Prepared::load(
            resources,
            679,
            Profile::Debug,
        )?);
        let fields = Fields::load(resources, 679, &spec)?;
        if seed.storage.state() != 0
            || seed.storage.data().is_some()
            || !seed.diagnostics.is_empty()
            || !spec.needs.is_empty()
            || !states.needs().is_empty()
        {
            return Err("herd marker resources outside acquired class679/state0 profile".into());
        }
        Ok(Self {
            seed,
            bindings: resources.class_state_bindings_for(679, Profile::Debug)?,
            flags: DirectFlags::project(679, p),
            spec,
            states,
            fields,
        })
    }
    pub fn place(
        &self,
        session: &mut Session,
        p: &giants_formats::world::WorldPlacement,
    ) -> Result<Id, String> {
        self.place_profile(session, p, false)
    }
    fn place_profile(
        &self,
        session: &mut Session,
        p: &giants_formats::world::WorldPlacement,
        mission: bool,
    ) -> Result<Id, String> {
        if p.object_type != 679 || !p.state_writes.is_empty() {
            return Err("herd marker placement outside acquired profile".into());
        }
        let values = p.values_f4.unwrap_or([0; 3]);
        if !values.map(f32::from_bits).into_iter().all(f32::is_finite) {
            return Err("herd marker values outside finite profile".into());
        }
        let mut fields = self.fields.placed(crate::collider_init::DeclaredPose {
            class: 679,
            position: p.position,
            yaw: p.yaw,
            pitch_roll: p.pitch_roll,
        })?;
        if let Some(scale) = p.scale {
            fields.set_declared_scale(scale)?;
        }
        if let Some(selector) = p.selector {
            fields.action = u32::from(selector);
        }
        let input = Construction {
            class: 679,
            seed: &self.seed,
            bindings: &self.bindings,
            flags: self.flags,
            specification: self.spec.clone(),
            state_resources: self.states.clone(),
            fields,
        };
        let marker = Marker {
            index: p.word_174.unwrap_or(0),
            values,
        };
        if mission {
            session.construct_mission_herd_marker(input, marker)
        } else {
            session.construct_herd_marker(input, marker)
        }
    }
}
