//! Bounded source-selected bursts. Named preparation consumes random operands
//! before activation. Audio, contact response and texture binding services remain
//! explicit needs; this is not a complete effect VM or native LOD policy.
use giants_formats::{effect_image::Image, effect_program::Record};
pub mod execution;

#[derive(Clone, Debug)]
enum Op {
    Lifetime([f32; 3]),
    RandomLifetime([[f32; 2]; 3]),
    Size(f32),
    Sizes([f32; 2]),
    RandomSize([f32; 2]),
    SizeRange([[f32; 2]; 2]),
    Spin([[f32; 2]; 3]),
    FixedSpin([f32; 3]),
    Speed([f32; 2]),
    RandomGravity([f32; 2]),
    ChildService(ChildServiceNeed),
    Gravity(f32),
    Damping([f32; 2]),
    Bounce(f32),
    Attribute(u32),
    PayloadB0(u32),
    Height(f32),
    Hidden,
    Sound(u32),
    RandomSound([u32; 2]),
    TextureAnimation(crate::effect_texture_animation::Configuration),
    Child {
        source_offset: u32,
        reference: u32,
        own_reference: u32,
        body: Body,
    },
}
#[derive(Clone, Debug)]
struct Body {
    class: i32,
    ops: Vec<Op>,
}
#[derive(Clone, Debug)]
pub struct Plan {
    sound: Option<[u32; 2]>,
    bodies: Vec<Body>,
}
#[derive(Clone, Debug)]
pub struct SoundNeed {
    pub id: u32,
    pub attached: bool,
}
/// Source command5b is retained and independently compared at native dispatch.
/// Its active child service is not implemented by the contact-free preview.
#[derive(Clone, Debug)]
pub struct ChildServiceNeed {
    pub name: String,
    pub words: [u32; 3],
}
impl ChildServiceNeed {
    /// Checked command5b profile: fixed periodic interval, zero remaining
    /// service operands. Construction does not consume an interval RNG sample.
    pub fn emission(
        &self,
        class: i32,
        image: &Image,
    ) -> Result<crate::effect_emitter::Recipe, &'static str> {
        let interval = f32::from_bits(self.words[0]);
        if !interval.is_finite() || interval <= 0. || self.words[1..] != [0, 0] {
            return Err("child service outside fixed periodic profile");
        }
        let particle = crate::effect_emitter::Particle::project(image, &self.name)?;
        if particle.retirement.is_some() {
            return Err("child service retirement chain not bound");
        }
        Ok(crate::effect_emitter::Recipe {
            class,
            child: self.name.clone(),
            interval: [interval; 2],
            particles: vec![particle],
        })
    }
}
#[derive(Clone, Debug)]
pub struct Spawn {
    pub class: i32,
    pub parent: Option<usize>,
    pub reference: u32,
    pub own_reference: u32,
    pub visual: crate::effect_visual::Configuration,
    pub height: f32,
    pub hidden: bool,
    pub spin: [f32; 3],
    /// Immediate parent requests first direction/angle initialization.
    pub direction_requested: bool,
    pub speed: f32,
    pub gravity_mass: f32,
    pub physics_enabled: bool,
    pub damping: [f32; 2],
    pub bounce: f32,
    /// Source command19 was present, even with a zero coefficient. This is
    /// configuration only; it does not acknowledge a contact service.
    pub contact_requested: bool,
    pub attribute: Option<u32>,
    /// Exact-D command67 writes this raw state78 word. Contact behavior is
    /// outside the free-flight consumer; do not infer a physical name for it.
    pub parameter_b0: u32,
    pub sounds: Vec<SoundNeed>,
    /// Retained state78 cue from command83; a request is not sound playback.
    pub retained_sound: Option<u32>,
    pub texture_animation: Option<crate::effect_texture_animation::Configuration>,
    pub size_configured: bool,
    pub child_services: Vec<ChildServiceNeed>,
}
/// Source-configured birth projection, before any actor scheduler or draw.
/// Hidden supports retain their places in the ordered reference hierarchy.
pub struct Birth {
    pub spawns: Vec<Spawn>,
    pub matrices: Vec<crate::procedural_aim::Matrix>,
}
fn word(r: &Record, at: usize) -> Result<u32, &'static str> {
    Ok(u32::from_le_bytes(
        r.bytes
            .get(at..at + 4)
            .ok_or("truncated burst operand")?
            .try_into()
            .map_err(|_| "burst word invalid")?,
    ))
}
fn floats<const N: usize>(r: &Record) -> Result<[f32; N], &'static str> {
    let mut values = [0.; N];
    for (i, v) in values.iter_mut().enumerate() {
        *v = f32::from_bits(word(r, 1 + i * 4)?);
        if !v.is_finite() {
            return Err("nonfinite burst operand");
        }
    }
    Ok(values)
}
fn pairs<const N: usize>(r: &Record) -> Result<[[f32; 2]; N], &'static str> {
    let mut ranges = [[0.; 2]; N];
    for (i, pair) in ranges.iter_mut().enumerate() {
        *pair = [
            f32::from_bits(word(r, 1 + i * 8)?),
            f32::from_bits(word(r, 5 + i * 8)?),
        ];
        if !pair.iter().all(|v| v.is_finite()) || pair[0] > pair[1] {
            return Err("invalid burst range");
        }
    }
    Ok(ranges)
}
fn body(records: &[Record], image: &Image, child: bool) -> Result<Body, &'static str> {
    if records.len() < 4
        || records[0].opcode != 3
        || records[1].opcode != 6
        || records.last().map(|r| r.opcode) != Some(7)
    {
        return Err("burst body outside detached object profile");
    }
    let class = word(&records[0], 1)? as i32;
    if class < 0 {
        return Err("invalid burst class");
    }
    let mut ops = Vec::new();
    let positional_impact = !child
        && records
            .iter()
            .map(|r| r.opcode)
            .eq([3, 6, 0x83, 9, 4, 0x32, 7]);
    for record in &records[2..records.len() - 1] {
        ops.push(match record.opcode {
            4 => Op::Lifetime(floats(record)?),
            5 if child => Op::RandomLifetime(pairs(record)?),
            0x1b => Op::Size(floats::<1>(record)?[0]),
            8 => Op::Sizes(floats(record)?),
            0x1c if child => Op::RandomSize(pairs::<1>(record)?[0]),
            9 if child || positional_impact => Op::SizeRange(pairs(record)?),
            0x33 if child => Op::Spin(pairs(record)?),
            0x32 if positional_impact => Op::FixedSpin(floats(record)?),
            0x83 if positional_impact => {
                let range = [word(record, 1)?, word(record, 5)?];
                if range[0] > range[1] || range[1] > i32::MAX as u32 {
                    return Err("invalid impact sound range");
                }
                Op::RandomSound(range)
            }
            0x12 if child => Op::Speed(pairs::<1>(record)?[0]),
            0x0f if child => Op::RandomGravity(floats(record)?),
            0x5b if child => {
                let end = record.bytes[1..]
                    .iter()
                    .position(|b| *b == 0)
                    .ok_or("child service name unterminated")?
                    + 1;
                if end == 1 || end + 13 != record.bytes.len() {
                    return Err("child service outside checked framing");
                }
                Op::ChildService(ChildServiceNeed {
                    name: std::str::from_utf8(&record.bytes[1..end])
                        .map_err(|_| "child service name invalid")?
                        .to_owned(),
                    words: [
                        word(record, end + 1)?,
                        word(record, end + 5)?,
                        word(record, end + 9)?,
                    ],
                })
            }
            0x0e => Op::Gravity(floats::<1>(record)?[0]),
            0x13 if child => Op::Damping(floats(record)?),
            0x19 if child => Op::Bounce(floats::<1>(record)?[0]),
            0x47 if child => Op::Attribute(word(record, 1)?),
            0x67 if child => Op::PayloadB0(word(record, 1)?),
            0x17 if !child => Op::Height(floats::<1>(record)?[0]),
            0x21 if !child => Op::Hidden,
            0x2c if !child => Op::Sound(word(record, 1)?),
            0x29 if !child => Op::TextureAnimation(
                crate::effect_texture_animation::Configuration {
                    frames: word(record, 1)?,
                    rate: f32::from_bits(word(record, 5)?),
                }
                .validate()?,
            ),
            0x10 if !child => {
                let end = record
                    .bytes
                    .get(5..)
                    .ok_or("burst child absent")?
                    .iter()
                    .position(|b| *b == 0)
                    .ok_or("burst child unterminated")?
                    + 5;
                if end == 5 || end + 5 != record.bytes.len() {
                    return Err("burst child framing invalid");
                }
                let name = std::str::from_utf8(&record.bytes[5..end])
                    .map_err(|_| "burst child invalid")?;
                // Full-detail source choice is supplied by this inspection plan.
                // Native distance/pressure selection remains a separate policy.
                let (_, entry) = image.find(name).ok_or("burst child program absent")?;
                let program = image
                    .program(entry.offset)
                    .map_err(|_| "burst child program invalid")?;
                if program.records.last().map(|r| r.opcode) != Some(2) {
                    return Err("burst child end absent");
                }
                Op::Child {
                    source_offset: entry.offset,
                    reference: word(record, 1)?,
                    own_reference: word(record, end + 1)?,
                    body: body(&program.records[..program.records.len() - 1], image, true)?,
                }
            }
            _ => return Err("burst command outside checked profile"),
        });
    }
    if !ops
        .iter()
        .any(|o| matches!(o, Op::Lifetime(_) | Op::RandomLifetime(_)))
    {
        return Err("burst lifetime absent");
    }
    if !child
        && ops.iter().any(|o| matches!(o, Op::Child { .. }))
        && !ops.iter().any(|o| matches!(o, Op::Hidden))
    {
        return Err("visible burst emitter outside checked profile");
    }
    Ok(Body { class, ops })
}
impl Plan {
    /// Acquired complete single-body positional-impact frame. This projects
    /// configuration and pending cues; it creates no actor or audio provider.
    pub fn project_positional_impact(image: &Image, name: &str) -> Result<Self, &'static str> {
        let (_, entry) = image.find(name).ok_or("impact effect symbol absent")?;
        let program = image
            .program(entry.offset)
            .map_err(|_| "impact effect program invalid")?;
        Self::positional_impact_frame(image, &program.records)
    }
    pub(crate) fn positional_impact_frame(
        image: &Image,
        records: &[Record],
    ) -> Result<Self, &'static str> {
        if !records
            .iter()
            .map(|r| r.opcode)
            .eq([3, 6, 0x83, 9, 4, 0x32, 7, 2])
        {
            return Err("positional impact outside acquired complete frame");
        }
        Self::from_records_profile(image, records, 1, true)
    }
    pub fn project(image: &Image, name: &str) -> Result<Self, &'static str> {
        let (_, entry) = image.find(name).ok_or("burst symbol absent")?;
        let program = image
            .program(entry.offset)
            .map_err(|_| "burst program invalid")?;
        let plan = Self::from_records(image, &program.records, 2)?;
        let mut seed = 1;
        if plan
            .create(&mut seed)?
            .iter()
            .any(|s| !s.child_services.is_empty())
        {
            return Err("burst motion/child service backend pending");
        }
        Ok(plan)
    }
    pub(crate) fn from_records(
        image: &Image,
        records: &[Record],
        minimum_bodies: usize,
    ) -> Result<Self, &'static str> {
        Self::from_records_profile(image, records, minimum_bodies, false)
    }
    fn from_records_profile(
        image: &Image,
        records: &[Record],
        minimum_bodies: usize,
        positional_impact: bool,
    ) -> Result<Self, &'static str> {
        let mut records = records;
        let sound = if records.first().map(|r| r.opcode) == Some(0x2b) {
            let range = [word(&records[0], 1)?, word(&records[0], 5)?];
            if range[0] > range[1] || range[1] > i32::MAX as u32 {
                return Err("invalid burst sound range");
            }
            records = &records[1..];
            Some(range)
        } else {
            None
        };
        let mut bodies = Vec::new();
        while records.len() > 1 {
            if bodies.len() >= 64 {
                return Err("burst body budget");
            }
            let end = records
                .iter()
                .position(|r| r.opcode == 7)
                .ok_or("burst body end absent")?;
            if !positional_impact
                && records[..=end]
                    .iter()
                    .map(|r| r.opcode)
                    .eq([3, 6, 0x83, 9, 4, 0x32, 7])
            {
                return Err("positional impact requires its explicit activation profile");
            }
            bodies.push(body(&records[..=end], image, false)?);
            records = &records[end + 1..];
        }
        if records.len() != 1 || records[0].opcode != 2 || bodies.len() < minimum_bodies {
            return Err("burst requires multiple terminated bodies");
        }
        let result = Self { sound, bodies };
        let mut seed = 1;
        result.create(&mut seed)?;
        Ok(result)
    }
    pub fn create(&self, seed: &mut u32) -> Result<Vec<Spawn>, &'static str> {
        self.create_phase(seed, true)
    }
    /// Activation after the actual context's null-object preparation. Child
    /// creations still execute their own preparation in construction order.
    /// Cue transport is unbound here. A real provider must interleave its own
    /// shared RNG effects before subsequent command draws, not replay this result.
    pub fn create_active(&self, seed: &mut u32) -> Result<Vec<Spawn>, &'static str> {
        self.create_phase(seed, false)
    }
    fn create_phase(&self, seed: &mut u32, prepare: bool) -> Result<Vec<Spawn>, &'static str> {
        let mut staged = *seed;
        let result = self.create_inner(&mut staged, prepare)?;
        *seed = staged;
        Ok(result)
    }
    fn create_inner(&self, seed: &mut u32, prepare: bool) -> Result<Vec<Spawn>, &'static str> {
        let mut result = Vec::new();
        // Native named preparation traverses the parent's random sound selector
        // once before activation, including ranges with identical endpoints.
        let sound = self.sound.map(|range| {
            if prepare {
                random_sound(seed, range);
            }
            random_sound(seed, range)
        });
        for b in &self.bodies {
            if prepare {
                for op in &b.ops {
                    op.sample(seed);
                }
            }
            b.create(seed, None, 0, 0, &mut result)?;
        }
        // The selected sound is a pending service, not an audio success.
        if let (Some(first), Some(id)) = (result.first_mut(), sound) {
            first.sounds.insert(
                0,
                SoundNeed {
                    id,
                    attached: false,
                },
            );
        }
        Ok(result)
    }
}
fn random_sound(seed: &mut u32, range: [u32; 2]) -> u32 {
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    let n = (*seed >> 16) & 32767;
    range[0] + ((u64::from(n) * u64::from(range[1] - range[0] + 1)) >> 15) as u32
}
impl Body {
    fn create(
        &self,
        seed: &mut u32,
        parent: Option<usize>,
        reference: u32,
        own_reference: u32,
        result: &mut Vec<Spawn>,
    ) -> Result<(), &'static str> {
        if result.len() >= 512 {
            return Err("burst spawn budget");
        }
        let mut spawn = Spawn {
            class: self.class,
            parent,
            reference,
            own_reference,
            visual: crate::effect_visual::Configuration {
                lifetime: -1.,
                fade_in: 0.,
                fade_out_start: 0.,
                start_size: 1.,
                end_size: 1.,
            },
            height: 0.,
            hidden: false,
            spin: [0.; 3],
            direction_requested: parent.is_some(),
            speed: 0.,
            gravity_mass: 1.,
            physics_enabled: false,
            damping: [0.; 2],
            bounce: 0.,
            contact_requested: false,
            attribute: None,
            parameter_b0: 1f32.to_bits(),
            sounds: Vec::new(),
            retained_sound: None,
            texture_animation: None,
            size_configured: false,
            child_services: Vec::new(),
        };
        if parent.is_some() {
            for op in &self.ops {
                op.sample(seed);
            }
        }
        let index = result.len();
        result.push(spawn.clone());
        for op in &self.ops {
            let cue = if let Op::RandomSound(range) = op {
                Some(random_sound(seed, *range))
            } else {
                None
            };
            let values = if cue.is_some() {
                Vec::new()
            } else {
                op.sample(seed)
            };
            match op {
                Op::Lifetime(v) => {
                    spawn.visual.lifetime = v[0];
                    spawn.visual.fade_in = v[1];
                    spawn.visual.fade_out_start = v[2];
                }
                Op::RandomLifetime(_) => {
                    spawn.visual.lifetime = values[0];
                    spawn.visual.fade_in = values[1];
                    spawn.visual.fade_out_start = values[2];
                }
                Op::Size(v) => {
                    spawn.size_configured = true;
                    spawn.visual.start_size = *v;
                    spawn.visual.end_size = *v;
                }
                Op::Sizes(v) => {
                    spawn.size_configured = true;
                    spawn.visual.start_size = v[0];
                    spawn.visual.end_size = v[1];
                }
                Op::RandomSize(_) => {
                    spawn.size_configured = true;
                    spawn.visual.start_size = values[0];
                    spawn.visual.end_size = values[0];
                }
                Op::SizeRange(_) => {
                    spawn.size_configured = true;
                    spawn.visual.start_size = values[0];
                    spawn.visual.end_size = values[1];
                }
                // Source pair order differs from the state78 Euler-rate fields.
                Op::Spin(_) => spawn.spin = [values[1], values[0], values[2]],
                Op::FixedSpin(v) => spawn.spin = [v[1], v[0], v[2]],
                Op::RandomSound(_) => {
                    let id = cue.unwrap();
                    spawn.retained_sound = Some(id);
                    spawn.sounds.push(SoundNeed { id, attached: true });
                }
                Op::Speed(_) => {
                    spawn.speed = values[0];
                    spawn.physics_enabled = true;
                }
                Op::RandomGravity(_) => {
                    spawn.gravity_mass = values[0];
                    spawn.physics_enabled = true;
                }
                Op::ChildService(need) => spawn.child_services.push(need.clone()),
                Op::Gravity(v) => {
                    spawn.gravity_mass = *v;
                    spawn.physics_enabled = true;
                }
                Op::Damping(v) => spawn.damping = *v,
                Op::Bounce(v) => {
                    spawn.bounce = *v;
                    spawn.contact_requested = true;
                }
                Op::Attribute(v) => spawn.attribute = Some(*v),
                Op::PayloadB0(v) => spawn.parameter_b0 = *v,
                Op::Height(v) => spawn.height = *v,
                Op::Hidden => spawn.hidden = true,
                Op::Sound(v) => spawn.sounds.push(SoundNeed {
                    id: *v,
                    attached: true,
                }),
                Op::TextureAnimation(v) => spawn.texture_animation = Some(*v),
                Op::Child {
                    reference,
                    own_reference,
                    body,
                    ..
                } => body.create(seed, Some(index), *reference, *own_reference, result)?,
            }
        }
        if spawn.visual.start_size <= 0.
            || spawn.visual.end_size <= 0.
            || spawn.damping.iter().any(|v| *v < 0.)
            || spawn.bounce < 0.
        {
            return Err("invalid burst visual/physics profile");
        }
        crate::effect_visual::Visual::new(spawn.visual)?;
        result[index] = spawn;
        Ok(())
    }
}
impl Op {
    fn sample(&self, seed: &mut u32) -> Vec<f32> {
        let sample = |r| crate::timed_effect::sample(seed, r);
        match self {
            Self::RandomSound(range) => {
                random_sound(seed, *range);
                Vec::new()
            }
            Self::RandomLifetime(r) | Self::Spin(r) => r.iter().copied().map(sample).collect(),
            Self::RandomSize(r) | Self::Speed(r) | Self::RandomGravity(r) => {
                vec![crate::timed_effect::sample(seed, *r)]
            }
            Self::SizeRange(r) => r.iter().copied().map(sample).collect(),
            _ => Vec::new(),
        }
    }
}

