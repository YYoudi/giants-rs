//! First-sequence event dispatch driven by a source director tick.
//! Actor setup is handled by existing loaders; text drawing and audio clock sync
//! remain separate. Reaching command 5 does not finish the director animation.
use giants_formats::flick::Program;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Voice,
    Subtitle,
    SyncTrack,
    /// Typed effect binding is resolved separately by command offset.
    Effect,
}
#[derive(Clone, Debug)]
pub struct Event {
    pub tick: i32,
    pub offset: usize,
    pub kind: EventKind,
    pub name: String,
    pub gain_db100: i32,
}
pub struct Step {
    pub dispatch: Vec<usize>,
    pub events: Vec<Event>,
    pub cursor: usize,
}
pub struct Timeline {
    program: Program,
    cursor: usize,
    last_tick: Option<i32>,
    ticks_per_frame: i32,
    voice_gain: f32,
    frame: crate::cinematic_frame::Frame,
    fade: crate::cinematic_fade::Fade,
    initial_fade: crate::cinematic_frame::Frame,
    start_tick: i32,
    actor_visibility: [bool; 32],
    actor_requests: [bool; 32],
    camera_command: Option<usize>,
    ordinary_range: f32,
    actor_offsets: [crate::cinematic_offset::Offset; 32],
    render_policy: crate::cinematic_render_policy::Policy,
    actor_scales: [Option<f32>; 32],
    actor_transparency: [crate::cinematic_frame::Frame; 32],
    completion_hold: Option<String>,
    credits_requests: Vec<crate::cinematic_credits::Request>,
    sound_requests: Vec<crate::cinematic_sound::Request>,
    shadow_declarations: crate::cinematic_shadow::Declarations,
    text: crate::cinematic_text::State,
}
impl Timeline {
    pub fn new(program: Program, ticks_per_frame: i32) -> Result<Self, &'static str> {
        Self::with_start(program, ticks_per_frame, ticks_per_frame)
    }
    pub fn with_start(
        program: Program,
        ticks_per_frame: i32,
        start_tick: i32,
    ) -> Result<Self, &'static str> {
        // Established inspection audio profile; normal runtime must pass its
        // independently resolved mixer gain through with_audio_gain.
        Self::with_audio_gain(program, ticks_per_frame, start_tick, 0.75)
    }
    pub fn with_audio_gain(
        program: Program,
        ticks_per_frame: i32,
        start_tick: i32,
        audio_gain: f32,
    ) -> Result<Self, &'static str> {
        if ticks_per_frame <= 0 || start_tick < 0 || program.commands.is_empty() {
            return Err("invalid flick tick profile");
        }
        if !(0. ..=1.).contains(&audio_gain) {
            return Err("invalid supplied flick audio gain");
        }
        Ok(Self {
            program,
            cursor: 0,
            last_tick: None,
            ticks_per_frame,
            voice_gain: audio_gain,
            frame: crate::cinematic_frame::Frame::default(),
            fade: crate::cinematic_fade::Fade::default(),
            initial_fade: crate::cinematic_frame::Frame::default(),
            start_tick,
            credits_requests: Vec::new(),
            sound_requests: Vec::new(),
            shadow_declarations: crate::cinematic_shadow::Declarations::default(),
            text: crate::cinematic_text::State::default(),
            actor_visibility: [true; 32],
            actor_requests: [false; 32],
            camera_command: None,
            ordinary_range: crate::cinematic_ordinary::INITIAL_RANGE,
            actor_offsets: [crate::cinematic_offset::Offset::default(); 32],
            render_policy: crate::cinematic_render_policy::Policy::default(),
            actor_scales: [None; 32],
            actor_transparency: [crate::cinematic_frame::Frame {
                fraction: 0.,
                target: 0.,
                speed: 0.,
            }; 32],
            completion_hold: None,
        })
    }
    pub fn credits_requests(&self) -> &[crate::cinematic_credits::Request] {
        &self.credits_requests
    }
    pub fn sound_requests(&self) -> &[crate::cinematic_sound::Request] {
        &self.sound_requests
    }
    /// Source-selected text block, independent of the voice event lifetime.
    pub fn text_block(&self) -> Option<usize> {
        self.text.block()
    }
    /// Explicit render-boundary consumer; font layout/drawing remain services.
    /// Caller supplies the corrected frame delta, never a voice duration.
    pub fn render_text(
        &mut self,
        dt: f32,
        precision: crate::arithmetic::Precision,
    ) -> Result<Option<crate::cinematic_text::Frame>, &'static str> {
        self.text.render(&self.program, dt, precision)
    }
    /// Reached source declaration only, before native suppression, allocation,
    /// silhouette texture generation and terrain projection are supplied.
    pub fn shadow_declaration(&self, slot: usize) -> Option<crate::cinematic_shadow::Declaration> {
        self.shadow_declarations.actor(slot)
    }
    /// Last dispatched command, excluding the wait currently blocking the cursor.
    pub fn reached_command_offset(&self) -> Option<usize> {
        self.cursor
            .checked_sub(1)
            .and_then(|i| self.program.commands.get(i))
            .map(|c| c.offset)
    }
    pub fn reset(&mut self) {
        self.sound_requests.clear();
        self.text = crate::cinematic_text::State::default();
        self.shadow_declarations = crate::cinematic_shadow::Declarations::default();
        self.credits_requests.clear();
        self.completion_hold = None;
        self.cursor = 0;
        self.last_tick = None;
        self.frame = crate::cinematic_frame::Frame::default();
        self.fade = crate::cinematic_fade::Fade::default();
        self.initial_fade = crate::cinematic_frame::Frame::default();
        self.actor_visibility.fill(true);
        self.actor_requests.fill(false);
        self.camera_command = None;
        self.ordinary_range = crate::cinematic_ordinary::INITIAL_RANGE;
        self.actor_offsets
            .fill(crate::cinematic_offset::Offset::default());
        self.render_policy = crate::cinematic_render_policy::Policy::default();
        self.actor_scales.fill(None);
        self.actor_transparency.fill(crate::cinematic_frame::Frame {
            fraction: 0.,
            target: 0.,
            speed: 0.,
        });
    }
    pub fn has_stepped(&self) -> bool {
        self.last_tick.is_some()
    }
    pub fn last_sampled_tick(&self) -> Option<i32> {
        self.last_tick
    }
    /// Reachable source effects at a supplied source sample, without advancing the
    /// live cursor or allocating effects. Future waits remain unsatisfied.
    /// Prototype PS2 lookup: FLK_FlickProcess__FP9FLK_Flick,
    /// FLK_FlickUpdateTime__FP9FLK_Flickf and FlickAnimate__FP9FLK_Flick.
    /// All relationships remain unknown; no prototype ABI/layout is imported.
    /// The acquired PC process retires on animation completion even when the
    /// command cursor is stopped at a later wait; declarations are not births.
    pub fn effect_offsets_through(
        &self,
        sample_tick: i32,
    ) -> Result<std::collections::BTreeSet<usize>, &'static str> {
        let mut cursor =
            Self::with_start(self.program.clone(), self.ticks_per_frame, self.start_tick)?;
        Ok(cursor
            .step(sample_tick)?
            .events
            .into_iter()
            .filter(|event| event.kind == EventKind::Effect)
            .map(|event| event.offset)
            .collect())
    }
    pub fn completion_hold(&self) -> Option<&str> {
        self.completion_hold.as_deref()
    }
    /// Normal animation completion retains a scene with a nonnull FLK40 label.
    /// Forced retirement bypasses that hold. This does not load the label.
    pub fn release_ready(&self, animation_complete: bool, forced: bool) -> bool {
        forced || (animation_complete && self.completion_hold.is_none())
    }
    pub fn camera_request_offset(&self) -> Option<usize> {
        self.camera_command
    }
    pub fn ordinary_range(&self) -> f32 {
        self.ordinary_range
    }
    pub fn actor_visible(&self, slot: usize) -> Option<bool> {
        self.actor_visibility.get(slot).copied()
    }
    /// Declaration reached by the cursor, independently of resource preparation
    /// and the enabled/transparency fields. This is not a backend readiness ack.
    pub fn actor_requested(&self, slot: usize) -> Option<bool> {
        self.actor_requests.get(slot).copied()
    }
    /// Render permission only. A hidden actor may still have an advancing
    /// animation clock; this is not an actor-update or on-screen-pixel gate.
    pub fn actor_render_visible(&self, slot: usize) -> Option<bool> {
        Some(self.actor_visible(slot)? && self.actor_transparency(slot)?.fraction != 1.)
    }
    pub fn actor_offset(&self, slot: usize) -> Option<crate::cinematic_offset::Offset> {
        self.actor_offsets.get(slot).copied()
    }
    /// Raw replacement field; consumers retain their finite/positive profile.
    /// Named director actors do not multiply their own matrix by this field,
    /// but attached children still divide by the current parent field.
    pub fn actor_scale(&self, slot: usize, declared: f32) -> Option<f32> {
        self.actor_scales.get(slot).map(|v| v.unwrap_or(declared))
    }
    /// Native actor field copied into the embedded object's transparency.
    pub fn actor_transparency(&self, slot: usize) -> Option<crate::cinematic_frame::Frame> {
        self.actor_transparency.get(slot).copied()
    }
    pub fn render_policy(&self) -> crate::cinematic_render_policy::Policy {
        self.render_policy
    }
    pub fn initial_fade(&self) -> crate::cinematic_fade::Fade {
        crate::cinematic_fade::Fade {
            envelope: self.initial_fade,
            color: [0; 3],
        }
    }
    /// Frozen inspection policy: source wait thresholds and continuous envelope
    /// time, not observed native frame pacing or a normal audio-clock replay.
    pub fn sampled_initial_fade(
        &self,
        tick: i32,
        timeline: [i32; 4],
    ) -> Result<crate::cinematic_fade::Fade, &'static str> {
        let [start, end, tpf, fps] = timeline;
        if tpf <= 0 || fps <= 0 || tick < start || tick > end {
            return Err("initial fade sampling outside timeline");
        }
        let mut frame = crate::cinematic_frame::Frame::default();
        let mut event_tick = i64::from(start);
        let mut sampled_tick = event_tick;
        let precision = crate::arithmetic::Precision::Double53;
        let advance = |frame: &mut crate::cinematic_frame::Frame,
                       from: i64,
                       to: i64|
         -> Result<(), &'static str> {
            let mut seconds = (to - from) as f64 / (f64::from(tpf) * f64::from(fps));
            while seconds > 0. && frame.speed != 0. {
                let dt = seconds.min(1.);
                frame.advance(dt as f32, precision)?;
                seconds -= dt;
            }
            Ok(())
        };
        for command in &self.program.commands {
            if command.opcode == 9 {
                event_tick = event_tick.max(i64::from(command.words[0] as i32) * i64::from(tpf));
            }
            if event_tick > i64::from(tick) || command.opcode == 5 {
                break;
            }
            if command.opcode == 31 {
                advance(&mut frame, sampled_tick, event_tick)?;
                frame.set(
                    f32::from_bits(command.words[0]),
                    f32::from_bits(command.words[1]),
                    precision,
                )?;
                sampled_tick = event_tick;
            }
        }
        advance(&mut frame, sampled_tick, i64::from(tick))?;
        Ok(crate::cinematic_fade::Fade {
            envelope: frame,
            color: [0; 3],
        })
    }
    pub fn fade(&self) -> crate::cinematic_fade::Fade {
        self.fade
    }
    pub fn frame(&self) -> crate::cinematic_frame::Frame {
        self.frame
    }
    /// Render-boundary update; caller supplies the corrected director dt.
    pub fn advance_frame(
        &mut self,
        dt: f32,
        precision: crate::arithmetic::Precision,
    ) -> Result<(), &'static str> {
        self.frame.advance(dt, precision)?;
        self.fade.advance(dt, precision)?;
        self.initial_fade.advance(dt, precision)?;
        for value in &mut self.actor_transparency {
            value.advance(dt, precision)?;
        }
        Ok(())
    }
    pub fn program(&self) -> &Program {
        &self.program
    }
    pub fn step(&mut self, tick: i32) -> Result<Step, &'static str> {
        if tick < 0 || self.last_tick.is_some_and(|v| tick < v) {
            return Err("flick tick moved backwards without reset");
        }
        self.last_tick = Some(tick);
        let mut dispatch = Vec::new();
        let mut events = Vec::new();
        loop {
            let command = self
                .program
                .commands
                .get(self.cursor)
                .ok_or("flick cursor outside program")?;
            dispatch.push(command.offset);
            if command.opcode == 5
                || command.opcode == 9 && tick / self.ticks_per_frame < command.words[0] as i32
                || command.opcode == 52 && tick <= self.start_tick
            {
                return Ok(Step {
                    dispatch,
                    events,
                    cursor: command.offset,
                });
            }
            let kind = match command.opcode {
                44 => Some(EventKind::Voice),
                16 => Some(EventKind::Subtitle),
                52 => Some(EventKind::SyncTrack),
                39 => Some(EventKind::Effect),
                _ => None,
            };
            if command.opcode == 14 {
                self.text.select(command.offset, &self.program)?;
            }
            if command.opcode == 2 {
                let slot = *command
                    .words
                    .get(2)
                    .ok_or("actor declaration slot absent")? as usize;
                *self
                    .actor_requests
                    .get_mut(slot)
                    .ok_or("actor declaration slot outside profile")? = true;
            }
            if let Some(request) = crate::cinematic_sound::Request::decode(command)? {
                self.sound_requests.push(request);
            }
            if matches!(command.opcode, 47 | 49 | 51 | 53) {
                self.credits_requests
                    .push(crate::cinematic_credits::Request {
                        command_offset: command.offset,
                        opcode: command.opcode,
                        text_key: command.label.clone(),
                        word: command
                            .words
                            .first()
                            .copied()
                            .filter(|_| command.opcode == 51),
                    });
            }
            if command.opcode == 4 {
                self.camera_command = Some(command.offset);
            }
            if command.opcode == 38 {
                let value =
                    f32::from_bits(*command.words.first().ok_or("ordinary range word absent")?);
                if !value.is_finite() || value < 0. {
                    return Err("ordinary range outside finite nonnegative profile");
                }
                self.ordinary_range = value;
            }
            // Retain reached declarations. This is not a claim that the native
            // global suppression policy was resolved or an image was created.
            self.shadow_declarations.apply(command, false)?;
            if matches!(command.opcode, 6 | 45 | 46) {
                self.render_policy.apply(command.opcode, &command.words)?;
            }
            if command.opcode == 32 {
                let slot = *command.words.last().ok_or("offset actor slot absent")? as i32;
                if slot >= 0 {
                    self.actor_offsets
                        .get_mut(slot as usize)
                        .ok_or("offset actor slot outside profile")?
                        .apply(&command.words)?;
                }
            }
            if command.opcode == 33 {
                let slot = command.words[0] as i32;
                if slot >= 0 {
                    *self
                        .actor_scales
                        .get_mut(slot as usize)
                        .ok_or("scale actor slot outside profile")? =
                        Some(f32::from_bits(command.words[1]));
                }
            }
            if command.opcode == 40 {
                self.completion_hold = command.label.clone();
            }
            if command.opcode == 42 {
                let slot = command.words[0] as i32;
                if slot >= 0 {
                    self.actor_transparency
                        .get_mut(slot as usize)
                        .ok_or("transparency actor slot outside profile")?
                        .set(
                            f32::from_bits(command.words[1]),
                            f32::from_bits(command.words[2]),
                            crate::arithmetic::Precision::Double53,
                        )?;
                }
            }
            if command.opcode == 31 {
                self.initial_fade.set(
                    f32::from_bits(command.words[0]),
                    f32::from_bits(command.words[1]),
                    crate::arithmetic::Precision::Double53,
                )?;
            }
            if command.opcode == 37 {
                self.fade.set(
                    std::array::from_fn(|i| f32::from_bits(command.words[i])),
                    f32::from_bits(command.words[3]),
                    f32::from_bits(command.words[4]),
                    crate::arithmetic::Precision::Double53,
                )?;
            }
            if command.opcode == 7 {
                self.frame.set(
                    f32::from_bits(command.words[0]),
                    f32::from_bits(command.words[1]),
                    crate::arithmetic::Precision::Double53,
                )?;
            }
            if matches!(command.opcode, 11 | 12) {
                let slot = *command.words.first().ok_or("visibility slot missing")? as usize;
                *self
                    .actor_visibility
                    .get_mut(slot)
                    .ok_or("visibility slot outside profile")? = command.opcode == 11;
            }
            if let Some(kind) = kind {
                events.push(Event {
                    tick,
                    offset: command.offset,
                    kind,
                    name: if kind == EventKind::Effect {
                        String::new()
                    } else {
                        command.label.clone().ok_or("event label missing")?
                    },
                    gain_db100: if kind == EventKind::Effect {
                        0
                    } else if self.voice_gain == 0. {
                        -10000
                    } else {
                        (2000. * f64::from(self.voice_gain).log10()) as i32
                    },
                });
            }
            self.cursor += 1;
        }
    }
}

