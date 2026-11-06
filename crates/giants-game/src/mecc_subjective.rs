//! Ordinary Mecc subjective header and interleaved ANM composition.
//! Camera/model admission, camera production and the actual source fire service
//! belong to the enclosing world. Source state is referenced, never duplicated.
use crate::{
    actor_affine::Cache,
    actor_fields::Fields,
    actor_references::Owned,
    animation_channels::{self as channels, Channels},
    animation_epoch::Stamp,
    arithmetic::Precision,
    first_person,
    specification::Prepared,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Prepared,
    Primary,
    PrimaryComplete,
    Selected,
    Placed,
    FirstAnimation,
    Fire,
    Finished,
    Failed,
}

#[derive(Clone, Debug)]
pub struct Header<K> {
    /// The source owns the shared Mecc payload and authoritative reference store.
    pub source: K,
    pub fields: Fields,
    pub motion: [f32; 4],
    pub secondary_flags: u32,
    pub affine: Option<Cache>,
    pub animation_stamp: u32,
}
pub struct Source<'a, K> {
    pub source: K,
    pub fields: &'a Fields,
    /// Current three model rates followed by angular rate, never previous rates.
    pub motion: [f32; 4],
    pub secondary_flags: u32,
    pub affine: Option<Cache>,
    pub animation_stamp: u32,
}
/// One world-shared selector history, supplied by its actual lifecycle owner.
pub struct History {
    pub previous_weapon: i32,
}
pub struct Post<K> {
    pub controlled: K,
    pub camera_owner: Option<K>,
    pub controlled_flags: u32,
    pub icon_index: usize,
    pub weapon: i32,
    pub inventory_present: bool,
    pub previous_camera: first_person::Camera,
    pub current_camera: first_person::Camera,
    pub offset: [f32; 3],
    pub seconds: f32,
    pub blending: channels::Blending,
}
/// Reference publication must update the source's actual shared storage before
/// firing. The callback must execute the actual source fire phase; there is no
/// default implementation or implied audio/target/muzzle provider.
pub trait Services<K> {
    /// Invoked after virtual selection and before placement/ANM. Supplied-camera
    /// controls must explicitly retain that boundary; owned adapters produce it.
    fn cameras(
        &mut self,
        source: K,
        number: u64,
        previous: first_person::Camera,
        current: first_person::Camera,
    ) -> Result<(first_person::Camera, first_person::Camera), String>;
    fn references(
        &mut self,
        source: K,
        context: &mut Owned,
        channels: &Channels,
        epoch: u32,
    ) -> Result<(), String>;
    fn fire(&mut self, source: K, number: u64) -> Result<(), String>;
}
#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub number: u64,
    pub first: channels::Tick,
    pub tail: channels::Tick,
    pub stamp: Option<u32>,
    pub fire_called: bool,
}
/// Camera production remains external. The Session derives current source
/// flags, selector metadata, offsets and object time from owned state/resources.
pub struct WorldPost {
    pub number: u64,
    pub controlled: crate::resident_states::Id,
    pub camera_owner: Option<crate::resident_states::Id>,
    pub previous_camera: first_person::Camera,
    pub current_camera: first_person::Camera,
}
/// Production entry: camera values come from the ordered world camera owner.
pub struct ProducedWorldPost {
    pub number: u64,
    pub controlled: crate::resident_states::Id,
    pub camera_owner: crate::resident_states::Id,
}
pub struct WorldServices<'a, 'r, 't> {
    pub animation: crate::mecc_animation_phase::Services<'a>,
    pub fire: crate::mecc_primary::FireServices<'a, 'r, 't>,
    pub held: &'a crate::mecc_wield_pose::Producer,
    pub globals: &'r giants_formats::global_data::GlobalData,
    pub catalog: &'r giants_formats::weapons::Catalog,
    pub global_seconds: f32,
    pub areas: &'r [crate::object_time::SlowArea],
}
pub struct WorldOutput {
    pub animation: Option<Output>,
    pub fire: Option<crate::mecc_fire_primary::Output>,
    pub camera: Option<crate::mecc_world_camera::Publication<crate::resident_states::Id>>,
}
pub struct State<K> {
    number: u64,
    phase: Phase,
    active: bool,
    channels: Channels,
    header: Option<Header<K>>,
    references: Owned,
    durations: Vec<Option<f32>>,
    stamp: Option<u32>,
    failure: Option<String>,
    owned_preparation: bool,
}
impl<K: Copy + Eq> State<K> {
    /// Initial channels are supplied explicitly. This does not infer native
    /// level reset or replace persistent world history with per-actor defaults.
    pub fn new(specification: Arc<Prepared>, initial: Channels) -> Result<Self, String> {
        if specification.class != 2
            || !specification.needs.is_empty()
            || specification
                .metadata
                .events
                .iter()
                .any(|t| !t.events.is_empty())
        {
            return Err(
                "subjective Mecc requires its prepared ordinary event-free specification".into(),
            );
        }
        let mut durations = vec![None; 256];
        for (&slot, descriptor) in &specification.animations {
            durations[usize::from(slot)] = Some(descriptor.template.duration);
        }
        Ok(Self {
            number: 0,
            phase: Phase::Idle,
            active: false,
            channels: initial,
            header: None,
            references: Owned::new(specification)?,
            durations,
            stamp: None,
            failure: None,
            owned_preparation: false,
        })
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub fn header(&self) -> Option<&Header<K>> {
        self.header.as_ref()
    }
    pub fn channels(&self) -> &Channels {
        &self.channels
    }
    pub fn specification(&self) -> &Arc<Prepared> {
        self.references.specification()
    }
    pub fn display(&self, camera_owner: Option<K>) -> first_person::Display<K> {
        first_person::Display {
            camera_owner,
            active: self.active,
        }
    }
    fn next(&self, number: u64) -> Result<(), String> {
        if self.failure.is_some()
            || self.number.checked_add(1) != Some(number)
            || !matches!(self.phase, Phase::Idle | Phase::Finished)
        {
            return Err(
                "subjective frame is failed, stale or its preceding post phase is unfinished"
                    .into(),
            );
        }
        Ok(())
    }
    /// A known inactive pre-world branch preserves the prior header/channels.
    /// Missing resource preparation is an error, not a reason to call this.
    pub fn inactive(&mut self, number: u64) -> Result<(), String> {
        self.next(number)?;
        self.number = number;
        self.active = false;
        self.owned_preparation = false;
        self.phase = Phase::Finished;
        Ok(())
    }
    pub fn prepare(&mut self, number: u64, input: Source<'_, K>) -> Result<(), String> {
        self.next(number)?;
        if input.fields.class != 2
            || input.fields.configuration[20] != 2
            || !input
                .motion
                .into_iter()
                .chain(input.fields.position)
                .chain(input.fields.angles)
                .chain([input.fields.scale])
                .all(f32::is_finite)
        {
            return Err("subjective pre-copy requires the finite ordinary Mecc source".into());
        }
        let mut fields = input.fields.clone();
        fields.channels = self.channels;
        self.header = Some(Header {
            source: input.source,
            fields,
            motion: input.motion,
            secondary_flags: input.secondary_flags,
            affine: input.affine,
            animation_stamp: input.animation_stamp,
        });
        self.stamp = Some(input.animation_stamp);
        self.number = number;
        self.active = true;
        self.owned_preparation = false;
        self.phase = Phase::Prepared;
        Ok(())
    }
    pub fn post(
        &mut self,
        input: Post<K>,
        history: &mut History,
        stamp: &mut Stamp,
        services: &mut impl Services<K>,
    ) -> Result<Option<Output>, String> {
        if self.failure.is_none() && !self.active && self.phase == Phase::Finished {
            return Ok(None);
        }
        if self.failure.is_some() || !matches!(self.phase, Phase::Prepared | Phase::PrimaryComplete)
        {
            return Err("subjective post requires the current unfailed pre-copy".into());
        }
        if input.camera_owner != Some(input.controlled) {
            self.phase = Phase::Finished;
            return Ok(None);
        }
        if self.header.as_ref().unwrap().source != input.controlled {
            return Err("subjective camera/source binding changed after pre-copy".into());
        }
        if !input.seconds.is_finite() || input.seconds < 0. {
            return Err("subjective post seconds must be finite and nonnegative".into());
        }
        let result = self.run(input, history, stamp, services);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
            self.phase = Phase::Failed;
        }
        result.map(Some)
    }
    fn run(
        &mut self,
        input: Post<K>,
        history: &mut History,
        stamp: &mut Stamp,
        services: &mut impl Services<K>,
    ) -> Result<Output, String> {
        let header = self.header.as_mut().unwrap();
        let selected = first_person::animation(first_person::AnimationInput {
            primary: self.channels[0][0],
            channel_flags: self.channels[0][8],
            controlled_flags: input.controlled_flags,
            secondary_flags: header.secondary_flags,
            // The selector reads current model rates1/2 and angular rate;
            // the ANM playback service consumes all four current rates.
            motion: header.motion[1..].try_into().unwrap(),
            icon_index: input.icon_index,
            weapon: input.weapon,
            previous_weapon: history.previous_weapon,
            inventory_present: input.inventory_present,
        })?;
        history.previous_weapon = selected.previous_weapon;
        if let Some(command) = selected.command {
            if selected.invalidate_primary {
                self.channels[0][0] = u32::MAX;
            }
            if self.channels[0][0] != command.slot {
                let flags = match command.method {
                    first_person::Method::Select => 0x20,
                    first_person::Method::User => 0x21,
                    first_person::Method::Restart => 0x30,
                };
                channels::select_primary(
                    &mut self.channels,
                    channels::Request {
                        slot: u8::try_from(command.slot)
                            .map_err(|_| "subjective clip outside byte profile")?,
                        flags,
                        speed_source: 0,
                        reference_speed: 0.,
                        blend_seconds: f32::from_bits(0x3e2aaaab),
                    },
                    input.blending,
                )?;
            }
        }
        header.fields.channels = self.channels;
        self.phase = Phase::Selected;
        let (previous, current) = services.cameras(
            header.source,
            self.number,
            input.previous_camera,
            input.current_camera,
        )?;
        let pose = first_person::place(previous, current, input.offset)?;
        header.fields.position = pose.position;
        header.fields.angles = pose.angles;
        let (flags, affine) = crate::actor_affine::update(
            crate::uncached_pose::Pose {
                position: pose.position,
                angles: pose.angles,
                scale: header.fields.scale,
            },
            header.fields.object_flags,
            header.affine,
            Precision::Double53,
        )?;
        header.fields.object_flags = flags;
        header.affine = Some(affine);
        self.phase = Phase::Placed;
        let first = self.animate(input.seconds, stamp, services)?;
        self.phase = Phase::FirstAnimation;
        let fire_called = input.controlled_flags & 0x0200_0000 == 0;
        if fire_called {
            services.fire(self.header.as_ref().unwrap().source, self.number)?;
        }
        self.phase = Phase::Fire;
        let tail = self.animate(0., stamp, services)?;
        self.phase = Phase::Finished;
        Ok(Output {
            number: self.number,
            first,
            tail,
            stamp: self.stamp,
            fire_called,
        })
    }
    fn animate(
        &mut self,
        seconds: f32,
        stamp: &mut Stamp,
        services: &mut impl Services<K>,
    ) -> Result<channels::Tick, String> {
        let header = self.header.as_mut().unwrap();
        let tick = channels::advance(
            &mut self.channels,
            &self.durations,
            header.motion,
            seconds,
            Precision::Double53,
        )?;
        header.fields.channels = self.channels;
        if tick.processed != 0 {
            let epoch = stamp.advance();
            self.stamp = Some(epoch);
            header.animation_stamp = epoch;
            services.references(header.source, &mut self.references, &self.channels, epoch)?;
        }
        Ok(tick)
    }
}

