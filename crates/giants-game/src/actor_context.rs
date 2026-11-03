//! CPU actor contexts from actual class and FLK declarations. Construction
//! projections are not live registration, state callbacks, visibility or Ready.
use crate::{
    animation_template::{Model, Template},
    arithmetic::Precision,
    object_lists::DirectFlags,
    scene_resources::SceneResources,
    specification,
};
use giants_formats::{anm::Animation, effect_image::Image};
use std::sync::Arc;

pub struct Class {
    pub declaration_class: i32,
    pub specification: Option<Arc<specification::Prepared>>,
    pub flags: DirectFlags,
    pub state: u32,
    pub scale: f32,
    pub declared_loop: Option<u8>,
}
impl Class {
    pub fn load(resources: &SceneResources, class: i32, effects: &Image) -> Result<Self, String> {
        if class < -1 {
            return Err("negative actor class outside source profile".into());
        }
        let initializer = if class == -1 { 0 } else { class };
        let definition = resources
            .classes
            .models
            .first_program(initializer)
            .ok_or("class program absent")?;
        let program = definition.program.as_ref().map_err(Clone::clone)?;
        if class == -1 && program.records.iter().any(|r| r.opcode == 1) {
            return Err("negative role initializer requests a specification".into());
        }
        Ok(Self {
            declaration_class: class,
            specification: if class == -1 {
                None
            } else {
                Some(Arc::new(specification::Prepared::load(
                    resources, class, effects,
                )?))
            },
            flags: DirectFlags::project(initializer, program),
            state: u32::from(
                crate::state_storage::Seed::from_program(program)?
                    .storage
                    .state(),
            ),
            scale: if class == -1 {
                1.
            } else {
                program.declared_scale().map_err(|e| e.to_string())?
            },
            declared_loop: crate::model_pose::declared_loop(program).map_err(str::to_owned)?,
        })
    }
}
pub struct Inline {
    pub animation: Animation,
    pub template: Template,
    pub tree: Option<String>,
    /// Private initially unresolved named-reference cache. Shared descriptor
    /// events and subsequent event phases are deliberately not copied here.
    pub references: Vec<Option<usize>>,
}
pub struct Prepared {
    pub class: Arc<Class>,
    pub command_offset: usize,
    pub actor_slot: usize,
    pub director_slot: Option<usize>,
    pub label: Option<String>,
    pub inline: Option<Inline>,
    /// Four native channel projections. Opaque words remain opaque; advancing
    /// clocks and blending is outside this constructor interface.
    pub channels: [[u32; 9]; 4],
    pub needs: Vec<String>,
}
/// A retained model buffer is an input, not a newly decoded neutral pose.
pub enum PositionUpdate {
    Keep,
    Replace(Vec<[f32; 3]>),
}
impl Prepared {
    /// Initial class channels without a Flick director or label. This is a CPU
    /// construction projection, not allocation, live FX state or registration.
    pub fn class_initial(class: Arc<Class>) -> Self {
        let channels = initial_channels(class.declared_loop, false);
        let needs = class
            .specification
            .as_ref()
            .map_or_else(Vec::new, |s| s.needs.clone());
        Self {
            class,
            command_offset: 0,
            actor_slot: 0,
            director_slot: None,
            label: None,
            inline: None,
            channels,
            needs,
        }
    }
    /// Explicit model-slot/clock inspection, before native shared-buffer cache
    /// routing, object/world transforms, normals and draw submission. The caller
    /// owns the shared position buffer. No skin or no selected channel leaves
    /// that buffer unchanged; it must not silently restore a neutral pose.
    pub fn position_update(
        &self,
        model_slot: usize,
        seconds: f32,
    ) -> Result<PositionUpdate, String> {
        if !seconds.is_finite() || seconds < 0. {
            return Err("invalid actor vertex clock".into());
        }
        if !self.needs.is_empty() {
            return Err("actor vertex preparation has unresolved needs".into());
        }
        let spec = self
            .class
            .specification
            .as_ref()
            .ok_or("actor has no model table")?;
        let model = spec
            .models
            .get(model_slot)
            .and_then(Option::as_ref)
            .ok_or("actor model slot absent")?;
        let Some(skin) = &model.prepared.skin else {
            return Ok(PositionUpdate::Keep);
        };
        let slot = u8::try_from(self.channels[0][0]).map_err(|e| e.to_string())?;
        if slot == 254 {
            return Ok(PositionUpdate::Keep);
        }
        let (animation, template) = self
            .descriptor(slot)
            .ok_or("actor vertex animation descriptor absent")?;
        let binding = template
            .bindings
            .get(model_slot)
            .and_then(Option::as_ref)
            .ok_or("actor vertex binding absent")?;
        let tick =
            crate::model_pose::seconds_tick(animation.timeline, seconds).map_err(str::to_owned)?;
        crate::skinning::positions_bound(skin, animation, binding, tick)
            .map(PositionUpdate::Replace)
            .map_err(|e| e.to_string())
    }