/// Immutable initial class references for a detached burst. Hidden support
/// objects participate in placement without becoming drawable scenery.
pub struct References {
    classes: std::collections::BTreeMap<i32, crate::effect_animation::Playback>,
}
impl References {
    pub fn load(
        resources: &crate::scene_resources::SceneResources,
        image: &Image,
        spawns: &[Spawn],
    ) -> Result<Self, String> {
        let mut needed = std::collections::BTreeSet::new();
        for spawn in spawns {
            if let Some(parent) = spawn.parent {
                needed.insert(spawns.get(parent).ok_or("burst parent absent")?.class);
                needed.insert(spawn.class);
            }
        }
        let mut classes = std::collections::BTreeMap::new();
        for class in needed {
            let actor = crate::actor_context::Prepared::class_initial(std::sync::Arc::new(
                crate::actor_context::Class::load(resources, class, image)?,
            ));
            classes.insert(
                class,
                crate::effect_animation::Playback::new(actor)?
                    .ok_or("burst reference animation absent")?,
            );
        }
        Ok(Self { classes })
    }
    pub fn matrices(
        &self,
        spawns: &[Spawn],
        origin: [f32; 3],
    ) -> Result<Vec<crate::procedural_aim::Matrix>, String> {
        let mut result = Vec::new();
        for (index, spawn) in spawns.iter().enumerate() {
            let matrix = if let Some(parent) = spawn.parent {
                if parent >= index {
                    return Err("burst parent is not earlier in construction order".into());
                }
                let reference = self
                    .classes
                    .get(&spawns[parent].class)
                    .ok_or("burst parent class absent")?
                    .reference(spawn.reference as usize)?;
                let own = self
                    .classes
                    .get(&spawn.class)
                    .ok_or("burst child class absent")?
                    .reference(spawn.own_reference as usize)?;
                crate::effect_attachment::initial_child_matrix(
                    result[parent],
                    reference,
                    own,
                    spawn.visual.start_size,
                    spawns[parent].visual.start_size,
                )
                .map_err(str::to_owned)?
            } else if !spawn.size_configured {
                if spawn.height != 0. {
                    return Err("unconfigured burst birth transform with height pending".into());
                }
                // Fresh native storage has the identity flag and zero matrix
                // payload, rather than a reconstructed zero-angle rotation.
                let mut values = [0.; 16];
                values[15] = 1.;
                if origin != [0.; 3] {
                    for i in [0, 5, 10] {
                        values[i] = 1.;
                    }
                    for (i, value) in [3, 7, 11].into_iter().zip(origin) {
                        values[i] = value;
                    }
                }
                crate::procedural_aim::Matrix { flags: 16, values }
            } else {
                let position = [
                    origin[0],
                    origin[1],
                    (f64::from(origin[2]) + f64::from(spawn.height)) as f32,
                ];
                crate::effect_orientation::matrix(position, [0.; 3], spawn.visual.start_size)
                    .map_err(str::to_owned)?
            };
            result.push(matrix);
        }
        Ok(result)
    }
}

