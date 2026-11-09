//! Owned positional-impact birth and bounded timed callback in source order.
//! Cue transport is required; actor retirement/world scheduling stay separate.
use crate::{
    actor_fields::Fields,
    attached_effect_context::{Id as ContextId, Pool},
    audio_parameters::Random,
    procedural_aim::Matrix,
    resident_states::{Construction, Id, Session},
    scene_resources::SceneResources,
    state_storage::{Profile, Seed},
};
use std::sync::Arc;

/// Read the acquired frame's cue range from the active source image, without
/// preparing a context, creating an actor or consuming either random stream.
pub fn cue_range(resources: &SceneResources, name: &str) -> Result<Option<[u32; 2]>, String> {
    let Some(effect) = resources.classes.models.effects.last() else {
        return Ok(None);
    };
    let Some((_, entry)) = effect.image.find(name) else {
        return Ok(None);
    };
    let p = effect
        .image
        .program(entry.offset)
        .map_err(|e| e.to_string())?;
    crate::effect_burst::Plan::positional_impact_frame(&effect.image, &p.records)?;
    Ok(Some([1, 5].map(|at| {
        u32::from_le_bytes(p.records[2].bytes[at..at + 4].try_into().unwrap())
    })))
}

#[derive(Clone)]
pub struct Metadata {
    /// Retained identity of the retired originating VM cursor, never an address.
    pub origin: ContextId,
    pub entry_offset: u32,
    pub matrix: Matrix,
    /// Native header EC/F0/F4/F8, distinct from the payload lifetime.
    pub fade: [f32; 4],
    /// Current native transparency (zero is opaque), retaining the birth-field
    /// identifier introduced before callback integration.
    pub initial_opacity: f32,
    /// All birth commands finished; lifetime and world completion stay separate.
    pub complete: bool,
    pub failure: Option<String>,
    pub(crate) visual: Option<crate::effect_visual::Visual>,
    pub(crate) cleanup_started: bool,
    source: Arc<giants_formats::effect_image::Image>,
}
impl Metadata {
    pub fn source_image(&self) -> &giants_formats::effect_image::Image {
        &self.source
    }
}
impl std::fmt::Debug for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PositionalImpact")
            .field("origin", &self.origin)
            .field("entry_offset", &self.entry_offset)
            .field("complete", &self.complete)
            .field("failure", &self.failure)
            .field("cleanup_started", &self.cleanup_started)
            .finish_non_exhaustive()
    }
}
/// A real provider receives the already-published actor and shared RNG at the
/// native cue boundary, before size/lifetime/angular commands execute.
/// This is the effect VM's RNG. It does not imply ownership by an audio DLL:
/// the acquired debug audio module uses its own CRT state. Keep those streams
/// separate; a provider may touch this stream only for an acquired VM-side draw.
pub trait Cue {
    fn play(
        &mut self,
        session: &mut Session,
        source: Id,
        cue: u32,
        random: &mut Random,
    ) -> Result<(), String>;
}
/// Expiry must stop the retained recipient-bound cue before deferred marking.
/// An implementation must use the actual transport; there is no default sink.
pub trait Stop {
    fn stop(&mut self, session: &mut Session, source: Id, cue: u32) -> Result<(), String>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    FirstFrame,
    Updated,
    Marked,
    Inactive,
}
struct Attempt {
    context: ContextId,
    actor: Id,
}
pub struct Factory<'r> {
    resources: &'r SceneResources,
    ticket: Option<u64>,
    attempts: Vec<Attempt>,
}
impl<'r> Factory<'r> {
    pub fn new(resources: &'r SceneResources) -> Self {
        Self {
            resources,
            ticket: None,
            attempts: vec![],
        }
    }
    pub fn actor(&self, context: ContextId) -> Option<Id> {
        self.attempts
            .iter()
            .find(|a| a.context == context)
            .map(|a| a.actor)
    }
    pub fn activate(
        &mut self,
        session: &mut Session,
        contexts: &mut Pool<Id>,
        context: ContextId,
        random: &mut Random,
        cue: &mut impl Cue,
    ) -> Result<Id, String> {
        if self.actor(context).is_some() {
            return Err("positional impact already started; replay is forbidden".into());
        }
        let c = contexts
            .get(context)
            .ok_or("positional impact context absent, foreign or retired")?
            .clone();
        let position = c
            .position()
            .ok_or("positional impact requires actual position binding")?;
        if c.flags() != 0 || c.recipient().is_some() {
            return Err("positional impact requires acquired flags0 VM state".into());
        }
        let image = c.source_image();
        if !self
            .resources
            .classes
            .models
            .effects
            .iter()
            .any(|e| e.image.same_image(&image))
        {
            return Err("positional impact image differs from constructor resources".into());
        }
        let p = c.program.as_ref().map_err(Clone::clone)?;
        crate::effect_burst::Plan::positional_impact_frame(&image, &p.records)
            .map_err(str::to_owned)?;
        let word = |r: usize, at: usize| {
            u32::from_le_bytes(p.records[r].bytes[at..at + 4].try_into().unwrap())
        };
        let class = word(0, 1) as i32;
        let sounds = [word(2, 1), word(2, 5)];
        let ranges = [
            [f32::from_bits(word(3, 1)), f32::from_bits(word(3, 5))],
            [f32::from_bits(word(3, 9)), f32::from_bits(word(3, 13))],
        ];
        let life = std::array::from_fn::<_, 3, _>(|i| f32::from_bits(word(4, 1 + i * 4)));
        let spin = std::array::from_fn::<_, 3, _>(|i| f32::from_bits(word(5, 1 + i * 4)));
        if ranges.iter().flatten().any(|v| !v.is_finite() || *v <= 0.) {
            return Err("positional impact sizes outside acquired positive profile".into());
        }
        let fade = fade(life)?;
        let template = Template::load(self.resources, class)?;
        let mut fields = template.fields.instantiate();
        fields.configuration[14] = (fields.configuration[14] & !0x20000) | 0x800;
        fields.configuration[22] = 50f32.to_bits();
        fields.object_flags |= 0x40010030;
        fields.control_flags |= 0x20;
        fields.position = position;
        fields.current = position;
        // Admit all possible interval results before publishing an actor/cue.
        for scale in ranges[0] {
            let matrix = crate::effect_orientation::matrix(position, fields.angles, scale)
                .map_err(str::to_owned)?;
            crate::effect_orientation::from_matrix(matrix).map_err(str::to_owned)?;
        }
        let mut matrix = Matrix::identity();
        matrix.flags = 0x10;
        matrix.values[3] = position[0];
        matrix.values[7] = position[1];
        matrix.values[11] = position[2];
        let mut seed = template.seed.instantiate()?;
        let body = seed.storage.data_mut().unwrap();
        put(body, 0, 1); // Explicit originating-cursor presence; identity is owned above.
        for at in [0xa4, 0xa8, 0xb0] {
            put(body, at, 1f32.to_bits());
        }
        let flags = crate::object_lists::DirectFlags {
            active: fields.object_flags,
            personality: fields.configuration[14],
        };
        contexts.claim_positional_impact(context)?;
        let constructed = session.construct_positional_impact(
            Construction {
                class,
                seed: &seed,
                bindings: &template.bindings,
                flags,
                specification: template.specification,
                state_resources: template.states,
                fields,
            },
            Metadata {
                origin: context,
                entry_offset: c.entry_offset,
                matrix,
                fade: [0.; 4],
                initial_opacity: 0.,
                complete: false,
                failure: None,
                visual: None,
                cleanup_started: false,
                source: image,
            },
            self.ticket,
        );
        let actor = match constructed {
            Ok(id) => id,
            Err(error) => {
                contexts.undo_positional_claim(context);
                return Err(error);
            }
        };
        self.ticket = Some(actor.ticket());
        self.attempts.push(Attempt { context, actor });
        let result: Result<Id, String> = (|| {
            let draw = random.next_15();
            let id =
                sounds[0] + ((u64::from(draw) * u64::from(sounds[1] - sounds[0] + 1)) >> 15) as u32;
            session.positional_impact_cue(actor, id)?;
            cue.play(session, actor, id, random)?;
            let sizes = ranges.map(|r| crate::timed_effect::sample(&mut random.0, r));
            session.configure_positional_impact(actor, context, sizes, life, spin, fade)?;
            contexts.complete_positional_impact(context)?;
            session.complete_positional_impact(actor)?;
            Ok(actor)
        })();
        if let Err(ref error) = result {
            session.fail_positional_impact(actor, error);
        }
        result
    }
}
fn put(body: &mut [u8], at: usize, value: u32) {
    body[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn fade(life: [f32; 3]) -> Result<[f32; 4], String> {
    let [duration, fade_in, fade_out] = life;
    if !life.into_iter().all(f32::is_finite) || duration <= 0. || fade_in < 0. || fade_out < 0. {
        return Err("positional impact lifetime outside finite timed profile".into());
    }
    let v = [
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
    if !v.into_iter().all(f32::is_finite) {
        return Err("positional impact fade reciprocal overflow".into());
    }
    Ok(v)
}
struct Template {
    seed: Seed,
    fields: Fields,
    bindings: Vec<crate::model_registry::StateBinding>,
    specification: Arc<crate::specification::Prepared>,
    states: Arc<crate::state_resources::Prepared>,
}
impl Template {
    fn load(r: &SceneResources, class: i32) -> Result<Self, String> {
        let p = r
            .classes
            .models
            .first_program(class)
            .ok_or("positional effect class absent")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let seed =
            Seed::from_program_for(p, Profile::Debug, crate::arithmetic::Precision::Double53)
                .map_err(|e| e.to_string())?;
        if seed.storage.state() != 78
            || !seed.diagnostics.is_empty()
            || !seed.unresolved.is_empty()
            || seed
                .storage
                .data()
                .is_none_or(|d| d.len() != 268 || d.iter().any(|b| *b != 0))
        {
            return Err("positional effect requires acquired zero D state78 template".into());
        }
        let specification = Arc::new(crate::specification::Prepared::load_registered(r, class)?);
        let states = Arc::new(crate::state_resources::Prepared::load(
            r,
            class,
            Profile::Debug,
        )?);
        if !specification.needs.is_empty() || !states.needs().is_empty() {
            return Err("positional effect constructor resources incomplete".into());
        }
        Ok(Self {
            seed,
            fields: Fields::load(r, class, &specification)?,
            bindings: r.class_state_bindings_for(class, Profile::Debug)?,
            specification,
            states,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Provider {
        calls: usize,
        draws: usize,
        fail: bool,
    }
    impl Cue for Provider {
        fn play(
            &mut self,
            s: &mut Session,
            id: Id,
            cue: u32,
            random: &mut Random,
        ) -> Result<(), String> {
            self.calls += 1;
            let actor = s.get(id).unwrap();
            let f = actor.construction_fields.as_ref().unwrap();
            let body = actor.seed.storage.data().unwrap();
            assert_eq!(f.scale, 1.);
            assert_eq!(f.control_flags, 0x20);
            for at in [4, 8, 0x10, 0x14, 0x30, 0x34, 0x38] {
                assert_eq!(u32::from_le_bytes(body[at..at + 4].try_into().unwrap()), 0);
            }
            assert_eq!(
                u32::from_le_bytes(body[0x108..0x10c].try_into().unwrap()),
                cue
            );
            assert_eq!(s.actor_current_affine(id).unwrap().flags, 0x10);
            for _ in 0..self.draws {
                random.next_15();
            }
            if self.fail {
                Err("declared unavailable cue transport".into())
            } else {
                Ok(())
            }
        }
    }
    fn prepare(pool: &mut Pool<Id>, random: &mut Random) -> ContextId {
        let (creation, _) = pool
            .prepare_positional(
                "fx_mecc_hard_land",
                [7., -5., 3.],
                crate::effect_initialization::Selection {
                    active_objects: 0,
                    quality: 1.,
                    lite_distance: 500.,
                    camera: [7., -5., 3.],
                    recipient_position: [7., -5., 3.],
                },
                random,
            )
            .unwrap();
        let crate::attached_effect_context::Creation::Prepared(id) = creation else {
            panic!("source impact context absent")
        };
        id
    }
    struct StopProvider {
        calls: usize,
        fail: bool,
    }
    struct ReleaseProvider {
        calls: usize,
        fail: bool,
        bind_peer: Option<Id>,
    }
    impl crate::actor_retirement::Services for ReleaseProvider {
        fn release(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
            self.calls += 1;
            let actor = session.get(id).unwrap();
            assert_eq!(
                actor.construction_fields.as_ref().unwrap().object_flags & 0x8000,
                0x8000
            );
            assert_eq!(actor.seed.storage.data().unwrap().len(), 268);
            assert!(session.specification(id).is_some());
            assert!(session.state_resources(id).is_some());
            assert!(session.list(0).contains(&id));
            if let Some(peer) = self.bind_peer {
                session.set_presentation_parent(peer, Some(id))?;
            }
            if self.fail {
                Err("declared late tracked release failure".into())
            } else {
                Ok(())
            }
        }
    }
    #[test]
    #[ignore = "requires GIANTS_MECC_WORLD_ROOT; silent owned retirement/control only"]
    fn source_positional_impact_retires_only_after_tracked_release_and_rechecks_references() {
        let root = std::env::var_os("GIANTS_MECC_WORLD_ROOT").expect("GIANTS_MECC_WORLD_ROOT");
        let resources =
            SceneResources::load(std::path::Path::new(&root), "w_Story1.bin", "English").unwrap();
        let mut pool = Pool::<Id>::from_active_resources(943, &resources);
        let mut session = Session::new(944);
        let mut factory = Factory::new(&resources);
        let mut random = Random(1);
        let mut cue = Provider {
            calls: 0,
            draws: 0,
            fail: false,
        };
        let mut release = ReleaseProvider {
            calls: 0,
            fail: false,
            bind_peer: None,
        };
        let mut stop = StopProvider {
            calls: 0,
            fail: false,
        };
        let mut stamp = crate::animation_epoch::Stamp::default();
        let mut ids = vec![];
        for _ in 0..2 {
            let context = prepare(&mut pool, &mut random);
            ids.push(
                factory
                    .activate(&mut session, &mut pool, context, &mut random, &mut cue)
                    .unwrap(),
            );
        }
        let [first, peer] = [ids[0], ids[1]];
        assert!(
            session
                .cleanup_positional_impact(first, &mut release)
                .is_err()
        );
        assert_eq!(release.calls, 0);
        session
            .advance_positional_impact(first, 0., &mut stamp, &mut stop)
            .unwrap();
        assert_eq!(
            session
                .advance_positional_impact(first, 1., &mut stamp, &mut stop)
                .unwrap(),
            Step::Marked
        );
        session.set_presentation_parent(peer, Some(first)).unwrap();
        assert!(
            session
                .cleanup_positional_impact(first, &mut release)
                .is_err()
        );
        assert_eq!(release.calls, 0);
        session.set_presentation_parent(peer, None).unwrap();
        let removed = session
            .cleanup_positional_impact(first, &mut release)
            .unwrap();
        assert_eq!(
            removed,
            crate::resident_states::Counts {
                residents: 1,
                payload_bytes: 268,
                registrations: 1
            }
        );
        assert_eq!(session.list(0), [peer]);
        assert!(session.get(first).is_none());
        assert!(session.specification(first).is_none());
        assert!(session.state_resources(first).is_none());
        assert!(
            session
                .cleanup_positional_impact(first, &mut release)
                .is_err()
        );
        assert_eq!(release.calls, 1);
        let context = prepare(&mut pool, &mut random);
        let next = factory
            .activate(&mut session, &mut pool, context, &mut random, &mut cue)
            .unwrap();
        assert_ne!(next, first);
        assert_eq!(session.list(0), [peer, next]);
        session
            .advance_positional_impact(next, 0., &mut stamp, &mut stop)
            .unwrap();
        session
            .advance_positional_impact(next, 1., &mut stamp, &mut stop)
            .unwrap();
        release.bind_peer = Some(peer); // A late successful service may create a new reference.
        let before = session.counts();
        assert!(
            session
                .cleanup_positional_impact(next, &mut release)
                .is_err()
        );
        assert_eq!(session.counts(), before);
        assert_eq!(release.calls, 2);
        assert!(session.positional_impact(next).unwrap().failure.is_some());
        session.set_presentation_parent(peer, None).unwrap();
        assert!(
            session
                .cleanup_positional_impact(next, &mut release)
                .is_err()
        );
        assert_eq!(release.calls, 2);
        // Check a distinct late release error after a real expiry.
        session
            .advance_positional_impact(peer, 0., &mut stamp, &mut stop)
            .unwrap();
        session
            .advance_positional_impact(peer, 1., &mut stamp, &mut stop)
            .unwrap();
        release.bind_peer = None;
        release.fail = true;
        let weak = Arc::downgrade(&session.positional_impact(peer).unwrap().source);
        drop(factory);
        drop(pool);
        drop(resources);
        assert!(weak.upgrade().is_some());
        assert!(
            session
                .cleanup_positional_impact(peer, &mut release)
                .is_err()
        );
        assert_eq!(session.counts(), before);
        let mut foreign = Session::new(945);
        assert!(
            foreign
                .cleanup_positional_impact(peer, &mut release)
                .is_err()
        );
        assert!(
            session
                .cleanup_positional_impact(peer, &mut release)
                .is_err()
        );
        assert_eq!(release.calls, 3);
        session.clear();
        assert!(weak.upgrade().is_none());
    }
    impl Stop for StopProvider {
        fn stop(&mut self, session: &mut Session, id: Id, cue: u32) -> Result<(), String> {
            self.calls += 1;
            let actor = session.get(id).unwrap();
            let data = actor.seed.storage.data().unwrap();
            let word = |at| u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
            assert_eq!(cue, word(0x108));
            assert!(f32::from_bits(word(8)) >= f32::from_bits(word(4)));
            assert_eq!(
                actor.construction_fields.as_ref().unwrap().object_flags & 0x8000,
                0
            );
            if self.fail {
                Err("declared late stop failure".into())
            } else {
                Ok(())
            }
        }
    }
    #[test]
    #[ignore = "requires GIANTS_MECC_WORLD_ROOT; silent actual actor callback/control only"]
    fn source_positional_impact_ticks_owned_clocks_and_marks_only_after_stop() {
        let root = std::env::var_os("GIANTS_MECC_WORLD_ROOT").expect("GIANTS_MECC_WORLD_ROOT");
        let resources =
            SceneResources::load(std::path::Path::new(&root), "w_Story1.bin", "English").unwrap();
        let mut pool = Pool::<Id>::from_active_resources(939, &resources);
        let mut session = Session::new(940);
        let mut factory = Factory::new(&resources);
        let mut random = Random(1);
        let mut cue = Provider {
            calls: 0,
            draws: 0,
            fail: false,
        };
        let mut ids = vec![];
        for _ in 0..2 {
            let context = prepare(&mut pool, &mut random);
            ids.push(
                factory
                    .activate(&mut session, &mut pool, context, &mut random, &mut cue)
                    .unwrap(),
            );
        }
        let mut stamp = crate::animation_epoch::Stamp::default();
        let mut stop = StopProvider {
            calls: 0,
            fail: false,
        };
        let before = session.counts();
        for &id in &ids {
            assert!(
                session
                    .advance_positional_impact(id, f32::NAN, &mut stamp, &mut stop)
                    .is_err()
            );
            assert_eq!(
                session
                    .advance_positional_impact(id, 0., &mut stamp, &mut stop)
                    .unwrap(),
                Step::FirstFrame
            );
            assert_eq!(session.get(id).unwrap().mecc_animation_stamp, stamp.0);
            assert_eq!(session.current_reference_matrices(id).unwrap().len(), 1);
            assert_eq!(
                session
                    .positional_impact(id)
                    .unwrap()
                    .visual
                    .as_ref()
                    .unwrap()
                    .age,
                0.
            );
        }
        assert_eq!(stamp.0, 2);
        for (index, &id) in ids.iter().enumerate() {
            for _ in 0..3 {
                assert_eq!(
                    session
                        .advance_positional_impact(id, 0.25, &mut stamp, &mut stop)
                        .unwrap(),
                    Step::Updated
                );
                session.actor_current_affine(id).unwrap();
            }
            stop.fail = index == 1;
            let before_stamp = stamp.0;
            let result = session.advance_positional_impact(id, 0.25, &mut stamp, &mut stop);
            assert_eq!(stamp.0, before_stamp);
            if stop.fail {
                assert!(result.is_err());
                assert!(session.positional_impact(id).unwrap().failure.is_some());
                let calls = stop.calls;
                assert!(
                    session
                        .advance_positional_impact(id, 0.25, &mut stamp, &mut stop)
                        .is_err()
                );
                assert_eq!(stop.calls, calls);
            } else {
                assert_eq!(result.unwrap(), Step::Marked);
                assert_eq!(
                    session
                        .advance_positional_impact(id, 0.25, &mut stamp, &mut stop)
                        .unwrap(),
                    Step::Inactive
                );
            }
        }
        assert_eq!(stop.calls, 2);
        assert_eq!(session.counts(), before); // Deferred marks do not retire storage.
        assert_eq!(
            session
                .get(ids[0])
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .object_flags
                & 0x8000,
            0x8000
        );
        assert_eq!(
            session
                .get(ids[1])
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .object_flags
                & 0x8000,
            0
        );
        let mut foreign = Session::new(941);
        assert!(
            foreign
                .advance_positional_impact(ids[0], 0., &mut stamp, &mut stop)
                .is_err()
        );
        session.clear();
        assert!(session.get(ids[0]).is_none());
    }
    #[test]
    #[ignore = "requires GIANTS_MECC_WORLD_ROOT; silent source-resource constructor/control only"]
    fn source_positional_impact_retains_ordered_actors_and_late_failure_without_replay() {
        let root = std::env::var_os("GIANTS_MECC_WORLD_ROOT").expect("GIANTS_MECC_WORLD_ROOT");
        let resources =
            SceneResources::load(std::path::Path::new(&root), "w_Story1.bin", "English").unwrap();
        let mut pool = Pool::<Id>::from_active_resources(839, &resources);
        let mut s = Session::new(840);
        let mut factory = Factory::new(&resources);
        let mut random = Random(1);
        let context = prepare(&mut pool, &mut random);
        let mut provider = Provider {
            calls: 0,
            draws: 3,
            fail: false,
        };
        let id = factory
            .activate(&mut s, &mut pool, context, &mut random, &mut provider)
            .unwrap();
        let mut expected = Random(1);
        for _ in 0..9 {
            expected.next_15();
        }
        assert_eq!(random.0, expected.0);
        assert!(s.positional_impact(id).unwrap().complete);
        assert!(pool.get(context).is_none());
        assert_eq!(provider.calls, 1);
        let before = random.0;
        assert!(
            factory
                .activate(&mut s, &mut pool, context, &mut random, &mut provider)
                .is_err()
        );
        assert_eq!(random.0, before);
        assert!(
            s.positional_impact(id)
                .unwrap()
                .source_image()
                .find("fx_mecc_hard_land")
                .is_some()
        );
        assert_eq!(
            s.actor_current_affine(id).unwrap().matrix.map(f32::to_bits),
            s.positional_impact(id)
                .unwrap()
                .matrix
                .values
                .map(f32::to_bits)
        );
        let foreign_context = prepare(&mut pool, &mut random);
        let before = random.0;
        let mut foreign = Session::new(841);
        assert!(
            factory
                .activate(
                    &mut foreign,
                    &mut pool,
                    foreign_context,
                    &mut random,
                    &mut provider
                )
                .is_err()
        );
        assert_eq!(random.0, before);
        pool.release_prepared(foreign_context).unwrap();
        let failed = prepare(&mut pool, &mut random);
        let mut provider = Provider {
            calls: 0,
            draws: 1,
            fail: true,
        };
        let before = random.0;
        let mut expected = Random(before);
        for _ in 0..2 {
            expected.next_15();
        }
        assert!(
            factory
                .activate(&mut s, &mut pool, failed, &mut random, &mut provider)
                .is_err()
        );
        assert_eq!(random.0, expected.0);
        let actor = factory.actor(failed).unwrap();
        assert!(s.positional_impact(actor).unwrap().failure.is_some());
        assert!(pool.get(failed).is_some());
        assert!(pool.release_prepared(failed).is_err());
        let before = random.0;
        let counts = s.counts();
        let mut other = Factory::new(&resources);
        assert!(
            other
                .activate(&mut s, &mut pool, failed, &mut random, &mut provider)
                .is_err()
        );
        assert_eq!(random.0, before);
        assert_eq!(s.counts(), counts);
        assert_eq!(provider.calls, 1);
        assert!(!s.positional_impact(actor).unwrap().complete);
        drop(other);
        drop(factory);
        drop(pool);
        drop(resources);
        assert!(
            s.positional_impact(id)
                .unwrap()
                .source_image()
                .find("fx_mecc_hard_land")
                .is_some()
        );
        s.clear();
        assert!(s.get(id).is_none());
        assert!(s.get(actor).is_none());
    }
}
