//! Owned bounded state78 timed callbacks over the actual inline payload.
//! Ordinary preparation, physical/world services and stock retirement remain
//! explicit callers. This object is not a world-admission or renderer token.
use crate::{
    actor_references::Owned,
    animation_epoch::Stamp,
    effect_actor::{Configured, Initial, Template},
    effect_scene::RetirementBinding,
    effect_visual::Step as VisualStep,
    procedural_aim::Matrix,
};
use giants_formats::effect_image::Image;

/// Required native service boundaries. There are no default success providers.
/// Physical services own the remaining motion/world state and update this
/// object's actual fields/payload. Retirement prepares its real child/services;
/// deferred marking follows success. Neither operation frees ordinary slots.
/// `seconds` is the effect callback's sampled time; a physical provider must
/// retain its own acquired clock queries rather than assume all phases share it.
pub trait Services<K> {
    fn physical(&mut self, object: &mut Initial<K>, seconds: f32) -> Result<(), String>;
    fn retire(
        &mut self,
        object: &Initial<K>,
        source: &Image,
        binding: Option<&RetirementBinding>,
    ) -> Result<(), String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    FirstFrame,
    Updated,
    Marked,
    Inactive,
}

pub struct Timed<K> {
    configured: Configured<K>,
    template: Template,
    references: Owned,
    durations: Vec<Option<f32>>,
    immutable_words: [u32; 3],
    epoch: u32,
    failure: Option<String>,
}

impl<K: Copy> Timed<K> {
    /// Stage ownership while the caller retains the context transaction. The
    /// pool's consume method additionally guards and retires its live identity.
    pub fn attached(
        resources: &crate::scene_resources::SceneResources,
        origin: crate::attached_effect_context::Id,
        context: &crate::attached_effect_context::Context<K>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<Self, String> {
        source_scope(resources, context)?;
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let recipe = crate::timed_effect::Recipe::project(program)?;
        let template = Template::load(resources, recipe.class)?;
        let mut staged = *random;
        let configured = template.timed_attached(origin, context, &mut staged)?;
        let owned = Self::new(template, configured)?;
        *random = staged;
        Ok(owned)
    }

    pub fn positioned(
        resources: &crate::scene_resources::SceneResources,
        origin: crate::attached_effect_context::Id,
        context: &crate::attached_effect_context::Context<K>,
    ) -> Result<Self, String> {
        source_scope(resources, context)?;
        let program = context.program.as_ref().map_err(Clone::clone)?;
        let recipe = crate::released_effect::Recipe::project(program)?;
        let template = Template::load(resources, recipe.class)?;
        let configured = template.gravity_positioned(origin, context)?;
        Self::new(template, configured)
    }
}

fn source_scope<K>(
    resources: &crate::scene_resources::SceneResources,
    context: &crate::attached_effect_context::Context<K>,
) -> Result<(), String> {
    let source = context.source_image();
    if !resources
        .classes
        .models
        .effects
        .iter()
        .any(|e| e.image.same_image(&source))
    {
        return Err("timed source image differs from constructor resources".into());
    }
    Ok(())
}

impl<K> Timed<K> {
    /// Retain all class/state/reference and source-image owners. This performs
    /// neither ordinary preparation nor the first callback, even at zero time.
    fn new(template: Template, configured: Configured<K>) -> Result<Self, String> {
        let initial = &configured.initial;
        if initial.fields.class != template.fields.class
            || !template
                .specification
                .metadata
                .events
                .iter()
                .all(|t| t.events.is_empty())
            || initial
                .fields
                .channels
                .iter()
                .any(|c| c[4] != 0 || c[5] != 0)
        {
            return Err("timed ownership outside acquired class/ANM callback profile".into());
        }
        let data = initial.seed.storage.data().ok_or("timed payload absent")?;
        if data.len() != 268 || initial.seed.storage.state() != 78 {
            return Err("timed ownership requires the acquired inline payload".into());
        }
        let immutable_words = [4, 0x10, 0x14].map(|at| word(data, at));
        let references = Owned::new(template.specification.clone())?;
        let mut durations = vec![None; 256];
        for (&slot, descriptor) in &template.specification.animations {
            durations[usize::from(slot)] = Some(descriptor.template.duration);
        }
        let owned = Self {
            configured,
            template,
            references,
            durations,
            immutable_words,
            epoch: 0,
            failure: None,
        };
        owned.preflight()?;
        Ok(owned)
    }

