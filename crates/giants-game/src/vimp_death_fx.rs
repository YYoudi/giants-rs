//! Source-program death effects in the ordinary partial mission component.
//! Own FX keys are not Session actor IDs. Source operands, class/ANM assets,
//! shared kernels and actual terrain are consumed. Host-frame scheduling,
//! terrain-only physics and splat retention are explicit preview policies;
//! this adapter does not acknowledge a complete native state78/world callback.
//! Prototype PS2 navigation: `vimp_shot` names the related damage work; this FX
//! graph consumer is not an asserted one-to-one counterpart of that function.
//! See docs/SYMBOL_NAVIGATION.md; exact PC/prototype attribution remains unknown.
use crate::{
    audio_parameters::Random,
    effect_visual::{Configuration, SizeCurve, Visual},
    procedural_aim::Matrix,
    scene_resources::{ModelAsset, SceneResources},
};
use giants_formats::effect_program::Record;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key(pub u64);
pub struct Draw {
    pub key: Key,
    pub class: i32,
    pub frame: u64,
    pub geometry: Arc<ModelAsset>,
    pub local: Vec<[f32; 3]>,
    pub matrix: Matrix,
    pub personality: u32,
    pub size: f32,
    pub transparency: f32,
}
struct Channel {
    name: String,
    timer: crate::effect_emission::Periodic,
    reference: u32,
    paired: bool,
    deadline: Option<f32>,
}
struct Body {
    class: i32,
    matrix: Matrix,
    inherited: Option<Matrix>,
    visual: Visual,
    playback: Option<crate::effect_animation::Playback>,
    geometry: Arc<ModelAsset>,
    personality: u32,
    motion: crate::mecc_motion::Motion,
    spin: [f32; 3],
    angles: [f32; 3],
    gravity: f32,
    damping: [f32; 2],
    friction: f32,
    bounce: f32,
    retention: f32,
    ground_offset: f32,
    contact_flags: u32,
    floor: f32,
    plane: [f32; 4],
    physics: bool,
    contacts: bool,
    initialized_velocity: bool,
    speed: f32,
    hidden: bool,
    rescale: bool,
    impact_child: Option<(String, u32)>,
    channels: Vec<Channel>,
    frame: u64,
}
pub struct Runtime {
    contexts: crate::attached_effect_context::Pool<u64>,
    image: Arc<giants_formats::effect_image::Image>,
    actors: BTreeMap<Key, Body>,
    next: u64,
    classes: BTreeMap<i32, Arc<crate::actor_context::Class>>,
    templates: BTreeMap<i32, crate::effect_actor::Template>,
    geometry: BTreeMap<i32, Arc<ModelAsset>>,
    textures: crate::catalog_registry::ResourceIndex,
    pub births: u64,
    pub retired: u64,
    pub impacts: u64,
}
fn word(r: &Record, at: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        r.bytes
            .get(at..at + 4)
            .ok_or("death FX word absent")?
            .try_into()
            .unwrap(),
    ))
}
fn scalar(r: &Record, at: usize) -> Result<f32, String> {
    let v = f32::from_bits(word(r, at)?);
    if !v.is_finite() {
        return Err("death FX scalar nonfinite".into());
    }
    Ok(v)
}
fn name(r: &Record, at: usize) -> Result<(String, usize), String> {
    let b = r.bytes.get(at..).ok_or("death FX name absent")?;
    let n = b
        .iter()
        .position(|&b| b == 0)
        .ok_or("death FX name unterminated")?;
    Ok((
        std::str::from_utf8(&b[..n])
            .map_err(|e| e.to_string())?
            .to_owned(),
        at + n + 1,
    ))
}
fn sample(r: &Record, at: usize, rng: &mut Random) -> Result<f32, String> {
    let a = scalar(r, at)?;
    let b = scalar(r, at + 4)?;
    Ok((f64::from(rng.next_15()) * (f64::from(b) - f64::from(a)) / 32768. + f64::from(a)) as f32)
}
impl Runtime {
    pub fn new(
        root: &std::path::Path,
        ticket: u64,
        source: &SceneResources,
    ) -> Result<Self, String> {
        Ok(Self {
            contexts: crate::attached_effect_context::Pool::from_active_resources(ticket, source),
            image: Arc::new(
                source
                    .classes
                    .models
                    .effects
                    .last()
                    .ok_or("death FX image absent")?
                    .image
                    .clone(),
            ),
            actors: BTreeMap::new(),
            next: 1,
            classes: BTreeMap::new(),
            templates: BTreeMap::new(),
            geometry: BTreeMap::new(),
            textures: crate::catalog_registry::ResourceIndex::scan(&root.join("Bin"))
                .map_err(|e| e.to_string())?,
            births: 0,
            retired: 0,
            impacts: 0,
        })
    }
    pub fn bind_ticket(&mut self, ticket: u64) -> Result<(), String> {
        if !self.actors.is_empty() || !self.contexts.active().is_empty() || self.births != 0 {
            return Err("death FX ticket already active".into());
        }
        self.contexts = crate::attached_effect_context::Pool::new(ticket, Some(self.image.clone()));
        Ok(())
    }
    pub fn active(&self) -> usize {
        self.actors.len()
    }
    pub fn death(
        &mut self,
        source: &SceneResources,
        effect: crate::vimp_damage::DeathEffect,
        selection: crate::effect_initialization::Selection,
        rng: &mut Random,
        world: &crate::WorldAsset,
    ) -> Result<(), String> {
        let (creation, _) = self.contexts.prepare_rotated_positional(
            "FXB_VIMP_death",
            effect.position,
            [effect.angle, 0., 0.],
            selection,
            rng,
        )?;
        let crate::attached_effect_context::Creation::Prepared(origin) = creation else {
            return Err("Vimp death source effect absent".into());
        };
        self.activate(source, origin, rng, world, 0)?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)] // Source child binding plus explicit scene/RNG/recursion ownership.
    fn named(
        &mut self,
        source: &SceneResources,
        label: &str,
        position: [f32; 3],
        parent: Option<(Key, u32, u32, Option<Matrix>)>,
        rng: &mut Random,
        world: &crate::WorldAsset,
        depth: usize,
    ) -> Result<Key, String> {
        let (creation, _) = if let Some((parent, reference, own_reference, basis)) = parent {
            let image = source
                .classes
                .models
                .effects
                .last()
                .ok_or("death FX image absent")?;
            let offset = image
                .image
                .find(label)
                .ok_or("death FX child absent")?
                .1
                .offset;
            let input = crate::attached_effect_context::ImmediateInput {
                binding: crate::attached_effect_context::Immediate {
                    parent: parent.0,
                    reference,
                    own_reference,
                },
                position,
                object_flags: 0,
                render_flags: 0,
            };
            if let Some(basis) = basis {
                self.contexts
                    .prepare_immediate_inherited_offset_invariant(offset, input, basis, rng)?
            } else {
                self.contexts
                    .prepare_immediate_offset_invariant(offset, input, rng)?
            }
        } else {
            self.contexts
                .prepare_positional_invariant(label, position, rng)?
        };
        let crate::attached_effect_context::Creation::Prepared(origin) = creation else {
            return Err("death FX child context absent".into());
        };
        self.activate(source, origin, rng, world, depth)
    }
    fn activate(
        &mut self,
        source: &SceneResources,
        origin: crate::attached_effect_context::Id,
        rng: &mut Random,
        world: &crate::WorldAsset,
        depth: usize,
    ) -> Result<Key, String> {
        if depth > 8 || self.actors.len() > 4096 {
            return Err("death FX source graph budget exceeded".into());
        }
        let context = self
            .contexts
            .get(origin)
            .ok_or("death FX context stale")?
            .clone();
        let p = context.program.as_ref().map_err(Clone::clone)?;
        let class = word(&p.records[0], 1)? as i32;
        if p.records[0].opcode != 3 || p.records.get(1).is_none_or(|r| r.opcode != 6) {
            return Err("death FX constructor frame differs".into());
        }
        if let std::collections::btree_map::Entry::Vacant(entry) = self.templates.entry(class) {
            entry.insert(crate::effect_actor::Template::load(source, class)?);
        }
        let template = &self.templates[&class];
        let binding = context.immediate();
        let initial = if binding.is_some() {
            if context.inherited_matrix().is_some() {
                template
                    .immediate_inherited_positioned(origin, &context)?
                    .actor
            } else {
                template.immediate_positioned(origin, &context)?.actor
            }
        } else if context.inherited_matrix().is_some() {
            template.rotated_positioned(origin, &context)?
        } else {
            template.positioned(origin, &context)?
        };
        let image = &source
            .classes
            .models
            .effects
            .last()
            .ok_or("death FX image absent")?
            .image;
        if let std::collections::btree_map::Entry::Vacant(e) = self.classes.entry(class) {
            e.insert(Arc::new(crate::actor_context::Class::load(
                source, class, image,
            )?));
        }
        if let std::collections::btree_map::Entry::Vacant(e) = self.geometry.entry(class) {
            e.insert(Arc::new(
                source
                    .model_slot_for_body(
                        class,
                        0,
                        crate::scene_resources::TextureEncoding::Rgb565OrArgb4444,
                    )
                    .map_err(|e| e.to_string())?
                    .ok_or("death FX model absent")?,
            ));
        }
        let geometry = self.geometry[&class].clone();
        let playback = crate::effect_animation::Playback::new(
            crate::actor_context::Prepared::class_initial(self.classes[&class].clone()),
        )?;
        let fields = initial.fields;
        let position = context.position().ok_or("death FX origin absent")?;
        let mut body = Body {
            class,
            matrix: initial.matrix,
            inherited: context.inherited_matrix(),
            visual: Visual::new(Configuration {
                lifetime: 1.,
                fade_in: 0.,
                fade_out_start: 0.,
                start_size: fields.scale,
                end_size: fields.scale,
            })?,
            playback,
            geometry,
            personality: fields.configuration[14],
            motion: crate::mecc_motion::Motion {
                position,
                velocity: [0.; 3],
                previous_velocity: [0.; 3],
                force: [0.; 3],
                mass: 1.,
                speed_limit: 0.,
                force_time_limit: 0.,
                stored_dt: 0.,
            },
            spin: [0.; 3],
            angles: fields.angles,
            gravity: 1.,
            damping: [0.; 2],
            friction: 1.,
            bounce: 0.,
            retention: 1.,
            ground_offset: f32::from_bits(fields.configuration[17]),
            contact_flags: 0,
            floor: 0.,
            plane: [0.; 4],
            physics: false,
            contacts: false,
            initialized_velocity: false,
            speed: 0.,
            hidden: false,
            rescale: false,
            impact_child: None,
            channels: vec![],
            frame: 0,
        };
        let key = Key(self.next);
        self.next = self
            .next
            .checked_add(1)
            .ok_or("death FX identity exhausted")?;
        self.births += 1;
        // Retain this actual owner before executing its nested requests.
        let mut life = [1., 0., 0.];
        let mut sizes = [fields.scale; 2];
        let mut curve = SizeCurve::Linear;
        for r in p.records.iter().skip(2) {
            match r.opcode {
                4 => life = [scalar(r, 1)?, scalar(r, 5)?, scalar(r, 9)?],
                5 => life = [sample(r, 1, rng)?, sample(r, 9, rng)?, sample(r, 17, rng)?],
                8 => sizes = [scalar(r, 1)?, scalar(r, 5)?],
                9 => sizes = [sample(r, 1, rng)?, sample(r, 9, rng)?],
                0x36 => {
                    sizes = [scalar(r, 1)?, scalar(r, 5)?];
                    curve = SizeCurve::EaseOut;
                }
                0x1c => {
                    let s = sample(r, 1, rng)?;
                    sizes = [s; 2];
                }
                0x21 => body.hidden = true,
                0x33 => {
                    let first = sample(r, 1, rng)?;
                    let second = sample(r, 9, rng)?;
                    let third = sample(r, 17, rng)?;
                    body.spin = [second, first, third];
                }
                0x13 => {
                    body.damping = [scalar(r, 1)?, scalar(r, 5)?];
                    body.physics = true;
                }
                0x69 => body.retention = scalar(r, 1)?,
                0x67 => body.friction = scalar(r, 1)?,
                0x12 => {
                    body.speed = sample(r, 1, rng)?;
                    body.physics = true;
                }
                0x5f => {
                    body.gravity = scalar(r, 1)?;
                }
                0x0e => {
                    body.gravity = scalar(r, 1)?;
                    body.physics = true;
                }
                0x16 => body.contacts = true,
                0x19 => {
                    body.bounce = scalar(r, 1)?;
                    body.contacts = true;
                }
                0x5a => {
                    let (label, at) = name(r, 1)?;
                    body.impact_child = Some((label, word(r, at)?));
                }
                0x17 => {
                    // Source positional height is separate from the class's
                    // physical ground offset. Surface alignment may replace Z.
                    body.motion.position[2] =
                        (f64::from(body.motion.position[2]) + f64::from(scalar(r, 1)?)) as f32;
                    body.matrix.values[11] = body.motion.position[2];
                }
                0x79 => {
                    let offset = sample(r, 1, rng)?;
                    let q = crate::terrain_queries::StaticTerrainQuery::new(
                        &world.terrain,
                        1. / world.terrain.spacing,
                        crate::arithmetic::Precision::Double53,
                    )?;
                    let ground = q.sample([position[0], position[1]])?;
                    let placed = crate::effect_surface::placement(
                        position,
                        ground.plane,
                        ground.height,
                        offset,
                    )?;
                    body.matrix = crate::effect_surface::alignment_matrix(
                        crate::effect_orientation::matrix(position, body.angles, sizes[0])?,
                        ground.plane[..3].try_into().unwrap(),
                        placed,
                    )?;
                    body.motion.position = placed.position;
                }
                0x27 | 0x5c => {
                    let (label, at) = name(r, 1)?;
                    let interval = if r.opcode == 0x27 {
                        scalar(r, at)?
                    } else {
                        sample(r, at, rng)?
                    };
                    if interval <= 0. {
                        return Err("death emitter interval is nonpositive".into());
                    }
                    let reference = word(r, at + if r.opcode == 0x27 { 4 } else { 8 })?;
                    if r.opcode == 0x5c && (0..4).any(|i| word(r, at + 8 + i * 4) != Ok(0)) {
                        return Err(
                            "death paired emitter outside acquired zero-selector profile".into(),
                        );
                    }
                    body.channels.push(Channel {
                        name: label,
                        timer: crate::effect_emission::Periodic {
                            interval,
                            remaining: 0.,
                            phase: 0.,
                            previous_position: position,
                        },
                        reference,
                        paired: r.opcode == 0x5c,
                        deadline: None,
                    });
                }
                0x6f => {
                    body.channels
                        .last_mut()
                        .ok_or("death emitter duration without channel")?
                        .deadline = Some(sample(r, 1, rng)?);
                }
                0x70 | 0x10 => {
                    let (label, at) = name(r, 5)?;
                    let reference = word(r, 1)?;
                    let own = word(r, at)?;
                    self.actors.insert(key, body);
                    self.named(
                        source,
                        &label,
                        position,
                        Some((key, reference, own, context.inherited_matrix())),
                        rng,
                        world,
                        depth + 1,
                    )?;
                    body = self
                        .actors
                        .remove(&key)
                        .ok_or("death root lost during nested creation")?;
                }
                0x76 | 0x78 => {
                    let (label, at) = name(r, 1)?;
                    let size = sample(r, at, rng)?;
                    self.splat(&label, position, size, life[0], rng, world)?;
                }
                7 | 2 => {}
                _ => return Err(format!("unconsumed death FX opcode {:x}", r.opcode)),
            }
        }
        body.rescale = sizes[0] != sizes[1];
        body.visual = Visual::with_size_curve(
            Configuration {
                lifetime: life[0],
                fade_in: life[1],
                fade_out_start: life[2],
                start_size: sizes[0],
                end_size: sizes[1],
            },
            curve,
        )?;
        if let Some(binding) = binding {
            let parent = self
                .actors
                .get(&Key(binding.parent))
                .ok_or("death FX immediate parent absent")?;
            let parent_ref = parent
                .playback
                .as_ref()
                .ok_or("death FX parent ANM absent")?
                .reference(binding.reference as usize)?;
            let own_ref = body
                .playback
                .as_ref()
                .ok_or("death FX child ANM absent")?
                .reference(binding.own_reference as usize)?;
            body.matrix = crate::effect_attachment::initial_child_matrix(
                parent.matrix,
                parent_ref,
                own_ref,
                sizes[0],
                parent.visual.size,
            )?;
            body.motion.position = [
                body.matrix.values[3],
                body.matrix.values[7],
                body.matrix.values[11],
            ];
        } else if !p.records.iter().any(|r| r.opcode == 0x79) {
            body.matrix = crate::effect_orientation::matrix(position, body.angles, sizes[0])?;
        }
        self.actors.insert(key, body);
        self.contexts.release_prepared(origin)?;
        Ok(key)
    }
    fn splat(
        &mut self,
        label: &str,
        position: [f32; 3],
        size: f32,
        lifetime: f32,
        rng: &mut Random,
        world: &crate::WorldAsset,
    ) -> Result<(), String> {
        // Preview retention: the real source texture/quad remains for its
        // requesting source object's lifetime; native decal-driver parity open.
        let filename = if label.to_ascii_lowercase().ends_with(".tga") {
            label.to_owned()
        } else {
            format!("{label}.tga")
        };
        let raw = self
            .textures
            .read_bin(&filename, "English")
            .map_err(|e| e.to_string())?;
        let giants_formats::tga::Image::TrueColor(t) =
            giants_formats::tga::decode_typed(&raw).map_err(|e| e.to_string())?
        else {
            return Err("death splat texture format unsupported".into());
        };
        let alpha = t
            .alpha
            .clone()
            .unwrap_or_else(|| vec![255; t.pixels.len() / 3]);
        let words =
            giants_formats::pixels::pack_argb4444(&t.pixels, &alpha).map_err(|e| e.to_string())?;
        let rgba = giants_formats::pixels::rgba8_argb4444(&words);
        let width = (f64::from(rng.next_15()) * (1.5 - f64::from(0.8f32)) / 32768.
            + f64::from(0.8f32)) as f32
            * size;
        let height = (f64::from(rng.next_15()) * (1.5 - f64::from(0.8f32)) / 32768.
            + f64::from(0.8f32)) as f32
            * size;
        let q = crate::terrain_queries::StaticTerrainQuery::new(
            &world.terrain,
            1. / world.terrain.spacing,
            crate::arithmetic::Precision::Double53,
        )?;
        let floor = q.sample([position[0], position[1]])?;
        let geometry = Arc::new(ModelAsset {
            name: format!("owned_splat_{label}"),
            mesh: giants_formats::gbs::Mesh {
                positions: vec![
                    [-0.5, -0.21875, 0.],
                    [0.5, -0.21875, 0.],
                    [0.5, 0.78125, 0.],
                    [-0.5, 0.78125, 0.],
                ],
                indices: vec![0, 1, 2, 3],
                uv: vec![[0., 1.], [1., 1.], [1., 0.], [0., 0.]],
                rgb: vec![[255; 3]; 4],
                groups: vec![],
                subobjects: vec![giants_formats::gbs::Subobject {
                    name: "source splat bounds".into(),
                    group: 0,
                    triangles: vec![[0, 1, 2], [0, 2, 3]],
                    vertex_start: 0,
                    vertex_count: 4,
                    material: giants_formats::gbs::Material {
                        texture: filename.to_ascii_lowercase(),
                        bump_texture: String::new(),
                        falloff: 0.,
                        blend: 0.,
                        flags: 0x40000010,
                        colors: [0; 4],
                        power: 0.,
                    },
                }],
                normal_count: 0,
                normal_words: vec![],
                normal_indices: vec![],
            },
            textures: vec![crate::LogoTexture {
                name: filename.to_ascii_lowercase(),
                width: t.width,
                height: t.height,
                words,
                rgba,
            }],
            alpha_textures: std::collections::BTreeSet::from([filename.to_ascii_lowercase()]),
        });
        let p = [position[0], position[1], floor.height + 0.02];
        let mut matrix = Matrix::identity();
        matrix.values[0] = width;
        matrix.values[5] = height;
        matrix.values[3] = p[0];
        matrix.values[7] = p[1];
        matrix.values[11] = p[2];
        let key = Key(self.next);
        self.next += 1;
        self.births += 1;
        self.actors.insert(
            key,
            Body {
                class: -1,
                matrix,
                inherited: None,
                visual: Visual::new(Configuration {
                    lifetime,
                    fade_in: 0.,
                    fade_out_start: 0.,
                    start_size: 1.,
                    end_size: 1.,
                })?,
                playback: None,
                geometry,
                personality: 0,
                motion: crate::mecc_motion::Motion {
                    position: p,
                    velocity: [0.; 3],
                    previous_velocity: [0.; 3],
                    force: [0.; 3],
                    mass: 1.,
                    speed_limit: 0.,
                    force_time_limit: 0.,
                    stored_dt: 0.,
                },
                spin: [0.; 3],
                angles: [0.; 3],
                gravity: 0.,
                damping: [0.; 2],
                friction: 0.,
                bounce: 0.,
                retention: 1.,
                ground_offset: 0.,
                contact_flags: 0,
                floor: floor.height,
                plane: floor.plane,
                physics: false,
                contacts: false,
                initialized_velocity: true,
                speed: 0.,
                hidden: false,
                rescale: false,
                impact_child: None,
                channels: vec![],
                frame: 0,
            },
        );
        Ok(())
    }
    pub fn advance(
        &mut self,
        source: &SceneResources,
        dt: f32,
        areas: &[crate::object_time::SlowArea],
        world: &crate::WorldAsset,
        rng: &mut Random,
    ) -> Result<(), String> {
        let keys = self.actors.keys().copied().collect::<Vec<_>>();
        let q = crate::terrain_queries::StaticTerrainQuery::new(
            &world.terrain,
            1. / world.terrain.spacing,
            crate::arithmetic::Precision::Double53,
        )?;
        for key in keys {
            let Some(mut b) = self.actors.remove(&key) else {
                continue;
            };
            let seconds =
                crate::object_time::seconds(dt, None, Some(b.motion.position), areas) as f32;
            let step = b.visual.advance(seconds)?;
            b.frame += 1;
            if step == crate::effect_visual::Step::Expired {
                self.retired += 1;
                continue;
            }
            if step != crate::effect_visual::Step::FirstFrame {
                if b.physics {
                    if !b.initialized_velocity {
                        let matrix = if let Some(inherited) = b.inherited {
                            inherited.mul(b.matrix)?
                        } else {
                            b.matrix
                        };
                        b.motion.velocity =
                            crate::effect_emitter::initial_velocity(matrix, b.speed, None)?;
                        b.angles = crate::effect_orientation::from_matrix(b.matrix)?;
                        b.initialized_velocity = true;
                    }
                    let previous = b.motion.position;
                    b.motion = crate::mecc_motion::integrate(
                        b.motion,
                        seconds,
                        false,
                        false,
                        crate::arithmetic::Precision::Double53,
                    )?;
                    if b.contacts {
                        let out = crate::physical_ground::resolve(
                            crate::physical_ground::Input {
                                position: b.motion.position,
                                previous,
                                velocity: b.motion.velocity,
                                scale: b.visual.size,
                                ground_offset: b.ground_offset,
                                center_offset: 50.,
                                gravity_mass: b.gravity,
                                restitution: b.bounce,
                                retention: b.retention,
                                force_z: b.motion.force[2],
                                flags: b.contact_flags,
                                floor: b.floor,
                                plane: b.plane,
                            },
                            crate::arithmetic::Precision::Double53,
                            |p| {
                                crate::physical_ground::Ground::from_sample(
                                    q.sample(p)?,
                                    world.terrain.width,
                                )
                            },
                        )?;
                        b.motion.position = out.position;
                        b.motion.velocity = out.velocity;
                        b.contact_flags = out.flags;
                        b.floor = out.floor;
                        b.plane = out.plane;
                        if out.terrain_responses > 0 {
                            self.impacts += 1;
                            if let Some((label, count)) = &mut b.impact_child
                                && *count != 0
                            {
                                self.named(source, label, b.motion.position, None, rng, world, 0)?;
                                *count -= 1;
                            }
                            if b.bounce == 0. {
                                self.retired += 1;
                                continue;
                            }
                        }
                    }
                    let gravity = crate::mecc_gravity::apply(
                        crate::mecc_gravity::Input {
                            force: b.motion.force,
                            velocity: b.motion.velocity,
                            normal: b.plane[..3].try_into().unwrap(),
                            gravity_mass: b.gravity,
                            mass: 1.,
                            friction_limit: b.friction,
                            damping_xy: b.damping[0],
                            damping_z: b.damping[1],
                            force_time_limit: 0.,
                            object_flags: 0,
                            physics_flags: b.contact_flags,
                            project_contact: b.contacts,
                        },
                        crate::arithmetic::Precision::Double53,
                    )?;
                    b.motion.force = gravity.force;
                }
                if b.class != -1 {
                    b.angles = crate::effect_orientation::advance(b.angles, b.spin, seconds)?;
                    if b.physics || b.spin != [0.; 3] || b.rescale {
                        b.matrix = crate::effect_orientation::matrix(
                            b.motion.position,
                            b.angles,
                            b.visual.size,
                        )?;
                    }
                }
                let mut births = vec![];
                for c in &mut b.channels {
                    if let Some(left) = &mut c.deadline {
                        *left -= seconds;
                        if *left < 0. {
                            continue;
                        }
                    }
                    if seconds > 0. {
                        let next = c.timer.advance(seconds, b.motion.position)?;
                        c.timer = next.state;
                        for mut position in next.positions {
                            if c.reference != u32::MAX {
                                let reference = b
                                    .playback
                                    .as_ref()
                                    .ok_or("emitter current ANM absent")?
                                    .reference(c.reference as usize)?;
                                let m = b.matrix.mul(reference)?;
                                position = [m.values[3], m.values[7], m.values[11]];
                            }
                            births.push((c.name.clone(), position, c.paired, c.reference));
                        }
                    }
                }
                self.actors.insert(key, b);
                for (label, p, paired, reference) in births {
                    // Inclusive fixed reference ranges still consume their
                    // native integer selections before child preparation.
                    if reference != u32::MAX {
                        rng.next_15();
                    }
                    if paired {
                        rng.next_15();
                    }
                    let basis = self.actors[&key].inherited;
                    self.named(
                        source,
                        &label,
                        p,
                        paired.then_some((key, reference, 0, basis)),
                        rng,
                        world,
                        0,
                    )?;
                }
                b = self
                    .actors
                    .remove(&key)
                    .ok_or("death FX owner lost during emission")?;
            }
            if let Some(playback) = &mut b.playback {
                playback.advance(seconds)?;
            }
            self.actors.insert(key, b);
        }
        Ok(())
    }
    pub fn draws(&self) -> Result<Vec<Draw>, String> {
        self.actors
            .iter()
            .filter(|(_, b)| !b.hidden && b.frame > 0)
            .map(|(&key, b)| {
                let local = if let Some(p) = &b.playback {
                    match p.positions()? {
                        crate::actor_context::PositionUpdate::Replace(v) => v,
                        crate::actor_context::PositionUpdate::Keep => {
                            b.geometry.mesh.positions.clone()
                        }
                    }
                } else {
                    b.geometry.mesh.positions.clone()
                };
                Ok(Draw {
                    key,
                    class: b.class,
                    frame: b.frame,
                    geometry: b.geometry.clone(),
                    local,
                    matrix: b.matrix,
                    personality: b.personality,
                    size: b.visual.size,
                    transparency: b.visual.transparency,
                })
            })
            .collect()
    }
}
