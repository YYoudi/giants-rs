//! Owned first active update of the acquired flags4 attached FX profile.
//! The temporary context retires; the state78 object and optional emitter live on.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    attached_effect_context::{Context, Id as ContextId},
    model_registry::StateBinding,
    resident_states::{Construction, Id, Session},
    scene_resources::SceneResources,
    state_storage::{Profile, Seed},
};
use giants_formats::{effect_image::Image, effect_program::Program};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct Emitter {
    pub entry_offset: u32,
    pub interval: f32,
    pub reference: u32,
    source: Arc<Image>,
}
impl Emitter {
    pub fn program(&self) -> Result<Program, giants_formats::Error> {
        self.source.program(self.entry_offset)
    }
    /// Normalized initial bank: one occupied slot; program pointer is an offset.
    /// No periodic callback, emission, or random initialization is executed.
    pub fn initial_words(&self) -> Vec<u32> {
        let mut words = vec![0; 0x4dc / 4];
        words[0] = self.entry_offset;
        words[0x28 / 4] = self.interval.to_bits();
        words[0x78 / 4] = self.reference;
        words[0xa0 / 4] = self.reference;
        words[0xc8 / 4] = u32::MAX;
        words[0xf0 / 4] = u32::MAX;
        words[0x168 / 4] = (-1f32).to_bits();
        words[0x190 / 4] = 1;
        words
    }
}
#[derive(Clone, Debug)]
pub struct Metadata {
    /// Historical origin only: this context identity is retired after creation.
    pub origin: ContextId,
    pub parent: Id,
    pub reference: u32,
    pub tag: u32,
    pub opaque: u32,
    /// Local initialization cache, NOT a resolved parent/world binding matrix.
    pub initial_matrix: crate::procedural_aim::Matrix,
    pub(crate) world_matrix: Option<crate::procedural_aim::Matrix>,
    pub emitter: Option<Emitter>,
    /// Requested name retained even when the real symbol lookup is absent.
    pub emitter_request: Option<String>,
}
pub(crate) struct Prepared {
    pub seed: Seed,
    pub fields: Fields,
    pub metadata: Metadata,
}
struct Recipe {
    class: i32,
    reference: u32,
    opaque: u32,
    scale: f32,
    emitter: Option<Emitter>,
    emitter_request: Option<String>,
}
impl Recipe {
    fn project(context: &Context<Id>) -> Result<Option<Self>, String> {
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let ops: Vec<_> = program.records.iter().map(|r| r.opcode).collect();
        if ops == [2] {
            return Ok(None);
        }
        if !matches!(
            ops.as_slice(),
            [0x24, 6, 0x1b, 7, 2] | [0x24, 6, 0x1b, 0x27, 7, 2]
        ) {
            return Err("attached object update outside acquired complete body".into());
        }
        let word = |record: usize, at: usize| -> Result<u32, String> {
            Ok(u32::from_le_bytes(
                program.records[record]
                    .bytes
                    .get(at..at + 4)
                    .ok_or("attached object operand truncated")?
                    .try_into()
                    .unwrap(),
            ))
        };
        let scale = f32::from_bits(word(2, 1)?);
        let class = word(0, 5)? as i32;
        if class < 0 || !scale.is_finite() || scale <= 0. || scale > 100000. {
            return Err("attached object class/scale outside acquired profile".into());
        }
        let mut emitter_request = None;
        let emitter = if ops[3] == 0x27 {
            let bytes = &program.records[3].bytes;
            let end = bytes[1..]
                .iter()
                .position(|b| *b == 0)
                .ok_or("emitter label unterminated")?
                + 1;
            let name = std::str::from_utf8(&bytes[1..end]).map_err(|e| e.to_string())?;
            let interval = f32::from_bits(word(3, end + 1)?);
            if !interval.is_finite() || interval <= 0. {
                return Err("attached periodic interval outside positive finite profile".into());
            }
            let source = context.source_image();
            // The native helper uses the first direct symbol, without lite selection.
            emitter_request = Some(name.to_owned());
            let reference = word(3, end + 5)?;
            if let Some((_, entry)) = source.find(name) {
                source
                    .program_tail(entry.offset)
                    .map_err(|e| e.to_string())?;
                Some(Emitter {
                    entry_offset: entry.offset,
                    interval,
                    reference,
                    source: source.clone(),
                })
            } else {
                None
            }
        } else {
            None
        };
        Ok(Some(Self {
            class,
            reference: word(0, 9)?,
            opaque: word(0, 1)?,
            scale,
            emitter,
            emitter_request,
        }))
    }
}
struct Template {
    seed: Seed,
    bindings: Vec<StateBinding>,
    fields: Fields,
    specification: Arc<crate::specification::Prepared>,
    states: Arc<crate::state_resources::Prepared>,
}
impl Template {
    fn load(resources: &SceneResources, class: i32) -> Result<Self, String> {
        let program = resources
            .classes
            .models
            .first_program(class)
            .ok_or("attached effect resource class absent")?
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
            return Err(
                "attached object requires acquired zero-initialized D state78 class".into(),
            );
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
            return Err("attached effect resources incomplete".into());
        }
        let fields = Fields::load(resources, class, &specification)?;
        Ok(Self {
            seed,
            bindings: resources.class_state_bindings_for(class, Profile::Debug)?,
            fields,
            specification,
            states,
        })
    }
    fn prepare(
        &self,
        recipe: Recipe,
        origin: ContextId,
        context: &Context<Id>,
    ) -> Result<Prepared, String> {
        let mut seed = self.seed.instantiate()?;
        let data = seed.storage.data_mut().ok_or("attached payload absent")?;
        for at in [0xa4, 0xa8, 0xb0] {
            data[at..at + 4].copy_from_slice(&1f32.to_le_bytes());
        }
        for at in [0x10, 0x14] {
            data[at..at + 4].copy_from_slice(&recipe.scale.to_le_bytes());
        }
        let mut fields = self.fields.instantiate();
        fields.configuration[14] = (fields.configuration[14] & !0x20000) | 0x800;
        fields.configuration[22] = 50f32.to_bits();
        fields.object_flags |= 0x40410030;
        fields.control_flags |= 0x20 | if recipe.emitter.is_some() { 0x2000 } else { 0 };
        fields.scale = recipe.scale;
        let mut matrix =
            crate::effect_orientation::matrix(fields.position, fields.angles, fields.scale)
                .map_err(str::to_owned)?;
        // The native writer touches only3x4 of the initially cleared cache.
        matrix.values[12..].fill(0.);
        Ok(Prepared {
            seed,
            fields,
            metadata: Metadata {
                origin,
                parent: context.recipient().ok_or("attached effect parent absent")?,
                reference: recipe.reference,
                tag: context.tag().ok_or("attached effect tag absent")?,
                opaque: recipe.opaque,
                initial_matrix: matrix,
                world_matrix: None,
                emitter: recipe.emitter,
                emitter_request: recipe.emitter_request,
            },
        })
    }
}
pub struct Factory<'a> {
    resources: &'a SceneResources,
    templates: BTreeMap<i32, Result<Template, String>>,
}
impl<'a> Factory<'a> {
    pub fn new(resources: &'a SceneResources) -> Self {
        Self {
            resources,
            templates: BTreeMap::new(),
        }
    }
    pub fn cached_classes(&self) -> usize {
        self.templates.len()
    }
    pub(crate) fn activate(
        &mut self,
        session: &mut Session,
        origin: ContextId,
        context: &Context<Id>,
    ) -> Result<Option<Id>, String> {
        if context.flags() != 4 {
            return Err("attached activation requires acquired flags4 profile".into());
        }
        let parent = session
            .get(context.recipient().ok_or("attached effect parent absent")?)
            .ok_or("attached recipient absent or foreign")?;
        if parent.construction_fields.is_none() {
            return Err("attached recipient fields absent".into());
        }
        let Some(recipe) = Recipe::project(context)? else {
            return Ok(None);
        };
        let template = self
            .templates
            .entry(recipe.class)
            .or_insert_with(|| Template::load(self.resources, recipe.class))
            .as_ref()
            .map_err(Clone::clone)?;
        let prepared = template.prepare(recipe, origin, context)?;
        let flags = crate::object_lists::DirectFlags {
            active: prepared.fields.object_flags,
            personality: prepared.fields.configuration[14],
        };
        session
            .construct_attached_effect(
                Construction {
                    class: prepared.fields.class,
                    seed: &prepared.seed,
                    bindings: &template.bindings,
                    flags,
                    specification: template.specification.clone(),
                    state_resources: template.states.clone(),
                    fields: prepared.fields.clone(),
                },
                prepared.metadata,
            )
            .map(Some)
    }
}
