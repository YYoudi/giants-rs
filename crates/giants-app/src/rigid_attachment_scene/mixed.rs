//! Source-ordered mixed FX preview. Native service completion stays false.
use super::*;
use giants_game::{effect_scene, effect_scene_runtime, procedural_aim::Matrix};
fn report_pending_services(
    cinematic: &crate::cinematic_camera::Cinematic,
    offset: usize,
    group: usize,
    phase: usize,
    bodies: &[effect_scene_runtime::Body],
    audio: &mut Vec<crate::cinematic_effect_audio::Request>,
) {
    let sequence = cinematic.sequence();
    for (object, body) in bodies.iter().enumerate() {
        if body.playback().is_some_and(|p| p.contact.is_some()) {
            println!(
                "cinematic_mixed_static_contact_bound sequence={sequence} offset={offset} group={group} class={} native_contacts_complete=false",
                body.construction.class
            );
        }
        for sound in &body.construction.context_sounds {
            audio.push(crate::cinematic_effect_audio::Request {
                key: crate::cinematic_effect_audio::Key {
                    family: "mixed",
                    generation: cinematic.generation(),
                    director: cinematic.director_index(),
                    command: offset,
                    group,
                    phase,
                    object,
                    record: sound.record_offset,
                },
                sequence: sequence.into(),
                id: sound.id,
                binding: crate::cinematic_effect_audio::Binding::Director,
                position: None,
            });
            println!(
                "cinematic_context_sound_pending sequence={sequence} offset={offset} group={group} record_offset={} id={} before_object=true audio_pending=true audio_executed=false",
                sound.record_offset, sound.id
            );
        }
        let needs = &body.construction.object_services;
        if needs.logged_context {
            println!(
                "cinematic_logged_context_pending sequence={sequence} offset={offset} group={group} class={} registration_pending=true external_resume_pending=true cleanup_pending=true native_service_complete=false",
                body.construction.class
            );
        }
        if let Some(binding) = &needs.retirement {
            println!(
                "cinematic_object_retirement_pending sequence={sequence} offset={offset} group={group} class={} effect={} program_offset={} retirement_pending=true callback_executed=false child_spawned=false",
                body.construction.class, binding.effect, binding.program_offset
            );
        }
        if needs.source16 {
            println!(
                "cinematic_object_service_pending sequence={sequence} offset={offset} group={group} class={} opcode=16 callback_executed=false",
                body.construction.class
            );
        }
        if let Some(binding) = &needs.source5a {
            println!(
                "cinematic_object_service_pending sequence={sequence} offset={offset} group={group} class={} opcode=5a effect={} parameter={} program_offset={} callback_executed=false child_spawned=false",
                body.construction.class, binding.effect, binding.parameter, binding.program_offset
            );
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Presentation {
    pub matrix: Matrix,
    pub visible: bool,
    pub size: f32,
    pub transparency: f32,
    pub angles: Option<[f32; 3]>,
    pub expired: bool,
}
pub(super) struct Slot {
    pub group: usize,
    pub phase: usize,
    pub object: usize,
    pub presentation: Option<Presentation>,
}
pub(super) struct Group {
    source: Source,
    plan: Arc<effect_scene::Plan>,
    references: Arc<effect_scene_runtime::References>,
    runtime: Option<effect_scene_runtime::Runtime>,
    triggered: bool,
    activation: Option<(effect_scene::Queued, [f32; 3])>,
    position: Option<[f32; 3]>,
    first: bool,
    contexts: Vec<ContextEmitter>,
    retirements: Vec<RetirementContext>,
    reported_contacts: std::collections::BTreeSet<(usize, usize)>,
    reported_retirements: usize,
}
struct RetirementContext {
    phase: usize,
    object: usize,
    member: usize,
    plan: Arc<effect_scene::Plan>,
    pending: Option<(effect_scene::Queued, [f32; 3])>,
    requested: bool,
}
impl RetirementContext {
    fn request(&mut self, phase: usize, object: usize, position: [f32; 3], seed: &mut u32) -> bool {
        if self.phase != phase || self.object != object || self.requested {
            return false;
        }
        self.pending = Some((self.plan.queue(seed), position));
        self.requested = true;
        true
    }
    fn reset(&mut self) {
        self.pending = None;
        self.requested = false;
    }
}
struct ContextEmitter {
    phase: usize,
    object: usize,
    service: usize,
    plan: Arc<effect_scene::Plan>,
    members: Vec<usize>,
    next: usize,
    first: bool,
    channel: Option<ContextChannel>,
    pending: Vec<(usize, effect_scene::Queued, [f32; 3])>,
}
enum ContextChannel {
    Reference(giants_game::effect_reference_emission::Channel),
    Position(giants_game::effect_emission::Periodic),
}
fn position_profile(construction: &effect_scene::Construction) -> Result<(), String> {
    if !matches!(
        construction.origin,
        effect_scene::Origin::Detached | effect_scene::Origin::Attached { reference: 0, .. }
    ) || construction.physics_enabled
        || construction.speed != 0.
        || construction.spin != [0.; 3]
        || construction.size_opcode == Some(0x34)
        || construction.height != 0.
        || construction.offset_z != 0.
        || construction.surface_offset.is_some()
    {
        return Err(
            "ranged position requires checked stationary or normal attached-zero profile".into(),
        );
    }
    Ok(())
}
pub(super) struct ReferenceBinding {
    pub group: usize,
    pub phase: usize,
    pub object: usize,
    pub service: usize,
    pub channel: Option<giants_game::effect_reference_emission::Channel>,
    pub parent: Option<ParentSample>,
}
pub(super) struct PositionBinding {
    pub group: usize,
    pub phase: usize,
    pub object: usize,
    pub service: usize,
}
pub(super) struct PositionSample {
    pub constructor_position: [f32; 3],
    pub position: [f32; 3],
    pub interval: f32,
    pub emitting: bool,
}
pub(super) fn position_sample(
    groups: &[Group],
    binding: &PositionBinding,
    cinematic: &crate::cinematic_camera::Cinematic,
) -> Result<Option<PositionSample>, String> {
    let Some(body) = groups
        .get(binding.group)
        .and_then(|g| g.runtime.as_ref())
        .and_then(|r| r.bodies.get(binding.phase))
        .and_then(|p| p.get(binding.object))
    else {
        return Ok(None);
    };
    let source = &groups
        .get(binding.group)
        .ok_or("position guide group absent")?
        .source;
    let (constructor_position, matrix) = position_pose(body, source, cinematic)?;
    let service = body
        .construction
        .services
        .get(binding.service)
        .ok_or("mixed position service absent")?;
    Ok(Some(PositionSample {
        constructor_position,
        position: [matrix.values[3], matrix.values[7], matrix.values[11]],
        interval: service
            .emission_configuration()
            .map_err(str::to_owned)?
            .interval,
        emitting: !body.expired(),
    }))
}
fn position_pose(
    body: &effect_scene_runtime::Body,
    source: &Source,
    cinematic: &crate::cinematic_camera::Cinematic,
) -> Result<([f32; 3], Matrix), String> {
    // Attached creation has a cold raw position cache. Its current position
    // follows the normal attachment refresh, independently of drawing/hidden.
    match body.construction.origin {
        effect_scene::Origin::Detached => {
            let matrix = body.matrix().map_err(str::to_owned)?;
            Ok((
                [matrix.values[3], matrix.values[7], matrix.values[11]],
                matrix,
            ))
        }
        effect_scene::Origin::Attached { reference: 0, .. } => {
            let (values, flags) = source.matrix(cinematic.animation(), cinematic.tick())?;
            let matrix = giants_game::effect_attachment::normal_matrix(
                Matrix { values, flags },
                body.size(),
                1.,
                body.local_reference(0)?,
            )
            .map_err(str::to_owned)?;
            Ok(([0.; 3], matrix))
        }
        _ => Err("position guide attachment profile pending".into()),
    }
}
#[derive(Clone)]
pub(super) struct ParentSample {
    pub matrix: Matrix,
    pub size: f32,
    pub references: BTreeMap<u32, Matrix>,
}
#[derive(Clone, Copy)]
pub(super) struct AttachmentParent {
    pub matrix: Matrix,
    pub size: f32,
    pub reference: Matrix,
}
pub(super) struct ReferenceSample {
    pub channel: giants_game::effect_reference_emission::Channel,
    pub points: BTreeMap<u32, [f32; 3]>,
    pub emitting: bool,
    pub parent: Option<ParentSample>,
}
pub(super) fn reference_sample(
    groups: &[Group],
    binding: &ReferenceBinding,
) -> Result<Option<ReferenceSample>, String> {
    let Some(body) = groups
        .get(binding.group)
        .and_then(|g| g.runtime.as_ref())
        .and_then(|r| r.bodies.get(binding.phase))
        .and_then(|p| p.get(binding.object))
    else {
        return Ok(None);
    };
    let service = body
        .construction
        .services
        .get(binding.service)
        .ok_or("mixed reference service index absent")?;
    let config = service.emission_configuration().map_err(str::to_owned)?;
    let channel =
        giants_game::effect_reference_emission::Channel::new(&config).map_err(str::to_owned)?;
    let range = channel.mode.parent_range();
    let paired = matches!(
        channel.mode,
        giants_game::effect_reference_emission::Mode::Paired(_)
    );
    if range[1] > 127 {
        return Err("mixed reference index budget".into());
    }
    let mut points = BTreeMap::new();
    let mut references = BTreeMap::new();
    if !body.expired() {
        for index in range[0]..=range[1] {
            points.insert(index, body.reference_point(index as usize)?);
            if paired {
                references.insert(index, body.local_reference(index as usize)?);
            }
        }
    }
    Ok(Some(ReferenceSample {
        channel,
        points,
        emitting: !body.expired(),
        parent: if paired && !body.expired() {
            Some(ParentSample {
                matrix: body.matrix().map_err(str::to_owned)?,
                size: body.size(),
                references,
            })
        } else {
            None
        },
    }))
}
type PreparedGroups = (Vec<Group>, Vec<Instance>, Vec<Emitter>);

pub(super) fn wait_for_trigger(groups: &mut [Group]) -> Result<(), String> {
    let root = groups.first_mut().ok_or("mixed trigger root absent")?;
    if effect_scene_runtime::templates(&root.plan)?
        .iter()
        .flatten()
        .any(|o| matches!(o.origin, effect_scene::Origin::Attached { .. }))
    {
        return Err("point context external attachment anchor unresolved".into());
    }
    root.triggered = true;
    Ok(())
}

pub(super) fn queue_root(
    scene: &mut Prepared,
    index: usize,
    position: [f32; 3],
    seed: &mut u32,
) -> Result<(), String> {
    let root = scene
        .mixed
        .get_mut(index)
        .ok_or("mixed trigger group absent")?;
    if !root.triggered
        || root.activation.is_some()
        || root.runtime.is_some()
        || !position.iter().all(|v| v.is_finite())
    {
        return Err("mixed point activation unavailable or repeated".into());
    }
    root.activation = Some((root.plan.queue(seed), position));
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare(
    request: &giants_formats::flick::EffectCommand,
    actors: Arc<giants_game::actor_links::Projection>,
    effects: &giants_formats::effect_image::Image,
    resources: &giants_game::scene_resources::SceneResources,
    surface_member: &str,
    registered: &mut giants_game::attachment_resources::Registered<'_>,
    group: usize,
    base: usize,
    emitter_base: usize,
    sequence: &str,
) -> Result<PreparedGroups, String> {
    prepare_inner(
        request,
        actors,
        effects,
        resources,
        surface_member,
        registered,
        group,
        base,
        emitter_base,
        sequence,
        0,
        // Preparation reserves contexts that may be activated at different
        // times. This bound is not the native live-context pool capacity.
        &mut 256,
    )
}
#[allow(clippy::too_many_arguments)]
fn prepare_inner(
    request: &giants_formats::flick::EffectCommand,
    actors: Arc<giants_game::actor_links::Projection>,
    effects: &giants_formats::effect_image::Image,
    resources: &giants_game::scene_resources::SceneResources,
    surface_member: &str,
    registered: &mut giants_game::attachment_resources::Registered<'_>,
    group: usize,
    base: usize,
    emitter_base: usize,
    sequence: &str,
    depth: usize,
    budget: &mut usize,
) -> Result<PreparedGroups, String> {
    if depth > 3 || *budget == 0 {
        return Err("composed context depth/group budget".into());
    }
    *budget -= 1;
    let name = request
        .name
        .as_deref()
        .ok_or("mixed numeric FX selection unresolved")?;
    let plan = Arc::new(effect_scene::Plan::project(effects, name).map_err(str::to_owned)?);
    let templates = effect_scene_runtime::templates(&plan).map_err(str::to_owned)?;
    if depth > 0
        && templates.iter().flatten().any(|construction| {
            matches!(construction.origin, effect_scene::Origin::Attached { .. })
        })
    {
        return Err("point context external attachment anchor unresolved".into());
    }
    let mut references = effect_scene_runtime::References::load(resources, effects, &templates)?;
    if templates.iter().flatten().any(|o| {
        o.surface_offset.is_some()
            || (o.contact_requested && o.physics_enabled && o.lifetime.is_some())
    }) {
        let bytes = resources
            .archives
            .read_bin(surface_member, &resources.language)
            .map_err(|e| e.to_string())?;
        let terrain = Arc::new(giants_formats::gti::parse(&bytes).map_err(|e| e.to_string())?);
        let inverse = (1.0_f64 / f64::from(terrain.spacing)) as f32;
        references = references.with_static_surface(terrain, inverse)?;
        println!(
            "cinematic_surface_profile sequence={sequence} static_terrain=true reciprocal_supplied=true dynamic_ripple_verified=false setter_side_effects_verified=false"
        );
    }
    if templates
        .iter()
        .flatten()
        .any(|o| o.contact_requested && o.physics_enabled && o.lifetime.is_some())
    {
        references = references.with_static_contacts(resources, &templates)?;
    }
    let references = Arc::new(references);
    for phase in &templates {
        references.validate(phase)?;
    }
    let mut prototypes = BTreeMap::<i32, Result<Instance, String>>::new();
    let mut instances = Vec::new();
    let mut deferred = 0;
    for (phase, objects) in templates.iter().enumerate() {
        for (object, construction) in objects.iter().enumerate() {
            if construction.hidden {
                continue;
            }
            let prototype = prototypes.entry(construction.class).or_insert_with(|| {
                let p = resources
                    .classes
                    .models
                    .first_program(construction.class)
                    .ok_or("mixed class absent")?
                    .program
                    .as_ref()
                    .map_err(Clone::clone)?;
                let facing =
                    giants_game::released_effect::state78_burst_facing(construction.class, p)
                        .map_err(str::to_owned)?;
                unlit_effect_instance(
                    construction.class,
                    facing,
                    request,
                    actors.clone(),
                    effects,
                    resources,
                    registered,
                )
            });
            let prototype = match prototype {
                Ok(p) => p,
                Err(error) => {
                    deferred += 1;
                    println!(
                        "cinematic_mixed_object_deferred sequence={sequence} offset={} phase={phase} object={object} class={} reason={error:?} native_visibility_unknown=true",
                        request.offset, construction.class
                    );
                    continue;
                }
            };
            instances.push(Instance {
                persistent_attachment: false,
                source: prototype.source.clone(),
                geometry: prototype.geometry.clone(),
                parts: prototype.parts.clone(),
                normals: None,
                facing: prototype.facing,
                animation: prototype
                    .animation
                    .as_ref()
                    .map(giants_game::effect_animation::Playback::fresh_instance)
                    .transpose()?,
                timed: None,
                released: None,
                projectile: None,
                burst: None,
                emitted: None,
                retirement_child: None,
                detached: false,
                creation_position: None,
                mixed: Some(Slot {
                    group,
                    phase,
                    object,
                    presentation: None,
                }),
            });
        }
    }
    let source = if let Some(instance) = instances.first() {
        instance.source.clone()
    } else {
        let class = templates
            .first()
            .and_then(|p| p.first())
            .ok_or("mixed source construction absent")?
            .class;
        Source::Effect {
            request: request.clone(),
            asset: registered.asset(class)?,
            actors: actors.clone(),
            class,
        }
    };
    let mut context_specs = Vec::new();
    let pending_services = usize::from(plan.requires_external_resume())
        + templates
            .iter()
            .flatten()
            .map(|o| o.services.len() + o.context_sounds.len() + o.object_services.pending_count())
            .sum::<usize>();
    let primary_count = instances.len();
    let mut emitters = Vec::new();
    for local in 0..primary_count {
        let (phase, object) = {
            let slot = instances[local]
                .mixed
                .as_ref()
                .ok_or("mixed body slot absent")?;
            (slot.phase, slot.object)
        };
        let construction = &templates[phase][object];
        for service in &construction.services {
            if service.kind != effect_scene::ServiceKind::FixedEmission {
                continue;
            }
            let prepared = (|| -> Result<_, String> {
                let config = service.emission_configuration().map_err(str::to_owned)?;
                let recipe = giants_game::effect_emitter::Recipe::from_child(
                    effects,
                    construction.class,
                    config.effect,
                    [config.interval; 2],
                )
                .map_err(str::to_owned)?;
                let reserve = recipe.reserve(0.).map_err(str::to_owned)?;
                periodic_instances(
                    recipe,
                    request,
                    actors.clone(),
                    effects,
                    resources,
                    registered,
                    (emitter_base + emitters.len(), reserve),
                )
            })();
            match prepared {
                Ok((mut emitter, slots)) => {
                    emitter.parent = Some(base + local);
                    instances.extend(slots);
                    emitters.push(emitter);
                }
                Err(error) => println!(
                    "cinematic_mixed_service_deferred sequence={sequence} offset={} phase={} object={} opcode={} reason={error:?}",
                    request.offset, phase, object, service.record.opcode
                ),
            }
        }
    }
    for (phase, objects) in templates.iter().enumerate() {
        for (object, construction) in objects.iter().enumerate() {
            for (service_index, service) in construction.services.iter().enumerate() {
                if service.kind != effect_scene::ServiceKind::RangedEmission {
                    continue;
                }
                let prepared = (|| -> Result<_, String> {
                    position_profile(construction)?;
                    let config = service.emission_configuration().map_err(str::to_owned)?;
                    let particle =
                        giants_game::effect_emitter::Particle::project(effects, &config.effect)
                            .map_err(str::to_owned)?;
                    if particle.retirement.is_some() {
                        return Err("ranged position retirement chain pending".into());
                    }
                    let recipe = giants_game::effect_emitter::Recipe {
                        class: construction.class,
                        child: config.effect,
                        interval: service.interval_bounds().map_err(str::to_owned)?,
                        particles: vec![particle],
                    };
                    // One render prototype; actual births grow the slot pool.
                    let reserve = 1;
                    periodic_instances(
                        recipe,
                        request,
                        actors.clone(),
                        effects,
                        resources,
                        registered,
                        (emitter_base + emitters.len(), reserve),
                    )
                })();
                match prepared {
                    Ok((mut emitter, slots)) => {
                        emitter.position = Some(PositionBinding {
                            group,
                            phase,
                            object,
                            service: service_index,
                        });
                        instances.extend(slots);
                        emitters.push(emitter);
                    }
                    Err(_) if depth < 3 && position_profile(construction).is_ok() => {
                        context_specs.push((phase, object, service_index));
                    }
                    Err(error) => println!(
                        "cinematic_mixed_service_deferred sequence={sequence} offset={} phase={phase} object={object} opcode={} reason={error:?}",
                        request.offset, service.record.opcode
                    ),
                }
            }
        }
    }
    let bound_position_previews = emitters.len();
    for (phase, objects) in templates.iter().enumerate() {
        for (object, construction) in objects.iter().enumerate() {
            for (service_index, service) in construction.services.iter().enumerate() {
                if !matches!(
                    service.kind,
                    effect_scene::ServiceKind::Source28 | effect_scene::ServiceKind::Source57
                ) {
                    continue;
                }
                let prepared = (|| -> Result<_, String> {
                    let config = service.emission_configuration().map_err(str::to_owned)?;
                    let channel = giants_game::effect_reference_emission::Channel::new(&config)
                        .map_err(str::to_owned)?;
                    let range = channel.mode.parent_range();
                    let paired = matches!(
                        channel.mode,
                        giants_game::effect_reference_emission::Mode::Paired(_)
                    );
                    if range[1] > 127 {
                        return Err("mixed reference index budget".into());
                    }
                    let particle =
                        giants_game::effect_emitter::Particle::project(effects, &config.effect)
                            .map_err(str::to_owned)?;
                    if particle.retirement.is_some() {
                        return Err("reference particle retirement chain pending".into());
                    }
                    if paired {
                        particle.attachment_profile().map_err(str::to_owned)?;
                        if effect_scene_runtime::curve_reference_pending(construction) {
                            return Err("paired parent curve34 scale pending".into());
                        }
                    }
                    let bounds = service.interval_bounds().map_err(str::to_owned)?;
                    let recipe = giants_game::effect_emitter::Recipe {
                        class: construction.class,
                        child: config.effect,
                        interval: [bounds[0].min(bounds[1]); 2],
                        particles: vec![particle],
                    };
                    let reserve = recipe.reserve(0.).map_err(str::to_owned)?;
                    let result = periodic_instances(
                        recipe,
                        request,
                        actors.clone(),
                        effects,
                        resources,
                        registered,
                        (emitter_base + emitters.len(), reserve),
                    )?;
                    if let giants_game::effect_reference_emission::Mode::Paired([own, _]) =
                        channel.mode
                    {
                        let instance =
                            result.1.first().ok_or("paired particle prototype absent")?;
                        if instance.facing.is_some_and(|f| f.personality & 0x84 != 4) {
                            return Err(
                                "paired camera-facing personality outside checked profile".into()
                            );
                        }
                        let pose = instance
                            .animation
                            .as_ref()
                            .ok_or("paired child reference pose absent")?;
                        if own[1] > 127 {
                            return Err("paired child reference index budget".into());
                        }
                        for index in own[0]..=own[1] {
                            pose.reference(index as usize)?;
                        }
                    }
                    Ok(result)
                })();
                match prepared {
                    Ok((mut emitter, slots)) => {
                        emitter.reference = Some(ReferenceBinding {
                            group,
                            phase,
                            object,
                            service: service_index,
                            channel: None,
                            parent: None,
                        });
                        instances.extend(slots);
                        emitters.push(emitter);
                    }
                    Err(error) => {
                        if service.kind == effect_scene::ServiceKind::Source28 && depth < 3 {
                            context_specs.push((phase, object, service_index));
                        } else {
                            println!(
                                "cinematic_mixed_service_deferred sequence={sequence} offset={} phase={phase} object={object} opcode={} reason={error:?}",
                                request.offset, service.record.opcode
                            );
                        }
                    }
                }
            }
        }
    }
    let bound_reference_previews = emitters.len() - bound_position_previews;
    let mut groups = vec![Group {
        source,
        plan: plan.clone(),
        references,
        runtime: None,
        triggered: false,
        activation: None,
        position: None,
        first: false,
        contexts: Vec::new(),
        retirements: Vec::new(),
        reported_contacts: std::collections::BTreeSet::new(),
        reported_retirements: 0,
    }];
    let bounds = plan.lifetime_bounds();
    for (phase, object, service_index) in context_specs {
        let result = (|| -> Result<_, String> {
            let service = &templates[phase][object].services[service_index];
            let config = service.emission_configuration().map_err(str::to_owned)?;
            if service.kind == effect_scene::ServiceKind::RangedEmission {
                position_profile(&templates[phase][object])?;
            } else {
                let channel = giants_game::effect_reference_emission::Channel::new(&config)
                    .map_err(str::to_owned)?;
                if !matches!(
                    channel.mode,
                    giants_game::effect_reference_emission::Mode::Single(_)
                ) {
                    return Err("composed child requires point channel".into());
                }
            }
            let life = bounds[phase][object]
                .ok_or("composed request reserve requires bounded parent lifetime")?;
            let interval = service.interval_bounds().map_err(str::to_owned)?;
            let capacity = (f64::from(life) / f64::from(interval[0].min(interval[1]))).ceil() + 2.;
            if !capacity.is_finite() || !(1. ..=64.).contains(&capacity) {
                return Err("composed request reserve budget".into());
            }
            let count = capacity as usize;
            let child_plan = Arc::new(
                effect_scene::Plan::project(effects, &config.effect).map_err(str::to_owned)?,
            );
            let mut child_request = request.clone();
            child_request.name = Some(config.effect);
            let mut children = Vec::new();
            let mut child_instances = Vec::new();
            let mut child_emitters = Vec::new();
            let mut members = Vec::new();
            for _ in 0..count {
                let index = group + groups.len() + children.len();
                let (mut prepared, objects, services) = prepare_inner(
                    &child_request,
                    actors.clone(),
                    effects,
                    resources,
                    surface_member,
                    registered,
                    index,
                    base + instances.len() + child_instances.len(),
                    emitter_base + emitters.len() + child_emitters.len(),
                    sequence,
                    depth + 1,
                    budget,
                )?;
                prepared[0].triggered = true;
                members.push(index);
                children.extend(prepared);
                child_instances.extend(objects);
                child_emitters.extend(services);
            }
            Ok((
                ContextEmitter {
                    phase,
                    object,
                    service: service_index,
                    plan: child_plan,
                    members,
                    next: 0,
                    first: true,
                    channel: None,
                    pending: Vec::new(),
                },
                children,
                child_instances,
                child_emitters,
            ))
        })();
        match result {
            Ok((context, children, objects, services)) => {
                println!(
                    "cinematic_composed_context_prepared sequence={sequence} offset={} parent_group={group} phase={phase} object={object} reserve={} request_satisfied=false",
                    request.offset,
                    context.members.len()
                );
                groups[0].contexts.push(context);
                groups.extend(children);
                instances.extend(objects);
                emitters.extend(services);
            }
            Err(error) => println!(
                "cinematic_mixed_service_deferred sequence={sequence} offset={} phase={phase} object={object} opcode={} reason={error:?}",
                request.offset,
                templates[phase][object].services[service_index]
                    .record
                    .opcode
            ),
        }
    }
    for (phase, objects) in templates.iter().enumerate() {
        for (object, construction) in objects.iter().enumerate() {
            let Some(binding) = &construction.object_services.retirement else {
                continue;
            };
            let result = (|| -> Result<_, String> {
                if construction.lifetime.is_none()
                    || construction.object_services.source16
                    || construction.object_services.source5a.is_some()
                    || construction.object_services.logged_context
                {
                    return Err("retirement requires isolated finite state78 ownership".into());
                }
                let program = resources
                    .classes
                    .models
                    .first_program(construction.class)
                    .ok_or("retirement class absent")?
                    .program
                    .as_ref()
                    .map_err(Clone::clone)?;
                giants_game::released_effect::state78_burst_facing(construction.class, program)
                    .map_err(str::to_owned)?;
                let child_plan = Arc::new(
                    effect_scene::Plan::project(effects, &binding.effect).map_err(str::to_owned)?,
                );
                let mut child_request = request.clone();
                child_request.name = Some(binding.effect.clone());
                let member = group + groups.len();
                let (mut prepared, children, services) = prepare_inner(
                    &child_request,
                    actors.clone(),
                    effects,
                    resources,
                    surface_member,
                    registered,
                    member,
                    base + instances.len(),
                    emitter_base + emitters.len(),
                    sequence,
                    depth + 1,
                    budget,
                )?;
                prepared[0].triggered = true;
                Ok((member, child_plan, prepared, children, services))
            })();
            match result {
                Ok((member, plan, children, objects, services)) => {
                    println!(
                        "cinematic_mixed_retirement_prepared sequence={sequence} offset={} parent_group={group} phase={phase} object={object} child_group={member} effect={} density_policy=full_detail_preview native_density_verified=false request_satisfied=false",
                        request.offset, binding.effect
                    );
                    groups[0].retirements.push(RetirementContext {
                        phase,
                        object,
                        member,
                        plan,
                        pending: None,
                        requested: false,
                    });
                    groups.extend(children);
                    instances.extend(objects);
                    emitters.extend(services);
                }
                Err(error) => println!(
                    "cinematic_mixed_retirement_deferred sequence={sequence} offset={} parent_group={group} phase={phase} object={object} effect={} reason={error:?} child_spawned=false",
                    request.offset, binding.effect
                ),
            }
        }
    }
    if instances.is_empty() {
        return Err("mixed context has no admitted drawable objects or descendants".into());
    }
    let sound_needs: usize = templates
        .iter()
        .flatten()
        .map(|o| o.context_sounds.len())
        .sum();
    if plan.requires_external_resume() {
        println!(
            "cinematic_context_resume_pending sequence={sequence} offset={} external_resume_pending=true timed_wait=false native_service_complete=false",
            request.offset
        );
    }
    if sound_needs > 0 {
        println!(
            "cinematic_mixed_context_sounds_prepared sequence={sequence} offset={} count={sound_needs} audio_pending=true audio_executed=false",
            request.offset
        );
    }
    println!(
        "cinematic_mixed_effect_prepared sequence={sequence} offset={} phases={} constructions={} drawable_objects={} deferred_objects={} service_requests={} request_satisfied=false native_cadence_verified=false native_raster_verified=false",
        request.offset,
        templates.len(),
        templates.iter().map(Vec::len).sum::<usize>(),
        primary_count,
        deferred,
        pending_services
    );
    println!(
        "cinematic_mixed_emitters_prepared sequence={sequence} offset={} bound_position_previews={} native_service_complete=false source_flag9=false",
        request.offset, bound_position_previews
    );
    println!(
        "cinematic_mixed_reference_emitters_prepared sequence={sequence} offset={} bound_reference_previews={} native_service_complete=false",
        request.offset, bound_reference_previews
    );
    Ok((groups, instances, emitters))
}
pub(super) fn reset(scene: &mut Prepared) {
    for group in &mut scene.mixed {
        group.runtime = None;
        group.activation = None;
        group.reported_contacts.clear();
        group.reported_retirements = 0;
        group.position = None;
        group.first = false;
        for retirement in &mut group.retirements {
            retirement.reset();
        }
        for emitter in &mut group.contexts {
            emitter.next = 0;
            emitter.first = true;
            emitter.channel = None;
            emitter.pending.clear();
        }
    }
    for instance in &mut scene.instances {
        if let Some(slot) = &mut instance.mixed {
            slot.presentation = None;
        }
    }
}
pub(super) fn capture_ready(scene: &Prepared) -> bool {
    scene
        .mixed
        .iter()
        .any(|g| g.runtime.as_ref().is_some_and(|r| r.bodies.len() > 1))
        && scene.instances.iter().any(|i| {
            i.mixed
                .as_ref()
                .and_then(|m| m.presentation)
                .is_some_and(|p| p.visible && p.transparency < 1.)
        })
}
/// The timed snapshot selector also covers finite bodies owned by a mixed
/// context. Its separate multi-phase selector keeps its stronger condition.
pub(super) fn finite_capture_ready(scene: &Prepared) -> bool {
    scene.instances.iter().any(|instance| {
        if instance.parts.is_empty() {
            return false;
        }
        let Some(slot) = &instance.mixed else {
            return false;
        };
        slot.presentation
            .is_some_and(|p| p.visible && p.transparency < 1.)
            && scene
                .mixed
                .get(slot.group)
                .and_then(|g| g.runtime.as_ref())
                .and_then(|r| r.bodies.get(slot.phase))
                .and_then(|p| p.get(slot.object))
                .is_some_and(|b| b.construction.lifetime.is_some())
    })
}
pub(super) fn update(
    scene: &mut Prepared,
    cinematic: &crate::cinematic_camera::Cinematic,
    reset_generation: bool,
    dt: f32,
    seed: &mut u32,
    audio: &mut Vec<crate::cinematic_effect_audio::Request>,
) -> Result<(), String> {
    if reset_generation {
        reset(scene);
    }
    if dt <= 0. {
        return Ok(());
    }
    let mut pending: Vec<_> = scene
        .mixed
        .iter_mut()
        .flat_map(|g| {
            g.contexts
                .iter_mut()
                .flat_map(|e| std::mem::take(&mut e.pending))
        })
        .collect();
    for (index, group) in scene.mixed.iter_mut().enumerate() {
        if let Some((queued, position)) = group.activation.take() {
            pending.push((index, queued, position));
        }
        for retirement in &mut group.retirements {
            if let Some((queued, position)) = retirement.pending.take() {
                pending.push((retirement.member, queued, position));
            }
        }
    }
    for (index, queued, position) in pending {
        let group = scene
            .mixed
            .get_mut(index)
            .ok_or("composed child group absent")?;
        if group.runtime.is_some() {
            return Err("composed context slot reused before director reset".into());
        }
        group.runtime = Some(effect_scene_runtime::Runtime::activate(
            queued,
            &group.references,
            Matrix::identity(),
            position,
            seed,
        )?);
        group.position = Some(position);
        group.first = true;
        println!(
            "cinematic_composed_context_activated sequence={} group={index} position_bits={:?} native_cadence_verified=false",
            cinematic.sequence(),
            position.map(f32::to_bits)
        );
    }
    for (index, group) in scene.mixed.iter_mut().enumerate() {
        if group.triggered {
            if let Some(runtime) = &mut group.runtime {
                let position = group
                    .position
                    .ok_or("composed child frozen position absent")?;
                let phase = if group.first {
                    group.first = false;
                    runtime.advance_bodies_with_references(dt, &group.references)?;
                    Some(0)
                } else {
                    runtime.advance(dt, &group.references, Matrix::identity(), position, seed)?
                };
                if let Some(phase) = phase {
                    let Source::Effect { request, .. } = &group.source else {
                        unreachable!()
                    };
                    report_pending_services(
                        cinematic,
                        request.offset,
                        index,
                        phase,
                        &runtime.bodies[phase],
                        audio,
                    );
                    println!(
                        "cinematic_mixed_phase sequence={} offset={} group={index} phase={phase} constructions={} services_pending={} native_cadence_verified=false",
                        cinematic.sequence(),
                        request.offset,
                        runtime.bodies[phase].len(),
                        runtime.bodies[phase]
                            .iter()
                            .map(|b| b.construction.services.len()
                                + b.construction.context_sounds.len()
                                + b.construction.object_services.pending_count())
                            .sum::<usize>()
                    );
                }
            }
            continue;
        }
        if group.runtime.is_none() && group.source.visible(cinematic) {
            let Source::Effect {
                request, actors, ..
            } = &group.source
            else {
                return Err("mixed source absent".into());
            };
            let (seconds, generation) = cinematic.segments()[cinematic.director_index()]
                .effect_sample(request.offset)
                .ok_or("mixed source clock absent")?;
            let (values, flags) = Source::effect_matrix(actors, request, seconds)?;
            let owner = giants_game::effect_attachment::source_creation_matrix(
                generation,
                request.actor,
                Matrix { values, flags },
            );
            let position = giants_game::effect_attachment::source_creation_position(
                generation,
                request.actor,
                owner,
            );
            let mut runtime = effect_scene_runtime::Runtime::start(
                group.plan.clone(),
                &group.references,
                owner,
                position,
                seed,
            )?;
            runtime.advance_bodies_with_references(dt, &group.references)?;
            report_pending_services(
                cinematic,
                request.offset,
                index,
                0,
                &runtime.bodies[0],
                audio,
            );
            println!(
                "cinematic_mixed_phase sequence={} offset={} group={index} phase=0 constructions={} services_pending={} owner_cache_generation={generation} native_cadence_verified=false",
                cinematic.sequence(),
                request.offset,
                runtime.bodies[0].len(),
                runtime.bodies[0]
                    .iter()
                    .map(|b| b.construction.services.len()
                        + b.construction.context_sounds.len()
                        + b.construction.object_services.pending_count())
                    .sum::<usize>()
            );
            group.runtime = Some(runtime);
            group.position = Some(position);
        } else if let Some(runtime) = &mut group.runtime {
            let (values, flags) = group
                .source
                .matrix(cinematic.animation(), cinematic.tick())?;
            let owner = Matrix { values, flags };
            // The support's raw constructor position is not its refreshed
            // attachment matrix translation. Later detached phases read it.
            let position = group
                .position
                .ok_or("mixed source constructor position absent")?;
            if let Some(phase) = runtime.advance(dt, &group.references, owner, position, seed)? {
                let Source::Effect { request, .. } = &group.source else {
                    unreachable!()
                };
                report_pending_services(
                    cinematic,
                    request.offset,
                    index,
                    phase,
                    &runtime.bodies[phase],
                    audio,
                );
                println!(
                    "cinematic_mixed_phase sequence={} offset={} group={index} phase={phase} constructions={} services_pending={} native_cadence_verified=false",
                    cinematic.sequence(),
                    request.offset,
                    runtime.bodies[phase].len(),
                    runtime.bodies[phase]
                        .iter()
                        .map(|b| b.construction.services.len()
                            + b.construction.context_sounds.len()
                            + b.construction.object_services.pending_count())
                        .sum::<usize>()
                );
            }
        }
    }
    for (index, group) in scene.mixed.iter_mut().enumerate() {
        let Some(runtime) = &group.runtime else {
            continue;
        };
        let Source::Effect { request, .. } = &group.source else {
            unreachable!()
        };
        for retired in runtime
            .retirements()
            .iter()
            .skip(group.reported_retirements)
        {
            if let Some(child) = group
                .retirements
                .iter_mut()
                .find(|child| child.phase == retired.phase && child.object == retired.object)
                && child.request(retired.phase, retired.object, retired.position, seed)
            {
                println!(
                    "cinematic_mixed_retirement_queued sequence={} offset={} parent_group={index} phase={} object={} child_group={} effect={} position_bits={:?} schedule=next_positive_viewer_update child_spawned=false native_cadence_verified=false",
                    cinematic.sequence(),
                    request.offset,
                    retired.phase,
                    retired.object,
                    child.member,
                    retired.binding.effect,
                    retired.position.map(f32::to_bits)
                );
            }
            println!(
                "cinematic_object_retirement_requested sequence={} offset={} group={index} phase={} object={} effect={} program_offset={} position_bits={:?} child_activation_pending=true child_spawned=false native_cadence_verified=false",
                cinematic.sequence(),
                request.offset,
                retired.phase,
                retired.object,
                retired.binding.effect,
                retired.binding.program_offset,
                retired.position.map(f32::to_bits)
            );
        }
        group.reported_retirements = runtime.retirements().len();
        for (phase, bodies) in runtime.bodies.iter().enumerate() {
            for (object, body) in bodies.iter().enumerate() {
                if let Some(contact) = body.playback().and_then(|p| p.contact.as_ref())
                    && contact.responses > 0
                    && group.reported_contacts.insert((phase, object))
                {
                    println!(
                        "cinematic_mixed_static_contact_response sequence={} group={index} phase={phase} object={object} class={} steps={} flags={} native_contacts_complete=false native_scene_fidelity_verified=false",
                        cinematic.sequence(),
                        body.construction.class,
                        contact.steps,
                        contact.flags
                    );
                }
            }
        }
        for emitter in &mut group.contexts {
            let Some(body) = runtime
                .bodies
                .get(emitter.phase)
                .and_then(|p| p.get(emitter.object))
            else {
                continue;
            };
            if body.expired() {
                continue;
            }
            if emitter.first {
                emitter.first = false;
                continue;
            }
            let service = body
                .construction
                .services
                .get(emitter.service)
                .ok_or("composed service absent")?;
            if emitter.channel.is_none() {
                let config = service.emission_configuration().map_err(str::to_owned)?;
                emitter.channel = Some(
                    if service.kind == effect_scene::ServiceKind::RangedEmission {
                        let (constructor_position, _) =
                            position_pose(body, &group.source, cinematic)?;
                        ContextChannel::Position(giants_game::effect_emission::Periodic {
                            interval: config.interval,
                            remaining: 0.,
                            phase: 0.,
                            previous_position: constructor_position,
                        })
                    } else {
                        ContextChannel::Reference(
                            giants_game::effect_reference_emission::Channel::new(&config)
                                .map_err(str::to_owned)?,
                        )
                    },
                );
            }
            let positions = match emitter.channel.as_mut().ok_or("composed channel absent")? {
                ContextChannel::Position(channel) => {
                    let (_, matrix) = position_pose(body, &group.source, cinematic)?;
                    let step = channel
                        .advance(dt, [matrix.values[3], matrix.values[7], matrix.values[11]])
                        .map_err(str::to_owned)?;
                    *channel = step.state;
                    step.positions
                        .into_iter()
                        .map(|p| (None, p))
                        .collect::<Vec<_>>()
                }
                ContextChannel::Reference(channel) => {
                    let step = channel.advance(dt, seed).map_err(str::to_owned)?;
                    *channel = step.state;
                    match step.request {
                        Some(giants_game::effect_reference_emission::Selected::Paired(_)) => {
                            return Err("composed context paired request unresolved".into());
                        }
                        Some(giants_game::effect_reference_emission::Selected::Single(
                            reference,
                        )) => vec![(Some(reference), body.reference_point(reference as usize)?)],
                        None => Vec::new(),
                    }
                }
            };
            for (reference, position) in positions {
                let child = *emitter
                    .members
                    .get(emitter.next)
                    .ok_or("composed request reserve exhausted; no request dropped")?;
                emitter.next += 1;
                emitter
                    .pending
                    .push((child, emitter.plan.queue(seed), position));
                let Source::Effect { request, .. } = &group.source else {
                    unreachable!()
                };
                println!(
                    "cinematic_composed_context_requested sequence={} offset={} parent_group={index} child_group={child} reference={} position_bits={:?} request_satisfied=false native_cadence_verified=false",
                    cinematic.sequence(),
                    request.offset,
                    reference.map_or_else(|| "none".to_owned(), |r| r.to_string()),
                    position.map(f32::to_bits)
                );
            }
        }
    }
    for instance in &mut scene.instances {
        let Some(slot) = &mut instance.mixed else {
            continue;
        };
        slot.presentation = if let Some(body) = scene.mixed[slot.group]
            .runtime
            .as_ref()
            .and_then(|r| r.bodies.get(slot.phase))
            .and_then(|p| p.get(slot.object))
        {
            Some(Presentation {
                matrix: body.matrix().map_err(str::to_owned)?,
                visible: body.visible(),
                size: body.size(),
                transparency: body.transparency(),
                angles: body.angles(),
                expired: body.expired(),
            })
        } else {
            None
        };
    }
    Ok(())
}

#[cfg(test)]
mod retirement_tests {
    #[test]
    fn routing_keeps_identity_and_preparation_rng_once_until_reset() {
        let mut program = vec![3];
        program.extend(7u32.to_le_bytes());
        program.extend([6, 0x1c]);
        program.extend(1f32.to_le_bytes());
        program.extend(2f32.to_le_bytes());
        program.push(4);
        for v in [1f32, 0., 0.] {
            program.extend(v.to_le_bytes());
        }
        program.extend([7, 2]);
        let mut body = b"child\0".to_vec();
        body.extend(10u32.to_le_bytes());
        body.extend(program);
        let mut bytes = 1u32.to_le_bytes().to_vec();
        bytes.extend((body.len() as u32).to_le_bytes());
        bytes.extend(1u32.to_le_bytes());
        bytes.extend(body);
        let image = giants_formats::effect_image::parse(&bytes).unwrap();
        let plan =
            std::sync::Arc::new(giants_game::effect_scene::Plan::project(&image, "child").unwrap());
        let mut child = super::RetirementContext {
            phase: 2,
            object: 3,
            member: 4,
            plan,
            pending: None,
            requested: false,
        };
        let mut seed = 17;
        assert!(!child.request(1, 3, [0.; 3], &mut seed));
        assert_eq!(seed, 17);
        assert!(child.request(2, 3, [7., -11., 13.], &mut seed));
        let prepared = seed;
        assert_ne!(prepared, 17);
        let (queued, position) = child.pending.take().unwrap();
        assert_eq!(position, [7., -11., 13.]);
        assert!(!child.request(2, 3, [0.; 3], &mut seed));
        assert_eq!(seed, prepared);
        let (_, objects, _) = queued.activate(&mut seed).unwrap();
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].class, 7);
        child.reset();
        assert!(child.pending.is_none());
        assert!(child.request(2, 3, [1.; 3], &mut seed));
        let before_cut = seed;
        assert!(child.pending.is_some());
        child.reset();
        assert!(child.pending.is_none());
        assert!(!child.requested);
        assert_eq!(seed, before_cut);
    }
}