    pub fn declaration(class: Arc<Class>, bytes: &[u8], offset: usize) -> Result<Self, String> {
        let metadata =
            giants_formats::flick::discover_dependencies(bytes).map_err(|e| e.to_string())?;
        let command = metadata
            .commands
            .iter()
            .find(|c| c.offset == offset && c.opcode == 2)
            .ok_or("actor declaration offset absent")?;
        if command.words.len() != 4 || command.words[3] as i32 != class.declaration_class {
            return Err("actor declaration class/context mismatch".into());
        }
        let slot = command.words[2] as usize;
        let actor = metadata
            .actors
            .iter()
            .find(|a| a.slot == slot)
            .ok_or("actor slot absent")?;
        let director_slot = (actor.director >= 0).then_some(actor.director as usize);
        let mut needs = class
            .specification
            .as_ref()
            .map_or_else(Vec::new, |s| s.needs.clone());
        let inline = if let Some(director) = director_slot {
            let binding = metadata
                .commands
                .iter()
                .rev()
                .find(|c| {
                    c.offset < offset
                        && c.opcode == 1
                        && c.words.first() == Some(&(director as u32))
                })
                .ok_or("actor director has no preceding binding")?;
            if binding.words.len() != 2
                || binding.words[1] as usize != metadata.animation_region_offset
            {
                return Err("actor director is outside the checked inline profile".into());
            }
            let animation = giants_formats::anm::inline_flk(bytes).map_err(|e| e.to_string())?;
            let models = class
                .specification
                .as_ref()
                .map_or(&[][..], |s| s.models.as_slice())
                .iter()
                .map(|m| match m {
                    None => Model::Empty,
                    Some(m) => match &m.prepared.skin {
                        Some(s) => Model::Skinned(s),
                        None => Model::Unskinned,
                    },
                })
                .collect::<Vec<_>>();
            let template = Template::prepare_inline(
                &animation,
                &models,
                actor.name.as_deref(),
                Precision::Double53,
            )
            .map_err(|e| e.to_string())?;
            needs.extend(
                template
                    .needs
                    .iter()
                    .map(|n| format!("inline actor binding: {n:?}")),
            );
            Some(Inline {
                animation,
                template,
                tree: actor.name.clone(),
                references: vec![
                    None;
                    class.specification.as_ref().map_or(0, |s| s
                        .metadata
                        .references
                        .names
                        .len())
                ],
            })
        } else {
            None
        };
        let channels = initial_channels(class.declared_loop, inline.is_some());
        Ok(Self {
            class,
            command_offset: offset,
            actor_slot: slot,
            director_slot,
            label: actor.name.clone(),
            inline,
            channels,
            needs,
        })
    }
    /// Only descriptor zero is replaced. The remaining descriptors are shared
    /// with the immutable class specification at the checked boundary.
    pub fn descriptor(&self, slot: u8) -> Option<(&Animation, &Template)> {
        if slot == 0
            && let Some(inline) = &self.inline
        {
            return Some((&inline.animation, &inline.template));
        }
        self.class
            .specification
            .as_ref()?
            .animations
            .get(&slot)
            .map(|d| (&d.animation, &d.template))
    }
    /// Selected scalar words for independent native comparison, including the
    /// constructor's local status bits. This does not emit runtime messages.
    pub fn constructor_words(&self) -> Vec<u32> {
        let mut result = vec![
            1,
            1,
            self.class.flags.active,
            self.class.flags.personality,
            self.class.state,
            self.class.scale.to_bits(),
        ];
        result.extend(self.channels.iter().flatten().copied());
        result
    }
}
pub fn initial_channels(declared_loop: Option<u8>, inline: bool) -> [[u32; 9]; 4] {
    let mut channels = [[0; 9]; 4];
    for channel in &mut channels {
        channel[0] = 254;
    }
    if let Some(slot) = declared_loop {
        channels[0][0] = u32::from(slot);
        channels[0][8] = 0x101;
        channels[1][8] = 0x100;
    }
    if inline {
        channels[0] = [0, 0, 0, 0, 1, 0, 0, 0, 0x120];
        channels[1] = [254, 0, 0, 0, 1, 0, 0, 0, 0x100];
    }
    channels
}