/// A source sequence identifier is a basename, never an arbitrary path.
pub fn sequence_path(
    root: &std::path::Path,
    sequence: &str,
) -> Result<std::path::PathBuf, &'static str> {
    if sequence.is_empty()
        || sequence.len() > 100
        || !sequence
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        return Err("invalid cinematic sequence identifier");
    }
    Ok(root.join("Bin").join(format!("flk_{sequence}.bin")))
}

#[cfg(test)]
mod tests {
    #[test]
    fn endpoint_includes_due_effect_but_never_dispatches_past_a_future_wait() {
        use giants_formats::flick::{Command, Program};
        let command = |offset, opcode, words| Command {
            offset,
            opcode,
            words,
            label: None,
        };
        let mut timeline = super::Timeline::new(
            Program {
                commands: vec![
                    command(0, 9, vec![2]),
                    command(12, 39, vec![]),
                    command(20, 9, vec![3]),
                    command(32, 39, vec![]),
                    command(40, 5, vec![]),
                ],
                next: None,
                program_end: 48,
            },
            160,
        )
        .unwrap();
        assert_eq!(timeline.effect_offsets_through(320).unwrap(), [12].into());
        assert!(!timeline.has_stepped()); // preparation cannot advance the live owner
        assert!(timeline.step(319).unwrap().events.is_empty());
        assert_eq!(timeline.last_sampled_tick(), Some(319));
        assert_eq!(timeline.step(320).unwrap().events[0].offset, 12);
        assert_eq!(timeline.last_sampled_tick(), Some(320));
        assert!(timeline.step(320).unwrap().events.is_empty());
        assert_eq!(
            timeline.effect_offsets_through(480).unwrap(),
            [12, 32].into()
        );
    }
    #[test]
    fn pending_numeric_sounds_are_ordered_once_and_reset_with_credits() {
        let commands = vec![
            giants_formats::flick::Command {
                offset: 40,
                opcode: 13,
                words: vec![5, 31],
                label: None,
            },
            giants_formats::flick::Command {
                offset: 56,
                opcode: 9,
                words: vec![2],
                label: None,
            },
            giants_formats::flick::Command {
                offset: 68,
                opcode: 27,
                words: vec![73],
                label: None,
            },
            giants_formats::flick::Command {
                offset: 80,
                opcode: 53,
                words: vec![],
                label: None,
            },
            giants_formats::flick::Command {
                offset: 88,
                opcode: 5,
                words: vec![],
                label: None,
            },
        ];
        let mut t = super::Timeline::new(
            giants_formats::flick::Program {
                commands,
                next: None,
                program_end: 96,
            },
            160,
        )
        .unwrap();
        t.step(160).unwrap();
        t.step(160).unwrap();
        assert_eq!(t.sound_requests().len(), 1);
        assert_eq!(t.sound_requests()[0].actor, Some(5));
        t.step(320).unwrap();
        t.step(480).unwrap();
        assert_eq!(
            t.sound_requests()
                .iter()
                .map(|r| r.sound_id)
                .collect::<Vec<_>>(),
            vec![31, 73]
        );
        assert_eq!(t.credits_requests()[0].opcode, 53);
        t.reset();
        assert!(t.sound_requests().is_empty() && t.credits_requests().is_empty());
        t.step(320).unwrap();
        assert_eq!(t.sound_requests().len(), 2);
        let c = giants_formats::flick::Command {
            offset: 0,
            opcode: 13,
            words: vec![u32::MAX, u32::MAX],
            label: None,
        };
        assert!(
            crate::cinematic_sound::Request::decode(&c)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn effect_offsets_dispatch_once_in_source_order_and_restart() {
        use super::*;
        use giants_formats::flick::Command;
        let command = |offset, opcode, words| Command {
            offset,
            opcode,
            words,
            label: None,
        };
        let mut timeline = Timeline::with_start(
            Program {
                commands: vec![
                    command(24, 39, vec![u32::MAX, 0, 1, u32::MAX, 0]),
                    command(52, 9, vec![2]),
                    command(64, 39, vec![u32::MAX, 0, 1, u32::MAX, 0]),
                    command(92, 5, vec![]),
                ],
                next: None,
                program_end: 100,
            },
            160,
            0,
        )
        .unwrap();
        let first = timeline.step(0).unwrap();
        assert_eq!(first.events.len(), 1);
        assert_eq!(first.events[0].kind, EventKind::Effect);
        assert_eq!(first.events[0].offset, 24);
        assert!(first.events[0].name.is_empty());
        assert!(timeline.step(0).unwrap().events.is_empty());
        assert!(timeline.step(319).unwrap().events.is_empty());
        assert_eq!(timeline.step(320).unwrap().events[0].offset, 64);
        timeline.reset();
        assert_eq!(timeline.step(0).unwrap().events[0].offset, 24);
    }
    #[test]
    fn credits_requests_wait_for_dispatch_and_remain_unbound() {
        use giants_formats::flick::{Command, Program};
        let command = |offset, opcode, words, label| Command {
            offset,
            opcode,
            words,
            label,
        };
        let program = Program {
            commands: vec![
                command(24, 9, vec![2], None),
                command(36, 47, vec![100], Some("credits".into())),
                command(48, 51, vec![2f32.to_bits()], None),
                command(60, 49, vec![], None),
                command(68, 5, vec![], None),
            ],
            next: None,
            program_end: 76,
        };
        let mut timeline = super::Timeline::with_start(program, 160, 0).unwrap();
        timeline.step(319).unwrap();
        assert!(timeline.credits_requests().is_empty());
        timeline.step(320).unwrap();
        let requests = timeline.credits_requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].text_key.as_deref(), Some("credits"));
        assert_eq!(requests[1].word, Some(2f32.to_bits()));
        timeline.step(320).unwrap();
        assert_eq!(timeline.credits_requests().len(), 3);
    }
    #[test]
    fn completion_hold_waits_for_clear_or_forced_release_and_resets() {
        use giants_formats::flick::{Command, Program};
        let c = |offset, opcode, words, label| Command {
            offset,
            opcode,
            words,
            label,
        };
        let mut t = super::Timeline::new(
            Program {
                commands: vec![
                    c(0, 40, vec![84], Some("hold".into())),
                    c(12, 9, vec![2], None),
                    c(24, 40, vec![0], None),
                    c(36, 5, vec![], None),
                ],
                next: None,
                program_end: 44,
            },
            160,
        )
        .unwrap();
        t.step(0).unwrap();
        assert_eq!(t.completion_hold(), Some("hold"));
        assert!(!t.release_ready(true, false));
        assert!(t.release_ready(false, true));
        t.step(319).unwrap();
        assert!(!t.release_ready(true, false));
        t.step(320).unwrap();
        assert!(t.release_ready(true, false));
        assert!(!t.release_ready(false, false));
        t.reset();
        assert!(t.completion_hold().is_none());
        t.step(0).unwrap();
        assert_eq!(t.completion_hold(), Some("hold"));
    }
    #[test]
    fn initial_fade_has_independent_state_dispatch_time_and_frozen_inspection() {
        use giants_formats::flick::{Command, Program};
        let command = |offset, opcode, words| Command {
            offset,
            opcode,
            words,
            label: None,
        };
        let mut t = super::Timeline::new(
            Program {
                commands: vec![
                    command(24, 31, vec![0, 0]),
                    command(40, 9, vec![30]),
                    command(52, 31, vec![1f32.to_bits(), 2f32.to_bits()]),
                    command(68, 5, vec![]),
                ],
                next: None,
                program_end: 76,
            },
            160,
        )
        .unwrap();
        let precision = crate::arithmetic::Precision::Double53;
        t.step(0).unwrap();
        assert_eq!(t.initial_fade().alpha_byte(precision), Ok(255));
        assert_eq!(t.fade().alpha_byte(precision), Ok(0));
        t.step(4799).unwrap();
        assert_eq!(t.initial_fade().envelope.speed, 0.);
        t.step(4800).unwrap();
        assert_eq!(t.initial_fade().envelope.speed, 0.5);
        // Dispatch/end alone must not consume envelope time.
        assert_eq!(t.initial_fade().envelope.fraction, 0.);
        t.advance_frame(0.5, precision).unwrap();
        assert_eq!(t.initial_fade().envelope.fraction, 0.25);
        assert_eq!(t.initial_fade().alpha_byte(precision), Ok(191));
        let source_time = [0, 20000, 160, 30];
        assert_eq!(
            t.sampled_initial_fade(7200, source_time)
                .unwrap()
                .envelope
                .fraction,
            0.25
        );
        assert_eq!(
            t.sampled_initial_fade(19200, source_time)
                .unwrap()
                .alpha_byte(precision),
            Ok(0)
        );
        assert!(t.sampled_initial_fade(-1, source_time).is_err());
        assert!(t.sampled_initial_fade(0, [0, 20000, 0, 30]).is_err());
        t.reset();
        assert_eq!(t.initial_fade().envelope.fraction, 1.);
        assert_eq!(t.initial_fade().envelope.speed, 0.);
        t.step(0).unwrap();
        assert_eq!(t.initial_fade().alpha_byte(precision), Ok(255));
    }
    #[test]
    fn screen_fade_waits_for_dispatch_advances_at_render_boundary_and_resets() {
        use giants_formats::flick::{Command, Program};
        let c = |offset, opcode, words| Command {
            offset,
            opcode,
            words,
            label: None,
        };
        let mut t = super::Timeline::new(
            Program {
                commands: vec![
                    c(24, 9, vec![3]),
                    c(36, 37, vec![0, 0, 0, 1f32.to_bits(), 0.25f32.to_bits()]),
                    c(64, 5, vec![]),
                ],
                next: None,
                program_end: 72,
            },
            160,
        )
        .unwrap();
        t.step(160).unwrap();
        assert_eq!(t.fade().envelope.speed, 0.);
        t.step(480).unwrap();
        assert_eq!(t.fade().envelope.speed, -4.);
        assert_eq!(t.fade().envelope.fraction, 1.);
        t.advance_frame(0.125, crate::arithmetic::Precision::Double53)
            .unwrap();
        assert_eq!(
            t.fade().alpha_byte(crate::arithmetic::Precision::Double53),
            Ok(127)
        );
        t.reset();
        assert_eq!(t.fade().envelope.fraction, 1.);
        assert_eq!(t.fade().envelope.speed, 0.);
    }
    #[test]
    fn scales_replace_fields_at_dispatch_and_reset_to_declared_values() {
        use giants_formats::flick::{Command, Program};
        let c = |offset, opcode, words| Command {
            offset,
            opcode,
            words,
            label: None,
        };
        let mut t = super::Timeline::new(
            Program {
                commands: vec![
                    c(24, 33, vec![1, 2f32.to_bits()]),
                    c(40, 9, vec![3]),
                    c(52, 33, vec![1, 3f32.to_bits()]),
                    c(68, 33, vec![u32::MAX, f32::NAN.to_bits()]),
                    c(84, 5, vec![]),
                ],
                next: None,
                program_end: 92,
            },
            160,
        )
        .unwrap();
        assert_eq!(t.actor_scale(1, 7.), Some(7.));
        t.step(160).unwrap();
        assert_eq!(t.actor_scale(1, 7.), Some(2.));
        assert_eq!(t.actor_scale(0, 7.), Some(7.));
        t.step(480).unwrap();
        assert_eq!(t.actor_scale(1, 7.), Some(3.));
        assert_eq!(t.actor_scale(32, 7.), None);
        t.reset();
        assert_eq!(t.actor_scale(1, 7.), Some(7.));
    }
    #[test]
    fn render_gates_follow_waits_without_clearing_sticky_lens_gate() {
        use giants_formats::flick::{Command, Program};
        let c = |offset, opcode, words| Command {
            offset,
            opcode,
            words,
            label: None,
        };
        let mut t = super::Timeline::new(
            Program {
                commands: vec![
                    c(24, 45, vec![]),
                    c(32, 46, vec![]),
                    c(40, 9, vec![3]),
                    c(52, 6, vec![]),
                    c(60, 5, vec![]),
                ],
                next: None,
                program_end: 68,
            },
            160,
        )
        .unwrap();
        t.render_policy.auxiliary_word = 123;
        t.step(479).unwrap();
        assert!(t.render_policy().actor_auxiliary_disabled && t.render_policy().lens_disabled);
        assert_eq!(t.render_policy().auxiliary_word, 123);
        t.step(480).unwrap();
        assert!(!t.render_policy().actor_auxiliary_disabled);
        assert!(t.render_policy().lens_disabled);
        assert_eq!(t.render_policy().auxiliary_word, 0);
        t.reset();
        assert_eq!(t.render_policy(), Default::default());
    }
    #[test]
    fn local_offsets_follow_waits_preserve_short_fields_and_reset() {
        use giants_formats::flick::{Command, Program};
        let command = |offset, opcode, words| Command {
            offset,
            opcode,
            words,
            label: None,
        };
        let program = Program {
            commands: vec![
                command(
                    24,
                    32,
                    vec![
                        2f32.to_bits(),
                        3f32.to_bits(),
                        5f32.to_bits(),
                        7f32.to_bits(),
                        1,
                    ],
                ),
                command(52, 9, vec![3]),
                command(64, 32, vec![11f32.to_bits(), 13f32.to_bits(), 1]),
                command(84, 32, vec![f32::NAN.to_bits(), 0, u32::MAX]),
                command(104, 5, vec![]),
            ],
            next: None,
            program_end: 112,
        };
        let mut t = super::Timeline::new(program, 160).unwrap();
        t.step(479).unwrap();
        let initial = t.actor_offset(1).unwrap();
        assert_eq!(initial.position, [2., 3., 5.]);
        assert_eq!(initial.angles, [0., 0., 7.]);
        t.step(480).unwrap();
        assert_eq!(t.actor_offset(1).unwrap().position, [11., 13., 5.]);
        assert_eq!(t.actor_offset(1).unwrap().angles, initial.angles);
        assert!(!t.actor_offset(0).unwrap().enabled);
        t.reset();
        assert!(!t.actor_offset(1).unwrap().enabled);
        assert_eq!(t.actor_offset(32), None);
    }
    use super::*;
    #[test]
    fn ordinary_range_changes_only_after_its_wait_and_resets_per_director() {
        use giants_formats::flick::Command;
        let command = |offset, opcode, words| Command {
            offset,
            opcode,
            words,
            label: None,
        };
        let program = Program {
            commands: vec![
                command(24, 9, vec![2]),
                command(36, 38, vec![500f32.to_bits()]),
                command(48, 9, vec![4]),
                command(60, 38, vec![2000f32.to_bits()]),
                command(72, 5, vec![]),
            ],
            next: None,
            program_end: 80,
        };
        let mut timeline = Timeline::new(program.clone(), 160).unwrap();
        assert_eq!(timeline.ordinary_range(), 1000.);
        timeline.step(319).unwrap();
        assert_eq!(timeline.ordinary_range(), 1000.);
        timeline.step(320).unwrap();
        assert_eq!(timeline.ordinary_range(), 500.);
        timeline.step(639).unwrap();
        assert_eq!(timeline.ordinary_range(), 500.);
        timeline.step(640).unwrap();
        assert_eq!(timeline.ordinary_range(), 2000.);
        timeline.reset();
        assert_eq!(timeline.ordinary_range(), 1000.);
        assert_eq!(Timeline::new(program, 160).unwrap().ordinary_range(), 1000.);
    }
    #[test]
    fn unobserved_invalid_range_stays_an_explicit_refusal() {
        for value in [f32::NAN, f32::INFINITY, -1.] {
            let program = Program {
                commands: vec![giants_formats::flick::Command {
                    offset: 24,
                    opcode: 38,
                    words: vec![value.to_bits()],
                    label: None,
                }],
                next: None,
                program_end: 36,
            };
            let mut timeline = Timeline::new(program, 160).unwrap();
            assert_eq!(
                timeline.step(160).err(),
                Some("ordinary range outside finite nonnegative profile")
            );
            assert_eq!(timeline.ordinary_range(), 1000.);
        }
    }
    #[test]
    fn camera_dispatch_obeys_wait_sync_order_and_reset() {
        use giants_formats::flick::Command;
        let command = |offset, opcode, words, label| Command {
            offset,
            opcode,
            words,
            label,
        };
        let program = Program {
            commands: vec![
                command(24, 9, vec![2], None),
                command(36, 4, vec![0, 0, 100], None),
                command(56, 9, vec![4], None),
                command(68, 52, vec![120], Some("sync".into())),
                command(80, 4, vec![0, 0, 110], None),
                command(100, 5, vec![], None),
            ],
            next: None,
            program_end: 108,
        };
        let mut t = Timeline::with_start(program, 160, 640).unwrap();
        assert!(!t.has_stepped());
        assert_eq!(t.camera_request_offset(), None);
        t.step(319).unwrap();
        assert!(t.has_stepped());
        assert_eq!(t.camera_request_offset(), None);
        t.step(320).unwrap();
        assert_eq!(t.camera_request_offset(), Some(36));
        t.step(639).unwrap();
        assert_eq!(t.camera_request_offset(), Some(36));
        t.step(640).unwrap();
        assert_eq!(t.camera_request_offset(), Some(36));
        t.step(641).unwrap();
        assert_eq!(t.camera_request_offset(), Some(80));
        t.step(900).unwrap();
        assert_eq!(t.camera_request_offset(), Some(80));
        t.reset();
        assert!(!t.has_stepped());
        assert_eq!(t.camera_request_offset(), None);
        t.step(320).unwrap();
        assert_eq!(t.camera_request_offset(), Some(36));
    }
    #[test]
    fn frame_transition_does_not_change_supplied_audio_gain_or_advance_from_ticks() {
        use giants_formats::flick::Command;
        let program = Program {
            commands: vec![
                Command {
                    offset: 24,
                    opcode: 7,
                    words: vec![0.25f32.to_bits(), 0.5f32.to_bits()],
                    label: None,
                },
                Command {
                    offset: 40,
                    opcode: 44,
                    words: vec![0],
                    label: Some("voice".into()),
                },
                Command {
                    offset: 52,
                    opcode: 5,
                    words: vec![],
                    label: None,
                },
            ],
            next: None,
            program_end: 60,
        };
        let mut timeline = Timeline::with_audio_gain(program, 160, 160, 1.).unwrap();
        let events = timeline.step(160).unwrap().events;
        assert_eq!(events[0].gain_db100, 0);
        assert_eq!(timeline.frame().fraction, 1.);
        timeline.step(10000).unwrap();
        assert_eq!(timeline.frame().fraction, 1.);
        timeline
            .advance_frame(0.25, crate::arithmetic::Precision::Double53)
            .unwrap();
        assert_eq!(timeline.frame().fraction, 0.625);
        timeline.reset();
        assert_eq!(timeline.frame().fraction, 1.);
        assert_eq!(timeline.step(160).unwrap().events[0].gain_db100, 0);
    }
    #[test]
    fn visibility_waits_preserves_state_and_reset_restores_initial_gate() {
        use giants_formats::flick::Command;
        let command = |offset, opcode, words| Command {
            offset,
            opcode,
            words,
            label: None,
        };
        let p = Program {
            commands: vec![
                command(24, 12, vec![2]),
                command(36, 9, vec![3]),
                command(48, 11, vec![2]),
                command(60, 5, vec![]),
            ],
            next: None,
            program_end: 68,
        };
        let mut timeline = Timeline::new(p, 160).unwrap();
        assert_eq!(timeline.actor_visible(2), Some(true));
        timeline.step(160).unwrap();
        assert_eq!(timeline.actor_visible(2), Some(false));
        timeline.step(479).unwrap();
        assert_eq!(timeline.actor_visible(2), Some(false));
        timeline.step(480).unwrap();
        assert_eq!(timeline.actor_visible(2), Some(true));
        assert_eq!(timeline.actor_visible(32), None);
        timeline.reset();
        assert_eq!(timeline.actor_visible(2), Some(true));
    }
    #[test]
    fn showing_a_fully_transparent_actor_does_not_make_it_drawable() {
        use giants_formats::flick::Command;
        let command = |offset, opcode, words| Command {
            offset,
            opcode,
            words,
            label: None,
        };
        let mut timeline = Timeline::new(
            Program {
                commands: vec![
                    command(0, 12, vec![2]),
                    command(12, 42, vec![2, 1f32.to_bits(), 0]),
                    command(32, 11, vec![2]),
                    command(44, 9, vec![2]),
                    command(56, 42, vec![2, 0, 0]),
                    command(76, 5, vec![]),
                ],
                next: None,
                program_end: 84,
            },
            160,
        )
        .unwrap();
        assert_eq!(timeline.actor_render_visible(2), Some(true));
        timeline.step(160).unwrap();
        assert_eq!(timeline.actor_visible(2), Some(true));
        assert_eq!(timeline.actor_render_visible(2), Some(false));
        assert_eq!(timeline.actor_render_visible(1), Some(true));
        assert_eq!(timeline.actor_render_visible(32), None);
        timeline.step(10000).unwrap();
        assert_eq!(timeline.actor_render_visible(2), Some(true));
        timeline.reset();
        assert_eq!(timeline.actor_render_visible(2), Some(true));
    }
    #[test]
    fn prepared_actor_waits_for_declaration_and_restart_rearms_it() {
        use giants_formats::flick::Command;
        let c = |offset, opcode, words| Command {
            offset,
            opcode,
            words,
            label: None,
        };
        let mut timeline = Timeline::new(
            Program {
                commands: vec![
                    c(24, 2, vec![0, 0, 2, 2]),
                    c(48, 11, vec![6]), // Enabled does not construct the future actor.
                    c(60, 9, vec![100]),
                    c(72, 2, vec![u32::MAX, 0, 6, 1]),
                    c(96, 12, vec![2]),
                    c(108, 5, vec![]),
                ],
                next: None,
                program_end: 116,
            },
            160,
        )
        .unwrap();
        assert_eq!(timeline.actor_requested(2), Some(false));
        timeline.step(160).unwrap();
        assert_eq!(timeline.actor_requested(2), Some(true));
        assert_eq!(timeline.actor_visible(6), Some(true));
        assert_eq!(timeline.actor_requested(6), Some(false));
        timeline.step(15999).unwrap();
        assert_eq!(timeline.actor_requested(6), Some(false));
        timeline.step(16000).unwrap();
        assert_eq!(timeline.actor_requested(6), Some(true));
        assert_eq!(timeline.actor_visible(2), Some(false));
        assert_eq!(timeline.actor_requested(2), Some(true));
        assert_eq!(timeline.actor_requested(32), None);
        timeline.reset();
        assert_eq!(timeline.actor_requested(6), Some(false));
        timeline.step(160).unwrap();
        assert_eq!(timeline.actor_requested(2), Some(true));
        assert_eq!(timeline.actor_requested(6), Some(false));
    }
    #[test]
    fn sync_waits_for_director_start_then_emits_once_and_restarts() {
        use giants_formats::flick::Command;
        let p = Program {
            commands: vec![
                Command {
                    offset: 24,
                    opcode: 52,
                    words: vec![0],
                    label: Some("track".into()),
                },
                Command {
                    offset: 36,
                    opcode: 5,
                    words: vec![],
                    label: None,
                },
            ],
            next: None,
            program_end: 44,
        };
        let mut timeline = Timeline::with_start(p, 160, 320).unwrap();
        for tick in [0, 160, 320, 320] {
            assert!(timeline.step(tick).unwrap().events.is_empty());
        }
        let events = timeline.step(321).unwrap().events;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::SyncTrack);
        assert_eq!(events[0].tick, 321);
        assert!(timeline.step(640).unwrap().events.is_empty());
        timeline.reset();
        assert_eq!(timeline.step(400).unwrap().events.len(), 1);
        for name in ["../escape", "a/b", "a\\b", "", ".", "a:b"] {
            assert!(sequence_path(std::path::Path::new("."), name).is_err());
        }
    }
    #[test]
    fn waits_use_absolute_frames_and_end_is_held() {
        use giants_formats::flick::Command;
        let p = Program {
            commands: vec![
                Command {
                    offset: 24,
                    opcode: 9,
                    words: vec![2],
                    label: None,
                },
                Command {
                    offset: 36,
                    opcode: 44,
                    words: vec![0],
                    label: Some("voice".into()),
                },
                Command {
                    offset: 48,
                    opcode: 5,
                    words: vec![],
                    label: None,
                },
            ],
            next: None,
            program_end: 56,
        };
        let mut t = Timeline::new(p, 160).unwrap();
        assert!(t.step(319).unwrap().events.is_empty());
        let step = t.step(320).unwrap();
        assert_eq!(step.dispatch, [24, 36, 48]);
        assert_eq!(step.events.len(), 1);
        assert_eq!(step.cursor, 48);
        assert!(t.step(321).unwrap().events.is_empty());
        assert!(t.step(319).is_err());
        t.reset();
        assert_eq!(t.step(320).unwrap().events.len(), 1);
    }
    #[test]
    fn actor_transparency_dispatch_is_per_slot_render_driven_and_resettable() {
        use giants_formats::flick::{Command, Program};
        let command = |offset, opcode, words| Command {
            offset,
            opcode,
            words,
            label: None,
        };
        let mut timeline = super::Timeline::new(
            Program {
                commands: vec![
                    command(0, 42, vec![u32::MAX, f32::NAN.to_bits(), 0]),
                    command(20, 9, vec![2]),
                    command(32, 42, vec![1, 0.75f32.to_bits(), 1f32.to_bits()]),
                    command(52, 5, vec![]),
                ],
                next: None,
                program_end: 60,
            },
            160,
        )
        .unwrap();
        timeline.step(320).unwrap();
        assert_eq!(timeline.actor_transparency(1).unwrap().fraction, 0.);
        timeline.step(321).unwrap();
        assert_eq!(timeline.actor_transparency(1).unwrap().speed, 0.75);
        timeline
            .advance_frame(1., crate::arithmetic::Precision::Double53)
            .unwrap();
        let value = timeline.actor_transparency(1).unwrap();
        assert_eq!((value.fraction, value.speed), (0.75, 0.75));
        assert_eq!(timeline.actor_transparency(0).unwrap().fraction, 0.);
        timeline
            .advance_frame(0.1, crate::arithmetic::Precision::Double53)
            .unwrap();
        assert_eq!(timeline.actor_transparency(1).unwrap().speed, 0.);
        timeline.reset();
        let value = timeline.actor_transparency(1).unwrap();
        assert_eq!((value.fraction, value.target, value.speed), (0., 0., 0.));
    }

    #[test]
    fn shadow_declarations_wait_replace_per_actor_and_clear_on_restart() {
        use giants_formats::flick::{Command, Program};
        let command = |offset, opcode, words| Command {
            offset,
            opcode,
            words,
            label: None,
        };
        let mut timeline = super::Timeline::new(
            Program {
                commands: vec![
                    command(0, 34, vec![2, 6, 5, 1, 2, 3, 4]),
                    command(36, 9, vec![2]),
                    command(48, 34, vec![2, 7, 6, 5, 6, 7, 8]),
                    command(84, 5, vec![]),
                ],
                next: None,
                program_end: 92,
            },
            160,
        )
        .unwrap();
        assert_eq!(timeline.shadow_declaration(2), None);
        assert_eq!(timeline.reached_command_offset(), None);
        timeline.step(160).unwrap();
        assert_eq!(timeline.reached_command_offset(), Some(0));
        assert_eq!(
            timeline.shadow_declaration(2).unwrap().texture_words,
            [6, 5]
        );
        assert_eq!(timeline.shadow_declaration(3), None);
        timeline.step(321).unwrap();
        assert_eq!(
            timeline.shadow_declaration(2).unwrap().texture_words,
            [7, 6]
        );
        assert_eq!(timeline.shadow_declaration(2).unwrap().command_offset, 48);
        timeline.reset();
        assert_eq!(timeline.shadow_declaration(2), None);
        assert_eq!(timeline.reached_command_offset(), None);
        timeline.step(160).unwrap();
        assert_eq!(
            timeline.shadow_declaration(2).unwrap().texture_words,
            [6, 5]
        );
    }
}
