//! Staged first active construction of a retained damage-smoke emitter.
//! This does not consume its context, register a Session actor, run a callback
//! or emit a particle. Those operations require their actual lifetime owner.
use crate::{
    attached_effect_context::{Context, Id},
    audio_parameters::Random,
    effect_actor::{Initial, Template},
    effect_scene::{EmissionConfiguration, Service, ServiceKind},
    scene_resources::SceneResources,
};
use giants_formats::effect_image::Image;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Destruction {
    pub origin: Id,
    pub parent: crate::resident_states::Id,
    pub payload_bytes: usize,
    pub registrations: usize,
    pub parent_invalidated: bool,
}

#[derive(Debug)]
pub struct Emission {
    pub configuration: EmissionConfiguration,
    pub entry_offset: u32,
    source: Arc<Image>,
}
impl Emission {
    pub(crate) fn source(&self) -> &Image {
        &self.source
    }
    pub fn program(
        &self,
    ) -> Result<giants_formats::effect_program::Program, giants_formats::Error> {
        self.source.program(self.entry_offset)
    }
    /// One occupied channel. Program identity is normalized to its image offset.
    /// No periodic clock, reference selection or descendant creation executes.
    pub fn initial_words(&self, position: [f32; 3]) -> Vec<u32> {
        let mut words = vec![0; 0x4dc / 4];
        words[0] = self.entry_offset;
        words[0x28 / 4] = self.configuration.interval.to_bits();
        for (offset, value) in [0x78, 0xa0, 0xc8, 0xf0]
            .into_iter()
            .zip(self.configuration.selectors.into_iter().flatten())
        {
            words[offset / 4] = value;
        }
        words[0x168 / 4] = (-1f32).to_bits();
        words[0x190 / 4] = 1;
        for (out, value) in words[0x43c / 4..0x448 / 4].iter_mut().zip(position) {
            *out = value.to_bits();
        }
        words
    }
}

/// Explicit actor-local invocation; its sequence is not a global frame clock.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub number: u64,
    pub seconds: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Callback {
    pub first: bool,
    pub child: Option<Id>,
}
/// Resolve the selected guide using the actual current world/reference owner.
/// No default point, presentation pose or success provider is installed.
pub trait Guide {
    fn point(
        &mut self,
        session: &crate::resident_states::Session,
        actor: crate::resident_states::Id,
        reference: u32,
    ) -> Result<[f32; 3], String>;
}
/// Acquired ordinary guide transform over the Session's preceding sampled
/// references. The enclosing owner must supply the actual world affine.
pub fn guide_point(
    session: &crate::resident_states::Session,
    actor: crate::resident_states::Id,
    reference: u32,
    world: crate::procedural_aim::Matrix,
) -> Result<[f32; 3], String> {
    if world.flags & 0x20 != 0 || !world.values.iter().all(|value| value.is_finite()) {
        return Err("smoke guide requires finite affine world matrix".into());
    }
    let matrices = session.current_reference_matrices(actor)?;
    let local = matrices
        .get(reference as usize)
        .ok_or("smoke guide reference absent")?;
    let point = world.transform_point([local.values[3], local.values[7], local.values[11]]);
    if !point.iter().all(|value| value.is_finite()) {
        return Err("smoke guide position nonfinite".into());
    }
    Ok(point)
}

