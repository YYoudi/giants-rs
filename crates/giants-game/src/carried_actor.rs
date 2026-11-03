//! Resource preparation for the measured state18 / Mecc pickup profile.
//! Source pose inspection remains distinct from actor construction, animation
//! blending/events, state callbacks and mission admission.
use crate::{animation::Evaluator, scene_resources::SceneResources};
use giants_formats::anm::Animation;

pub struct Prepared {
    pub class: i32,
    pub body: crate::cast::Body,
    pub animation: Animation,
    pub animation_name: String,
    pub tree: String,
    pub template: crate::animation_template::Template,
    pub seed: crate::state_storage::Seed,
    pub state_bindings: Vec<crate::model_registry::StateBinding>,
    /// Measured wrapper request. The preview remains unblended; the transition
    /// duration is not a multiplier for its playback rate.
    pub pickup_animation: crate::smartie_carry::AnimationRequest,
    pub scale: f32,
    pub parent_reference: String,
    pub child_reference: String,
    /// Declared correction counts. Only the requested current parent reference
    /// consumes its prepared link; the parent body/procedural pose stays separate.
    pub parent_corrections: usize,
    pub child_corrections: usize,
    pub parent_reference_links: Vec<crate::animation_links::Link>,
}

fn spec(
    resources: &SceneResources,
    class: i32,
) -> Result<&giants_formats::object_spec::Program, Box<dyn std::error::Error>> {
    let selected = resources
        .classes
        .models
        .first_selection(class)
        .ok_or("carried actor selection absent")?
        .as_ref()
        .map_err(Clone::clone)?;
    let definition = resources
        .classes
        .models
        .specs
        .get(&selected.spec_id)
        .and_then(|s| s.first())
        .ok_or("carried actor specification absent")?;
    if definition.catalog != selected.spec_catalog {
        return Err("carried actor specification context changed".into());
    }
    Ok(definition.program.as_ref().map_err(Clone::clone)?)
}