impl State<crate::resident_states::Id> {
    pub(crate) fn prepared_for_primary(
        &self,
        session: &crate::resident_states::Session,
        source: crate::resident_states::Id,
        number: u64,
    ) -> Result<(), String> {
        if !self.owned_preparation
            || self.failure.is_some()
            || self.phase != Phase::Prepared
            || !self.active
            || self.number != number
            || self.header.as_ref().is_none_or(|h| h.source != source)
            || !Arc::ptr_eq(
                self.specification(),
                session.mecc_subjective_specification(source)?,
            )
        {
            return Err("subjective primary requires the matching prepared source/frame".into());
        }
        Ok(())
    }
    pub(crate) fn primary_started(&mut self) {
        self.phase = Phase::Primary;
    }
    pub(crate) fn primary_completed(&mut self) {
        self.phase = Phase::PrimaryComplete;
    }
    pub(crate) fn fail(&mut self, error: String) {
        self.failure.get_or_insert(error);
        self.phase = Phase::Failed;
    }
    pub(crate) fn frame_number(&self) -> u64 {
        self.number
    }
    /// Bind the actual state-resource specification. A normalized pointer word
    /// in serialized storage is not evidence that the resource is absent.
    pub fn for_session(
        session: &crate::resident_states::Session,
        source: crate::resident_states::Id,
        initial: Channels,
    ) -> Result<Self, String> {
        Self::new(
            session.mecc_subjective_specification(source)?.clone(),
            initial,
        )
    }

