//! Owned first active update of the acquired flags4 attached FX profile.
//! The temporary context retires; the state78 object and optional emitter live on.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    attached_effect_context::{Context, Id as ContextId},
    effect_actor::Template,
    resident_states::{Construction, Id, Session},
    scene_resources::SceneResources,
    state_storage::Seed,
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
pub struct Metadata<K = Id> {
    /// Historical origin only: this context identity is retired after creation.
    pub origin: ContextId,
    pub parent: K,
    pub reference: u32,
    pub tag: u32,
    pub opaque: u32,
    /// Independent effect/render routing word, distinct from the actor's
    /// shared response/status flags. Inheritance must not dirty that word.
    pub render_flags: u32,
    /// Local initialization cache, NOT a resolved parent/world binding matrix.
    pub initial_matrix: crate::procedural_aim::Matrix,
    pub(crate) world_matrix: Option<crate::procedural_aim::Matrix>,
    pub emitter: Option<Emitter>,
    /// Requested name retained even when the real symbol lookup is absent.
    pub emitter_request: Option<String>,
}
pub(crate) struct Prepared<K = Id> {
    pub seed: Seed,
    pub fields: Fields,
    pub metadata: Metadata<K>,
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
    fn project<K>(context: &Context<K>) -> Result<Option<Self>, String> {
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
fn prepare<K: Copy>(
    template: &Template,
    recipe: Recipe,
    origin: ContextId,
    context: &Context<K>,
) -> Result<Prepared<K>, String> {
    let mut initial = template.attached(origin, context)?;
    initial.size([recipe.scale; 2], false)?;
    let seed = initial.seed;
    let mut fields = initial.fields;
    fields.control_flags |= if recipe.emitter.is_some() { 0x2000 } else { 0 };
    let matrix = initial.matrix;
    Ok(Prepared {
        seed,
        fields,
        metadata: Metadata {
            origin,
            parent: context.recipient().ok_or("attached effect parent absent")?,
            reference: recipe.reference,
            tag: context.tag().ok_or("attached effect tag absent")?,
            opaque: recipe.opaque,
            render_flags: initial.render_flags,
            initial_matrix: matrix,
            world_matrix: None,
            emitter: recipe.emitter,
            emitter_request: recipe.emitter_request,
        },
    })
}
/// Immutable constructor resources shared by cinematic objects. This reuses
/// the ordinary attached constructor; it is not another actor/state factory.
pub struct CinematicTemplate(Template);
impl CinematicTemplate {
    pub fn load(resources: &SceneResources, class: i32) -> Result<Self, String> {
        Ok(Self(Template::load(resources, class)?))
    }
    pub fn construct<K: Copy>(
        &self,
        origin: ContextId,
        context: &Context<K>,
    ) -> Result<CinematicObject<K>, String> {
        if context.flags() != 0x8004 || context.tag() != Some(u32::MAX) {
            return Err("cinematic object requires source-marked attached support".into());
        }
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let bounded = crate::effect_attachment::Recipe::project(program).map_err(str::to_owned)?;
        if bounded.class != self.0.fields.class {
            return Err("cinematic effect constructor class mismatch".into());
        }
        let recipe = Recipe::project(context)?.ok_or("persistent effect program absent")?;
        let prepared = prepare(&self.0, recipe, origin, context)?;
        let references = crate::actor_references::Owned::new(self.0.specification.clone())
            .map_err(str::to_owned)?;
        let mut durations = vec![None; 254];
        for (slot, descriptor) in &self.0.specification.animations {
            *durations
                .get_mut(usize::from(*slot))
                .ok_or("cinematic effect descriptor outside channel profile")? =
                Some(descriptor.template.duration);
        }
        Ok(CinematicObject {
            fields: prepared.fields,
            seed: prepared.seed,
            metadata: prepared.metadata,
            references,
            durations,
            updates: 0,
        })
    }
}
/// The context can retire after this owned object has been constructed. Its
/// class resources, channels, reference cache and state payload remain alive.
pub struct CinematicObject<K> {
    pub fields: Fields,
    pub seed: Seed,
    pub metadata: Metadata<K>,
    references: crate::actor_references::Owned,
    durations: Vec<Option<f32>>,
    pub updates: u64,
}
impl<K> CinematicObject<K> {
    pub fn world_matrix(&self) -> Option<crate::procedural_aim::Matrix> {
        self.metadata.world_matrix
    }
    /// Consume the actual source support and this object's own current channel.
    /// No scene-specific offset, clock, reference or lifetime is substituted.
    pub fn update(
        &mut self,
        support: crate::procedural_aim::Matrix,
        seconds: f32,
    ) -> Result<(), String> {
        let mut channels = self.fields.channels;
        let tick = crate::animation_channels::advance(
            &mut channels,
            &self.durations,
            [0.; 3],
            seconds,
            Precision::Double53,
        )
        .map_err(str::to_owned)?;
        if tick.missing != 0 {
            return Err("cinematic effect animation descriptor missing".into());
        }
        // The acquired constant state78 callback publishes preparation even at
        // zero delta. Its first successful entry consumes the constructor gate;
        // the independent response bit can retain ordinary-draw suppression.
        crate::effect_actor::callback_entry(&mut self.fields, self.metadata.render_flags);
        let mut references = self.references.clone();
        references.update_current_unblended(&channels)?;
        let reference = references
            .current_matrices(&channels)?
            .get(self.metadata.reference as usize)
            .copied()
            .ok_or("cinematic effect own reference absent")?;
        let world = crate::effect_actor::bind_parented(
            &mut self.fields,
            support,
            1., // Native Flick support scale is independent of its carrier.
            reference,
        )?;
        self.fields.channels = channels;
        self.references = references;
        self.metadata.world_matrix = Some(world);
        self.updates += 1;
        Ok(())
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
        let prepared = prepare(template, recipe, origin, context)?;
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