    pub fn object(&self) -> &Initial<K> {
        &self.configured.initial
    }
    /// Access for the acquired ordinary preparation writer, not permission to
    /// rewrite a lifetime, install a guessed placement or bypass world admission.
    pub fn object_mut(&mut self) -> &mut Initial<K> {
        &mut self.configured.initial
    }
    pub fn transparency(&self) -> f32 {
        self.configured.transparency
    }
    pub fn fade(&self) -> [f32; 4] {
        self.configured.fade
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub fn retirement(&self) -> Option<&RetirementBinding> {
        self.configured.retirement.as_ref()
    }
    pub fn animation_epoch(&self) -> u32 {
        self.epoch
    }
    pub fn state_resources(&self) -> &crate::state_resources::Prepared {
        &self.template.states
    }
    pub fn current_references(&self) -> Result<&[Matrix], String> {
        self.references
            .current_matrices(&self.configured.initial.fields.channels)
    }

    /// Acquired parented, stationary-relative-to-history preparation profile.
    /// The callback (including retirement) precedes finalization and attachment.
    /// Support pose/scale are real caller inputs. Dispatcher sleep/model-rate
    /// producers, ordinary slots and the detached environment remain external.
    pub fn prepare_attached(
        &mut self,
        support: Matrix,
        support_scale: f32,
        seconds: f32,
        stamp: &mut Stamp,
        services: &mut impl Services<K>,
    ) -> Result<Step, String> {
        self.preflight()?;
        let b = &self.configured.initial;
        let crate::effect_actor::Placement::Attached { reference, .. } = &b.placement else {
            return Err("parented preparation requires actual attachment binding".into());
        };
        let reference = *reference as usize;
        if b.fields.configuration[14] & 0x10 != 0
            || b.fields.position.map(f32::to_bits) != b.fields.current.map(f32::to_bits)
        {
            return Err("parented preparation needs acquired dispatcher/history producers".into());
        }
        if !seconds.is_finite()
            || seconds < 0.
            || !support_scale.is_finite()
            || support_scale <= 0.
            || support.flags & 0x20 != 0
            || !support.values.iter().all(|v| v.is_finite())
        {
            return Err("parented preparation outside finite pose/time profile".into());
        }
        if b.fields.object_flags & 0xc000 != 0 {
            return Ok(Step::Inactive);
        }
        self.configured.initial.fields.object_flags &= !0x40;
        let next = self.advance(seconds, stamp, services)?;
        let result = (|| -> Result<(), String> {
            // This profile has a real parent and physical storage, no relative
            // displacement. Reuse common finalization; any unexpected required
            // footprint/environment service refuses rather than succeeding.
            let f = &mut self.configured.initial.fields;
            let mut fields = crate::collider_copy::Fields::zeroed();
            fields.class = f.class as u32;
            fields.current = f.position;
            fields.previous = f.current;
            fields.scale = f.scale;
            fields.angles = f.angles;
            fields.object_flags = f.object_flags;
            fields.response_flags = f.response_flags;
            fields.collision_flags = f.configuration[14];
            fields.ground_offset = f32::from_bits(f.configuration[17]);
            fields.center_offset = f32::from_bits(f.configuration[22]);
            fields.height = f32::from_bits(f.configuration[23]);
            fields.physical = true;
            let finalized = crate::actor_finalization::finalize(
                crate::actor_finalization::State {
                    fields,
                    previous_physical_velocity: [0.; 3],
                    friction: 0.,
                    parent_present: true,
                    footprint: None,
                    affine: None,
                    physical_argument_word: None,
                },
                crate::actor_finalization::Time {
                    global_seconds: seconds,
                    context: f.dispatch_context,
                    areas: &[],
                },
                &mut RequiredFinalization,
            )
            .map_err(|e| e.to_string())?;
            f.position = finalized.fields.current;
            f.object_flags = finalized.fields.object_flags;
            let own = self
                .references
                .current_matrices(&f.channels)?
                .get(reference)
                .copied()
                .ok_or("parented effect reference absent")?;
            self.configured.initial.matrix =
                crate::effect_actor::bind_parented(f, support, support_scale, own)?;
            Ok(())
        })();
        if let Err(ref error) = result {
            self.failure = Some(error.clone());
        }
        result?;
        Ok(next)
    }

    fn preflight(&self) -> Result<(), String> {
        if self.failure.is_some() {
            return Err("timed callback is quarantined after a late failure".into());
        }
        let b = &self.configured.initial;
        let data = b
            .seed
            .storage
            .data()
            .ok_or("timed callback payload absent")?;
        let allowed = 0x800000 | 0x80 | 0x20 | 0x08 | 0x04 | 0x02 | 0x01;
        let physical = matches!(b.placement, crate::effect_actor::Placement::Position(_));
        if b.seed.storage.profile() != crate::state_storage::Profile::Debug
            || b.seed.storage.state() != 78
            || data.len() != 268
            || word(data, 0) != 1
            || b.fields.class != self.template.fields.class
            || b.fields.control_flags & !allowed != 0
            || b.fields.control_flags & 2 == 0
            || (b.fields.control_flags & 0x80 != 0) != physical
            || (b.fields.control_flags & 8 != 0)
                != (self.immutable_words[1] != self.immutable_words[2])
            || (b.fields.control_flags & 1 != 0) != self.configured.retirement.is_some()
            || !matches!(b.render_flags, 0 | 0x1000)
            || [4, 0x10, 0x14].map(|at| word(data, at)) != self.immutable_words
            || word(data, 8) != self.configured.visual.age.to_bits()
            || data[0xf8..0x10c].iter().any(|&b| b != 0)
        {
            return Err("timed callback outside acquired clock/payload/service profile".into());
        }
        Ok(())
    }

    /// Caller supplies the already-resolved object timestep and the shared ANM
    /// epoch. Ordinary preparation may itself call this first entry; it must not
    /// then be replayed by a second initialization clock.
    pub fn advance(
        &mut self,
        seconds: f32,
        stamp: &mut Stamp,
        services: &mut impl Services<K>,
    ) -> Result<Step, String> {
        if !seconds.is_finite() || seconds < 0. {
            return Err("timed callback requires finite nonnegative object time".into());
        }
        self.preflight()?;
        if self.configured.initial.fields.object_flags & 0xc000 != 0 {
            return Ok(Step::Inactive);
        }
        let mut visual = self.configured.visual.clone();
        let next = visual.advance(seconds)?;
        let first = self.configured.initial.fields.control_flags & 0x20 != 0;
        if first != (next == VisualStep::FirstFrame) {
            return Err("timed constructor gate and lifetime phase disagree".into());
        }
        // Completed writes survive later service/ANM errors; quarantine prevents
        // replaying a retirement or physical boundary after partial publication.
        let result = (|| -> Result<Step, String> {
            let b = &mut self.configured.initial;
            crate::effect_actor::callback_entry(&mut b.fields, b.render_flags);
            b.seed.storage.data_mut().unwrap()[8..12].copy_from_slice(&visual.age.to_le_bytes());
            self.configured.visual = visual;
            if next == VisualStep::Expired {
                services.retire(
                    b,
                    &self.configured.source,
                    self.configured.retirement.as_ref(),
                )?;
                b.fields.object_flags |= 0x8000;
                return Ok(Step::Marked);
            }
            if next == VisualStep::VisibleState {
                self.configured.transparency = self.configured.visual.transparency;
                b.fields.scale = self.configured.visual.size;
                if self.configured.fade[0] <= self.configured.visual.age {
                    b.fields.control_flags &= !0x800000;
                }
                if b.fields.control_flags & 0x80 != 0 {
                    services.physical(b, seconds)?;
                }
                if b.fields.control_flags & 8 != 0 {
                    let mut matrix = crate::effect_orientation::matrix(
                        b.fields.position,
                        b.fields.angles,
                        b.fields.scale,
                    )?;
                    matrix.values[12..].copy_from_slice(&b.matrix.values[12..]);
                    b.matrix = matrix;
                }
            }
            let tick = crate::animation_channels::advance(
                &mut b.fields.channels,
                &self.durations,
                [],
                seconds,
                crate::arithmetic::Precision::Double53,
            )?;
            if tick.missing != 0 {
                return Err("timed ANM descriptor absent".into());
            }
            if tick.processed != 0 {
                self.epoch = stamp.advance();
                self.references
                    .update_current(&b.fields.channels, self.epoch)?;
            }
            Ok(if first {
                Step::FirstFrame
            } else {
                Step::Updated
            })
        })();
        if let Err(ref error) = result {
            self.failure = Some(error.clone());
        }
        result
    }
}

fn word(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}

struct RequiredFinalization;
impl crate::actor_finalization::Services for RequiredFinalization {
    fn release_footprint(&mut self, _: Option<u32>) -> Result<(), String> {
        Err("parented effect footprint producer unavailable".into())
    }
    fn environment(&mut self, _: &mut crate::actor_finalization::State) -> Result<(), String> {
        Err("parented effect environment producer unavailable".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Unavailable {
        physical: usize,
        retirement: usize,
    }
    impl Services<u32> for Unavailable {
        fn physical(&mut self, b: &mut Initial<u32>, _: f32) -> Result<(), String> {
            self.physical += 1;
            b.seed.storage.data_mut().unwrap()[0xb4..0xb8].copy_from_slice(&37f32.to_le_bytes());
            Err("declared late physical service failure".into())
        }
        fn retire(
            &mut self,
            _: &Initial<u32>,
            _: &Image,
            _: Option<&RetirementBinding>,
        ) -> Result<(), String> {
            self.retirement += 1;
            Err("declared late retirement service failure".into())
        }
    }

    #[test]
    #[ignore = "Requires GIANTS_FLICK_ROOT; silent source ownership/failure controls, no world admission"]
    fn source_timed_effect_owns_resources_and_quarantines_late_services() {
        use crate::attached_effect_context::{Creation, Pool};
        use std::{path::PathBuf, sync::Arc};
        let root = PathBuf::from(std::env::var_os("GIANTS_FLICK_ROOT").expect("GIANTS_FLICK_ROOT"));
        let world = crate::world_source::resolve(&root, "Story4").unwrap();
        let resources =
            crate::scene_resources::SceneResources::load(&root, &world.catalog, "English").unwrap();
        let image = Arc::new(
            resources
                .classes
                .models
                .effects
                .last()
                .unwrap()
                .image
                .clone(),
        );
        let mut pool = Pool::<u32>::new(400, Some(image));
        let mut random = crate::audio_parameters::Random(7);
        let (creation, _) = pool
            .prepare_cinematic_timed("fx_shotgun_flk", 9, 0, u32::MAX, &mut random)
            .unwrap();
        let Creation::Prepared(origin) = creation else {
            panic!("timed source absent")
        };
        let mut parent = pool
            .consume_timed_attached(origin, &resources, &mut random)
            .unwrap();
        assert!(pool.get(origin).is_none());
        let after_birth = random.0;
        assert!(
            pool.consume_timed_attached(origin, &resources, &mut random)
                .is_err()
        );
        assert_eq!(random.0, after_birth);
        let (creation, _) = pool
            .prepare_retirement_offset_invariant(
                Some(parent.retirement().unwrap().program_offset),
                parent.object().fields.position,
                parent.object().fields.object_flags,
                parent.object().render_flags,
                &mut random,
            )
            .unwrap();
        let Creation::Prepared(origin) = creation else {
            panic!("retirement source absent")
        };
        let mut child = pool.consume_timed_positioned(origin, &resources).unwrap();
        assert!(pool.get(origin).is_none());
        assert!(pool.consume_timed_positioned(origin, &resources).is_err());
        // Context and external resource owners can retire; actual class/state/
        // reference/image owners must remain alive in each timed object.
        drop(pool);
        drop(resources);
        let mut services = Unavailable {
            physical: 0,
            retirement: 0,
        };
        let mut stamp = Stamp(u32::MAX - 1);
        assert_eq!(
            parent
                .prepare_attached(Matrix::identity(), 1., 0., &mut stamp, &mut services)
                .unwrap(),
            Step::FirstFrame
        );
        assert!(parent.current_references().is_ok());
        let duration = f32::from_bits(word(parent.object().seed.storage.data().unwrap(), 4));
        let prepared_position = parent.object().fields.position;
        let mut later_support = Matrix::identity();
        later_support.values[3] = 97.;
        assert!(
            parent
                .prepare_attached(later_support, 1., duration * 2., &mut stamp, &mut services)
                .is_err()
        );
        assert!(parent.failure().is_some());
        assert_eq!(parent.object().fields.position, prepared_position);
        assert_eq!(parent.object().fields.object_flags & 0x8000, 0);
        assert!(f32::from_bits(word(parent.object().seed.storage.data().unwrap(), 8)) >= duration);
        assert!(parent.advance(0., &mut stamp, &mut services).is_err());
        assert_eq!(services.retirement, 1);
        assert_eq!(
            child.advance(0., &mut stamp, &mut services).unwrap(),
            Step::FirstFrame
        );
        assert_ne!(child.animation_epoch(), 0);
        assert!(child.current_references().is_ok());
        assert!(child.advance(0.01, &mut stamp, &mut services).is_err());
        assert_eq!(child.object().physical_words().unwrap()[5], 37f32.to_bits());
        assert!(child.advance(0.01, &mut stamp, &mut services).is_err());
        assert_eq!(services.physical, 1);
    }
}