pub struct Staged<K> {
    // Keep the actual specification, state bindings and model/ANM resources
    // alive after the caller drops SceneResources. This is no Session token.
    pub(crate) template: Template,
    pub initial: Initial<K>,
    pub emission: Emission,
    /// Required result of registration/VM completion, not a committed pool flag.
    pub retained_flags: u32,
    pub retained_phase: u32,
    /// Next program byte after the retaining command, relative to the entry.
    pub resume_cursor: usize,
}
impl<K: Copy> Staged<K> {
    /// The caller supplies an origin/context pair from its own pool. Success
    /// commits the active interval draw only; dropping this staging result is
    /// not cancellation of a registered context or acknowledgement of emission.
    pub fn attached(
        resources: &SceneResources,
        origin: Id,
        context: &Context<K>,
        random: &mut Random,
    ) -> Result<Self, String> {
        if context.flags() != 0x20004 || context.recipient().is_none() {
            return Err("damage smoke requires an extended attached context".into());
        }
        let image = context.source_image();
        if !resources
            .classes
            .models
            .effects
            .last()
            .is_some_and(|active| active.image.same_image(&image))
        {
            return Err("damage smoke source differs from active resources".into());
        }
        let program = context.program.as_ref().map_err(Clone::clone)?;
        if program.records.iter().map(|r| r.opcode).collect::<Vec<_>>()
            != [0x24, 6, 0x21, 0x28, 0x49, 7, 0x4a, 2]
        {
            return Err("damage smoke outside acquired complete retained frame".into());
        }
        let first = &program.records[0].bytes;
        let class = i32::from_le_bytes(
            first
                .get(5..9)
                .ok_or("smoke class absent")?
                .try_into()
                .unwrap(),
        );
        let template = Template::load(resources, class)?;
        let mut initial = template.smoke_attached(origin, context)?;
        let record = program.records[3].clone();
        // Context preparation has already consumed its null-object pass draw.
        // Stage the active draw only; any resource/profile refusal preserves it.
        let mut staged = *random;
        let service = Service {
            kind: ServiceKind::Source28,
            record,
            random_sample: Some(staged.next_15()),
        };
        let configuration = service.emission_configuration()?;
        let [low, high] = configuration.selectors[0];
        if low > high || high > 65535 {
            return Err("smoke reference selector outside acquired inclusive range".into());
        }
        let entry_offset = image
            .find(&configuration.effect)
            .ok_or("smoke descendant program absent")?
            .1
            .offset;
        image
            .program_tail(entry_offset)
            .map_err(|e| e.to_string())?;
        initial.render_flags |= 0x10000;
        initial.fields.object_flags |= 0x40000000;
        initial.fields.control_flags |= 0x2000;
        let result = Self {
            template,
            initial,
            emission: Emission {
                configuration,
                entry_offset,
                source: image,
            },
            retained_flags: context.flags() | 0x40000,
            retained_phase: 4,
            resume_cursor: program.records[7].offset,
        };
        *random = staged;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attached_effect_context::{Creation, Pool};

    #[test]
    #[ignore = "Requires GIANTS_MECC_WORLD_ROOT; silent installed constructor ownership, no world admission"]
    fn source_mecc_retained_smoke_stages_without_consuming_context() {
        let root = std::path::PathBuf::from(
            std::env::var_os("GIANTS_MECC_WORLD_ROOT").expect("GIANTS_MECC_WORLD_ROOT"),
        );
        let resources = SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
        let mut pool = Pool::from_active_resources(912, &resources);
        let mut random = Random(42);
        let mut retained = Vec::new();
        for name in [
            "FX_mecc_smoking_light",
            "FX_mecc_smoking_medium",
            "FX_mecc_smoking_heavy",
        ] {
            let (creation, intervals) = pool
                .prepare_extended_invariant(name, 71u32, 0, 3, &mut random)
                .unwrap();
            let Creation::Prepared(id) = creation else {
                panic!("source smoke absent")
            };
            assert_eq!(intervals.len(), 1);
            let context = pool.get(id).unwrap();
            let before = random;
            // Public ordinary/timed constructor gates remain strict.
            let program = context.program.as_ref().unwrap();
            let class = i32::from_le_bytes(program.records[0].bytes[5..9].try_into().unwrap());
            let template = Template::load(&resources, class).unwrap();
            assert!(template.attached(id, context).is_err());
            assert!(template.timed_attached(id, context, &mut random).is_err());
            assert_eq!(random.0, before.0);
            let mut altered = context.clone();
            let mut frame = program.as_ref().clone();
            frame.records[4].opcode = 2;
            altered.program = Ok(Arc::new(frame));
            assert!(Staged::attached(&resources, id, &altered, &mut random).is_err());
            assert_eq!(random.0, before.0);
            // Refusals after interval sampling must also leave the live RNG.
            let mut missing = program.as_ref().clone();
            missing.records[3].bytes[1] = b'!';
            altered.program = Ok(Arc::new(missing));
            assert!(Staged::attached(&resources, id, &altered, &mut random).is_err());
            assert_eq!(random.0, before.0);
            let staged = Staged::attached(&resources, id, context, &mut random).unwrap();
            let mut expected = before;
            expected.next_15();
            assert_eq!(random.0, expected.0);
            assert_eq!(staged.retained_flags, 0x60004);
            assert_eq!(staged.retained_phase, 4);
            assert_eq!(staged.resume_cursor, program.records[7].offset);
            assert_eq!(staged.initial.render_flags, 0x10000);
            assert_ne!(staged.initial.fields.control_flags & 0x2000, 0);
            assert_eq!(context.flags(), 0x20004);
            assert_eq!(context.cursor(), 0);
            assert_eq!(context.emitted_objects(), 0);
            let bank = staged.emission.initial_words([2., -3., 4.]);
            assert_eq!(bank[0x190 / 4], 1);
            assert_eq!(bank[0x50 / 4], 0);
            assert_eq!(bank[0x168 / 4], (-1f32).to_bits());
            assert_eq!(
                &bank[0x43c / 4..0x448 / 4],
                &[2f32.to_bits(), (-3f32).to_bits(), 4f32.to_bits()]
            );
            retained.push(staged);
            // A staging result has committed no actor or context activation.
            pool.release_prepared(id).unwrap();
        }
        assert!(pool.active().is_empty());
        drop(pool);
        drop(resources);
        for staged in retained {
            assert!(staged.emission.program().is_ok());
            assert_eq!(staged.initial.seed.storage.state(), 78);
            assert_eq!(staged.initial.physical_words().unwrap()[1], 1f32.to_bits());
        }
    }
}