/// Source lifetime/motion with optionally bound static terrain contact.
/// World services, sound and full native contact fidelity remain separate.
pub struct Playback {
    pub spawn: Spawn,
    pub visual: crate::effect_visual::Visual,
    pub motion: crate::mecc_motion::Motion,
    pub birth: crate::procedural_aim::Matrix,
    pub angles: Option<[f32; 3]>,
    pub expired: bool,
    pub contact: Option<crate::effect_contact::State>,
    pub texture_animation: Option<crate::effect_texture_animation::State>,
    inherited_direction: Option<crate::procedural_aim::Matrix>,
}
impl Playback {
    pub fn new(spawn: Spawn, birth: crate::procedural_aim::Matrix) -> Result<Self, &'static str> {
        Self::with_size_curve(spawn, birth, crate::effect_visual::SizeCurve::Linear)
    }
    pub fn with_size_curve(
        spawn: Spawn,
        birth: crate::procedural_aim::Matrix,
        curve: crate::effect_visual::SizeCurve,
    ) -> Result<Self, &'static str> {
        if !spawn.child_services.is_empty() {
            return Err("burst motion/child service backend pending");
        }
        Self::body(spawn, birth, curve)
    }
    pub fn with_bound_emitters(
        spawn: Spawn,
        birth: crate::procedural_aim::Matrix,
        bindings: &[std::sync::Arc<crate::effect_emitter::Recipe>],
    ) -> Result<Self, &'static str> {
        if bindings.len() != spawn.child_services.len()
            || spawn
                .child_services
                .iter()
                .zip(bindings)
                .any(|(need, recipe)| {
                    recipe.class != spawn.class
                        || recipe.particles.len() != 1
                        || !need.name.eq_ignore_ascii_case(&recipe.child)
                        || recipe.interval.map(f32::to_bits) != [need.words[0]; 2]
                        || need.words[1..] != [0, 0]
                })
        {
            return Err("burst child service binding mismatch");
        }
        Self::body(spawn, birth, crate::effect_visual::SizeCurve::Linear)
    }
    fn body(
        spawn: Spawn,
        birth: crate::procedural_aim::Matrix,
        curve: crate::effect_visual::SizeCurve,
    ) -> Result<Self, &'static str> {
        crate::effect_projectile::initial_velocity(birth, spawn.speed)?;
        let visual = crate::effect_visual::Visual::with_size_curve(spawn.visual, curve)?;
        let texture_animation = spawn
            .texture_animation
            .map(crate::effect_texture_animation::State::new)
            .transpose()?;
        let motion = crate::mecc_motion::Motion {
            position: [birth.values[3], birth.values[7], birth.values[11]],
            velocity: [0.; 3],
            previous_velocity: [0.; 3],
            force: [0.; 3],
            mass: 1.,
            speed_limit: 0.,
            force_time_limit: 0.,
            stored_dt: 0.,
        };
        Ok(Self {
            spawn,
            visual,
            motion,
            birth,
            angles: None,
            expired: false,
            contact: None,
            texture_animation,
            inherited_direction: None,
        })
    }
    /// Source context inheritance affects initial velocity, not reference
    /// placement or the Euler extraction from the object's own birth matrix.
    /// A zero-flag, zero-filled frame is retained literally, not normalized.
    /// Prototype PS2 `fx_logic` is an unknown-attribution navigation candidate;
    /// this PC callback contract does not import its signature or layout.
    pub fn bind_inherited_direction(
        &mut self,
        frame: crate::procedural_aim::Matrix,
    ) -> Result<(), &'static str> {
        if self.inherited_direction.is_some()
            || self.angles.is_some()
            || self.visual.first_frame_consumed()
            || frame.flags & 0x20 != 0
            || !frame.values.into_iter().all(f32::is_finite)
        {
            return Err("effect inherited direction requires a fresh finite affine frame");
        }
        self.inherited_direction = Some(frame);
        Ok(())
    }
    pub fn visible(&self) -> bool {
        !self.expired && !self.spawn.hidden
    }
    pub fn advance_free(&mut self, dt: f32) -> Result<bool, &'static str> {
        self.advance_body(dt, None)
    }
    pub fn bind_static_contact(
        &mut self,
        seed: crate::effect_contact::Seed,
    ) -> Result<(), &'static str> {
        if self.contact.is_some() || self.angles.is_some() || self.visual.age != 0. {
            return Err("effect contact must bind before motion callbacks");
        }
        self.contact = Some(seed.instantiate(&self.spawn)?);
        Ok(())
    }
    pub fn advance_static(
        &mut self,
        dt: f32,
        mut query: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
    ) -> Result<bool, &'static str> {
        if self.contact.is_none() {
            return Err("static effect contact has not been bound");
        }
        self.advance_body(dt, Some(&mut query))
    }
    fn advance_body(
        &mut self,
        dt: f32,
        mut query: Option<
            &mut dyn FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        >,
    ) -> Result<bool, &'static str> {
        if !dt.is_finite() || dt < 0. {
            return Err("invalid burst callback time");
        }
        if self.expired {
            return Ok(false);
        }
        if self.contact.is_some() && query.is_none() {
            return Err("bound static effect contact requires a ground query");
        }
        match self.visual.advance(dt)? {
            crate::effect_visual::Step::FirstFrame => {}
            crate::effect_visual::Step::Expired => self.expired = true,
            crate::effect_visual::Step::VisibleState => {
                if let Some(clock) = &mut self.texture_animation {
                    clock.advance(dt)?;
                }
                let angles = if let Some(angles) = self.angles {
                    angles
                } else if self.spawn.direction_requested {
                    let direction = self
                        .inherited_direction
                        .map_or(Ok(self.birth), |frame| frame.mul(self.birth))?;
                    self.motion.velocity =
                        crate::effect_projectile::initial_velocity(direction, self.spawn.speed)?;
                    crate::effect_orientation::from_matrix(self.birth)?
                } else {
                    [0.; 3]
                };
                if self.spawn.physics_enabled {
                    let previous = self.motion.position;
                    self.motion = crate::mecc_motion::integrate(
                        self.motion,
                        dt,
                        false,
                        false,
                        crate::arithmetic::Precision::Double53,
                    )?;
                    if let Some(contact) = &mut self.contact {
                        contact.resolve(
                            &mut self.motion,
                            previous,
                            self.visual.size,
                            &self.spawn,
                            query.as_mut().ok_or("effect ground query absent")?,
                        )?;
                    }
                    let gravity = crate::mecc_gravity::apply(
                        crate::mecc_gravity::Input {
                            force: self.motion.force,
                            velocity: self.motion.velocity,
                            normal: self
                                .contact
                                .as_ref()
                                .map_or([0.; 3], |c| [c.plane[0], c.plane[1], c.plane[2]]),
                            gravity_mass: self.spawn.gravity_mass,
                            mass: 1.,
                            friction_limit: 0.,
                            damping_xy: self.spawn.damping[0],
                            damping_z: self.spawn.damping[1],
                            force_time_limit: self.motion.force_time_limit,
                            object_flags: 0,
                            physics_flags: self.contact.as_ref().map_or(0, |c| c.flags),
                            project_contact: self.contact.is_some(),
                        },
                        crate::arithmetic::Precision::Double53,
                    )?;
                    self.motion.force = gravity.force;
                    self.motion.force_time_limit = gravity.force_time_limit;
                }
                self.angles = Some(crate::effect_orientation::advance(
                    angles,
                    self.spawn.spin,
                    dt,
                )?);
            }
        }
        Ok(true)
    }
    pub fn matrix(&self) -> Result<crate::procedural_aim::Matrix, &'static str> {
        let Some(angles) = self.angles else {
            return Ok(self.birth);
        };
        let mut matrix =
            crate::effect_orientation::matrix(self.motion.position, angles, self.visual.size)?;
        matrix.flags = self.birth.flags | 3;
        Ok(matrix)
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn inherited_frame_changes_velocity_without_rotating_birth_or_angles() {
        use crate::procedural_aim::Matrix;
        let birth = crate::effect_orientation::matrix([1., 2., 100.], [0.; 3], 1.).unwrap();
        let raw_zero = Matrix {
            flags: 0,
            values: [0.; 16],
        };
        let rotated = Matrix {
            flags: 1,
            values: [
                0., -1., 0., 0., 1., 0., 0., 0., 0., 0., 1., 0., 0., 0., 0., 0.,
            ],
        };
        let mut bodies = [None, Some(raw_zero), Some(rotated)].map(|frame| {
            let mut source = spawn();
            source.visual.lifetime = 10.;
            source.visual.fade_out_start = 0.;
            source.spin = [0.; 3];
            let mut body = super::Playback::new(source, birth).unwrap();
            if let Some(frame) = frame {
                body.bind_inherited_direction(frame).unwrap();
            }
            assert_eq!(body.birth.values, birth.values);
            body.advance_free(0.01).unwrap();
            assert_eq!(body.motion.position, [1., 2., 100.]);
            assert_eq!(body.motion.velocity, [0.; 3]);
            assert!(body.bind_inherited_direction(raw_zero).is_err());
            body.advance_free(0.01).unwrap();
            body
        });
        assert_eq!(bodies[0].motion.velocity, bodies[1].motion.velocity);
        assert_eq!(bodies[2].motion.velocity[0], -bodies[0].motion.velocity[1]);
        assert_eq!(bodies[2].motion.velocity[1], bodies[0].motion.velocity[0]);
        assert_eq!(bodies[2].motion.velocity[2], bodies[0].motion.velocity[2]);
        assert_eq!(bodies[0].angles, bodies[2].angles);
        // Raw source frame identity is retained separately from a null frame.
        assert!(bodies[0].inherited_direction.is_none());
        assert_eq!(bodies[1].inherited_direction.unwrap().values, [0.; 16]);
        for body in &mut bodies {
            assert!(body.advance_free(0.02).unwrap());
        }
    }
    fn spawn() -> super::Spawn {
        super::Spawn {
            class: 900,
            parent: None,
            reference: 0,
            own_reference: 0,
            visual: crate::effect_visual::Configuration {
                lifetime: 0.02,
                fade_in: 0.,
                fade_out_start: 0.01,
                start_size: 1.,
                end_size: 3.,
            },
            height: 0.,
            hidden: false,
            spin: [10., 20., 30.],
            direction_requested: true,
            speed: 10.,
            gravity_mass: 0.,
            physics_enabled: true,
            damping: [0.; 2],
            bounce: 0.,
            contact_requested: false,
            attribute: None,
            parameter_b0: 1f32.to_bits(),
            sounds: Vec::new(),
            retained_sound: None,
            texture_animation: None,
            size_configured: false,
            child_services: Vec::new(),
        }
    }
    #[test]
    fn bound_contacts_require_a_backend_and_skip_first_or_terminal_callbacks() {
        let program = giants_formats::class_program::parse(&[0x1e, 78, 0x36, 6, 0xff]).unwrap();
        let profile = crate::effect_contact::Seed::from_class(&program).unwrap();
        let mut source = spawn();
        assert!(profile.instantiate(&source).is_err());
        source.contact_requested = true;
        let mut playback =
            super::Playback::new(source, crate::procedural_aim::Matrix::identity()).unwrap();
        playback.bind_static_contact(profile).unwrap();
        assert!(playback.advance_free(0.01).is_err());
        assert!(
            playback
                .advance_static(0.01, |_| Err("unexpected first query"))
                .unwrap()
        );
        assert!(
            playback
                .advance_static(0.005, |_| Err("query unavailable"))
                .is_err()
        );
        let unsupported = giants_formats::class_program::parse(&[0x1e, 78, 0x36, 7, 0xff]).unwrap();
        assert!(crate::effect_contact::Seed::from_class(&unsupported).is_err());
    }
    #[test]
    fn zero_callback_consumes_first_gate_and_terminal_callback_keeps_last_pose() {
        let birth = crate::procedural_aim::Matrix::identity();
        let mut p = super::Playback::new(spawn(), birth).unwrap();
        assert!(p.advance_free(-0.01).is_err());
        assert!(p.advance_free(0.).unwrap());
        assert_eq!(p.visual.age, 0.);
        assert_eq!(p.motion.position, [0.; 3]);
        assert!(p.angles.is_none());
        p.advance_free(0.01).unwrap();
        assert!(p.visible());
        assert!(p.motion.position[1] < 0.);
        assert!(p.angles.is_some());
        let last = p.motion.position;
        p.advance_free(0.01).unwrap();
        assert!(p.expired);
        assert!(!p.visible());
        assert_eq!(p.motion.position, last);
        assert!(!p.advance_free(1.).unwrap());
        assert_eq!(p.motion.position, last);
    }
    #[test]
    fn cyclic_or_forward_parent_cannot_produce_a_pose() {
        let mut s = spawn();
        s.parent = Some(0);
        let references = super::References {
            classes: std::collections::BTreeMap::new(),
        };
        assert!(references.matrices(&[s], [0.; 3]).is_err());
    }
    #[test]
    fn world_start_and_attached_sound_are_distinct_pending_requests() {
        let plan = super::Plan {
            sound: Some([77, 77]),
            bodies: vec![super::Body {
                class: 900,
                ops: vec![super::Op::Lifetime([1., 0., 0.5]), super::Op::Sound(77)],
            }],
        };
        let mut seed = 1;
        let spawns = plan.create(&mut seed).unwrap();
        assert_eq!(spawns[0].sounds.len(), 2);
        assert!(!spawns[0].sounds[0].attached);
        assert!(spawns[0].sounds[1].attached);
        assert_eq!(spawns[0].sounds[0].id, 77);
        assert_eq!(spawns[0].sounds[1].id, 77);
    }
    #[test]
    fn prepared_activation_preserves_cue_bits_and_does_not_repeat_root_draws() {
        let plan = super::Plan {
            sound: None,
            bodies: vec![super::Body {
                class: 900,
                ops: vec![
                    super::Op::RandomSound([0x1000001, 0x1000001]),
                    super::Op::SizeRange([[1., 2.], [3., 4.]]),
                    super::Op::FixedSpin([7., 11., 13.]),
                    super::Op::Lifetime([1., 0., 0.5]),
                ],
            }],
        };
        let mut full_seed = 1;
        let full = plan.create(&mut full_seed).unwrap().remove(0);
        let mut prepared = crate::audio_parameters::Random(1);
        for _ in 0..3 {
            prepared.next_15();
        }
        let active = plan.create_active(&mut prepared.0).unwrap().remove(0);
        assert_eq!(full_seed, 1030492215);
        assert_eq!(prepared.0, full_seed);
        assert_eq!(active.retained_sound, Some(0x1000001));
        assert_eq!(active.sounds[0].id, full.retained_sound.unwrap());
        assert!(active.sounds[0].attached);
        assert_eq!(
            active.visual.start_size.to_bits(),
            full.visual.start_size.to_bits()
        );
        assert_eq!(
            active.visual.end_size.to_bits(),
            full.visual.end_size.to_bits()
        );
        assert_eq!(active.spin, [11., 7., 13.]);
        let mut invalid = plan;
        invalid.bodies[0].ops[1] = super::Op::SizeRange([[-1.; 2], [3., 4.]]);
        let before = prepared.0;
        assert!(invalid.create_active(&mut prepared.0).is_err());
        assert_eq!(prepared.0, before);
    }
    #[test]
    fn single_impact_does_not_admit_an_unacquired_general_phase() {
        let name = b"synthetic\0";
        let mut program = vec![3];
        program.extend(900u32.to_le_bytes());
        program.extend([6, 0x83]);
        for v in [101u32, 103] {
            program.extend(v.to_le_bytes());
        }
        program.push(9);
        for v in [1f32, 3., 4., 6.] {
            program.extend(v.to_le_bytes());
        }
        for (op, values) in [(4, [2f32, 0., 1.]), (0x32, [7., 11., 13.])] {
            program.push(op);
            for v in values {
                program.extend(v.to_le_bytes());
            }
        }
        program.extend([7, 2]);
        let mut payload = name.to_vec();
        payload.extend(((name.len() + 4) as u32).to_le_bytes());
        payload.extend(program);
        let mut bytes = vec![];
        for v in [1, payload.len() as u32, 1] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.extend(payload);
        let image = giants_formats::effect_image::parse(&bytes).unwrap();
        let frame = image.program(image.entries[0].offset).unwrap();
        assert!(super::Plan::from_records(&image, &frame.records, 1).is_err());
        assert!(super::Plan::project_positional_impact(&image, "synthetic").is_ok());
    }
    #[test]
    fn default_gravity_value_without_physics_driver_does_not_move_a_flash() {
        let mut s = spawn();
        s.physics_enabled = false;
        s.gravity_mass = 1.;
        s.speed = 0.;
        let mut birth = crate::procedural_aim::Matrix::identity();
        birth.values[3] = 125.;
        let mut p = super::Playback::new(s, birth).unwrap();
        for _ in 0..3 {
            p.advance_free(0.005).unwrap();
        }
        assert_eq!(p.motion.position, [125., 0., 0.]);
        assert_eq!(p.motion.force, [0.; 3]);
    }
}
