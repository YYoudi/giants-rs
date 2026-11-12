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
/// An immediate child is still awaiting ordinary reference placement. Keep its
/// actual zero birth pose distinct from the positional origin of its context.
pub struct ImmediateInitial<K> {
    pub actor: Initial<K>,
    pub binding: crate::attached_effect_context::Immediate<K>,
    pub context_position: [f32; 3],
}
/// Acquired following VM commands, before ordinary preparation or any callback.
/// The caller still owns the template, source scope, context and world admission.
pub struct Configured<K> {
    pub initial: Initial<K>,
    pub fade: [f32; 4],
    pub transparency: f32,
    pub visual: crate::effect_visual::Visual,
    pub retirement: Option<crate::effect_scene::RetirementBinding>,
    pub(crate) ground: Option<crate::effect_ground_emission::Source>,
    pub(crate) source: Arc<giants_formats::effect_image::Image>,
}

/// Shared acquired callback entry, before lifetime, physical or ANM services.
pub(crate) fn callback_entry(fields: &mut Fields, render_flags: u32) -> bool {
    fields.object_flags |= 0x40;
    let first = fields.control_flags & 0x20 != 0;
    if first {
        fields.control_flags &= !0x20;
        if render_flags & 0x10000 == 0 {
            fields.object_flags &= !0x40000000;
        }
    }
    first
}

/// Source attachment writer after callback/finalization, including its actual
/// history publication. Never use this to move a retirement request early.
pub(crate) fn bind_parented(
    fields: &mut Fields,
    support: Matrix,
    support_scale: f32,
    reference: Matrix,
) -> Result<Matrix, String> {
    let mut world =
        crate::effect_attachment::normal_matrix(support, fields.scale, support_scale, reference)?;
    world.values[11] =
        (f64::from(world.values[11]) + f64::from(f32::from_bits(fields.configuration[17]))) as f32;
    if !world.values.iter().all(|v| v.is_finite()) {
        return Err("effect attachment world matrix nonfinite".into());
    }
    fields.previous = fields.current;
    fields.position = [world.values[3], world.values[7], world.values[11]];
    fields.current = fields.position;
    Ok(world)
}

/// Shared acquired lifetime operands. This does not publish an actor or advance
/// time. Kept separate from size, spin, gravity and retirement command effects.
pub(crate) fn lifetime_fade(life: [f32; 3]) -> Result<[f32; 4], String> {
    let [duration, fade_in, fade_out] = life;
    if !life.into_iter().all(f32::is_finite) || duration <= 0. || fade_in < 0. || fade_out < 0. {
        return Err("effect lifetime outside finite timed profile".into());
    }
    let fade = [
        if fade_in > 0. { fade_in } else { 0. },
        if fade_out > 0. { fade_out } else { 0. },
        if fade_out > 0. && fade_out < duration {
            (1. / (f64::from(duration) - f64::from(fade_out))) as f32
        } else {
            0.
        },
        if fade_in > 0. {
            (1. / f64::from(fade_in)) as f32
        } else {
            0.
        },
    ];
    if !fade.into_iter().all(f32::is_finite) {
        return Err("effect fade reciprocal overflow".into());
    }
    Ok(fade)
}

