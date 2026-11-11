//! Source-owned class818 death auxiliary, including its cold78 -> state84 switch.
//! The state84 update, animation, rendering and damage caller remain separate.
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
pub struct Proxy {
    pub parameters: [u32; 3],
    pub ghost: Option<Id>,
    pub context: Option<crate::attached_effect_context::Id>,
    pub number: u64,
    pub failed: bool,
}
#[derive(Clone, Copy)]
pub struct Meat {
    pub class: i32,
    pub transparency: f32,
    pub control: u32,
    pub position: [f32; 3],
}
/// Supplied source-ordered common-list fields. This does not invent a meat
/// population or acknowledge any mana-eligible player branch.
pub fn meat_present(position: [f32; 3], radius: f32, meat: &[Meat]) -> Result<bool, String> {
    if !radius.is_finite() || radius < 0. || !position.into_iter().all(f32::is_finite) {
        return Err("soul source radius/position outside finite profile".into());
    }
    let mut present = false;
    for item in meat {
        if !(1117..=1119).contains(&item.class) || item.transparency > 0. || item.control & 2 != 0 {
            continue;
        }
        if !item.transparency.is_finite() || !item.position.into_iter().all(f32::is_finite) {
            return Err("soul meat fields nonfinite".into());
        }
        let d = std::array::from_fn::<_, 3, _>(|i| {
            f64::from(item.position[i]) - f64::from(position[i])
        });
        present |=
            (d[0] * d[0] + d[1] * d[1]) + d[2] * d[2] < f64::from(radius) * f64::from(radius);
    }
    Ok(present)
}
pub struct Factory {
    seed: Seed,
    bindings: Vec<StateBinding>,
    fields: Fields,
    flags: DirectFlags,
    specification: Arc<specification::Prepared>,
    states: Arc<state_resources::Prepared>,
}
impl Factory {
    pub fn load(resources: &SceneResources) -> Result<Self, String> {
        let program = resources
            .classes
            .models
            .first_program(818)
            .ok_or("death auxiliary class818 absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = Seed::from_program_for(program, Profile::Debug, Precision::Double53)?;
        let bindings = resources.class_state_bindings_for(818, Profile::Debug)?;
        if seed.storage.state() != 78
            || !seed.diagnostics.is_empty()
            || !seed.unresolved.is_empty()
            || !bindings.is_empty()
            || seed
                .storage
                .data()
                .is_none_or(|d| d.len() != 268 || d.iter().any(|&b| b != 0))
        {
            return Err("death auxiliary requires the acquired empty cold78 source profile".into());
        }
        let specification = Arc::new(specification::Prepared::load_registered(resources, 818)?);
        let states = Arc::new(state_resources::Prepared::load(
            resources,
            818,
            Profile::Debug,
        )?);
        if !specification.needs.is_empty() || !states.needs().is_empty() {
            return Err("death auxiliary source resources incomplete".into());
        }
        let fields = Fields::load(resources, 818, &specification)?;
        Ok(Self {
            seed,
            bindings,
            fields,
            flags: DirectFlags::project(818, program),
            specification,
            states,
        })
    }
    /// The damage caller supplies its source position and acquired size tier.
    /// Only A0 is written: AC/B8 and the source geometric scale stay unchanged.
    pub fn spawn(
        &self,
        session: &mut Session,
        position: [f32; 3],
        tier: u32,
    ) -> Result<Id, String> {
        if !position.into_iter().all(f32::is_finite) || tier > 2 {
            return Err("death auxiliary pose/tier outside acquired profile".into());
        }
        let mut fields = self.fields.instantiate();
        fields.position = position;
        fields.control_flags = tier;
        session.construct_vimp_death_proxy(
            Construction {
                class: 818,
                seed: &self.seed,
                bindings: &self.bindings,
                flags: self.flags,
                specification: self.specification.clone(),
                state_resources: self.states.clone(),
                fields,
            },
            Proxy {
                parameters: [1, 32, 1_f32.to_bits()],
                ghost: None,
                context: None,
                number: 0,
                failed: false,
            },
        )
    }
}
