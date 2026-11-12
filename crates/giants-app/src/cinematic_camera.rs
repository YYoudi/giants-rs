//! Data-driven director selection, event order and audio-clock handoff.
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use giants_game::animation::{Camera as SourceCamera, Evaluator};
#[derive(SystemParam)]
pub struct Timing<'w> {
    time: Res<'w, Time>,
    master: Option<Res<'w, crate::cinematic_audio::Master>>,
    cube: Option<Res<'w, crate::reflection_cube::Build>>,
}
pub struct Segment {
    sequence: String,
    animation: giants_game::animation::Animation,
    timeline: giants_game::flick::Timeline,
    initial_tick: i32,
    preview: bool,
    effects: Vec<giants_formats::flick::EffectCommand>,
    effect_requests: Vec<(i32, giants_formats::flick::EffectCommand)>,
    effect_samples: std::collections::BTreeMap<usize, (f32, u64)>,
    update_generation: u64,
    camera_bindings: std::collections::BTreeMap<
        usize,
        Result<giants_game::director_initial::CameraBinding, String>,
    >,
}
impl Segment {
    /// Source requests already reached by the actual director command cursor.
    /// No effect allocator or renderer success is represented by these records.
    pub fn effect_requests(&self) -> &[(i32, giants_formats::flick::EffectCommand)] {
        &self.effect_requests
    }
    pub fn effect_sample(&self, offset: usize) -> Option<(f32, u64)> {
        self.effect_samples.get(&offset).copied()
    }
    pub(crate) fn expected_effect_offsets(
        &self,
    ) -> Result<std::collections::BTreeSet<usize>, &'static str> {
        self.timeline.last_sampled_tick().map_or_else(
            || Ok(Default::default()),
            |tick| self.timeline.effect_offsets_through(tick),
        )
    }
    fn step(&mut self, tick: i32, seconds: f32) -> Result<giants_game::flick::Step, &'static str> {
        let sounds = self.timeline.sound_requests().len();
        let credits = self.timeline.credits_requests().len();
        let step = self.timeline.step(tick)?;
        for request in &self.timeline.sound_requests()[sounds..] {
            println!(
                "cinematic_numeric_sound_request sequence={} tick={} offset={} opcode={} actor={} sound_id={} backend_ready=false request_satisfied=false",
                self.sequence,
                tick,
                request.command_offset,
                request.opcode,
                request.actor.map_or(-1, |a| a as i32),
                request.sound_id
            );
        }
        for request in &self.timeline.credits_requests()[credits..] {
            println!(
                "cinematic_credits_request sequence={} tick={} offset={} opcode={} renderer_ready=false request_satisfied=false",
                self.sequence, tick, request.command_offset, request.opcode
            );
        }
        for offset in &step.dispatch {
            if let Some(command) = self
                .timeline
                .program()
                .commands
                .iter()
                .find(|c| c.offset == *offset)
                && let Some(shadow) = giants_game::cinematic_shadow::Declaration::decode(command)?
            {
                println!(
                    "cinematic_shadow_declaration sequence={} tick={} offset={} actor={} texture_words={:?} bounds_words={:?} native_policy_resolved=false texture_created=false projection_created=false renderer_ready=false",
                    self.sequence,
                    tick,
                    shadow.command_offset,
                    shadow.actor_slot,
                    shadow.texture_words,
                    shadow.bounds_words,
                );
            }
            if let Some(effect) = self.effects.iter().find(|e| e.offset == *offset) {
                println!(
                    "cinematic_effect_request sequence={} tick={} offset={} slot={} name={:?} mode={} actor={} reference={:?} request_index={} creation_pending=true renderer_pending=true",
                    self.sequence,
                    tick,
                    effect.offset,
                    effect.slot,
                    effect.name,
                    effect.mode,
                    effect.actor,
                    effect.reference,
                    self.effect_requests().len()
                );
                self.effect_requests.push((tick, effect.clone()));
                self.effect_samples
                    .insert(effect.offset, (seconds, self.update_generation));
            }
        }
        self.update_generation += 1;
        Ok(step)
    }
    pub fn preview(&self) -> bool {
        self.preview
    }
    pub fn sequence(&self) -> &str {
        &self.sequence
    }
    pub fn camera(&self, tick: i32) -> Result<SourceCamera, giants_formats::Error> {
        let evaluator = Evaluator {
            animation: &self.animation,
        };
        let request = if self.timeline.has_stepped() {
            self.timeline.camera_request_offset()
        } else {
            // Future preloaded segments have no live cursor yet. Evaluate the
            // same bounded timeline for this explicit preparation tick.
            let mut timeline = giants_game::flick::Timeline::with_start(
                self.timeline.program().clone(),
                self.animation.timeline[2],
                self.animation.timeline[0],
            )
            .map_err(giants_formats::Error)?;
            timeline.step(tick).map_err(giants_formats::Error)?;
            timeline.camera_request_offset()
        }
        .ok_or(giants_formats::Error(
            "camera request has not been dispatched",
        ))?;
        let binding = self
            .camera_bindings
            .get(&request)
            .ok_or(giants_formats::Error("camera request was not prepared"))?
            .as_ref()
            .map_err(|_| giants_formats::Error("dispatched camera request cannot resolve"))?;
        evaluator.camera_named_preview_in_context(
            binding.tree.as_deref(),
            &binding.name,
            tick,
            giants_game::cinematic_viewport::Context {
                fraction: self.timeline.frame().fraction,
                ..giants_game::cinematic_viewport::Context::INSPECTION
            },
            self.preview,
        )
    }
    pub fn animation(&self) -> &giants_game::animation::Animation {
        &self.animation
    }
    pub fn tick(&self) -> i32 {
        self.initial_tick
    }
    pub fn has_actor_class(&self, class: i32) -> bool {
        self.timeline
            .program()
            .commands
            .iter()
            .any(|command| command.opcode == 2 && command.words.get(3) == Some(&(class as u32)))
    }
    fn load(
        root: &std::path::Path,
        sequence: String,
        tick: Option<i32>,
        preview: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let animation = giants_game::animation::load_director(root, &sequence)?;
        let [start, end, tpf, _] = animation.timeline;
        let initial_tick = tick.unwrap_or(start);
        if !(start..=end).contains(&initial_tick) {
            return Err("Cinematic tick outside director timeline".into());
        }
        let bytes = std::fs::read(giants_game::flick::sequence_path(root, &sequence)?)?;
        let effects = giants_formats::flick::effect_commands(&bytes)?;
        let program = if preview {
            giants_formats::flick::parse_preview(&bytes)?
        } else {
            giants_formats::flick::parse(&bytes)?
        };
        let camera_bindings = program
            .commands
            .iter()
            .filter(|c| c.opcode == 4)
            .map(|c| {
                let binding = giants_game::director_initial::CameraBinding::at(
                    &bytes, &program, &animation, c.offset,
                );
                println!(
                    "cinematic_camera_request sequence={} offset={} result={:?}",
                    sequence, c.offset, binding
                );
                (c.offset, binding)
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        for binding in camera_bindings.values().filter_map(|b| b.as_ref().ok()) {
            let node = animation.node(binding.node_offset)?;
            let (_, track) = node.camera.ok_or("camera request is not a camera node")?;
            let keys = if track == 0 {
                0
            } else {
                giants_formats::anm::read_word(animation.bytes(), track + 8)?
            };
            match (Evaluator {
                animation: &animation,
            })
            .camera_fov(binding.node_offset, initial_tick)
            {
                Ok(fov) => println!(
                    "cinematic_lens_request sequence={} node={} tick={} track={} key_count={} fov_bits={} evaluated=true",
                    sequence,
                    binding.node_offset,
                    initial_tick,
                    track,
                    keys,
                    fov.to_bits()
                ),
                Err(error) => println!(
                    "cinematic_lens_unresolved sequence={} node={} error={}",
                    sequence, binding.node_offset, error
                ),
            }
        }
        if preview {
            for command in &program.commands {
                if matches!(
                    command.opcode,
                    11 | 12 | 13 | 27 | 38 | 39 | 47 | 49 | 51 | 53
                ) {
                    eprintln!(
                        "cinematic_preview_omission sequence={sequence} opcode={} offset={} effect_unverified=true",
                        command.opcode, command.offset
                    );
                }
            }
        }
        if !preview && program.commands.iter().any(|c| matches!(c.opcode, 11 | 12)) {
            return Err("Actor visibility schedule is not yet integrated into the renderer".into());
        }
        let timeline = giants_game::flick::Timeline::with_start(program, tpf, start)?;
        println!(
            "cinematic_segment={} initial_tick={} timeline={:?}",
            sequence, initial_tick, animation.timeline
        );
        Ok(Self {
            sequence,
            animation,
            timeline,
            initial_tick,
            preview,
            effects,
            effect_requests: Vec::new(),
            effect_samples: Default::default(),
            update_generation: 0,
            camera_bindings,
        })
    }
}
#[derive(Resource)]
pub struct Cinematic {
    segments: Vec<Segment>,
    index: usize,
    frozen_tick: Option<i32>,
    seconds: f64,
    paused: bool,
    last_tick: i32,
    last_second: i32,
    events: Vec<giants_game::flick::Event>,
    event_seconds: f32,
    event_tick: i32,
    revision: u64,
    generation: u64,
    benchmark: bool,
    clock_initialized: bool,
    sync_origin: Option<f32>,
    clock_steps: u32,
    animation_dt: f32,
}
impl Cinematic {
    pub(crate) fn cameras_bound(&self) -> bool {
        self.segments
            .iter()
            .all(|s| !s.camera_bindings.is_empty() && s.camera_bindings.values().all(Result::is_ok))
    }
    pub(crate) fn event_seconds(&self) -> f32 {
        self.event_seconds
    }
    /// Tick actually evaluated by the command cursor, before clock advancement.
    pub(crate) fn event_tick(&self) -> i32 {
        self.event_tick
    }
    pub(crate) fn suspend(&mut self, paused: bool) {
        self.paused = paused;
        if paused {
            self.animation_dt = 0.;
        }
    }
    /// Elapsed time of this viewer frame, zero while frozen/loading/paused.
    /// This does not establish the original game's FX callback cadence.
    pub fn animation_dt(&self) -> f32 {
        self.animation_dt
    }
    /// Called once at the presentation boundary, after the director update.
    pub(crate) fn render_text_frame(
        &mut self,
    ) -> Result<Option<giants_game::cinematic_text::Frame>, &'static str> {
        if self.realtime() && !self.clock_initialized {
            return Ok(None);
        }
        self.segments[self.index].timeline.render_text(
            self.animation_dt,
            giants_game::arithmetic::Precision::Double53,
        )
    }
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }
    pub fn camera(&self) -> Result<SourceCamera, giants_formats::Error> {
        self.segments[self.index].camera(self.tick())
    }
    pub fn ordinary_range(&self) -> f32 {
        self.segments[self.index].timeline.ordinary_range()
    }
    pub fn preview(&self) -> bool {
        self.segments[self.index].preview()
    }
    pub fn director_index(&self) -> usize {
        self.index
    }
    pub fn sequence(&self) -> &str {
        self.segments[self.index].sequence()
    }
    pub fn benchmark(&self) -> bool {
        self.benchmark
    }
    pub fn sync_started(&self) -> bool {
        self.sync_origin.is_some()
    }
    pub fn events(&self) -> &[giants_game::flick::Event] {
        // Construction evaluates the initial pose and retains its events.
        // Live consumers must wait for the first initialized playback step;
        // otherwise they consume this batch again when that step publishes it.
        if self.realtime() && !self.clock_initialized {
            &[]
        } else {
            &self.events
        }
    }
    pub fn event_revision(&self) -> u64 {
        self.revision
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn clock_steps(&self) -> u32 {
        self.clock_steps
    }
    pub fn paused(&self) -> bool {
        self.paused
    }
    pub fn realtime(&self) -> bool {
        self.frozen_tick.is_none()
    }
    fn director_end(&self) -> bool {
        self.last_tick >= self.animation().timeline[1]
    }
    pub fn end_reached(&self) -> bool {
        self.director_end() && self.index + 1 == self.segments.len()
    }
    pub fn release_ready(&self, forced: bool) -> bool {
        self.segments[self.index]
            .timeline
            .release_ready(self.director_end(), forced)
    }
    pub fn program(&self) -> &giants_formats::flick::Program {
        self.segments[self.index].timeline.program()
    }
    pub fn render_policy(&self) -> giants_game::cinematic_render_policy::Policy {
        self.segments[self.index].timeline.render_policy()
    }
    pub fn actor_transparency(&self, slot: usize) -> Option<f32> {
        self.segments[self.index]
            .timeline
            .actor_transparency(slot)
            .map(|v| v.fraction)
    }
    pub fn actor_visible(&self, slot: usize) -> Option<bool> {
        self.segments[self.index].timeline.actor_visible(slot)
    }
    pub fn actor_render_visible(&self, slot: usize) -> Option<bool> {
        let timeline = &self.segments[self.index].timeline;
        Some(timeline.actor_requested(slot)? && timeline.actor_render_visible(slot)?)
    }
    pub fn actor_requested(&self, slot: usize) -> Option<bool> {
        self.segments[self.index].timeline.actor_requested(slot)
    }
    pub(crate) fn shadow_declaration(
        &self,
        slot: usize,
    ) -> Option<giants_game::cinematic_shadow::Declaration> {
        self.segments[self.index].timeline.shadow_declaration(slot)
    }
    pub(crate) fn reached_command_offset(&self) -> Option<usize> {
        self.segments[self.index].timeline.reached_command_offset()
    }
    pub(crate) fn shadow_local(
        &self,
        slot: usize,
    ) -> Result<Option<giants_game::procedural_aim::Matrix>, &'static str> {
        let offset = self.segments[self.index]
            .timeline
            .actor_offset(slot)
            .ok_or("shadow actor offset absent")?;
        if !offset.enabled {
            return Ok(None);
        }
        let (values, flags) = offset.matrix(1.)?;
        Ok(Some(giants_game::procedural_aim::Matrix { flags, values }))
    }
    pub(crate) fn actor_scale(&self, slot: usize, declared: f32) -> Option<f32> {
        self.segments[self.index]
            .timeline
            .actor_scale(slot, declared)
    }
    pub(crate) fn pose_seconds(&self) -> f32 {
        if let Some(tick) = self.frozen_tick {
            let [start, _, tpf, fps] = self.animation().timeline;
            ((f64::from(tick - start)) / (f64::from(tpf) * f64::from(fps))) as f32
        } else {
            self.seconds as f32
        }
    }
    pub fn initial_fade(&self) -> giants_game::cinematic_fade::Fade {
        let segment = &self.segments[self.index];
        if let Some(tick) = self.frozen_tick {
            segment
                .timeline
                .sampled_initial_fade(tick, segment.animation.timeline)
                .expect("validated frozen initial fade")
        } else {
            segment.timeline.initial_fade()
        }
    }
    pub fn fade(&self) -> giants_game::cinematic_fade::Fade {
        self.segments[self.index].timeline.fade()
    }

    pub fn programs(&self) -> impl Iterator<Item = &giants_formats::flick::Program> {
        self.segments.iter().map(|s| s.timeline.program())
    }
    pub fn animation(&self) -> &giants_game::animation::Animation {
        self.segments[self.index].animation()
    }
    pub fn tick(&self) -> i32 {
        self.last_tick
    }
    pub fn world_seconds(&self) -> f32 {
        (self.sync_origin.unwrap_or(0.) + self.seconds as f32).max(0.)
    }
    /// Local director clock before audio-origin handoff; source-frame capture
    /// thresholds refer to this clock, not the shared transport position.
    pub fn director_seconds(&self) -> f32 {
        self.seconds as f32
    }
    pub fn load(
        root: &std::path::Path,
        request: Load<'_>,
    ) -> Result<(Self, SourceCamera), Box<dyn std::error::Error>> {
        request.validate()?;
        let Load {
            tick,
            benchmark,
            directors,
            sequence: selected_sequence,
            full_chain,
            preview,
        } = request;
        let sequence = selected_sequence.unwrap_or(if benchmark {
            "story4_T_square_1"
        } else {
            "story4_T_square_2"
        });
        let preview = preview || selected_sequence.is_some() && tick.is_none();
        let mut segments = vec![Segment::load(root, sequence.into(), tick, preview)?];
        while full_chain || segments.len() < directors {
            if full_chain && segments.last().unwrap().timeline.program().next.is_none() {
                break;
            }
            if segments.len() >= 128 {
                return Err("Cinematic chain exceeds preview budget".into());
            }
            let next = segments
                .last()
                .unwrap()
                .timeline
                .program()
                .next
                .clone()
                .ok_or("Requested director count exceeds the source chain")?;
            if segments
                .iter()
                .any(|s| s.sequence.eq_ignore_ascii_case(&next))
            {
                return Err("Cyclic cinematic chain".into());
            }
            segments.push(Segment::load(root, next, None, preview)?);
        }
        let initial = segments[0].initial_tick;
        let [start, _, tpf, fps] = segments[0].animation.timeline;
        let initial_seconds = ((initial - start) as f64 / (f64::from(tpf) * f64::from(fps))) as f32;
        let events = segments[0].step(initial, initial_seconds)?.events;
        let camera = segments[0].camera(initial)?;
        println!(
            "cinematic_completion sequence={} animation_complete={} release_ready={} hold_label={}",
            sequence,
            initial >= segments[0].animation.timeline[1],
            segments[0]
                .timeline
                .release_ready(initial >= segments[0].animation.timeline[1], false),
            segments[0].timeline.completion_hold().unwrap_or("none")
        );
        println!(
            "cinematic_camera_source=flk_{}.bin initial_tick={} timeline={:?} frozen={} camera_resource=true flk_vm=false directors={}",
            sequence,
            initial,
            segments[0].animation.timeline,
            tick.is_some(),
            segments.len()
        );
        println!(
            "cinematic_playback benchmark={benchmark} linked={full_chain} directors={}",
            segments.len()
        );
        Ok((
            Self {
                segments,
                index: 0,
                frozen_tick: tick,
                seconds: 0.,
                paused: false,
                last_tick: initial,
                last_second: -1,
                events,
                event_seconds: 0.,
                event_tick: initial,
                revision: 1,
                generation: 0,
                benchmark,
                clock_initialized: false,
                sync_origin: None,
                clock_steps: 0,
                animation_dt: 0.,
            },
            camera,
        ))
    }
    fn restart(&mut self) {
        self.animation_dt = 0.;
        self.index = 0;
        self.seconds = 0.;
        self.event_seconds = 0.;
        self.last_tick = self.segments[0].initial_tick;
        self.last_second = -1;
        for s in &mut self.segments {
            s.timeline.reset();
            s.effect_requests.clear();
            s.effect_samples.clear();
            s.update_generation = 0;
        }
        self.events.clear();
        self.revision += 1;
        self.generation += 1;
        self.clock_initialized = false;
        self.sync_origin = None;
        self.clock_steps = 0;
    }
    fn next_director(&mut self) -> Result<(), &'static str> {
        info!(
            "cinematic_director_transition={} from={} to={} audio_carried={}",
            self.index,
            self.sequence(),
            self.segments[self.index + 1].sequence,
            self.sync_origin.is_some()
        );
        if let Some(origin) = self.sync_origin {
            let next = giants_game::cinematic_clock::handoff(origin, self.seconds as f32)?;
            info!(
                "cinematic_director_handoff={} from={} to={} previous_seconds_bits={} old_origin_bits={} new_origin_bits={}",
                self.index,
                self.sequence(),
                self.segments[self.index + 1].sequence,
                (self.seconds as f32).to_bits(),
                origin.to_bits(),
                next.to_bits()
            );
            self.sync_origin = Some(next);
        }
        self.index += 1;
        self.seconds = 0.;
        self.last_tick = self.animation().timeline[0];
        self.last_second = -1;
        self.clock_initialized = false;
        self.events.clear();
        self.revision += 1;
        Ok(())
    }
}
pub struct Load<'a> {
    pub tick: Option<i32>,
    pub benchmark: bool,
    pub directors: usize,
    pub sequence: Option<&'a str>,
    pub full_chain: bool,
    pub preview: bool,
}
impl Load<'_> {
    fn validate(&self) -> Result<(), &'static str> {
        if !(1..=128).contains(&self.directors)
            || self.tick.is_some() && (self.full_chain || self.directors > 1)
        {
            return Err("Linked directors require a live, bounded selection");
        }
        if self.benchmark && self.sequence.is_some() {
            return Err("Benchmark selection and explicit source selection are separate");
        }
        if !self.benchmark && (self.full_chain || self.directors > 1) && self.sequence.is_none() {
            return Err("Normal linked playback requires its initial source");
        }
        Ok(())
    }
}
pub fn convert(camera: &SourceCamera) -> crate::validation_camera::ValidationCamera {
    crate::validation_camera::ValidationCamera {
        view: camera.view,
        projection: camera.projection,
        viewport: UVec4::from_array(camera.viewport),
    }
}
pub fn control(camera: &SourceCamera) -> Vec<u8> {
    let mut bytes = b"GCAM0001".to_vec();
    bytes.extend(
        camera
            .view
            .into_iter()
            .chain(camera.projection)
            .flat_map(f32::to_le_bytes),
    );
    bytes.extend(camera.viewport.into_iter().flat_map(u32::to_le_bytes));
    bytes.extend(camera.origin.into_iter().flat_map(f32::to_le_bytes));
    bytes
}
fn playback_failure(
    showcase: &mut Option<ResMut<crate::mission_showcase::State>>,
    exit: &mut MessageWriter<AppExit>,
    reason: &str,
) {
    if !showcase.as_mut().is_some_and(|s| s.playback_failed(reason)) {
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
    }
}
#[allow(clippy::too_many_arguments)] // Independent ECS resources and presentation failure routing.
pub fn update(
    timing: Timing,
    capture: Option<Res<crate::Capture>>,
    pipelines: Res<crate::PipelinesReady>,
    keys: Res<ButtonInput<KeyCode>>,
    cinematic: Option<ResMut<Cinematic>>,
    mut scene: ResMut<crate::world_scene::Scene>,
    mut materials: crate::world_scene::SceneMaterials,
    mut exit: MessageWriter<AppExit>,
    mut showcase: Option<ResMut<crate::mission_showcase::State>>,
) {
    let Some(mut cine) = cinematic else {
        return;
    };
    cine.animation_dt = 0.;
    if cine.frozen_tick.is_some()
        || !pipelines.0
        || timing.cube.as_ref().is_some_and(|b| !b.playback_ready())
    {
        return;
    }
    // Keep the state actually reached by live playback while the GPU prepares it.
    // Never replace the director clock with the requested threshold.
    let inspection_hold = capture.as_ref().is_some_and(|c| {
        c.hold_cinematic
            && c.after_seconds
                .is_some_and(|s| cine.director_seconds() >= s)
    });
    if inspection_hold {
        if !cine.paused {
            println!(
                "cinematic_inspection_hold sequence={} tick={} seconds={} native_timing_verified=false",
                cine.sequence(),
                cine.tick(),
                cine.director_seconds()
            );
            cine.suspend(true);
        }
    } else if keys.just_pressed(KeyCode::Space) {
        cine.paused = !cine.paused;
    }
    let restarted = keys.just_pressed(KeyCode::KeyR);
    let mut changed = restarted || !cine.clock_initialized;
    let mut render_dt = 0.;
    if restarted {
        cine.restart();
    }
    if !cine.paused && cine.director_end() && !cine.end_reached() && cine.release_ready(false) {
        if let Err(error) = cine.next_director() {
            error!("Director transition failed: {}", error);
            playback_failure(&mut showcase, &mut exit, error);
            return;
        }
        changed = true;
    }
    if !cine.paused && !cine.end_reached() {
        let old_tick = if restarted {
            cine.animation().timeline[0]
        } else {
            cine.last_tick
        };
        let index = cine.index;
        cine.event_seconds = cine.seconds as f32;
        let seconds = cine.event_seconds;
        let scheduled = cine.segments[index].step(old_tick, seconds);
        match scheduled {
            Ok(step) => {
                cine.event_tick = old_tick;
                if cine.clock_initialized {
                    cine.events = step.events;
                } else {
                    cine.events.extend(step.events);
                }
            }
            Err(error) => {
                error!("Opening event schedule failed: {}", error);
                playback_failure(&mut showcase, &mut exit, error);
                return;
            }
        }
        let sync_restarted = cine
            .events
            .iter()
            .any(|e| e.kind == giants_game::flick::EventKind::SyncTrack);
        if sync_restarted {
            cine.sync_origin = Some(-(cine.seconds as f32));
        }
        cine.revision += 1;
        let audio_seconds = if sync_restarted {
            0.
        } else {
            timing.master.as_ref().map_or(0., |m| m.position())
        };
        let before = cine.seconds as f32;
        let initialized = cine.clock_initialized;
        let raw_dt = if cine.benchmark {
            1. / 60.
        } else {
            timing.time.delta_secs()
        };
        let step =
            match giants_game::cinematic_clock::advance(giants_game::cinematic_clock::Input {
                seconds: cine.seconds as f32,
                raw_dt,
                finished: !cine.clock_initialized,
                sync: cine
                    .sync_origin
                    .map(|origin| giants_game::cinematic_clock::AudioTime {
                        origin,
                        seconds: audio_seconds,
                    }),
            }) {
                Ok(step) => step,
                Err(error) => {
                    error!("Opening clock failed: {}", error);
                    playback_failure(&mut showcase, &mut exit, error);
                    return;
                }
            };
        info!(
            "cinematic_clock_step={} before_bits={} input_dt_bits={} reset={} sync={} origin_bits={} audio_seconds={} seconds_bits={} dt_bits={} director={}",
            cine.clock_steps,
            before.to_bits(),
            raw_dt.to_bits(),
            !initialized,
            cine.sync_origin.is_some(),
            cine.sync_origin.unwrap_or(0.).to_bits(),
            audio_seconds,
            step.seconds.to_bits(),
            step.dt.to_bits(),
            cine.index
        );
        cine.clock_steps += 1;
        cine.seconds = f64::from(step.seconds);
        render_dt = step.dt;
        cine.clock_initialized = true;
    }
    cine.animation_dt = render_dt;
    let [start, end, ticks, fps] = cine.animation().timeline;
    let tick = ((cine.seconds as f32 as f64 * f64::from(ticks) * f64::from(fps) + f64::from(start))
        as i32)
        .clamp(start, end);
    let index = cine.index;
    let before = cine.segments[index].timeline.frame().fraction.to_bits();
    if let Err(error) = cine.segments[index]
        .timeline
        .advance_frame(render_dt, giants_game::arithmetic::Precision::Double53)
    {
        error!("Cinematic frame update failed: {}", error);
        playback_failure(&mut showcase, &mut exit, error);
        return;
    }
    changed |= before != cine.segments[index].timeline.frame().fraction.to_bits();
    if tick == cine.last_tick && !changed {
        return;
    }
    let camera = match cine.segments[index].camera(tick) {
        Ok(camera) => camera,
        Err(error) => {
            error!("Cinematic camera evaluation failed: {}", error);
            playback_failure(&mut showcase, &mut exit, &error.to_string());
            return;
        }
    };
    scene.validation_camera = Some(convert(&camera));
    scene.camera_origin = Some(camera.origin);
    scene.cinematic_capture_ready = tick == end && cine.index + 1 == cine.segments.len();
    crate::world_scene::apply_camera(&scene, &mut materials);
    cine.last_tick = tick;
    let second = cine.seconds as i32;
    if cine.last_second != second || tick == end {
        if tick == end {
            info!(
                "cinematic_completion sequence={} animation_complete=true release_ready={} hold_label={}",
                cine.sequence(),
                cine.release_ready(false),
                cine.segments[cine.index]
                    .timeline
                    .completion_hold()
                    .unwrap_or("none")
            );
        }
        info!(
            "cinematic_camera_tick={} origin={:?} end_reached={}",
            tick,
            camera.origin,
            tick == end
        );
        cine.last_second = second;
    }
}

