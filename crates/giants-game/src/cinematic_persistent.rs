//! Source-marked persistent attachments in the main film. This implements a
//! bounded queue/object path; it does not acknowledge world admission or fidelity.
use crate::{
    actor_links::Projection,
    attached_effect_context::{Creation, Pool},
    attached_effect_object::{CinematicObject, CinematicTemplate},
    cinematic_fx::{Request, TerminalProxy},
    procedural_aim::Matrix,
    scene_resources::SceneResources,
};
use std::{collections::BTreeMap, sync::Arc};

struct Bound {
    request: Request,
    template: Option<Arc<CinematicTemplate>>,
    terminal: Option<giants_formats::effect_program::Program>,
}
pub struct Bindings {
    requests: BTreeMap<usize, Bound>,
    actors: Projection,
    image: Arc<giants_formats::effect_image::Image>,
}
impl Bindings {
    /// Whole-source capability checking: never enable just the convenient
    /// requests in a source whose other nonterminal families remain unknown.
    pub fn load(resources: &SceneResources, bytes: &[u8]) -> Result<Self, String> {
        giants_formats::flick::parse_preview(bytes).map_err(|e| e.to_string())?;
        let metadata =
            giants_formats::flick::discover_dependencies(bytes).map_err(|e| e.to_string())?;
        let declarations = crate::cinematic_fx::declarations(bytes, &metadata)?;
        let image = Arc::new(
            resources
                .classes
                .models
                .effects
                .last()
                .ok_or("persistent effect image absent")?
                .image
                .clone(),
        );
        let actors = crate::actor_links::Factory::new(resources, &image).project(bytes)?;
        let mut templates = BTreeMap::new();
        let mut requests = BTreeMap::new();
        let mut persistent = 0;
        for request in declarations {
            crate::cinematic_fx::validate_request_prefix(&metadata, &actors, &request)?;
            let (template, terminal) = if request.slot < 0 {
                (None, None)
            } else {
                if request.mode != 1 {
                    return Err("persistent cinematic queue requires attached mode1".into());
                }
                let program = resources.bound_effect_program(&request.target(resources)?)?;
                if crate::cinematic_fx::terminator_only(&program) {
                    (None, Some(program))
                } else {
                    let recipe = crate::effect_attachment::Recipe::project(&program)
                        .map_err(str::to_owned)?;
                    let class = resources
                        .classes
                        .models
                        .first_program(recipe.class)
                        .ok_or("persistent effect class absent")?
                        .program
                        .as_ref()
                        .map_err(Clone::clone)?;
                    recipe.facing(class).map_err(str::to_owned)?;
                    let template = templates
                        .entry(recipe.class)
                        .or_insert_with(|| {
                            CinematicTemplate::load(resources, recipe.class).map(Arc::new)
                        })
                        .as_ref()
                        .map_err(Clone::clone)?
                        .clone();
                    persistent += 1;
                    (Some(template), None)
                }
            };
            requests.insert(
                request.command_offset,
                Bound {
                    request,
                    template,
                    terminal,
                },
            );
        }
        if persistent == 0 {
            return Err("source has no bounded persistent requests".into());
        }
        Ok(Self {
            requests,
            actors,
            image,
        })
    }
    pub fn queue(&self, ticket: u64) -> Queue {
        Queue {
            contexts: Pool::new(ticket, Some(self.image.clone())),
            objects: BTreeMap::new(),
            supports: BTreeMap::new(),
            terminal: vec![],
            dispatched: 0,
            retired: 0,
        }
    }
    pub fn owns(&self, offset: usize) -> bool {
        self.requests
            .get(&offset)
            .is_some_and(|r| r.template.is_some())
    }
    pub fn persistent_requests(&self) -> usize {
        self.requests
            .values()
            .filter(|r| r.template.is_some())
            .count()
    }
    fn placement(
        &self,
        request: &Request,
        seconds: f32,
    ) -> Result<crate::cinematic_fx::ProxyPlacement, String> {
        if !seconds.is_finite() || seconds < 0. {
            return Err("persistent support clock invalid".into());
        }
        let clocks = self
            .actors
            .actors
            .iter()
            .map(|(slot, actor)| {
                (
                    *slot,
                    if actor.director_slot.is_some() {
                        seconds
                    } else {
                        0.
                    },
                )
            })
            .collect();
        let locals = self
            .actors
            .actors
            .keys()
            .map(|slot| (*slot, None))
            .collect();
        request
            .placement(&self.actors, &clocks, &locals)?
            .ok_or("persistent support request inactive".into())
    }
}
#[derive(Clone, Copy)]
pub struct Frame {
    pub world: Matrix,
    pub channels: crate::animation_channels::Channels,
    pub updates: u64,
    class: i32,
    position: [f32; 3],
    active: u32,
    render_flags: u32,
}
impl Frame {
    /// Fresh constant attachments have no source exclusion registrations,
    /// transparency callback or camera ownership. Both measured traversals
    /// consume the same owned geometry; only the ordinary one uses this range.
    pub fn draw_routes(
        &self,
        eye: [f32; 3],
        range: f32,
    ) -> Result<crate::draw_routes::Routes, &'static str> {
        crate::draw_routes::with_ordinary_admission(
            &crate::cinematic_ordinary::Input {
                class: self.class,
                state: 78,
                flick_slot: -1,
                exclusion_words: [0; 3],
                position: self.position,
                eye,
                range,
            },
            self.active,
            self.render_flags,
        )
    }
}
pub struct Queue {
    contexts: Pool<usize>,
    objects: BTreeMap<usize, CinematicObject<usize>>,
    supports: BTreeMap<usize, usize>,
    terminal: Vec<TerminalProxy>,
    pub dispatched: usize,
    pub retired: usize,
}
impl Queue {
    pub fn advanced_objects(&self) -> usize {
        self.objects
            .values()
            .filter(|object| object.updates > 1)
            .count()
    }
    pub fn objects(&self) -> usize {
        self.objects.len()
    }
    pub fn contexts(&self) -> usize {
        self.contexts.active().len()
    }
    pub fn frame(&self, offset: usize) -> Option<Frame> {
        let object = self.objects.get(&offset)?;
        Some(Frame {
            world: object.world_matrix()?,
            channels: object.fields.channels,
            updates: object.updates,
            class: object.fields.class,
            position: object.fields.position,
            active: object.fields.object_flags,
            render_flags: object.metadata.render_flags,
        })
    }
    /// The supplied event is the actual FLK39 dispatch. A temporary context is
    /// consumed only after successful construction; object ownership is separate.
    pub fn dispatch(
        &mut self,
        binding: &Bindings,
        offset: usize,
        seconds: f32,
    ) -> Result<(), String> {
        let bound = binding
            .requests
            .get(&offset)
            .ok_or("persistent event not bound")?;
        let request = &bound.request;
        if request.slot < 0 {
            self.dispatched += 1;
            return Ok(());
        }
        let placement = binding.placement(request, seconds)?;
        let slot = placement.slot;
        if let Some(program) = &bound.terminal {
            self.terminal
                .push(TerminalProxy::new(placement, program.clone()).map_err(str::to_owned)?);
        } else {
            if self.objects.contains_key(&offset) {
                return Err("persistent request dispatched twice in one director".into());
            }
            let name = request.effect.as_deref().ok_or("persistent name absent")?;
            let Creation::Prepared(id) = self.contexts.prepare_cinematic(name, slot)? else {
                return Err("persistent effect name absent from active image".into());
            };
            let result = bound
                .template
                .as_ref()
                .ok_or("persistent constructor absent")?
                .construct(
                    id,
                    self.contexts.get(id).ok_or("persistent context absent")?,
                );
            let object = match result {
                Ok(object) => object,
                Err(error) => {
                    self.contexts.release_prepared(id)?;
                    return Err(error);
                }
            };
            self.contexts.release_prepared(id)?;
            self.objects.insert(offset, object);
        }
        // Every active source request refreshes this shared support, including
        // a terminal program that creates no new object. Earlier objects live on.
        self.supports.insert(slot, offset);
        self.dispatched += 1;
        Ok(())
    }
    pub fn update(&mut self, binding: &Bindings, seconds: f32, delta: f32) -> Result<(), String> {
        for object in self.objects.values_mut() {
            let offset = self
                .supports
                .get(&object.metadata.parent)
                .ok_or("persistent source support absent")?;
            let placement = binding.placement(&binding.requests[offset].request, seconds)?;
            object.update(placement.matrix, delta)?;
        }
        for proxy in &mut self.terminal {
            proxy.update().map_err(str::to_owned)?;
        }
        self.terminal.retain(TerminalProxy::active);
        Ok(())
    }
    /// Only source-marked objects inhabit this queue. A cut/cancel drops them,
    /// without lifetime children, RNG, audio or another ticket's resources.
    pub fn release(&mut self) -> Result<usize, String> {
        let contexts = self.contexts.active().to_vec();
        for id in contexts {
            self.contexts.release_prepared(id)?;
        }
        let count = self.objects.len();
        self.objects.clear();
        self.supports.clear();
        self.terminal.clear();
        self.retired += count;
        Ok(count)
    }
}
