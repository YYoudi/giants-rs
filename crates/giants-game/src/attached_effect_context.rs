//! Owned exact-D named context preparation, before its first object update.
//! A live program/recipient/queue entry is owned; no emitted actor is implied.
use giants_formats::{effect_image::Image, effect_program::Program};
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Id {
    ticket: u64,
    slot: usize,
    generation: u64,
}
impl Id {
    pub fn slot(&self) -> usize {
        self.slot
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Binding<K> {
    Attached { recipient: K, tag: u32 },
    Position([f32; 3]),
}
#[derive(Clone, Debug)]
pub struct Context<K> {
    pub binding: Binding<K>,
    pub label: [u8; 16],
    pub entry_index: usize,
    pub entry_offset: u32,
    pub program: Result<Arc<Program>, String>,
    source: Arc<Image>,
    flags: u32,
}
impl<K> Context<K> {
    pub fn tag(&self) -> Option<u32> {
        match &self.binding {
            Binding::Attached { tag, .. } => Some(*tag),
            Binding::Position(_) => None,
        }
    }
    pub fn position(&self) -> Option<[f32; 3]> {
        match &self.binding {
            Binding::Attached { .. } => None,
            Binding::Position(p) => Some(*p),
        }
    }
    pub(crate) fn source_image(&self) -> Arc<Image> {
        self.source.clone()
    }
    pub fn cursor(&self) -> usize {
        0
    }
    pub fn flags(&self) -> u32 {
        self.flags
    }
    pub fn emitted_objects(&self) -> usize {
        0
    }
    pub fn source_program(&self) -> Result<&[u8], giants_formats::Error> {
        self.source.program_tail(self.entry_offset)
    }
    /// Preparation is not an effect interpreter or a completed update.
    pub fn require_first_update(&self) -> Result<(), &'static str> {
        Err("first update requires owned factory and recipient session")
    }
}
impl<K: Copy> Context<K> {
    pub fn recipient(&self) -> Option<K> {
        match self.binding {
            Binding::Attached { recipient, .. } => Some(recipient),
            Binding::Position(_) => None,
        }
    }
}
impl Pool<crate::resident_states::Id> {
    /// All fallible resource preparation and construction precede context
    /// retirement. A failed staged constructor leaves the request retryable;
    /// success consumes its identity, so it cannot create the object twice.
    pub fn activate(
        &mut self,
        id: Id,
        session: &mut crate::resident_states::Session,
        factory: &mut crate::attached_effect_object::Factory<'_>,
    ) -> Result<Option<crate::resident_states::Id>, String> {
        let context = self
            .get(id)
            .ok_or("effect context absent, foreign or retired")?
            .clone();
        let result = factory.activate(session, id, &context)?;
        self.release_prepared(id)?;
        Ok(result)
    }
}
impl<K: Copy + Eq> Pool<K> {
    /// Flick mode1 supplies the measured parameter4 cinematic mark. The
    /// ordinary named preparation still checks the complete deferred program
    /// and requires identical normal/lite selection, without invented inputs.
    pub fn prepare_cinematic(&mut self, name: &str, recipient: K) -> Result<Creation, String> {
        let creation = self.prepare(name, recipient, 0, u32::MAX)?;
        if let Creation::Prepared(id) = creation {
            self.slots[id.slot].context.as_mut().unwrap().flags |= 0x8000;
        }
        Ok(creation)
    }
    /// Acquired cinematic mark for a complete timed attached program whose
    /// null-object pass consumes a size draw. The caller owns the effect VM RNG;
    /// this is not an audio-module random stream or a new seed producer.
    pub fn prepare_cinematic_timed(
        &mut self,
        name: &str,
        recipient: K,
        recipient_flags: u32,
        tag: u32,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(Creation, Vec<crate::effect_initialization::Interval>), String> {
        if name.contains('\0') {
            return Err("effect name contains an embedded terminator".into());
        }
        let Some(image) = &self.image else {
            return Ok((Creation::Absent, vec![]));
        };
        let Some((_, entry)) = image.find(name) else {
            return Ok((Creation::Absent, vec![]));
        };
        let program = image.program(entry.offset).map_err(|e| e.to_string())?;
        if program.records.iter().map(|r| r.opcode).collect::<Vec<_>>()
            != [0x24, 6, 0x1c, 4, 0x0d, 7, 2]
        {
            return Err("cinematic random preparation outside acquired timed frame".into());
        }
        let (creation, intervals) =
            self.prepare_extended_invariant(name, recipient, recipient_flags, tag, random)?;
        if let Creation::Prepared(id) = creation {
            self.slots[id.slot].context.as_mut().unwrap().flags = 0x8004;
        }
        Ok((creation, intervals))
    }

    /// Private transaction staging for context preparation only. Existing
    /// identities keep their ticket/generation; no external update is replayed.
    pub(crate) fn staged_copy(&self) -> Self {
        Self {
            ticket: self.ticket,
            image: self.image.clone(),
            slots: self
                .slots
                .iter()
                .map(|slot| Slot {
                    generation: slot.generation,
                    context: slot.context.clone(),
                    sound_started: slot.sound_started,
                    positional_started: slot.positional_started,
                })
                .collect(),
            free: self.free.clone(),
            active: self.active.clone(),
        }
    }
    /// Consume a positional flat burst into source-configured birth descriptors
    /// and reference-resolved matrices. This creates no mission actor identities,
    /// does not tick particles and does not acknowledge collision/audio services.
    /// Resource or profile failures preserve the context and random state.
    pub fn consume_positional_burst(
        &mut self,
        id: Id,
        resources: &crate::scene_resources::SceneResources,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<crate::effect_burst::Birth, String> {
        let context = self
            .get(id)
            .ok_or("positional burst context absent, foreign or retired")?;
        if context.flags() != 0 {
            return Err("positional burst requires acquired flags0 context".into());
        }
        let position = context
            .position()
            .ok_or("positional burst has no position")?;
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let plan = crate::effect_burst::Plan::from_records(&context.source, &program.records, 2)
            .map_err(str::to_owned)?;
        let mut staged = *random;
        let spawns = plan.create_active(&mut staged.0).map_err(str::to_owned)?;
        if spawns
            .iter()
            .any(|s| !s.child_services.is_empty() || !s.sounds.is_empty())
        {
            return Err("positional burst has unbound child or sound services".into());
        }
        let references =
            crate::effect_burst::References::load(resources, &context.source, &spawns)?;
        let matrices = references.matrices(&spawns, position)?;
        self.release_prepared(id)?;
        *random = staged;
        Ok(crate::effect_burst::Birth { spawns, matrices })
    }
}
/// Acquired null-object skip profiles: an empty terminator, or one attached
/// block with constant scale and an optional deferred child, or a constant wait
/// followed by one hidden attached block with a fixed sound cue. Checking the
/// complete body is essential: other blocks starting with24 execute RNG even
/// during preparation. Neither names nor source offsets select this profile.
fn is_deferred_program(program: &Program) -> bool {
    let ops: Vec<_> = program.records.iter().map(|r| r.opcode).collect();
    matches!(
        ops.as_slice(),
        [2] | [0x24, 6, 0x1b, 7, 2] | [0x24, 6, 0x1b, 0x27, 7, 2]
    ) || (ops.as_slice() == [0x0b, 0x24, 6, 0x21, 0x2c, 7, 2]
        && program.records[0]
            .bytes
            .get(1..5)
            .and_then(|b| b.try_into().ok())
            .map(f32::from_le_bytes)
            .is_some_and(|wait| wait.is_finite() && wait >= 0.))
}
struct Slot<K> {
    generation: u64,
    context: Option<Context<K>>,
    sound_started: bool,
    positional_started: bool,
}
/// Real deferred context provider for the acquired enemy launch boundary.
/// The remaining services stay required and are delegated to their owner.
pub struct ProjectileBackend<'a, A> {
    pub contexts: &'a mut Pool<crate::resident_states::Id>,
    pub other: &'a mut A,
}
impl<E, A: crate::primary_enemy_fire::Dependencies<E>> crate::primary_enemy_fire::Dependencies<E>
    for ProjectileBackend<'_, A>
{
    fn prepare_effect(
        &mut self,
        session: &mut crate::resident_states::Session,
        request: crate::enemy_projectile::EffectRequest<'_>,
        _random: &mut crate::audio_parameters::Random,
    ) -> Result<crate::enemy_projectile::EffectOutcome, String> {
        let metadata = session
            .enemy_projectile(request.recipient)
            .ok_or("attached projectile effect recipient absent or foreign")?;
        if request.flags != 4 || metadata.phase != crate::enemy_projectile::Phase::Services {
            return Err(
                "attached projectile context requires actual launch service phase/flags4".into(),
            );
        }
        let recipient = session
            .get(request.recipient)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("attached effect recipient fields absent")?;
        Ok(
            match self.contexts.prepare(
                request.name,
                request.recipient,
                recipient.object_flags,
                request.tag,
            )? {
                Creation::Absent => crate::enemy_projectile::EffectOutcome::Rejected,
                Creation::Prepared(_) => crate::enemy_projectile::EffectOutcome::Prepared,
            },
        )
    }
    fn positional_sound(
        &mut self,
        session: &mut crate::resident_states::Session,
        cue: u32,
        position: [f32; 3],
    ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
        self.other.positional_sound(session, cue, position)
    }
    fn owner_sound(
        &mut self,
        session: &mut crate::resident_states::Session,
        source: crate::resident_states::Id,
        state: &crate::ripper_primary_turn::State<crate::resident_states::Id, E>,
        cue: u32,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
        self.other.owner_sound(session, source, state, cue, random)
    }
    fn remove_effect(
        &mut self,
        session: &mut crate::resident_states::Session,
        source: crate::resident_states::Id,
        effect: E,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(), String> {
        self.other.remove_effect(session, source, effect, random)
    }
}
pub struct Pool<K> {
    ticket: u64,
    image: Option<Arc<Image>>,
    slots: Vec<Slot<K>>,
    free: Vec<usize>,
    active: Vec<Id>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Creation {
    Absent,
    Prepared(Id),
}
impl<K: Copy + Eq> Pool<K> {
    pub fn new(ticket: u64, image: Option<Arc<Image>>) -> Self {
        Self {
            ticket,
            image,
            slots: vec![],
            free: vec![],
            active: vec![],
        }
    }
    pub fn from_active_resources(
        ticket: u64,
        resources: &crate::scene_resources::SceneResources,
    ) -> Self {
        Self::new(
            ticket,
            resources
                .classes
                .models
                .effects
                .last()
                .map(|e| Arc::new(e.image.clone())),
        )
    }
    pub fn active(&self) -> &[Id] {
        &self.active
    }
    pub(crate) fn uses_image(&self, image: &Image) -> bool {
        self.image
            .as_ref()
            .is_some_and(|active| active.same_image(image))
    }
    pub fn get(&self, id: Id) -> Option<&Context<K>> {
        if id.ticket != self.ticket {
            return None;
        }
        let slot = self.slots.get(id.slot)?;
        (slot.generation == id.generation)
            .then_some(slot.context.as_ref())
            .flatten()
    }
    pub fn prepare(
        &mut self,
        name: &str,
        recipient: K,
        recipient_flags: u32,
        tag: u32,
    ) -> Result<Creation, String> {
        if name.contains('\0') {
            return Err("effect name contains an embedded terminator".into());
        }
        let Some(image) = &self.image else {
            return Ok(Creation::Absent);
        };
        // The general quality/distance producer is not assumed. Admit a named
        // request only when both acquired selection outcomes identify the same
        // first symbol, including the real absence outcome.
        let normal = image
            .adaptive_find(name, false)
            .map_err(|e| e.to_string())?;
        let lite = image.adaptive_find(name, true).map_err(|e| e.to_string())?;
        if normal.map(|(_, e)| e.offset) != lite.map(|(_, e)| e.offset) {
            return Err("effect selection requires the actual quality/distance producer".into());
        }
        let Some((entry_index, entry)) = normal else {
            return Ok(Creation::Absent);
        };
        if recipient_flags & 0x400 != 0 {
            return Err(
                "attached context requires inherited parameter producer for recipient flag400"
                    .into(),
            );
        }
        let tail = image
            .program_tail(entry.offset)
            .map_err(|e| e.to_string())?;
        let parsed = giants_formats::effect_program::parse(tail).map_err(|e| e.to_string())?;
        if !is_deferred_program(&parsed) {
            return Err("effect initial skip pass outside acquired deferred profile".into());
        }
        let program = Ok(Arc::new(parsed));
        let mut label = [0; 16];
        let count = name.len().min(label.len());
        label[..count].copy_from_slice(&name.as_bytes()[..count]);
        let context = Context {
            binding: Binding::Attached { recipient, tag },
            label,
            entry_index,
            entry_offset: entry.offset,
            program,
            source: image.clone(),
            flags: 4,
        };
        self.insert(context)
    }
    /// Exact-D flags14 direct source-offset request. The canonical label comes
    /// from the active image; quality/distance and adaptive names are bypassed.
    /// This prepares a context, not an active object or completed FX update.
    pub fn prepare_offset(
        &mut self,
        offset: Option<u32>,
        recipient: K,
        recipient_flags: u32,
        tag: u32,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(Creation, Vec<crate::effect_initialization::Interval>), String> {
        let Some(offset) = offset else {
            return Ok((Creation::Absent, vec![]));
        };
        let image = self
            .image
            .as_ref()
            .ok_or("direct offset requires an active effect image")?;
        if recipient_flags & 0x400 != 0 {
            return Err(
                "direct context requires inherited parameter producer for recipient flag400".into(),
            );
        }
        let mut entries = image
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.offset == offset);
        let (entry_index, entry) = entries
            .next()
            .ok_or("direct effect offset is not a registered source entry")?;
        if entries.next().is_some() {
            return Err("direct effect canonical label is ambiguous".into());
        }
        let program = Arc::new(image.program(offset).map_err(|e| e.to_string())?);
        let mut staged = *random;
        let intervals = crate::effect_initialization::skip(&program, &mut staged)?;
        let mut label = [0; 16];
        let count = entry.name.len().min(label.len());
        label[..count].copy_from_slice(&entry.name.as_bytes()[..count]);
        let creation = self.insert(Context {
            binding: Binding::Attached { recipient, tag },
            label,
            entry_index,
            entry_offset: offset,
            program: Ok(program),
            source: image.clone(),
            flags: 4,
        })?;
        *random = staged;
        Ok((creation, intervals))
    }
    /// Exact-D direct-offset flags18 positional preparation. No named/adaptive
    /// lookup, recipient or active interpreter update is supplied implicitly.
    pub fn prepare_positional_offset(
        &mut self,
        offset: Option<u32>,
        position: [f32; 3],
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(Creation, Vec<crate::effect_initialization::Interval>), String> {
        self.prepare_direct_position(offset, position, 0, false, random)
    }
    /// Acquired retirement-helper direct-offset mode38. The caller supplies the
    /// parent's current object/render fields, not a projected class template.
    /// Quality/load/camera producers are unnecessary only when both source
    /// selections resolve to this same offset; differing selections refuse.
    /// This prepares the child context without marking or freeing its parent.
    pub fn prepare_retirement_offset_invariant(
        &mut self,
        offset: Option<u32>,
        position: [f32; 3],
        object_flags: u32,
        render_flags: u32,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(Creation, Vec<crate::effect_initialization::Interval>), String> {
        let flags = if object_flags & 0x400 != 0 {
            0x10000
        } else {
            0
        } | if render_flags & 0x400 != 0 { 0x4000 } else { 0 }
            | if render_flags & 0x1000 != 0 {
                0x8000
            } else {
                0
            };
        self.prepare_direct_position(offset, position, flags, true, random)
    }
    fn prepare_direct_position(
        &mut self,
        offset: Option<u32>,
        position: [f32; 3],
        flags: u32,
        require_invariant: bool,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(Creation, Vec<crate::effect_initialization::Interval>), String> {
        let Some(offset) = offset else {
            return Ok((Creation::Absent, vec![]));
        };
        if !position.into_iter().all(f32::is_finite) {
            return Err("direct positional effect position is nonfinite".into());
        }
        let image = self
            .image
            .as_ref()
            .ok_or("direct positional effect image absent")?;
        let mut entries = image
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.offset == offset);
        let (entry_index, entry) = entries
            .next()
            .ok_or("direct positional offset is not a source entry")?;
        if entries.next().is_some() {
            return Err("direct positional effect label ambiguous".into());
        }
        if require_invariant {
            let selected = image
                .adaptive_find(&entry.name, true)
                .map_err(|e| e.to_string())?;
            if selected.map(|(_, e)| e.offset) != Some(offset) {
                return Err(
                    "retirement selection requires actual quality/load/camera inputs".into(),
                );
            }
        }
        let program = Arc::new(image.program(offset).map_err(|e| e.to_string())?);
        let mut staged = *random;
        let intervals = crate::effect_initialization::skip(&program, &mut staged)?;
        let mut label = [0; 16];
        let count = entry.name.len().min(label.len());
        label[..count].copy_from_slice(&entry.name.as_bytes()[..count]);
        let creation = self.insert(Context {
            binding: Binding::Position(position),
            label,
            entry_index,
            entry_offset: offset,
            program: Ok(program),
            source: image.clone(),
            flags,
        })?;
        *random = staged;
        Ok((creation, intervals))
    }
    /// Exact-D named flags24/parameter8 profile. Selection uses actual supplied
    /// current load/camera inputs; the full null-object pass executes its ranges.
    /// Activation is a distinct, still-required interpreter profile.
    pub fn prepare_extended(
        &mut self,
        name: &str,
        recipient: K,
        recipient_flags: u32,
        tag: u32,
        selection: crate::effect_initialization::Selection,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(Creation, Vec<crate::effect_initialization::Interval>), String> {
        if name.contains('\0') {
            return Err("effect name contains an embedded terminator".into());
        }
        let Some(image) = self.image.clone() else {
            return Ok((Creation::Absent, vec![]));
        };
        let lite = selection.prefer_lite()?;
        let Some((entry_index, entry)) =
            image.adaptive_find(name, lite).map_err(|e| e.to_string())?
        else {
            return Ok((Creation::Absent, vec![]));
        };
        if recipient_flags & 0x400 != 0 {
            return Err(
                "extended context requires acquired recipient parameter inheritance".into(),
            );
        }
        self.prepare_extended_entry(
            name,
            recipient,
            tag,
            (entry_index, entry.offset),
            image,
            random,
        )
    }
    /// Resolve only when normal and lite lookup identify the same actual entry.
    /// No synthetic load, quality or camera inputs stand in for their producer.
    pub fn prepare_extended_invariant(
        &mut self,
        name: &str,
        recipient: K,
        recipient_flags: u32,
        tag: u32,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(Creation, Vec<crate::effect_initialization::Interval>), String> {
        if name.contains('\0') {
            return Err("effect name contains an embedded terminator".into());
        }
        let Some(image) = self.image.clone() else {
            return Ok((Creation::Absent, vec![]));
        };
        let normal = image
            .adaptive_find(name, false)
            .map_err(|e| e.to_string())?;
        let lite = image.adaptive_find(name, true).map_err(|e| e.to_string())?;
        if normal.map(|(_, e)| e.offset) != lite.map(|(_, e)| e.offset) {
            return Err("effect selection requires the actual quality/distance producer".into());
        }
        let Some((entry_index, entry)) = normal else {
            return Ok((Creation::Absent, vec![]));
        };
        if recipient_flags & 0x400 != 0 {
            return Err(
                "extended context requires acquired recipient parameter inheritance".into(),
            );
        }
        let entry_offset = entry.offset;
        self.prepare_extended_entry(
            name,
            recipient,
            tag,
            (entry_index, entry_offset),
            image,
            random,
        )
    }
    fn prepare_extended_entry(
        &mut self,
        name: &str,
        recipient: K,
        tag: u32,
        entry: (usize, u32),
        image: Arc<Image>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(Creation, Vec<crate::effect_initialization::Interval>), String> {
        let (entry_index, entry_offset) = entry;
        let program = Arc::new(image.program(entry_offset).map_err(|e| e.to_string())?);
        let mut staged = *random;
        let intervals = crate::effect_initialization::skip(&program, &mut staged)?;
        let mut label = [0; 16];
        let count = name.len().min(label.len());
        label[..count].copy_from_slice(&name.as_bytes()[..count]);
        let creation = self.insert(Context {
            binding: Binding::Attached { recipient, tag },
            label,
            entry_index,
            entry_offset,
            program: Ok(program),
            source: image,
            flags: 0x20004,
        })?;
        *random = staged;
        Ok((creation, intervals))
    }
    /// Exact-D named flags8 position profile. No synthetic recipient is created.
    /// All fallible framing and skip preparation precede publication/RNG commit.
    pub fn prepare_positional(
        &mut self,
        name: &str,
        position: [f32; 3],
        mut selection: crate::effect_initialization::Selection,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(Creation, Vec<crate::effect_initialization::Interval>), String> {
        if name.contains('\0') {
            return Err("effect name contains an embedded terminator".into());
        }
        let Some(image) = self.image.clone() else {
            return Ok((Creation::Absent, vec![]));
        };
        selection.recipient_position = position;
        let lite = selection.prefer_lite()?;
        let Some((entry_index, entry)) =
            image.adaptive_find(name, lite).map_err(|e| e.to_string())?
        else {
            return Ok((Creation::Absent, vec![]));
        };
        let entry_offset = entry.offset;
        self.prepare_positional_entry(name, position, (entry_index, entry_offset), image, random)
    }
    /// Resolve the positional request only when both acquired quality/distance
    /// outcomes identify the same entry. No camera/load values are invented.
    pub fn prepare_positional_invariant(
        &mut self,
        name: &str,
        position: [f32; 3],
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(Creation, Vec<crate::effect_initialization::Interval>), String> {
        if name.contains('\0') || !position.into_iter().all(f32::is_finite) {
            return Err("positional effect requires finite position and valid name".into());
        }
        let Some(image) = self.image.clone() else {
            return Ok((Creation::Absent, vec![]));
        };
        let normal = image
            .adaptive_find(name, false)
            .map_err(|e| e.to_string())?;
        let lite = image.adaptive_find(name, true).map_err(|e| e.to_string())?;
        if normal.map(|(_, e)| e.offset) != lite.map(|(_, e)| e.offset) {
            return Err("effect selection requires the actual quality/distance producer".into());
        }
        let Some((index, entry)) = normal else {
            return Ok((Creation::Absent, vec![]));
        };
        let offset = entry.offset;
        self.prepare_positional_entry(name, position, (index, offset), image, random)
    }
    fn prepare_positional_entry(
        &mut self,
        name: &str,
        position: [f32; 3],
        entry: (usize, u32),
        image: Arc<Image>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<(Creation, Vec<crate::effect_initialization::Interval>), String> {
        let (entry_index, entry_offset) = entry;
        let program = Arc::new(image.program(entry_offset).map_err(|e| e.to_string())?);
        let mut staged = *random;
        let intervals = crate::effect_initialization::skip(&program, &mut staged)?;
        let mut label = [0; 16];
        let count = name.len().min(label.len());
        label[..count].copy_from_slice(&name.as_bytes()[..count]);
        let creation = self.insert(Context {
            binding: Binding::Position(position),
            label,
            entry_index,
            entry_offset,
            program: Ok(program),
            source: image,
            flags: 0,
        })?;
        *random = staged;
        Ok((creation, intervals))
    }
    fn insert(&mut self, context: Context<K>) -> Result<Creation, String> {
        let slot = self.free.last().copied().unwrap_or(self.slots.len());
        let generation = if let Some(previous) = self.slots.get(slot) {
            previous
                .generation
                .checked_add(1)
                .ok_or("effect generation exhausted")?
        } else {
            1
        };
        if slot == self.slots.len() {
            self.slots.push(Slot {
                generation,
                context: Some(context),
                sound_started: false,
                positional_started: false,
            });
        } else {
            self.free.pop();
            self.slots[slot] = Slot {
                generation,
                context: Some(context),
                sound_started: false,
                positional_started: false,
            };
        }
        let id = Id {
            ticket: self.ticket,
            slot,
            generation,
        };
        self.active.push(id);
        Ok(Creation::Prepared(id))
    }
    /// Only the unadvanced preparation profile is represented by this pool.
    /// Releasing it cannot stand in for cleanup of emitted effect actors.
    pub fn release_prepared(&mut self, id: Id) -> Result<Context<K>, String> {
        self.get(id)
            .ok_or("prepared effect absent, foreign or stale")?;
        if self.slots[id.slot].sound_started || self.slots[id.slot].positional_started {
            return Err("effect activation began; context requires active cleanup".into());
        }
        self.retire_context(id)
    }
    pub(crate) fn claim_positional_impact(&mut self, id: Id) -> Result<(), String> {
        self.get(id)
            .ok_or("positional context absent, foreign or stale")?;
        let slot = &mut self.slots[id.slot];
        if slot.sound_started || slot.positional_started {
            return Err("positional context activation already started".into());
        }
        slot.positional_started = true;
        Ok(())
    }
    /// Only a constructor which published no actor can return this reservation.
    pub(crate) fn undo_positional_claim(&mut self, id: Id) {
        if self.get(id).is_some() {
            self.slots[id.slot].positional_started = false;
        }
    }
    pub(crate) fn complete_positional_impact(&mut self, id: Id) -> Result<Context<K>, String> {
        self.get(id)
            .ok_or("positional context absent, foreign or stale at completion")?;
        if !self.slots[id.slot].positional_started || self.slots[id.slot].sound_started {
            return Err("positional context completion ownership changed".into());
        }
        self.retire_context(id)
    }
    fn retire_context(&mut self, id: Id) -> Result<Context<K>, String> {
        let index = self
            .active
            .iter()
            .position(|&a| a == id)
            .ok_or("effect active order inconsistent")?;
        self.active.remove(index);
        let context = self.slots[id.slot].context.take().unwrap();
        self.free.push(id.slot);
        Ok(context)
    }
    /// Exact-D first active update of a complete positional [2b,2] program.
    /// Preparation already consumed its draw. The returned identity is the
    /// next active context, as at 4528f0, rather than a completion boolean.
    /// A service error retains its effects and RNG and quarantines this identity.
    pub fn activate_positional_sound(
        &mut self,
        id: Id,
        random: &mut crate::audio_parameters::Random,
        service: &mut impl crate::positional_effect_audio::Service,
    ) -> Result<Option<Id>, String> {
        let context = self
            .get(id)
            .ok_or("sound context absent, foreign or retired")?;
        if self.slots[id.slot].sound_started {
            return Err("sound activation already began; replay refused".into());
        }
        if context.flags() != 0 {
            return Err("positional sound requires acquired flags0".into());
        }
        let position = context
            .position()
            .ok_or("sound context is not positional")?;
        let program = context.program.as_ref().map_err(Clone::clone)?;
        if program.records.iter().map(|r| r.opcode).collect::<Vec<_>>() != [0x2b, 2] {
            return Err("active sound requires complete sound-only program".into());
        }
        let recipe =
            crate::positional_effect_audio::Recipe::prepare(program).map_err(|e| e.to_string())?;
        let index = self
            .active
            .iter()
            .position(|&a| a == id)
            .ok_or("sound active order inconsistent")?;
        let next = self.active.get(index + 1).copied();
        self.slots[id.slot].sound_started = true;
        let selected =
            recipe.range[0] + ((random.next_15() * (recipe.range[1] - recipe.range[0] + 1)) >> 15);
        service.request(selected, position, random)?;
        self.retire_context(id)?;
        Ok(next)
    }
    /// Acquired ordinary active-list traversal for sound-only contexts.
    /// Successful entries stay retired if a later entry/service fails. Unknown
    /// programs are refused at their actual place in the list, without replaying
    /// preceding requests or staging the shared random state.
    pub fn advance_positional_sounds(
        &mut self,
        random: &mut crate::audio_parameters::Random,
        service: &mut impl crate::positional_effect_audio::Service,
    ) -> Result<usize, String> {
        let mut next = self.active.first().copied();
        let mut retired = 0;
        while let Some(id) = next {
            next = self.activate_positional_sound(id, random, service)?;
            retired += 1;
        }
        Ok(retired)
    }
    pub fn release_recipient_prepared(&mut self, recipient: K) -> Result<usize, String> {
        let ids = self
            .active
            .iter()
            .copied()
            .filter(|&id| {
                self.get(id)
                    .is_some_and(|c| c.recipient() == Some(recipient))
            })
            .collect::<Vec<_>>();
        for id in &ids {
            self.release_prepared(*id)?;
        }
        Ok(ids.len())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn timed_image(distinct_lite: bool) -> Arc<Image> {
        let offset = if distinct_lite { 17u32 } else { 6 };
        let mut payload = b"T\0".to_vec();
        payload.extend(offset.to_le_bytes());
        let mut body = vec![0x24];
        for word in [0u32, 42, 0] {
            body.extend(word.to_le_bytes());
        }
        body.extend([6, 0x1c]);
        for value in [2f32, 5.] {
            body.extend(value.to_le_bytes());
        }
        body.push(4);
        for value in [1f32, 0.1, 0.9] {
            body.extend(value.to_le_bytes());
        }
        body.extend(b"\x0dChild\0\x07\x02");
        if distinct_lite {
            payload.extend(b"T_lite\0");
            payload.extend((offset + body.len() as u32).to_le_bytes());
        }
        payload.extend(body);
        if distinct_lite {
            payload.push(2);
        }
        let mut raw = vec![];
        for word in [1, payload.len() as u32, if distinct_lite { 2 } else { 1 }] {
            raw.extend(word.to_le_bytes());
        }
        raw.extend(payload);
        Arc::new(giants_formats::effect_image::parse(&raw).unwrap())
    }
    #[test]
    fn cinematic_timed_preparation_keeps_one_draw_and_the_actual_mark() {
        let mut pool = Pool::<u32>::new(7, Some(timed_image(false)));
        let mut random = crate::audio_parameters::Random(1729);
        let mut expected = random;
        expected.next_15();
        let (Creation::Prepared(id), intervals) = pool
            .prepare_cinematic_timed("T", 31, 0, 9, &mut random)
            .unwrap()
        else {
            panic!("absent");
        };
        let c = pool.get(id).unwrap();
        assert_eq!(c.flags(), 0x8004);
        assert_eq!(c.recipient(), Some(31));
        assert_eq!(c.tag(), Some(9));
        assert_eq!(random.0, expected.0);
        assert_eq!(intervals.len(), 1);
        pool.release_prepared(id).unwrap();
        let before = random.0;
        assert!(
            pool.prepare_cinematic_timed("T", 31, 0x400, 9, &mut random)
                .is_err()
        );
        assert_eq!(random.0, before);
        assert!(pool.active().is_empty());
    }
    #[test]
    fn cinematic_timed_preparation_refuses_unproven_selection_before_consumption() {
        let mut pool = Pool::<u32>::new(7, Some(timed_image(true)));
        let mut random = crate::audio_parameters::Random(1729);
        assert!(
            pool.prepare_cinematic_timed("T", 31, 0, 9, &mut random)
                .is_err()
        );
        assert_eq!(random.0, 1729);
        assert!(pool.active().is_empty());
        let mut pool = Pool::<u32>::new(7, Some(sound_image()));
        assert!(
            pool.prepare_cinematic_timed("S", 31, 0, 9, &mut random)
                .is_err()
        );
        assert_eq!(random.0, 1729);
        assert!(pool.active().is_empty());
    }
    #[test]
    fn retirement_context_preserves_parent_flags_position_and_reused_ownership() {
        let mut pool = Pool::<u32>::new(9, Some(sound_image()));
        let mut random = crate::audio_parameters::Random(1729);
        let expected = [
            0, 0x10000, 0x4000, 0x14000, 0x8000, 0x18000, 0xc000, 0x1c000,
        ];
        let mut previous: Option<Id> = None;
        for (bits, flags) in expected.into_iter().enumerate() {
            let object = if bits & 1 != 0 { 0x400 } else { 0 };
            let response =
                if bits & 2 != 0 { 0x400 } else { 0 } | if bits & 4 != 0 { 0x1000 } else { 0 };
            let position = [bits as f32, -7., 23.];
            let mut plain = Pool::<u32>::new(10, Some(sound_image()));
            let mut plain_random = random;
            plain
                .prepare_positional_offset(Some(6), position, &mut plain_random)
                .unwrap();
            let (Creation::Prepared(id), _) = pool
                .prepare_retirement_offset_invariant(
                    Some(6),
                    position,
                    object,
                    response,
                    &mut random,
                )
                .unwrap()
            else {
                panic!("retirement context absent")
            };
            let context = pool.get(id).unwrap();
            assert_eq!(context.flags(), flags);
            assert_eq!(context.position(), Some(position));
            assert_eq!(context.recipient(), None);
            assert_eq!(random.0, plain_random.0);
            assert_eq!(pool.active(), &[id]);
            assert!(plain.get(id).is_none());
            if let Some(stale) = previous {
                assert_eq!(id.slot(), stale.slot());
                assert_ne!(id, stale);
                assert!(pool.get(stale).is_none());
            }
            pool.release_prepared(id).unwrap();
            assert!(pool.active().is_empty());
            previous = Some(id);
        }
    }
    #[test]
    fn retirement_context_refuses_distinct_lite_selection_without_rng_or_allocation() {
        let mut payload = b"S\0".to_vec();
        payload.extend(17u32.to_le_bytes());
        payload.extend(b"S_lite\0");
        payload.extend(27u32.to_le_bytes());
        payload.push(0x2b);
        payload.extend(40u32.to_le_bytes());
        payload.extend(41u32.to_le_bytes());
        payload.extend([2, 2]);
        let mut raw = vec![];
        for word in [1, payload.len() as u32, 2] {
            raw.extend(word.to_le_bytes());
        }
        raw.extend(payload);
        let image = Arc::new(giants_formats::effect_image::parse(&raw).unwrap());
        let mut pool = Pool::<u32>::new(9, Some(image));
        let mut random = crate::audio_parameters::Random(1729);
        assert!(
            pool.prepare_retirement_offset_invariant(Some(17), [0.; 3], 0, 0x1000, &mut random)
                .is_err()
        );
        assert_eq!(random.0, 1729);
        assert!(pool.active().is_empty());
        assert!(
            pool.prepare_retirement_offset_invariant(Some(17), [f32::NAN; 3], 0, 0, &mut random)
                .is_err()
        );
        assert_eq!(random.0, 1729);
        let (absent, _) = pool
            .prepare_retirement_offset_invariant(None, [0.; 3], 0, 0, &mut random)
            .unwrap();
        assert_eq!(absent, Creation::Absent);
        assert!(pool.active().is_empty());
    }
    #[test]
    fn invariant_extended_lookup_refuses_distinct_entries_before_allocation_or_rng() {
        let mut payload = b"A\0".to_vec();
        payload.extend(17u32.to_le_bytes());
        payload.extend(b"A_lite\0");
        payload.extend(18u32.to_le_bytes());
        payload.extend([2, 2]);
        let mut bytes = vec![];
        for word in [1, payload.len() as u32, 2] {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend(payload);
        let image = Arc::new(giants_formats::effect_image::parse(&bytes).unwrap());
        let mut pool = Pool::new(12, Some(image));
        let mut random = crate::audio_parameters::Random(123);
        assert!(
            pool.prepare_extended_invariant("A", 1u32, 0, u32::MAX, &mut random)
                .is_err()
        );
        assert!(pool.active().is_empty());
        assert_eq!(random.0, 123);
    }
    #[test]
    fn cinematic_mark_survives_preparation_and_retirement_invalidates_reused_identity() {
        let mut pool = Pool::new(7, Some(image()));
        let Creation::Prepared(first) = pool.prepare_cinematic("A", 12).unwrap() else {
            panic!("prepared")
        };
        let context = pool.get(first).unwrap();
        assert_eq!(context.flags(), 0x8004);
        assert_eq!(context.tag(), Some(u32::MAX));
        assert_eq!(context.recipient(), Some(12));
        assert_eq!(context.cursor(), 0);
        assert_eq!(context.emitted_objects(), 0);
        assert!(Pool::<u32>::new(8, Some(image())).get(first).is_none());
        pool.release_prepared(first).unwrap();
        let Creation::Prepared(second) = pool.prepare_cinematic("A", 13).unwrap() else {
            panic!("prepared")
        };
        assert_eq!(first.slot(), second.slot());
        assert_ne!(first, second);
        assert!(pool.get(first).is_none());
        assert!(pool.release_prepared(first).is_err());
        assert_eq!(pool.get(second).unwrap().recipient(), Some(13));
    }
    #[test]
    fn active_sound_preserves_failed_service_effects_and_refuses_replay() {
        struct Failure(usize);
        impl crate::positional_effect_audio::Service for Failure {
            fn request(
                &mut self,
                _: u32,
                _: [f32; 3],
                random: &mut crate::audio_parameters::Random,
            ) -> Result<(), String> {
                self.0 += 1;
                random.next_15();
                Err("device failed after side effect".into())
            }
        }
        let mut pool = Pool::<u32>::new(1, Some(sound_image()));
        let mut random = crate::audio_parameters::Random(7);
        let (Creation::Prepared(id), _) = pool
            .prepare_positional_offset(Some(6), [1., 2., 3.], &mut random)
            .unwrap()
        else {
            panic!()
        };
        let mut expected = random;
        expected.next_15();
        expected.next_15();
        let mut service = Failure(0);
        assert!(
            pool.activate_positional_sound(id, &mut random, &mut service)
                .is_err()
        );
        assert_eq!(random.0, expected.0);
        assert_eq!(service.0, 1);
        assert!(pool.get(id).is_some());
        assert!(pool.release_prepared(id).is_err());
        assert!(
            pool.activate_positional_sound(id, &mut random, &mut service)
                .is_err()
        );
        assert_eq!(random.0, expected.0);
        assert_eq!(service.0, 1);
        let mut copied = pool.staged_copy();
        assert!(
            copied
                .activate_positional_sound(id, &mut random, &mut service)
                .is_err()
        );
    }
    fn sound_image() -> Arc<Image> {
        let mut payload = b"S\0".to_vec();
        payload.extend(6u32.to_le_bytes());
        payload.push(0x2b);
        payload.extend(40u32.to_le_bytes());
        payload.extend(41u32.to_le_bytes());
        payload.push(2);
        let mut raw = vec![];
        for word in [1, payload.len() as u32, 1] {
            raw.extend(word.to_le_bytes());
        }
        raw.extend(payload);
        Arc::new(giants_formats::effect_image::parse(&raw).unwrap())
    }
    #[test]
    fn sound_queue_keeps_successful_prefix_and_stops_at_failed_service() {
        struct Audio(usize);
        impl crate::positional_effect_audio::Service for Audio {
            fn request(
                &mut self,
                _: u32,
                _: [f32; 3],
                random: &mut crate::audio_parameters::Random,
            ) -> Result<(), String> {
                self.0 += 1;
                random.next_15();
                if self.0 == 2 {
                    Err("late audio failure".into())
                } else {
                    Ok(())
                }
            }
        }
        let mut pool = Pool::<u32>::new(77, Some(sound_image()));
        let mut random = crate::audio_parameters::Random(1729);
        let mut ids = vec![];
        for i in 0..3 {
            let (Creation::Prepared(id), _) = pool
                .prepare_positional_offset(Some(6), [i as f32, 0., 0.], &mut random)
                .unwrap()
            else {
                panic!()
            };
            ids.push(id);
        }
        let mut expected = random;
        for _ in 0..4 {
            expected.next_15();
        }
        let mut audio = Audio(0);
        assert!(
            pool.advance_positional_sounds(&mut random, &mut audio)
                .is_err()
        );
        assert_eq!(random.0, expected.0);
        assert_eq!(audio.0, 2);
        assert!(pool.get(ids[0]).is_none());
        assert_eq!(pool.active(), &ids[1..]);
        assert!(
            pool.advance_positional_sounds(&mut random, &mut audio)
                .is_err()
        );
        assert_eq!(random.0, expected.0);
        assert_eq!(audio.0, 2);
        assert_eq!(pool.active(), &ids[1..]);
    }
    fn image() -> Arc<Image> {
        let mut payload = b"A\0".to_vec();
        payload.extend(6u32.to_le_bytes());
        payload.push(0x24);
        for word in [884_u32, 1113, 0] {
            payload.extend(word.to_le_bytes());
        }
        payload.extend([6, 0x1b]);
        payload.extend(0.5f32.to_bits().to_le_bytes());
        payload.extend([7, 2]);
        let mut raw = vec![];
        for word in [1, payload.len() as u32, 1] {
            raw.extend(word.to_le_bytes());
        }
        raw.extend(payload);
        Arc::new(giants_formats::effect_image::parse(&raw).unwrap())
    }
    #[test]
    fn positional_birth_claim_survives_staging_and_forbids_prepared_cleanup_or_replay() {
        let mut pool = Pool::<u32>::new(937, Some(image()));
        let (Creation::Prepared(id), _) = pool
            .prepare_positional(
                "A",
                [1., 2., 3.],
                crate::effect_initialization::Selection {
                    active_objects: 0,
                    quality: 1.,
                    lite_distance: 500.,
                    camera: [0.; 3],
                    recipient_position: [0.; 3],
                },
                &mut crate::audio_parameters::Random(1),
            )
            .unwrap()
        else {
            panic!()
        };
        pool.claim_positional_impact(id).unwrap();
        assert!(pool.claim_positional_impact(id).is_err());
        assert!(pool.release_prepared(id).is_err());
        let mut staged = pool.staged_copy();
        assert!(staged.claim_positional_impact(id).is_err());
        assert!(staged.release_prepared(id).is_err());
        pool.complete_positional_impact(id).unwrap();
        assert!(pool.get(id).is_none());
        assert!(pool.complete_positional_impact(id).is_err());
        staged.undo_positional_claim(id);
        assert!(staged.release_prepared(id).is_ok());
    }
    #[test]
    fn positional_context_is_not_retired_with_an_unrelated_or_zero_recipient() {
        let mut pool = Pool::new(9, Some(image()));
        let selection = crate::effect_initialization::Selection {
            active_objects: 0,
            quality: 1.,
            lite_distance: 500.,
            camera: [0.; 3],
            recipient_position: [999.; 3],
        };
        let mut random = crate::audio_parameters::Random(1729);
        let (Creation::Prepared(positional), _) = pool
            .prepare_positional("A", [1., 2., 3.], selection, &mut random)
            .unwrap()
        else {
            panic!()
        };
        let Creation::Prepared(attached) = pool.prepare("A", 7, 0, u32::MAX).unwrap() else {
            panic!()
        };
        assert_eq!(pool.release_recipient_prepared(0).unwrap(), 0);
        assert_eq!(pool.release_recipient_prepared(7).unwrap(), 1);
        assert!(pool.get(attached).is_none());
        let context = pool.get(positional).unwrap();
        assert_eq!(context.recipient(), None);
        assert_eq!(context.tag(), None);
        assert_eq!(context.position(), Some([1., 2., 3.]));
        assert_eq!(pool.active(), [positional]);
        assert_eq!(random.0, 1729);
        pool.release_prepared(positional).unwrap();
        assert!(pool.active().is_empty());
    }
    #[test]
    fn ownership_release_and_slot_reuse_preserve_peer_and_reject_old_identity() {
        let mut pool = Pool::new(3, Some(image()));
        let Creation::Prepared(a) = pool.prepare("A", 1, 0, u32::MAX).unwrap() else {
            panic!()
        };
        let Creation::Prepared(b) = pool.prepare("a", 2, 0, 4).unwrap() else {
            panic!()
        };
        assert!(Pool::<u32>::new(4, Some(image())).get(a).is_none());
        assert_eq!(pool.release_recipient_prepared(1).unwrap(), 1);
        assert!(pool.get(a).is_none());
        assert!(pool.get(b).is_some());
        let Creation::Prepared(c) = pool.prepare("A", 2, 0, 7).unwrap() else {
            panic!()
        };
        assert_eq!(a.slot, c.slot);
        assert_ne!(a, c);
        assert_eq!(pool.active(), [b, c]);
        assert!(pool.release_prepared(a).is_err());
        assert_eq!(pool.get(c).unwrap().emitted_objects(), 0);
        assert!(pool.get(c).unwrap().require_first_update().is_err());
        assert_eq!(pool.release_recipient_prepared(2).unwrap(), 2);
        assert!(pool.active().is_empty());
    }
    #[test]
    fn first_attached_opcode_cannot_hide_random_initialization() {
        // A framed random-range command in the block requires the real random
        // producer even though the attached actor itself is deferred.
        let mut body = image().program_tail(6).unwrap().to_vec();
        let mut range = vec![0x28];
        range.extend(b"range\0");
        for word in [0f32.to_bits(), 1f32.to_bits(), 0, 0] {
            range.extend(word.to_le_bytes());
        }
        body.splice(14..19, range);
        let mut payload = b"A\0".to_vec();
        payload.extend(6u32.to_le_bytes());
        payload.extend(body);
        let mut raw = vec![];
        for word in [1, payload.len() as u32, 1] {
            raw.extend(word.to_le_bytes());
        }
        raw.extend(payload);
        let source = giants_formats::effect_image::parse(&raw).unwrap();
        let mut pool = Pool::new(3, Some(Arc::new(source)));
        assert!(pool.prepare("A", 1, 0, 0).is_err());
        assert!(pool.active().is_empty());
    }
    #[test]
    fn missing_registry_and_name_remain_absence_but_unacquired_parameters_are_errors() {
        assert_eq!(
            Pool::new(1, None).prepare("A", 1, 0, 0).unwrap(),
            Creation::Absent
        );
        let mut pool = Pool::new(1, Some(image()));
        assert_eq!(pool.prepare("missing", 1, 0, 0).unwrap(), Creation::Absent);
        assert!(pool.prepare("A", 1, 0x400, 0).is_err());
        assert!(pool.active().is_empty());
        assert!(pool.prepare("A\0other", 1, 0, 0).is_err());
        let mut ambiguous = (*image()).clone();
        ambiguous.entries.push(giants_formats::effect_image::Entry {
            name: "A_lite".into(),
            offset: 19,
        });
        assert!(
            Pool::new(2, Some(Arc::new(ambiguous)))
                .prepare("A", 1, 0, 0)
                .is_err()
        );
    }
}