/// Live playback status, including measured frame cadence rather than a target FPS.
pub fn update_title(
    cinematic: Option<Res<Cinematic>>,
    time: Res<Time<Real>>,
    mut sample: Local<(f64, u32)>,
    mut windows: Query<&mut Window>,
) {
    let Some(cinematic) = cinematic else {
        // Playback owns this caption only while its resource is present. Keep
        // unrelated diagnostic captions, and start the next FPS sample at entry.
        for mut window in &mut windows {
            if window.title.starts_with("Giants - PARTIAL cinematic - ") {
                window.title = "Giants-rs".into();
            }
        }
        *sample = (time.elapsed_secs_f64(), 0);
        return;
    };
    sample.1 += 1;
    let elapsed = time.elapsed_secs_f64() - sample.0;
    if elapsed < 1. {
        return;
    }
    let fps = f64::from(sample.1) / elapsed;
    for mut window in &mut windows {
        window.title = format!(
            "Giants - PARTIAL cinematic - {}/{} - {} - {:.0} FPS - Space pause / R restart / Esc",
            cinematic.index + 1,
            cinematic.segments.len(),
            cinematic.sequence(),
            fps
        );
    }
    sample.0 = time.elapsed_secs_f64();
    sample.1 = 0;
}

#[cfg(test)]
mod tests {
    use super::Load;
    #[test]
    fn linked_selection_keeps_benchmark_and_frozen_requests_separate() {
        let mut request = Load {
            tick: None,
            benchmark: false,
            directors: 1,
            sequence: Some("source"),
            full_chain: true,
            preview: true,
        };
        assert!(request.validate().is_ok());
        request.benchmark = true;
        assert!(request.validate().is_err());
        request.sequence = None;
        assert!(request.validate().is_ok());
        request.tick = Some(160);
        assert!(request.validate().is_err());
        request.tick = None;
        request.benchmark = false;
        assert!(request.validate().is_err());
        request.full_chain = false;
        assert!(request.validate().is_ok());
        request.directors = 129;
        assert!(request.validate().is_err());
    }
}