    /// Copy before the source primary. Buttons come from the actual input
    /// snapshot for this frame; the physical view supplies the preceding rates.
    /// Camera/model eligibility remains the enclosing pre-world decision.
    pub fn prepare_from_session(
        &mut self,
        number: u64,
        session: &crate::resident_states::Session,
        source: crate::resident_states::Id,
        buttons: u32,
    ) -> Result<(), String> {
        if !Arc::ptr_eq(
            self.specification(),
            session.mecc_subjective_specification(source)?,
        ) {
            return Err("subjective specification belongs to another prepared source".into());
        }
        let fields = session
            .get(source)
            .and_then(|r| r.construction_fields.as_ref())
            .ok_or("subjective source fields absent")?;
        let view = session.mecc_physical_cycle(source)?;
        // An ordinary pose regenerates its affine. External orientation must
        // retain a genuinely current copied cache instead of an identity.
        let affine = if fields.object_flags & 0x10000 != 0 {
            Some(session.actor_current_affine(source)?)
        } else {
            None
        };
        self.prepare(
            number,
            Source {
                source,
                fields,
                motion: [
                    view.model_rates[0],
                    view.model_rates[1],
                    view.model_rates[2],
                    view.angular_rate,
                ],
                secondary_flags: buttons,
                affine,
                animation_stamp: session.get(source).unwrap().mecc_animation_stamp,
            },
        )?;
        self.owned_preparation = true;
        Ok(())
    }
}