impl Prepared {
    pub fn load(
        resources: &SceneResources,
        class: i32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let program = resources
            .classes
            .models
            .first_program(class)
            .ok_or("carried actor class absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed = crate::state_storage::Seed::from_program_for(
            program,
            crate::state_storage::Profile::Debug,
            crate::arithmetic::Precision::Double53,
        )?;
        if seed.storage.state() != 18 {
            return Err("carried actor outside measured state18 pickup profile".into());
        }
        let state_bindings =
            resources.class_state_bindings_for(class, crate::state_storage::Profile::Debug)?;
        let scale = program.declared_scale()?;
        if !scale.is_finite() || scale <= 0. {
            return Err("carried actor scale outside finite positive profile".into());
        }
        let selected = resources
            .classes
            .models
            .first_selection(class)
            .ok_or("carried actor selection absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        let pickup_animation = crate::smartie_carry::MECC_PICKUP_ANIMATION;
        // Selection has source-ordered writes: use the first requested binding,
        // as the common specification descriptor loader does.
        let declared = selected
            .selection
            .animations
            .iter()
            .find(|a| a.slot == pickup_animation.slot)
            .ok_or("carried actor pickup animation41 absent")?;
        let animation_name = declared.name.clone();
        let animation = resources.animation(&animation_name)?;
        let tree = animation
            .trees
            .iter()
            .find(|t| t.offset == animation.default_tree)
            .ok_or("carried actor default tree absent")?
            .name
            .clone();
        let body = crate::cast::load_registered_body(resources, class, 0)?;
        let template = crate::animation_template::Template::prepare(
            &animation,
            &[crate::animation_template::Model::Skinned(&body.skin)],
            crate::arithmetic::Precision::Double53,
        )?;
        if !template.needs.is_empty() {
            return Err("carried actor animation has unresolved skin bindings".into());
        }
        if !template.duration.is_finite() || template.duration <= 0. {
            return Err("carried actor preview requires a positive animation duration".into());
        }
        let parent = spec(resources, 2)?;
        let parent_reference = parent
            .reference_names()?
            .get(crate::smartie_carry::MECC_PARENT_REFERENCE as usize)
            .and_then(Clone::clone)
            .ok_or("carried actor parent reference10 undeclared")?;
        let count_corrections = |p: &giants_formats::object_spec::Program| {
            p.records
                .iter()
                .filter(|r| r.opcode == 0x54 && r.bytes.get(1) != Some(&0))
                .count()
        };
        let parent_corrections = count_corrections(parent);
        let parent_reference_links = crate::animation_links::prepare_registered(resources, 2)?;
        let child_spec = spec(resources, class)?;
        let child_corrections = count_corrections(child_spec);
        let child_reference = child_spec
            .reference_names()?
            .get(crate::smartie_carry::MECC_CHILD_REFERENCE as usize)
            .and_then(Clone::clone)
            .ok_or("carried actor child reference1 undeclared")?;
        let personality = crate::object_lists::DirectFlags::project(class, program).personality;
        if child_corrections != 0 || personality & (0x2000000 | 0x8000 | 0x1000) != 0 {
            return Err(
                "carried lock requires ordinary relative-scale/current-child-reference profile"
                    .into(),
            );
        }
        Ok(Self {
            class,
            body,
            animation,
            animation_name,
            tree,
            template,
            seed,
            state_bindings,
            pickup_animation,
            scale,
            parent_reference,
            child_reference,
            parent_corrections,
            child_corrections,
            parent_reference_links,
        })
    }

    /// Unblended source clip inspection at a supplied clock. No callbacks,
    /// pickup transition blending or state19 update are inferred from the pose.
    pub fn positions(&self, seconds: f32) -> Result<Vec<[f32; 3]>, Box<dyn std::error::Error>> {
        if !seconds.is_finite() || seconds < 0. || seconds > self.template.duration {
            return Err("carried actor pose clock outside supplied clip".into());
        }
        let tick = crate::model_pose::seconds_tick(self.animation.timeline, seconds)?;
        let binding = self
            .template
            .bindings
            .first()
            .and_then(Option::as_ref)
            .ok_or("carried actor skin binding absent")?;
        Ok(crate::skinning::positions_bound(
            &self.body.skin,
            &self.animation,
            binding,
            tick,
        )?)
    }

    /// Initial pickup location only. The render lock below also needs the
    /// current child reference inverse; this point alone is not its world origin.
    pub fn parent_point(
        &self,
        animation: &Animation,
        tree: &str,
        tick: i32,
        world: [f32; 16],
    ) -> Result<[f32; 3], Box<dyn std::error::Error>> {
        let tree = animation
            .trees
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(tree))
            .ok_or("carried actor parent tree absent")?;
        let node = tree
            .entries
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(&self.parent_reference))
            .ok_or("carried actor parent node absent")?
            .1;
        let reference = crate::animation_links::apply(
            crate::smartie_carry::MECC_PARENT_REFERENCE as u8,
            crate::animation_references::node_matrix(animation, node, tick)?,
            &self.parent_reference_links,
        )?
        .values;
        // This slice contains the already selected reference10, not a fabricated
        // full reference array. Index0 here addresses that supplied one entry.
        Ok(crate::object_reference::point(
            &crate::object_reference::Object {
                position: [world[3], world[7], world[11]],
                angles: [0.; 3],
                scale: 1.,
                flags: 0x20,
                cached: world,
                references: &[reference],
            },
            0,
        )?)
    }
    /// Ordinary object lock, sharing the checked weapon attachment kernel.
    /// Parent and child references use their respective current clip clocks.
    /// Declared reference corrections are applied; procedural Mecc posing and
    /// channel pose blending remain separate consumers.
    pub fn world_matrix(
        &self,
        parent_animation: &Animation,
        parent_tree: &str,
        parent_tick: i32,
        parent_world: [f32; 16],
        parent_scale: f32,
        child_seconds: f32,
    ) -> Result<[f32; 16], Box<dyn std::error::Error>> {
        let child_node = self.animation.default_node(&self.child_reference)?.offset;
        let child_tick = crate::model_pose::seconds_tick(self.animation.timeline, child_seconds)?;
        let child_eval = Evaluator {
            animation: &self.animation,
        };
        self.world_matrix_with_child_reference(
            parent_animation,
            parent_tree,
            parent_tick,
            parent_world,
            parent_scale,
            crate::procedural_aim::Matrix {
                values: child_eval.matrix(child_node, child_tick)?,
                flags: child_eval.matrix_flags(child_node, child_tick)?,
            },
        )
    }
    /// Consume the current child reference owned by its actor session. Parent
    /// procedural posing and complete actor construction remain separate.
    pub fn world_matrix_with_child_reference(
        &self,
        parent_animation: &Animation,
        parent_tree: &str,
        parent_tick: i32,
        parent_world: [f32; 16],
        parent_scale: f32,
        child: crate::procedural_aim::Matrix,
    ) -> Result<[f32; 16], Box<dyn std::error::Error>> {
        if !parent_scale.is_finite() || parent_scale <= 0. {
            return Err("carried lock parent scale outside positive finite profile".into());
        }
        let node = parent_animation
            .named_node(Some(parent_tree), &self.parent_reference)?
            .offset;
        let parent = crate::animation_links::apply(
            crate::smartie_carry::MECC_PARENT_REFERENCE as u8,
            crate::animation_references::node_matrix(parent_animation, node, parent_tick)?,
            &self.parent_reference_links,
        )?
        .values;
        self.world_matrix_from_parent_reference(parent, parent_world, parent_scale, child)
    }
    /// Consume the already corrected current parent reference without sampling
    /// the parent's old development animation again.
    pub fn world_matrix_from_parent_reference(
        &self,
        parent: [f32; 16],
        parent_world: [f32; 16],
        parent_scale: f32,
        child: crate::procedural_aim::Matrix,
    ) -> Result<[f32; 16], Box<dyn std::error::Error>> {
        if !parent_scale.is_finite() || parent_scale <= 0. {
            return Err("carried lock parent scale outside positive finite profile".into());
        }
        let carried_scale = (f64::from(self.scale) * f64::from(0.8_f32)) as f32;
        Ok(crate::held_weapon::lock_matrix(
            parent_world,
            parent,
            carried_scale / parent_scale,
            child.values,
            child.flags,
        )?)
    }
}