pub(crate) fn lifetime_flags(fields: &mut Fields, life: [f32; 3]) {
    fields.control_flags |= 2;
    if life[1] > 0. {
        fields.control_flags |= 0x800000;
    }
    if life[2] > 0. && life[2] < life[0] {
        fields.control_flags |= 4;
    }
}
impl<K> Initial<K> {
    fn inherit_matrix(&mut self, matrix: Matrix) -> Result<(), String> {
        let data = self
            .seed
            .storage
            .data_mut()
            .ok_or("inherited effect payload absent")?;
        data[0x3c..0x40].copy_from_slice(&matrix.flags.to_le_bytes());
        for (bytes, value) in data[0x40..0x80]
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(matrix.values)
        {
            bytes.copy_from_slice(&value.to_le_bytes());
        }
        self.fields.control_flags |= 0x800;
        Ok(())
    }
    /// The native size writer replaces the affine 3x4 and leaves its bottom row.
    pub(crate) fn size(&mut self, sizes: [f32; 2], interpolate: bool) -> Result<(), String> {
        if sizes.into_iter().any(|v| !v.is_finite() || v <= 0.) {
            return Err("effect size outside positive finite profile".into());
        }
        let mut matrix =
            crate::effect_orientation::matrix(self.fields.position, self.fields.angles, sizes[0])?;
        matrix.values[12..].copy_from_slice(&self.matrix.values[12..]);
        self.fields.scale = sizes[0];
        if interpolate {
            self.fields.control_flags |= 8;
        }
        let data = self
            .seed
            .storage
            .data_mut()
            .ok_or("effect size payload absent")?;
        data[0x10..0x14].copy_from_slice(&sizes[0].to_le_bytes());
        data[0x14..0x18].copy_from_slice(&sizes[1].to_le_bytes());
        self.matrix = matrix;
        Ok(())
    }
    fn timed(
        mut self,
        configuration: crate::effect_visual::Configuration,
        source: Arc<giants_formats::effect_image::Image>,
    ) -> Result<Configured<K>, String> {
        let life = [
            configuration.lifetime,
            configuration.fade_in,
            configuration.fade_out_start,
        ];
        let fade = lifetime_fade(life)?;
        let visual = crate::effect_visual::Visual::new(configuration)?;
        lifetime_flags(&mut self.fields, life);
        let data = self
            .seed
            .storage
            .data_mut()
            .ok_or("effect lifetime payload absent")?;
        data[4..8].copy_from_slice(&life[0].to_le_bytes());
        data[8..12].copy_from_slice(&0f32.to_le_bytes());
        Ok(Configured {
            initial: self,
            fade,
            transparency: visual.transparency,
            visual,
            retirement: None,
            ground: None,
            source,
        })
    }
    /// A view of the actual owned payload, never a second physical allocation.
    pub fn physical_words(&self) -> Result<[u32; 15], String> {
        let data = physical_data(&self.seed)?;
        Ok(std::array::from_fn(|i| {
            u32::from_le_bytes(data[i * 4..i * 4 + 4].try_into().unwrap())
        }))
    }
}
pub(crate) fn physical_data(seed: &Seed) -> Result<&[u8], String> {
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
    for (chunk, word) in data[0xa0..0xdc]
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(words)
    {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    Ok(())
}
impl Template {
    /// Complete source3 surface frame, including its real terrain sample.
    /// A context already ran its null pass; this consumes only active ranges.
    /// Prototype search key `fx_orientToGround__FP10FXSTACKTAGf` is a stub;
    /// preserve the acquired PC query/alignment rather than importing that stub.
    pub fn surface_positioned<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
        random: &mut crate::audio_parameters::Random,
        mut query: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
    ) -> Result<Configured<K>, String> {
        let program = context.program.as_ref().map_err(Clone::clone)?;
        if program.records.iter().map(|r| r.opcode).collect::<Vec<_>>()
            != [3, 6, 4, 0x17, 0x1c, 0x79, 7, 2]
            || context.flags() & !0x1c000 != 0
        {
            return Err("surface constructor outside acquired positional frame".into());
        }
        self.validate(context, 3, 1)?;
        let position = context
            .position()
            .ok_or("surface context position absent")?;
        if !position.into_iter().all(f32::is_finite) {
            return Err("surface context position nonfinite".into());
        }
        let image = context.source_image();
        let entry = image
            .entries
            .get(context.entry_index)
            .ok_or("surface context entry absent")?;
        if entry.offset != context.entry_offset {
            return Err("surface context entry changed".into());
        }
        let plan = crate::effect_scene::Plan::project(&image, &entry.name)?;
        let mut staged = *random;
        let (objects, cursor) = plan.activate(&mut staged.0)?;
        // Active draws precede the real surface query. A later query failure
        // cannot rewind the composing world's already consumed stream.
        *random = staged;
        if objects.len() != 1 || cursor.remaining.is_some() || plan.requires_external_resume() {
            return Err("surface constructor requires one complete source object".into());
        }
        let object = &objects[0];
        let mut initial = self.initial(
            origin,
            Placement::Position(position),
            context.flags(),
            false,
        )?;
        // Source17 translates the current point without a history copy.
        initial.fields.position[2] =
            (f64::from(initial.fields.position[2]) + f64::from(object.height)) as f32;
        let size = object.size.ok_or("surface source size absent")?;
        initial.size(size, false)?;
        let point = initial.fields.position;
        let sample = query([point[0], point[1]])?;
        let placement = crate::effect_surface::placement(
            point,
            sample.plane,
            sample.height,
            object
                .surface_offset
                .ok_or("surface source offset absent")?,
        )?;
        let matrix = crate::effect_surface::alignment_matrix(
            initial.matrix,
            [sample.plane[0], sample.plane[1], sample.plane[2]],
            placement,
        )?;
        // Source79's location setter copies history, then the helper publishes
        // the aligned affine's translation. No live spatial-list admission is
        // implied by this unadvanced constructor.
        initial.fields.previous = initial.fields.current;
        initial.fields.position = placement.position;
        initial.fields.current = placement.position;
        initial.matrix = matrix;
        let life = object.lifetime.ok_or("surface source lifetime absent")?;
        let configured = initial.timed(
            crate::effect_visual::Configuration {
                lifetime: life[0],
                fade_in: life[1],
                fade_out_start: life[2],
                start_size: size[0],
                end_size: size[1],
            },
            image,
        )?;
        *random = staged;
        Ok(configured)
    }
    /// Active source3 ground-emission frame. Its context has already executed
    /// the null-object pass; only the active ranges are consumed here.
    pub fn ground_positioned<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<Configured<K>, String> {
        let program = context.program.as_ref().map_err(Clone::clone)?;
        if program.records.iter().map(|r| r.opcode).collect::<Vec<_>>()
            != [3, 6, 5, 9, 0x12, 0x16, 0x0e, 0x5a, 7, 2]
        {
            return Err("ground constructor outside acquired source frame".into());
        }
        let image = context.source_image();
        let entry = image
            .entries
            .get(context.entry_index)
            .ok_or("ground context entry absent")?;
        if entry.offset != context.entry_offset {
            return Err("ground context entry changed".into());
        }
        let plan = crate::effect_scene::Plan::project(&image, &entry.name)?;
        let mut staged = *random;
        let (objects, cursor) = plan.activate(&mut staged.0)?;
        if objects.len() != 1 || cursor.remaining.is_some() || plan.requires_external_resume() {
            return Err("ground constructor requires one complete source object".into());
        }
        let object = &objects[0];
        let source =
            crate::effect_ground_emission::Source::resolve(&object.object_services, &image)?
                .ok_or("ground source callback configuration absent")?;
        let mut initial = self.positioned(origin, context)?;
        let size = object.size.ok_or("ground source sizes absent")?;
        let life = object.lifetime.ok_or("ground source lifetime absent")?;
        initial.size(size, size[0] != size[1])?;
        let mut configured = initial.timed(
            crate::effect_visual::Configuration {
                lifetime: life[0],
                fade_in: life[1],
                fade_out_start: life[2],
                start_size: size[0],
                end_size: size[1],
            },
            image,
        )?;
        configured.initial.fields.object_flags &= !0x10;
        configured.initial.fields.control_flags |= 0x80;
        let data = configured.initial.seed.storage.data_mut().unwrap();
        data[0x1c..0x20].copy_from_slice(&object.speed.to_le_bytes());
        data[0xa4..0xa8].copy_from_slice(&object.gravity.to_le_bytes());
        source.configure(&mut configured.initial.fields, &mut configured.initial.seed)?;
        configured.ground = Some(source);
        *random = staged;
        Ok(configured)
    }
    /// Consumes only the active size draw: context preparation already consumed
    /// its own draw. No context retirement, stock allocation or callback occurs.
    pub fn timed_attached<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<Configured<K>, String> {
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let recipe = crate::timed_effect::Recipe::project(program)?;
        let mut initial = self.attached(origin, context)?;
        let image = context.source_image();
        let (_, entry) = image
            .find(&recipe.child)
            .ok_or("timed retirement program absent")?;
        image
            .program_tail(entry.offset)
            .map_err(|e| e.to_string())?;
        // Validate all fallible scalar preparation before consuming the draw.
        let life = [recipe.lifetime, recipe.fade_in, recipe.fade_out];
        lifetime_fade(life)?;
        for size in recipe.range {
            crate::effect_orientation::matrix(
                initial.fields.position,
                initial.fields.angles,
                size,
            )?;
        }
        let mut seed = random.0;
        let size = crate::timed_effect::sample(&mut seed, recipe.range);
        initial.size([size; 2], false)?;
        let mut configured = initial.timed(
            crate::effect_visual::Configuration {
                lifetime: life[0],
                fade_in: life[1],
                fade_out_start: life[2],
                start_size: size,
                end_size: size,
            },
            image.clone(),
        )?;
        configured.initial.fields.control_flags |= 1;
        configured.retirement = Some(crate::effect_scene::RetirementBinding {
            effect: recipe.child,
            program_offset: entry.offset,
        });
        random.0 = seed;
        Ok(configured)
    }
    /// Exact detached linear-size/gravity frame, before ordinary physical and
    /// reference preparation. No contact, stock or world schedule is implied.
    pub fn gravity_positioned<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
    ) -> Result<Configured<K>, String> {
        let program = context.program.as_ref().map_err(Clone::clone)?;
        if program.records.iter().map(|r| r.opcode).collect::<Vec<_>>() != [3, 6, 8, 4, 0x0e, 7, 2]
        {
            return Err("effect configuration outside acquired gravity frame".into());
        }
        let recipe = crate::released_effect::Recipe::project(program)?;
        let mut initial = self.positioned(origin, context)?;
        initial.size(
            [recipe.visual.start_size, recipe.visual.end_size],
            recipe.visual.start_size != recipe.visual.end_size,
        )?;
        let mut configured = initial.timed(recipe.visual, context.source_image())?;
        configured.initial.fields.object_flags &= !0x10;
        configured.initial.fields.control_flags |= 0x80;
        configured.initial.seed.storage.data_mut().unwrap()[0xa4..0xa8]
            .copy_from_slice(&recipe.gravity_mass.to_le_bytes());
        Ok(configured)
    }
    /// Complete active smoke child. The null pass belongs to its real context.
    pub fn smoke_positioned<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<Configured<K>, String> {
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let recipe = crate::mecc_smoke_particle::Recipe::project(program)?;
        if recipe.class != self.fields.class {
            return Err("smoke child class differs from constructor".into());
        }
        let mut initial = self.positioned(origin, context)?;
        let mut staged = *random;
        let (visual, gravity, side) = recipe.activate(&mut staged);
        initial.size([visual.start_size, visual.end_size], false)?;
        if visual.start_size != visual.end_size {
            initial.fields.control_flags |= 0x100000;
        }
        let mut configured = initial.timed(visual, context.source_image())?;
        configured.visual = crate::effect_visual::Visual::with_size_curve(
            visual,
            crate::effect_visual::SizeCurve::EaseOut,
        )?;
        configured.initial.fields.object_flags &= !0x10;
        configured.initial.fields.control_flags |= 0xc0;
        configured.initial.render_flags |= 0x8000;
        let data = configured.initial.seed.storage.data_mut().unwrap();
        data[0xa4..0xa8].copy_from_slice(&gravity.to_le_bytes());
        data[0xf4..0xf8].copy_from_slice(&side.to_le_bytes());
        *random = staged;
        Ok(configured)
    }
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
        // Source3 keeps the context's cinematic inheritance in the response
        // word as well as the render mark. Source24 is a separate constructor.
        if !attached && flags & 0x8000 != 0 {
            fields.response_flags |= 0x1000;
        }
        if !attached && flags & 0x4000 != 0 {
            fields.response_flags |= 0x400;
        }
        if !attached && flags & 0x10000 != 0 {
            fields.object_flags |= 0x400;
        }
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
        self.attached_initial(origin, context)
    }
    /// Only the complete retained smoke frame's caller may enter this profile.
    /// The public ordinary/timed constructors keep their existing flag gates.
    pub(crate) fn smoke_attached<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
    ) -> Result<Initial<K>, String> {
        if context.flags() != 0x20004 {
            return Err("smoke constructor requires the extended attached context".into());
        }
        self.attached_initial(origin, context)
    }
    fn attached_initial<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
    ) -> Result<Initial<K>, String> {
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
    pub(crate) fn ghost_positioned<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
    ) -> Result<Initial<K>, String> {
        if context.flags() != 0x20000 {
            return Err("ghost constructor requires its retained positional context".into());
        }
        self.validate(context, 3, 1)?;
        crate::vimp_ghost::Recipe::project(context.program.as_ref().map_err(Clone::clone)?)?;
        let point = context.position().ok_or("ghost context position absent")?;
        self.initial(origin, Placement::Position(point), context.flags(), false)
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
    pub fn rotated_positioned<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
    ) -> Result<Initial<K>, String> {
        if !matches!(context.flags(), 0x1000 | 0x9000) {
            return Err("rotated positional constructor context flags differ".into());
        }
        self.validate(context, 3, 1)?;
        let point = context
            .position()
            .ok_or("rotated positional origin absent")?;
        let mut initial =
            self.initial(origin, Placement::Position(point), context.flags(), false)?;
        initial.inherit_matrix(
            context
                .inherited_matrix()
                .ok_or("rotated positional basis absent")?,
        )?;
        Ok(initial)
    }
    /// Source3/6 creation under the immediate wrapper, before VM configuration,
    /// ordinary ANM, reference placement or its first state callback. This is
    /// not a permanent attached effect and does not acknowledge world admission.
    pub fn immediate_positioned<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
    ) -> Result<ImmediateInitial<K>, String> {
        self.immediate_initial(origin, context, false)
    }
    pub fn immediate_inherited_positioned<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
    ) -> Result<ImmediateInitial<K>, String> {
        self.immediate_initial(origin, context, true)
    }
    fn immediate_initial<K: Copy>(
        &self,
        origin: Id,
        context: &Context<K>,
        inherited: bool,
    ) -> Result<ImmediateInitial<K>, String> {
        let flags = if inherited {
            context.flags() & !0x1000
        } else {
            context.flags()
        };
        if !matches!(flags, 2 | 0x8002) || inherited != context.inherited_matrix().is_some() {
            return Err("immediate effect requires its acquired direct context flags".into());
        }
        let binding = context
            .immediate()
            .ok_or("immediate parent binding absent")?;
        let context_position = context
            .position()
            .ok_or("immediate context origin absent")?;
        if context_position.into_iter().any(|v| !v.is_finite()) {
            return Err("immediate context origin is nonfinite".into());
        }
        self.validate(context, 3, 1)?;
        let mut actor =
            self.initial(origin, Placement::Position([0.; 3]), context.flags(), false)?;
        actor.fields.control_flags |= 0x40;
        // The immediate wrapper has not yet resolved either reference. Native
        // fresh storage retains all sixteen zero words even with identity flag10.
        actor.matrix = Matrix {
            flags: 0x10,
            values: [0.; 16],
        };
        if inherited {
            actor.inherit_matrix(context.inherited_matrix().unwrap())?;
        }
        Ok(ImmediateInitial {
            actor,
            binding,
            context_position,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_NPC_ROOT; actual death root/child constructor resources with supplied selection, position and angle; no active FX graph, placement or GPU"]
    fn source_death_angle_basis_is_payload_inheritance_not_birth_rotation() {
        let root = std::env::var_os("GIANTS_NPC_ROOT").unwrap();
        let resources =
            SceneResources::load(std::path::Path::new(&root), "w_Story2.bin", "English").unwrap();
        let mut pool =
            crate::attached_effect_context::Pool::<u32>::from_active_resources(17, &resources);
        let mut random = crate::audio_parameters::Random(1);
        let selection = crate::effect_initialization::Selection {
            active_objects: 0,
            quality: 1.,
            lite_distance: 500.,
            camera: [1., 2., 3.],
            recipient_position: [1., 2., 3.],
        };
        let (crate::attached_effect_context::Creation::Prepared(origin), _) = pool
            .prepare_rotated_positional(
                "FXB_VIMP_death",
                [1., 2., 3.],
                [37.5, 0., 0.],
                selection,
                &mut random,
            )
            .unwrap()
        else {
            panic!("death context absent")
        };
        let context = pool.get(origin).unwrap();
        let template = Template::load(&resources, 919).unwrap();
        assert!(template.positioned(origin, context).is_err());
        let actor = template.rotated_positioned(origin, context).unwrap();
        assert_eq!(actor.fields.control_flags & 0x820, 0x820);
        assert_eq!(actor.matrix.values[0], 1.);
        assert_eq!(
            [
                actor.matrix.values[3],
                actor.matrix.values[7],
                actor.matrix.values[11]
            ],
            [1., 2., 3.]
        );
        let basis = context.inherited_matrix().unwrap();
        assert_ne!(basis.values[0], actor.matrix.values[0]);
        let data = actor.seed.storage.data().unwrap();
        assert_eq!(u32::from_le_bytes(data[0x3c..0x40].try_into().unwrap()), 1);
        assert_eq!(
            data[0x40..0x80]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|v| u32::from_le_bytes(*v))
                .collect::<Vec<_>>(),
            basis.values.map(f32::to_bits)
        );
        let image = &resources.classes.models.effects.last().unwrap().image;
        let offset = image.find("FXB_chunk1").unwrap().1.offset;
        let (crate::attached_effect_context::Creation::Prepared(child), intervals) = pool
            .prepare_immediate_inherited_offset_invariant(
                offset,
                crate::attached_effect_context::ImmediateInput {
                    binding: crate::attached_effect_context::Immediate {
                        parent: 31,
                        reference: 11,
                        own_reference: 0,
                    },
                    position: [1., 2., 3.],
                    object_flags: 0,
                    render_flags: 0,
                },
                basis,
                &mut random,
            )
            .unwrap()
        else {
            panic!("child context absent")
        };
        assert_eq!(intervals.len(), 6);
        let context = pool.get(child).unwrap();
        let template = Template::load(&resources, 906).unwrap();
        assert!(template.immediate_positioned(child, context).is_err());
        let initial = template
            .immediate_inherited_positioned(child, context)
            .unwrap();
        assert_eq!(
            (initial.binding.own_reference, initial.binding.reference),
            (0, 11)
        );
        assert_eq!(initial.actor.fields.position, [0.; 3]);
        assert_eq!(initial.actor.matrix.values.map(f32::to_bits), [0; 16]);
        assert_eq!(initial.actor.fields.control_flags & 0x860, 0x860);
        assert_eq!(pool.active(), [origin, child]);
    }
    #[test]
    #[ignore = "requires GIANTS_MECC_WORLD_ROOT; immediate source resources/constructor prefix only, no live actor, placement or rendering"]
    fn source_immediate_particle_preserves_zero_birth_and_actual_reference_binding() {
        let root = std::env::var_os("GIANTS_MECC_WORLD_ROOT").expect("GIANTS_MECC_WORLD_ROOT");
        let resources =
            SceneResources::load(std::path::Path::new(&root), "w_Story1.bin", "English").unwrap();
        let image = &resources.classes.models.effects.last().unwrap().image;
        let (_, entry) = image.find("FX_bullet_debri").unwrap();
        let mut pool =
            crate::attached_effect_context::Pool::<u32>::from_active_resources(11, &resources);
        let mut random = crate::audio_parameters::Random(17);
        let binding = crate::attached_effect_context::Immediate {
            parent: 31,
            reference: 0,
            own_reference: 1,
        };
        let (crate::attached_effect_context::Creation::Prepared(id), _) = pool
            .prepare_immediate_offset_invariant(
                entry.offset,
                crate::attached_effect_context::ImmediateInput {
                    binding,
                    position: [3., 4., 7.],
                    object_flags: 0,
                    render_flags: 0,
                },
                &mut random,
            )
            .unwrap()
        else {
            panic!("source child context absent")
        };
        let context = pool.get(id).unwrap();
        let template = Template::load(&resources, 913).unwrap();
        assert!(template.positioned(id, context).is_err());
        let initial = template.immediate_positioned(id, context).unwrap();
        assert_eq!(initial.binding, binding);
        assert_eq!(initial.context_position, [3., 4., 7.]);
        assert_eq!(initial.actor.fields.position, [0.; 3]);
        assert_eq!(initial.actor.fields.current, [0.; 3]);
        assert_eq!(initial.actor.fields.control_flags, 0x60);
        assert_eq!(initial.actor.fields.object_flags, 0x40010030);
        assert_eq!(initial.actor.render_flags, 0);
        assert_eq!(initial.actor.matrix.flags, 0x10);
        assert_eq!(initial.actor.matrix.values.map(f32::to_bits), [0; 16]);
        let physics = initial.actor.physical_words().unwrap();
        assert_eq!(physics[1], 1f32.to_bits());
        assert_eq!(physics[2], 1f32.to_bits());
        assert_eq!(physics[4], 1f32.to_bits());
        assert_eq!(
            pool.active(),
            &[id],
            "constructor prefix cannot retire its context"
        );
    }
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
            .as_chunks::<4>()
            .0
            .iter()
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
