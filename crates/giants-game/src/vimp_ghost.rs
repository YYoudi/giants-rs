//! Bounded ghost effect commands: fixed size, context logging, deferred
//! lifetime and external resume. Actor/context allocation and ANM stay upstream.
use crate::effect_visual::{Configuration, Step, Visual};
use giants_formats::effect_program::Program;

#[derive(Clone, Copy, Debug)]
pub struct Recipe {
    pub class: i32,
    pub ending: Configuration,
    pub continuation: usize,
}
impl Recipe {
    pub fn project(program: &Program) -> Result<Self, String> {
        if !program
            .records
            .iter()
            .map(|r| r.opcode)
            .eq([3, 6, 8, 0x49, 0x55, 4, 7, 0x4a, 2])
        {
            return Err("ghost effect outside acquired complete deferred frame".into());
        }
        let word = |record: usize, at: usize| -> Result<u32, String> {
            let bytes = program.records[record]
                .bytes
                .get(at..at + 4)
                .ok_or("ghost command truncated")?;
            Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
        };
        let class = word(0, 1)? as i32;
        if class != 859 {
            return Err("ghost effect requires acquired class859".into());
        }
        let ending = Configuration {
            lifetime: f32::from_bits(word(5, 1)?),
            fade_in: f32::from_bits(word(5, 5)?),
            fade_out_start: f32::from_bits(word(5, 9)?),
            start_size: f32::from_bits(word(2, 1)?),
            end_size: f32::from_bits(word(2, 5)?),
        };
        Visual::new(ending)?;
        if ending.start_size <= 0. || ending.end_size <= 0. {
            return Err("ghost source sizes must be positive".into());
        }
        Ok(Self {
            class,
            ending,
            continuation: program.records[5].offset,
        })
    }
}
#[derive(Clone, Debug)]
pub struct State {
    recipe: Recipe,
    visual: Option<Visual>,
    first_frame_pending: bool,
    pub size: f32,
    pub transparency: f32,
}
impl State {
    pub fn new(recipe: Recipe) -> Result<Self, String> {
        if recipe.class != 859 {
            return Err("ghost state requires acquired class859".into());
        }
        Visual::new(recipe.ending)?;
        Ok(Self {
            recipe,
            visual: None,
            first_frame_pending: true,
            size: recipe.ending.start_size,
            transparency: 0.,
        })
    }
    pub fn waiting(&self) -> bool {
        self.visual.is_none()
    }
    pub fn age(&self) -> f32 {
        self.visual.as_ref().map_or(0., |v| v.age)
    }
    pub fn control_flags(&self) -> u32 {
        if let Some(visual) = &self.visual {
            visual.control_flags() | if self.first_frame_pending { 0x20 } else { 0 }
        } else {
            0x400000
                | if self.first_frame_pending { 0x20 } else { 0 }
                | if self.recipe.ending.start_size != self.recipe.ending.end_size {
                    8
                } else {
                    0
                }
        }
    }
    pub fn resume(&mut self) -> Result<(), String> {
        if self.visual.is_some() {
            return Err("ghost context already resumed".into());
        }
        let visual = Visual::resumed(self.recipe.ending, self.first_frame_pending)?;
        self.transparency = visual.transparency;
        self.visual = Some(visual);
        Ok(())
    }
    pub fn advance(&mut self, seconds: f32) -> Result<Step, String> {
        if !seconds.is_finite() || seconds < 0. {
            return Err("ghost clock outside finite acquired profile".into());
        }
        let first = self.first_frame_pending;
        self.first_frame_pending = false;
        if let Some(visual) = &mut self.visual {
            let step = visual.advance(seconds)?;
            self.size = visual.size;
            self.transparency = visual.transparency;
            Ok(step)
        } else {
            Ok(if first {
                Step::FirstFrame
            } else {
                Step::VisibleState
            })
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Owned {
    pub origin: crate::attached_effect_context::Id,
    pub recipe: Recipe,
    pub state: State,
    pub matrix: crate::procedural_aim::Matrix,
    pub number: u64,
    pub epoch: u32,
    pub failure: Option<String>,
    pub cleanup_started: bool,
}
#[derive(Clone, Copy)]
pub struct Draw {
    pub number: u64,
    pub epoch: u32,
    pub matrix: crate::procedural_aim::Matrix,
    pub transparency: f32,
}
pub struct Factory<'a> {
    resources: &'a crate::scene_resources::SceneResources,
}
impl<'a> Factory<'a> {
    pub fn new(resources: &'a crate::scene_resources::SceneResources) -> Self {
        Self { resources }
    }
    pub fn activate(
        &self,
        session: &mut crate::resident_states::Session,
        contexts: &mut crate::attached_effect_context::Pool<crate::resident_states::Id>,
        origin: crate::attached_effect_context::Id,
    ) -> Result<crate::resident_states::Id, String> {
        let context = contexts
            .get(origin)
            .ok_or("ghost context absent or stale")?
            .clone();
        let image = context.source_image();
        if !self
            .resources
            .classes
            .models
            .effects
            .iter()
            .any(|e| e.image.same_image(&image))
        {
            return Err("ghost namespace differs from constructor resources".into());
        }
        let recipe = Recipe::project(context.program.as_ref().map_err(Clone::clone)?)?;
        let template = crate::effect_actor::Template::load(self.resources, recipe.class)?;
        let mut initial = template.ghost_positioned(origin, &context)?;
        if initial.fields.control_flags != 0x20 || initial.fields.action != 0 {
            return Err("ghost class has additional unacquired control/render defaults".into());
        }
        initial.size(
            [recipe.ending.start_size, recipe.ending.end_size],
            recipe.ending.start_size != recipe.ending.end_size,
        )?;
        initial.fields.control_flags |= 0x400000;
        // Script pointers are normalized presence words; the actual source
        // continuation remains separately typed in the retained recipe.
        initial.seed.storage.data_mut().unwrap()[0x9c..0xa0].copy_from_slice(&1u32.to_le_bytes());
        let flags = crate::object_lists::DirectFlags {
            active: initial.fields.object_flags,
            personality: initial.fields.configuration[14],
        };
        let metadata = Owned {
            origin,
            recipe,
            state: State::new(recipe)?,
            matrix: initial.matrix,
            number: 0,
            epoch: 0,
            failure: None,
            cleanup_started: false,
        };
        contexts.claim_positional_impact(origin)?;
        let result = session.construct_vimp_ghost(
            crate::resident_states::Construction {
                class: recipe.class,
                seed: &initial.seed,
                bindings: &template.bindings,
                flags,
                specification: template.specification,
                state_resources: template.states,
                fields: initial.fields,
            },
            metadata,
        );
        if result.is_err() {
            contexts.undo_positional_claim(origin);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn program() -> Program {
        let mut bytes = vec![3];
        bytes.extend(859u32.to_le_bytes());
        bytes.extend([6, 8]);
        for value in [0.25f32, 0.05] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([0x49, 0x55, 4]);
        for value in [5f32, 0., 1.5] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend([7, 0x4a, 2]);
        giants_formats::effect_program::parse(&bytes).unwrap()
    }
    #[test]
    fn deferred_lifetime_waits_for_external_resume_and_preserves_existing_first_frame() {
        let program = program();
        let mut random = crate::audio_parameters::Random(123);
        assert!(
            crate::effect_initialization::skip(&program, &mut random)
                .unwrap()
                .is_empty()
        );
        assert_eq!(random.0, 123);
        let recipe = Recipe::project(&program).unwrap();
        let mut state = State::new(recipe).unwrap();
        assert_eq!(state.advance(100.).unwrap(), Step::FirstFrame);
        assert_eq!(state.advance(100.).unwrap(), Step::VisibleState);
        assert_eq!(state.age(), 0.);
        assert_eq!(state.size, 0.25);
        assert!(state.waiting());
        state.resume().unwrap();
        assert!(state.resume().is_err());
        assert_eq!(state.advance(0.1).unwrap(), Step::VisibleState);
        assert_eq!(state.age(), 0.1);
        assert_eq!(state.size.to_bits(), 1048307565);
        assert_eq!(state.advance(5.).unwrap(), Step::Expired);
        assert!(state.advance(0.).is_err());
        let mut state = State::new(recipe).unwrap();
        state.resume().unwrap();
        assert_eq!(state.advance(1.).unwrap(), Step::FirstFrame);
        assert_eq!(state.age(), 0.);
    }
}
