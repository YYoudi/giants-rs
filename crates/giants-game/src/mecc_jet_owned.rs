//! Source-owned normal jet phase, children and deferred sound sequences.
//! Devices are mandatory providers. Sequence creation is pending; no global
//! sequence update, emitted particle, renderer or whole primary is implied.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    audio_sequence::{Backend, Phase, Playback, Route},
    mecc_audio::{self, JetInputs, JetRequest, Sequence},
    mecc_jet,
    mecc_physical_cycle::View,
    mecc_wield::Factory,
    resident_states::{Id as Actor, Session},
};
use giants_formats::global_data::GlobalData;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Id {
    ticket: u64,
    serial: u64,
}
struct Entry {
    id: Id,
    actor: Actor,
    playback: Playback<u32>,
}
pub struct Pool {
    ticket: u64,
    next: u64,
    entries: Vec<Entry>,
    last_update: u64,
    failure: Option<String>,
    owner_ticket: Option<u64>,
}
pub struct SequenceView {
    pub id: Id,
    pub actor: Actor,
    pub definition: Sequence,
    pub phase: Phase,
    pub remaining: f32,
    pub duration: f32,
}
impl Pool {
    pub fn new(ticket: u64) -> Self {
        Self {
            ticket,
            next: 0,
            entries: vec![],
            last_update: 0,
            failure: None,
            owner_ticket: None,
        }
    }
    pub fn views(&self) -> Vec<SequenceView> {
        self.entries
            .iter()
            .map(|e| SequenceView {
                id: e.id,
                actor: e.actor,
                definition: e.playback.definition,
                phase: e.playback.phase(),
                remaining: e.playback.remaining(),
                duration: e.playback.release_duration(),
            })
            .collect()
    }
    fn require(&self, id: Id, actor: Actor) -> Result<usize, String> {
        self.entries
            .iter()
            .position(|e| e.id == id && e.actor == actor)
            .ok_or_else(|| "jet sequence absent, foreign or stale".into())
    }
    pub fn create(
        &mut self,
        actor: Actor,
        key: u64,
        definition: Sequence,
        backend: &mut dyn Backend<Handle = u32>,
    ) -> Result<Id, String> {
        if self
            .owner_ticket
            .is_some_and(|ticket| ticket != actor.ticket())
            || self.failure.is_some()
        {
            return Err("jet sound queue is foreign or its update failed".into());
        }
        let serial = self
            .next
            .checked_add(1)
            .ok_or("jet sequence identity exhausted")?;
        let playback = Playback::new(definition, key, Route::Local)?;
        self.owner_ticket = Some(actor.ticket());
        let id = Id {
            ticket: self.ticket,
            serial,
        };
        self.entries.push(Entry {
            id,
            actor,
            playback,
        });
        self.next = serial;
        // Append first, then retire only the first older matching root/cues,
        // including an already interrupted sequence, as in D49c8cf..49c99b.
        if let Some(index) = self.entries.iter().position(|e| {
            e.id != id
                && e.actor == actor
                && e.playback.definition.start == definition.start
                && e.playback.definition.continuous == definition.continuous
                && e.playback.definition.release == definition.release
        }) {
            if let Err(error) = self.entries[index].playback.cancel(backend) {
                self.failure = Some(error.clone());
                return Err(error);
            }
            self.entries.remove(index);
        }
        Ok(id)
    }
    pub fn interrupt(
        &mut self,
        id: Id,
        actor: Actor,
        backend: &mut dyn Backend<Handle = u32>,
    ) -> Result<(), String> {
        if self.failure.is_some() {
            return Err("jet sound update failed; only cleanup remains allowed".into());
        }
        let index = self.require(id, actor)?;
        let result = self.entries[index].playback.interrupt(backend);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    /// One explicitly scheduled local-player sequence pass. Caller placement
    /// among world phases is separate; this does not acknowledge a device.
    /// Partial backend failures quarantine the pass instead of replaying it.
    pub fn update(
        &mut self,
        number: u64,
        seconds: f32,
        paused: bool,
        precision: Precision,
        backend: &mut dyn Backend<Handle = u32>,
    ) -> Result<(), String> {
        if self.failure.is_some()
            || self.last_update.checked_add(1) != Some(number)
            || !seconds.is_finite()
            || seconds < 0.
        {
            return Err("jet sound pass is failed, stale or has unsupported time".into());
        }
        self.last_update = number;
        for entry in &mut self.entries {
            if let Err(error) = entry
                .playback
                .update(seconds, paused, false, precision, backend)
            {
                self.failure = Some(error.clone());
                return Err(error);
            }
        }
        self.entries
            .retain(|entry| entry.playback.phase() != Phase::Finished);
        Ok(())
    }
    /// Keep ownership on any backend failure; a subsequent cancellation may retry.
    /// This is cancellation, not a successful native global audio update.
    pub fn clear(&mut self, backend: &mut dyn Backend<Handle = u32>) -> Result<(), String> {
        for entry in &mut self.entries {
            entry.playback.cancel(backend)?;
        }
        self.entries.clear();
        Ok(())
    }
}

#[cfg(test)]
mod pool_tests {
    use super::*;
    struct Device {
        plays: u32,
        stopped: Vec<u32>,
        stop_fails: bool,
    }
    impl Backend for Device {
        type Handle = u32;
        fn stop_sound(&mut self, _: u32, _: u64) -> Result<(), String> {
            Ok(())
        }
        fn play(&mut self, _: u32, _: u64, _: Route) -> Result<u32, String> {
            self.plays += 1;
            if self.plays == 3 {
                Err("missing device voice".into())
            } else {
                Ok(self.plays)
            }
        }
        fn duration(&mut self, _: u32) -> Result<f32, String> {
            Ok(0.25)
        }
        fn volume(&mut self, _: u32, _: f32) -> Result<(), String> {
            Ok(())
        }
        fn stop(&mut self, handle: u32) -> Result<(), String> {
            self.stopped.push(handle);
            if std::mem::take(&mut self.stop_fails) {
                Err("cleanup not ready".into())
            } else {
                Ok(())
            }
        }
    }
    #[test]
    fn failed_pass_keeps_handles_for_cleanup_and_never_replays_audio() {
        let seed = crate::state_storage::Seed::from_program_for(
            &giants_formats::class_program::parse(&[0x1e, 0, 0xff]).unwrap(),
            crate::state_storage::Profile::Debug,
            Precision::Double53,
        )
        .unwrap();
        let mut session = Session::new(71);
        let actor = session
            .register(2, &seed, &[], crate::object_lists::DirectFlags::default())
            .unwrap();
        let mut foreign = Session::new(72);
        let other = foreign
            .register(2, &seed, &[], crate::object_lists::DirectFlags::default())
            .unwrap();
        let mut pool = Pool::new(71);
        let mut device = Device {
            plays: 0,
            stopped: Vec::new(),
            stop_fails: true,
        };
        let definition = Sequence {
            start: 1,
            continuous: 2,
            release: 3,
            flags: 3,
        };
        let first = pool.create(actor, 1, definition, &mut device).unwrap();
        assert!(pool.create(other, 1, definition, &mut device).is_err());
        assert_eq!(pool.views().len(), 1);
        pool.create(
            actor,
            1,
            Sequence {
                start: 4,
                continuous: 5,
                release: 6,
                flags: 3,
            },
            &mut device,
        )
        .unwrap();
        assert!(
            pool.update(1, f32::NAN, false, Precision::Double53, &mut device)
                .is_err()
        );
        assert_eq!(device.plays, 0);
        pool.update(1, 0., true, Precision::Double53, &mut device)
            .unwrap();
        assert_eq!(device.plays, 0);
        assert!(
            pool.update(2, 0.1, false, Precision::Double53, &mut device)
                .is_err()
        );
        assert_eq!(device.plays, 3);
        assert_eq!(pool.views()[0].phase, Phase::Running);
        assert_eq!(pool.views()[1].phase, Phase::Failed);
        assert!(
            pool.update(3, 0.1, false, Precision::Double53, &mut device)
                .is_err()
        );
        assert!(pool.create(actor, 1, definition, &mut device).is_err());
        assert!(pool.interrupt(first, actor, &mut device).is_err());
        assert_eq!(device.plays, 3);
        assert!(pool.clear(&mut device).is_err());
        assert_eq!(pool.views().len(), 2);
        pool.clear(&mut device).unwrap();
        assert!(pool.views().is_empty());
        assert_eq!(device.stopped, [2, 2]);
        assert!(
            pool.update(3, 0.1, false, Precision::Double53, &mut device)
                .is_err()
        );
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct Owner {
    pub pool: u64,
    pub key: u64,
    pub current: Option<Id>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub buttons: u32,
    pub paused: bool,
    pub disable_boost: bool,
    pub suppressed: bool,
    /// Actual owner key expected by the supplied audio backend, not a native pointer.
    pub audio_key: u64,
}
pub struct Services<'a, 'r> {
    pub wield: &'a mut Factory<'r>,
    pub sequences: &'a mut Pool,
    pub backend: &'a mut dyn Backend<Handle = u32>,
    pub input: Input,
}
pub struct Projection {
    pub prefix: [u32; 13],
    pub current: Option<Id>,
}
pub(crate) struct Frame<'a> {
    pub fields: &'a Fields,
    pub payload: &'a [u8],
    pub view: &'a View,
    pub globals: &'a GlobalData,
    pub seconds: f64,
    pub controls: [i8; 2],
}
pub(crate) struct Output {
    pub fields: Fields,
    pub payload: Vec<u8>,
    pub projection: Projection,
}
pub(crate) trait PhaseDriver {
    fn preflight(&self, session: &Session, actor: Actor) -> Result<(), String>;
    fn buttons(&self) -> u32;
    fn run(
        &mut self,
        session: &mut Session,
        actor: Actor,
        frame: Frame<'_>,
    ) -> Result<Output, String>;
}
pub(crate) struct Driver<'a> {
    pub fire: bool,
    pub sequences: &'a mut Pool,
    pub backend: &'a mut dyn Backend<Handle = u32>,
    pub input: Input,
}
fn word(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}
fn put(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
impl PhaseDriver for Driver<'_> {
    fn preflight(&self, session: &Session, actor: Actor) -> Result<(), String> {
        session.check_wield_owner(actor)?;
        if self.input.buttons & !(0xc00 | if self.fire { 0x1001 } else { 0 }) != 0
            || self.input.audio_key == 0
        {
            return Err(
                "owned jet requires declared jet/boost input and actual audio owner key".into(),
            );
        }
        if let Some(owner) = session.mecc_jet_owner(actor) {
            if owner.pool != self.sequences.ticket || owner.key != self.input.audio_key {
                return Err("owned jet sound pool or owner key changed".into());
            }
            if let Some(id) = owner.current {
                self.sequences.require(id, actor)?;
            }
        }
        Ok(())
    }
    fn buttons(&self) -> u32 {
        self.input.buttons
    }
    fn run(
        &mut self,
        session: &mut Session,
        actor: Actor,
        frame: Frame<'_>,
    ) -> Result<Output, String> {
        self.preflight(session, actor)?;
        let mut fields = frame.fields.clone();
        let mut body = frame.payload.to_vec();
        if body.len() != 524
            || [0x1e0, 0x144].into_iter().any(|at| word(&body, at) != 0)
            || word(&body, 0x190) != u32::MAX
            || fields.control_flags & !0xbc00 != 0
        {
            return Err(
                "owned jet requires unmounted normal cold inventory and owned sequence binding"
                    .into(),
            );
        }
        let f = |at| f32::from_bits(word(&body, at));
        if f64::from(frame.seconds as f32) != frame.seconds {
            return Err("owned jet requires the acquired exactly representable local delta".into());
        }
        let step = mecc_jet::step_with_ceiling(
            mecc_jet::Input {
                state: mecc_jet::State {
                    fuel: f(0x4c),
                    boost: f(0xc0),
                    boost_cooldown: f(0xc4),
                    ignition_delay: f(0x138),
                    boost_active: word(&body, 0xc8) != 0,
                },
                matrix: frame.view.affine.ok_or("jet affine absent")?.matrix,
                force: [f(0x9c), f(0xa0), f(0xa4)],
                scale: fields.scale,
                height: fields.position[2],
                dt: frame.seconds as f32,
                controls: frame.controls,
                // Historical kernel names: object_flags is native E0; motion_flags D8.
                object_flags: fields.control_flags,
                motion_flags: fields.object_flags,
                input_flags: self.input.buttons,
                physics_flags: word(&body, 0x84),
                recharge_blocked: false,
                recharge_equipment: false,
                disable_boost: self.input.disable_boost,
                paused: self.input.paused,
            },
            mecc_jet::Parameters::from_globals(frame.globals).map_err(|e| e.to_string())?,
            Precision::Double53,
            mecc_jet::HeightCeiling::DebugFull,
        )?;
        if step.shield_depleted {
            return Err("owned jet depletion services pending".into());
        }
        if step.boost_started {
            self.backend
                .play(0x5b3, self.input.audio_key, Route::Local)?;
        }
        for (at, value) in [0x4c, 0xc0, 0xc4, 0x138].into_iter().zip([
            step.state.fuel,
            step.state.boost,
            step.state.boost_cooldown,
            step.state.ignition_delay,
        ]) {
            put(&mut body, at, value.to_bits());
        }
        put(&mut body, 0xc8, step.state.boost_active as u32);
        body[0x13c] = step.throttle as u8;
        for (at, value) in [0x9c, 0xa0, 0xa4].into_iter().zip(step.force) {
            put(&mut body, at, value.to_bits());
        }
        fields.object_flags = step.motion_flags;
        session.publish_jet_children(actor, fields.object_flags, step.state.boost_active)?;
        // Mirror only after child routing: the release frame can retain the
        // prior2000 bit for one final visible child pass, as in D.
        fields.object_flags =
            (fields.object_flags & !0x2000) | ((fields.object_flags & 0x1000) << 1);
        let mut owner = session.mecc_jet_owner(actor).unwrap_or(Owner {
            pool: self.sequences.ticket,
            key: self.input.audio_key,
            current: None,
        });
        match mecc_audio::jet_request(JetInputs {
            thrust_active: step.motion_flags & 0x1000 != 0,
            suppressed: self.input.suppressed,
            sequence_exists: owner.current.is_some(),
            special_item: -1,
            ..Default::default()
        }) {
            JetRequest::None => {}
            JetRequest::Start(definition) => {
                owner.current = Some(self.sequences.create(
                    actor,
                    self.input.audio_key,
                    definition,
                    self.backend,
                )?);
            }
            JetRequest::Interrupt => {
                self.sequences
                    .interrupt(owner.current.unwrap(), actor, self.backend)?;
                owner.current = None;
            }
        }
        session.publish_mecc_jet_owner(actor, owner)?;
        Ok(Output {
            fields,
            payload: body,
            projection: Projection {
                prefix: step.words(),
                current: owner.current,
            },
        })
    }
}
