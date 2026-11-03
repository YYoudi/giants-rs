//! Source pistol geometry and explicit animated-hand attachment. Procedural
//! Mecc arm aiming, first-person weapon overlays and live equipment are separate.
use crate::{
    animation::Evaluator,
    scene_resources::{ModelAsset, SceneResources},
};
use giants_formats::{anm::Animation, weapons::Catalog};

pub struct Weapon {
    pub class: i32,
    pub model: ModelAsset,
    pub animation: Animation,
    pub scale: f32,
    pub parent_node: String,
    pub handle: String,
    pub references: Vec<[f32; 16]>,
}

/// Native icon table words2/1 for left/right. Last ordered write wins.
pub fn wield_class(catalog: &Catalog, icon: i32, right: bool) -> Result<i32, &'static str> {
    let icon = catalog.icons.get(&icon).ok_or("weapon icon absent")?;
    let mut class = 0;
    for &offset in &icon.records {
        let record = catalog
            .records
            .iter()
            .find(|r| r.offset == offset)
            .ok_or("weapon record offset outside table")?;
        if record.opcode == if right { 0x27 } else { 0x28 } {
            class = *record.args.first().ok_or("weapon class argument absent")? as i32;
        }
    }
    Ok(class)
}

/// Ordinary lock branch: world * current parent reference * scale ratio *
/// inverse current child reference. No lock flags or location callback inferred.
pub fn lock_matrix(
    world: [f32; 16],
    parent: [f32; 16],
    ratio: f32,
    child: [f32; 16],
    child_flags: u32,
) -> Result<[f32; 16], giants_formats::Error> {
    if !world
        .into_iter()
        .chain(parent)
        .chain(child)
        .chain([ratio])
        .all(f32::is_finite)
        || ratio <= 0.
    {
        return Err(giants_formats::Error(
            "held weapon transform outside finite positive profile",
        ));
    }
    let scale = [
        ratio, 0., 0., 0., 0., ratio, 0., 0., 0., 0., ratio, 0., 0., 0., 0., 1.,
    ];
    let multiply = crate::attachments::multiply;
    Ok(multiply(
        multiply(multiply(world, parent), scale),
        crate::attachments::inverse(child, child_flags)?,
    ))
}

impl Weapon {
    pub fn load(
        resources: &SceneResources,
        catalog: &Catalog,
        icon: i32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let class = wield_class(catalog, icon, true)?;
        if class <= 0 {
            return Err("right-hand weapon absent".into());
        }
        let selected = resources
            .classes
            .models
            .first_selection(class)
            .ok_or("weapon selection absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        let reference_names = |class| -> Result<Vec<Option<String>>, Box<dyn std::error::Error>> {
            let selected = resources
                .classes
                .models
                .first_selection(class)
                .ok_or("reference class absent")?
                .as_ref()
                .map_err(Clone::clone)?;
            let spec = resources
                .classes
                .models
                .specs
                .get(&selected.spec_id)
                .and_then(|s| s.first())
                .ok_or("reference spec absent")?
                .program
                .as_ref()
                .map_err(Clone::clone)?;
            // The development body already samples an uncorrected ANM pose.
            // Its spec also declares corrections: do not silently certify its
            // hand as the live reference array. Only the child's references are
            // required to be correction-free in this explicit pose profile.
            if class != 2 && spec.records.iter().any(|r| r.opcode == 0x54) {
                return Err("held weapon reference corrections not implemented".into());
            }
            Ok(spec.reference_names()?)
        };
        let parent_node = reference_names(2)?
            .get(1)
            .and_then(Clone::clone)
            .ok_or("right hand absent")?;
        let names = reference_names(class)?;
        let handle = names
            .first()
            .and_then(Clone::clone)
            .ok_or("weapon handle absent")?;
        let selected_animation = selected
            .selection
            .animations
            .iter()
            .filter(|a| a.slot == 1)
            .collect::<Vec<_>>();
        if selected_animation.len() != 1 {
            return Err("weapon animation slot one ambiguous".into());
        }
        let animation = resources.animation(&selected_animation[0].name)?;
        let evaluator = Evaluator {
            animation: &animation,
        };
        let references = names
            .iter()
            .map(|name| {
                let node =
                    animation.default_node(name.as_deref().ok_or("sparse weapon reference")?)?;
                Ok(evaluator.matrix(node.offset, 0)?)
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
        if references.len() < 2 {
            return Err("weapon muzzle reference absent".into());
        }
        let scale = resources
            .classes
            .models
            .first_program(class)
            .ok_or("weapon class program absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?
            .declared_scale()?;
        let model = resources.model_rgb565(class, 0)?;
        // Share the previously checked ordinary lit pistol pass; reject new surfaces.
        if model
            .mesh
            .subobjects
            .iter()
            .any(|p| p.material.flags != 0xc0000010)
        {
            return Err("held weapon surface outside checked pistol profile".into());
        }
        Ok(Self {
            class,
            model,
            animation,
            scale,
            parent_node,
            handle,
            references,
        })
    }
    pub fn matrix(
        &self,
        parent: &Animation,
        tree: &str,
        tick: i32,
        world: [f32; 16],
        parent_scale: f32,
    ) -> Result<[f32; 16], Box<dyn std::error::Error>> {
        let tree = parent
            .trees
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(tree))
            .ok_or("parent tree absent")?;
        let node = tree
            .entries
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(&self.parent_node))
            .ok_or("animated hand absent")?
            .1;
        self.matrix_from_reference(
            Evaluator { animation: parent }.matrix(node, tick)?,
            world,
            parent_scale,
        )
    }
    /// Supplied current parent reference, including its specification corrections.
    /// Used when the native subjective parent replaces the world-body parent.
    pub fn matrix_from_reference(
        &self,
        parent: [f32; 16],
        world: [f32; 16],
        parent_scale: f32,
    ) -> Result<[f32; 16], Box<dyn std::error::Error>> {
        let child_node = self.animation.default_node(&self.handle)?.offset;
        let evaluator = Evaluator {
            animation: &self.animation,
        };
        Ok(lock_matrix(
            world,
            parent,
            self.scale / parent_scale,
            evaluator.matrix(child_node, 0)?,
            evaluator.matrix_flags(child_node, 0)?,
        )?)
    }
    pub fn muzzle(&self, matrix: [f32; 16]) -> Result<[f32; 3], &'static str> {
        crate::object_reference::point(
            &crate::object_reference::Object {
                position: [matrix[3], matrix[7], matrix[11]],
                angles: [0.; 3],
                scale: self.scale,
                flags: 0x20,
                cached: matrix,
                references: &self.references,
            },
            1,
        )
    }
}
