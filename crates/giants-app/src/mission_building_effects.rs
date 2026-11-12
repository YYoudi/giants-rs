//! Ticket-owned stone bursts for the ordinary partial Mecc world.
//! Prototype PS2 `fx_logic` / `fx_processOpCodes` are unknown search routes.
//! Source counts, references, models, ANM, textures and shared kernels are real.
//! Host-frame scheduling and terrain-only particle physics are preview policies;
//! attraction to other character abilities and native whole-world fidelity remain
//! unimplemented. This consumer neither acknowledges canonical state78 callbacks
//! nor manufactures successful world-force/contact services for them.
use giants_game::{
    arithmetic::Precision,
    attached_effect_context::{Creation, Pool},
    audio_parameters::Random,
    building_damage::StoneEffect,
    effect_burst::{Playback, References},
    scene_resources::{ModelAsset, SceneResources, TextureEncoding},
};
use std::{collections::BTreeMap, sync::Arc};

struct Model {
    geometry: Arc<ModelAsset>,
    actor: Arc<giants_game::actor_context::Class>,
    texture_names: Vec<String>,
    texture_configuration: Option<giants_game::effect_texture_animation::Configuration>,
    context: u32,
}
struct Body {
    serial: u64,
    model: Arc<Model>,
    playback: Playback,
    animation: Option<giants_game::effect_animation::Playback>,
    frame: u64,
}
pub(crate) struct Runtime {
    image: Option<Arc<giants_formats::effect_image::Image>>,
    source: Arc<SceneResources>,
    contexts: Pool<u64>,
    models: BTreeMap<i32, Arc<Model>>,
    bodies: Vec<Body>,
    ticket: Option<u64>,
    next: u64,
    sample: Option<u64>,
    pub(crate) births: u64,
    pub(crate) retired: u64,
}
impl Runtime {
    pub(crate) fn new(source: Arc<SceneResources>) -> Self {
        Self {
            image: source
                .classes
                .models
                .effects
                .last()
                .map(|e| Arc::new(e.image.clone())),
            contexts: Pool::from_active_resources(0, &source),
            source,
            models: BTreeMap::new(),
            bodies: vec![],
            ticket: None,
            next: 0,
            sample: None,
            births: 0,
            retired: 0,
        }
    }
    pub(crate) fn active(&self) -> usize {
        self.bodies.len()
    }
    fn bind(&mut self, ticket: u64) -> Result<(), String> {
        if self.ticket == Some(ticket) {
            return Ok(());
        }
        if self.ticket.is_some() || !self.bodies.is_empty() {
            return Err("building burst belongs to another mission ticket".into());
        }
        self.contexts = Pool::from_active_resources(ticket, &self.source);
        self.ticket = Some(ticket);
        Ok(())
    }
    fn model(&mut self, spawn: &giants_game::effect_burst::Spawn) -> Result<Arc<Model>, String> {
        if let Some(model) = self.models.get(&spawn.class) {
            if model.texture_configuration != spawn.texture_animation {
                return Err("building particle changed its cached texture configuration".into());
            }
            return Ok(model.clone());
        }
        let image = &self
            .source
            .classes
            .models
            .effects
            .last()
            .ok_or("building FX image absent")?
            .image;
        let actor = Arc::new(giants_game::actor_context::Class::load(
            &self.source,
            spawn.class,
            image,
        )?);
        if actor.state != 78 {
            return Err("building burst requires its source state78 class".into());
        }
        let fields = giants_game::actor_fields::Fields::load(
            &self.source,
            spawn.class,
            actor
                .specification
                .as_ref()
                .ok_or("building FX specification absent")?,
        )?;
        let mut geometry = self
            .source
            .model_slot_for_body(spawn.class, 0, TextureEncoding::Rgb565OrArgb4444)
            .map_err(|e| e.to_string())?
            .ok_or("building FX source model absent")?;
        if geometry.mesh.normal_count != 0 {
            return Err("building burst needs its lit model consumer".into());
        }
        let texture_names = if let Some(configuration) = spawn.texture_animation {
            if geometry.mesh.subobjects.len() != 1 {
                return Err("building animated texture needs separate material routing".into());
            }
            let frames = crate::rigid_attachment_scene::texture_animation::load(
                configuration,
                &geometry,
                &self.source,
            )?;
            let names = frames.iter().map(|t| t.name.clone()).collect();
            geometry.textures.extend(frames);
            names
        } else {
            vec![]
        };
        let model = Arc::new(Model {
            geometry: Arc::new(geometry),
            actor,
            texture_names,
            texture_configuration: spawn.texture_animation,
            context: fields.dispatch_context,
        });
        self.models.insert(spawn.class, model.clone());
        Ok(model)
    }
    /// Real positional source preparation and its active birth projection. The
    /// existing mission audio owner consumes returned positional cues; its async
    /// device/RNG policy is distinct from native interleaved audio execution.
    pub(crate) fn emit(
        &mut self,
        ticket: u64,
        kind: StoneEffect,
        point: [f32; 3],
        selection: giants_game::effect_initialization::Selection,
        random: &mut Random,
    ) -> Result<Vec<crate::mission_audio::Event>, String> {
        self.bind(ticket)?;
        let label = match kind {
            StoneEffect::Small => "FX_stone_explode_small",
            StoneEffect::Large => "FX_stone_explode_Large",
        };
        let (creation, _) = self
            .contexts
            .prepare_positional(label, point, selection, random)?;
        self.consume_prepared(
            ticket,
            creation,
            point,
            match kind {
                StoneEffect::Small => "Small",
                StoneEffect::Large => "Large",
            },
            random,
        )
    }
    /// Direct flags18 request from the class's captured registration namespace.
    /// Refuse older images rather than backfilling a matching name/offset from
    /// the current one. A future multi-image owner must retain that distinction.
    pub(crate) fn emit_offset(
        &mut self,
        ticket: u64,
        binding: &giants_game::model_registry::StateBinding,
        point: [f32; 3],
        random: &mut Random,
    ) -> Result<Vec<crate::mission_audio::Event>, String> {
        self.bind(ticket)?;
        let giants_game::model_registry::StateTarget::Effect {
            image,
            entry,
            offset,
        } = binding.target.as_ref().map_err(Clone::clone)?
        else {
            return Err("building death-FX reference is not an effect".into());
        };
        let active = self
            .source
            .classes
            .models
            .effects
            .last()
            .ok_or("building direct FX image absent")?;
        if &active.path != image
            || active.image.entries.get(*entry).map(|e| e.offset) != Some(*offset)
        {
            return Err("building direct FX needs its older captured image owner".into());
        }
        let (creation, _) =
            self.contexts
                .prepare_positional_offset(Some(*offset), point, random)?;
        self.consume_prepared(ticket, creation, point, "Direct", random)
    }
    fn consume_prepared(
        &mut self,
        ticket: u64,
        creation: Creation,
        point: [f32; 3],
        kind: &str,
        random: &mut Random,
    ) -> Result<Vec<crate::mission_audio::Event>, String> {
        let Creation::Prepared(origin) = creation else {
            return Err("source building burst absent".into());
        };
        let context = self
            .contexts
            .get(origin)
            .ok_or("building FX context lost")?;
        let image = self
            .image
            .clone()
            .ok_or("building captured FX image absent")?;
        let entry = image
            .entries
            .get(context.entry_index)
            .ok_or("building FX selected entry lost")?;
        if entry.offset != context.entry_offset {
            return Err("building captured effect selection changed".into());
        }
        let plan =
            giants_game::effect_burst::Plan::project(&image, &entry.name).map_err(str::to_owned)?;
        let spawns = plan.create_active(&mut random.0).map_err(str::to_owned)?;
        if self.bodies.len() + spawns.len() > 4096 {
            return Err("building FX owned body budget exceeded".into());
        }
        let references = References::load(&self.source, &image, &spawns)?;
        let matrices = references.matrices(&spawns, point)?;
        let mut sounds = vec![];
        for (spawn, matrix) in spawns.into_iter().zip(matrices) {
            if spawn.attribute.is_some_and(|v| v != 1) {
                return Err(
                    "building FX outside reviewed world-attraction attribute profile".into(),
                );
            }
            for sound in &spawn.sounds {
                if sound.attached {
                    return Err("building FX needs its attached audio owner".into());
                }
                println!(
                    "mission_building_sound_request ticket={ticket} sound={} point={point:?} route=position sink_acknowledged=false",
                    sound.id
                );
                sounds.push(crate::mission_audio::Event::NpcPosition {
                    sound: sound.id,
                    position: point,
                });
            }
            let model = self.model(&spawn)?;
            let mut playback = Playback::new(spawn, matrix).map_err(str::to_owned)?;
            if playback.spawn.contact_requested {
                let program = self
                    .source
                    .classes
                    .models
                    .first_program(playback.spawn.class)
                    .ok_or("building particle class absent")?
                    .program
                    .as_ref()
                    .map_err(Clone::clone)?;
                playback
                    .bind_static_contact(
                        giants_game::effect_contact::Seed::from_class(program)
                            .map_err(str::to_owned)?,
                    )
                    .map_err(str::to_owned)?;
            }
            let animation = giants_game::effect_animation::Playback::new(
                giants_game::actor_context::Prepared::class_initial(model.actor.clone()),
            )?;
            self.next = self
                .next
                .checked_add(1)
                .ok_or("building FX identity exhausted")?;
            self.bodies.push(Body {
                serial: self.next,
                model,
                playback,
                animation,
                frame: 0,
            });
            self.births += 1;
        }
        self.contexts.release_prepared(origin)?;
        println!(
            "mission_building_burst ticket={ticket} kind={kind} bodies={} births={} policy=source_births_terrain_only native_fidelity=false",
            self.bodies.len(),
            self.births
        );
        Ok(sounds)
    }
    pub(crate) fn advance(
        &mut self,
        ticket: u64,
        sample: u64,
        delta: f32,
        areas: &[giants_game::object_time::SlowArea],
        world: &giants_game::WorldAsset,
    ) -> Result<(), String> {
        if self.sample == Some(sample) {
            return Ok(());
        }
        self.bind(ticket)?;
        let query = giants_game::terrain_queries::StaticTerrainQuery::new(
            &world.terrain,
            1. / world.terrain.spacing,
            Precision::Double53,
        )?;
        for body in &mut self.bodies {
            let dt = giants_game::object_time::seconds(
                delta,
                Some(giants_game::object_time::Sample {
                    owner: body.model.context,
                    position: body.playback.motion.position,
                }),
                None,
                areas,
            ) as f32;
            if body.playback.spawn.contact_requested {
                body.playback
                    .advance_static(dt, |xy| {
                        giants_game::physical_ground::Ground::from_sample(
                            query.sample(xy)?,
                            world.terrain.width,
                        )
                    })
                    .map_err(str::to_owned)?;
            } else {
                body.playback.advance_free(dt).map_err(str::to_owned)?;
            }
            if !body.playback.expired
                && let Some(animation) = &mut body.animation
            {
                animation.advance(dt).map_err(str::to_owned)?;
            }
            body.frame += 1;
        }
        let before = self.bodies.len();
        self.bodies.retain(|b| !b.playback.expired);
        self.retired += (before - self.bodies.len()) as u64;
        if before != self.bodies.len() {
            println!(
                "mission_building_effect_retired ticket={ticket} sample={sample} world_delta={delta} births={} retired={} active={} actual_mission_clock=true",
                self.births,
                self.retired,
                self.bodies.len()
            );
        }
        self.sample = Some(sample);
        Ok(())
    }
    pub(crate) fn draws(
        &self,
        view: [f32; 16],
    ) -> Result<Vec<crate::mission_runtime::NpcDraw>, String> {
        let mut draws = vec![];
        for body in &self.bodies {
            if !body.playback.visible() {
                continue;
            }
            let matrix = body.playback.matrix().map_err(str::to_owned)?;
            let values = giants_game::camera_facing::draw_matrix(
                matrix.values,
                body.model.actor.flags.personality,
                giants_game::camera_facing::Input {
                    view,
                    position: body.playback.motion.position,
                    scale: body.playback.visual.size,
                    angle: body.playback.angles.map_or(0., |a| a[2]),
                },
                Precision::Double53,
            )?;
            let geometry = body.model.geometry.clone();
            let local = match body.animation.as_ref().map(|a| a.positions()).transpose()? {
                Some(giants_game::actor_context::PositionUpdate::Replace(p)) => p,
                _ => geometry.mesh.positions.clone(),
            };
            let positions = local
                .into_iter()
                .map(|p| {
                    let p = giants_game::uncached_pose::transform(
                        values[..12].try_into().unwrap(),
                        p,
                        Precision::Double53,
                    )?;
                    Ok([p[0], p[2], -p[1]])
                })
                .collect::<Result<Vec<_>, String>>()?;
            let texture = body
                .playback
                .texture_animation
                .as_ref()
                .map(|clock| {
                    body.model
                        .texture_names
                        .get(clock.frame() as usize)
                        .cloned()
                        .ok_or_else(|| "building texture frame missing".to_owned())
                })
                .transpose()?;
            draws.push(crate::mission_runtime::NpcDraw {
                class: body.playback.spawn.class,
                id: crate::mission_runtime::NpcDrawOwner::BuildingEffect(body.serial),
                geometry,
                positions,
                frame: body.frame,
                transparency: body.playback.visual.transparency,
                center: body.playback.motion.position,
                texture,
            });
        }
        Ok(draws)
    }
}
