//! Source-selected hidden periodic emitter and finite detached descendants.
//! Construction only: dispatch cadence, retirement placement and rendering are
//! separate consumers, never implied by a successful projection.
use giants_formats::{effect_image::Image, effect_program::Record};

#[derive(Clone, Debug)]
enum Operand {
    Spin([f32; 3]),
    FixedSize(f32),
    Size([f32; 2]),
    Sizes([[f32; 2]; 2]),
    Lifetime([f32; 3]),
    Lifetimes([[f32; 2]; 3]),
    Gravity(f32),
    Gravities([f32; 2]),
    Side([f32; 2]),
}
#[derive(Clone, Debug)]
pub struct Particle {
    pub name: String,
    pub offset: u32,
    pub class: i32,
    pub retirement: Option<String>,
    /// Source tracer configuration; history/ribbon submission is separate.
    /// This is not an illumination source.
    pub tracer: Option<crate::effect_projectile::Tracer>,
    /// Absent size commands leave constructor fields untouched. The unit-size
    /// visual default must be checked against the renderer's constructed class.
    pub size_configured: bool,
    operands: Vec<Operand>,
}
#[derive(Clone, Debug)]
pub struct Configuration {
    pub angular_velocity: Option<[f32; 3]>,
    pub class: i32,
    pub visual: crate::effect_visual::Configuration,
    pub gravity_mass: f32,
    pub side_speed: Option<f32>,
    pub physics_enabled: bool,
}
#[derive(Clone, Debug)]
pub struct Recipe {
    pub class: i32,
    pub child: String,
    pub interval: [f32; 2],
    pub particles: Vec<Particle>,
}
fn word(r: &Record, at: usize) -> Result<u32, &'static str> {
    Ok(u32::from_le_bytes(
        r.bytes
            .get(at..at + 4)
            .ok_or("emitter operand truncated")?
            .try_into()
            .map_err(|_| "emitter word invalid")?,
    ))
}
fn values<const N: usize>(r: &Record) -> Result<[f32; N], &'static str> {
    let mut out = [0.; N];
    for (i, v) in out.iter_mut().enumerate() {
        *v = f32::from_bits(word(r, 1 + i * 4)?);
        if !v.is_finite() {
            return Err("nonfinite emitter operand");
        }
    }
    Ok(out)
}
fn ranges<const N: usize>(r: &Record) -> Result<[[f32; 2]; N], &'static str> {
    let mut out = [[0.; 2]; N];
    for (i, pair) in out.iter_mut().enumerate() {
        *pair = [
            f32::from_bits(word(r, 1 + i * 8)?),
            f32::from_bits(word(r, 5 + i * 8)?),
        ];
        // Original floating ranges allow descending endpoints (negative gravity).
        if pair.iter().any(|v| !v.is_finite()) {
            return Err("nonfinite emitter range");
        }
    }
    Ok(out)
}
fn name(r: &Record) -> Result<String, &'static str> {
    let bytes = r.bytes.get(1..).ok_or("emitter name absent")?;
    let end = bytes
        .iter()
        .position(|v| *v == 0)
        .ok_or("emitter name unterminated")?;
    if end == 0 {
        return Err("emitter name empty");
    }
    std::str::from_utf8(&bytes[..end])
        .map(str::to_owned)
        .map_err(|_| "emitter name invalid")
}
impl Recipe {
    pub fn project(image: &Image, label: &str) -> Result<Self, &'static str> {
        let (_, entry) = image.find(label).ok_or("emitter program absent")?;
        let p = image
            .program(entry.offset)
            .map_err(|_| "emitter program invalid")?;
        if p.records.iter().map(|r| r.opcode).collect::<Vec<_>>() != [3, 6, 0x21, 0x1e, 7, 2] {
            return Err("program outside hidden periodic emitter profile");
        }
        let class = word(&p.records[0], 1)? as i32;
        let child = name(&p.records[3])?;
        let at = child.len() + 2;
        let interval = [
            f32::from_bits(word(&p.records[3], at)?),
            f32::from_bits(word(&p.records[3], at + 4)?),
        ];
        if class < 0
            || p.records[3].bytes.len() != at + 8
            || interval.iter().any(|v| !v.is_finite() || *v <= 0.)
        {
            return Err("invalid emitter class or positive interval range");
        }
        Self::from_child(image, class, child, interval)
    }
    /// Resolve the same bounded retirement chain from an already constructed
    /// service. This does not sample or register its interval again.
    pub fn from_child(
        image: &Image,
        class: i32,
        child: String,
        interval: [f32; 2],
    ) -> Result<Self, &'static str> {
        if class < 0 || interval.iter().any(|v| !v.is_finite() || *v <= 0.) {
            return Err("invalid emitter class or positive interval range");
        }
        let mut particles = Vec::new();
        let mut next = Some(child.clone());
        while let Some(label) = next {
            if particles.len() >= 8
                || particles
                    .iter()
                    .any(|p: &Particle| p.name.eq_ignore_ascii_case(&label))
            {
                return Err("cyclic or excessive emitter retirement chain");
            }
            let particle = Particle::project(image, &label)?;
            next = particle.retirement.clone();
            particles.push(particle);
        }
        Ok(Self {
            class,
            child,
            interval,
            particles,
        })
    }
    /// Named skip pass then active registration. These consume independent
    /// random values, even for equal floating endpoints.
    pub fn create(
        &self,
        seed: &mut u32,
        position: [f32; 3],
    ) -> Result<crate::effect_emission::Periodic, &'static str> {
        if position.iter().any(|v| !v.is_finite()) {
            return Err("nonfinite emitter position");
        }
        crate::timed_effect::sample(seed, self.interval);
        let interval = crate::timed_effect::sample(seed, self.interval);
        Ok(crate::effect_emission::Periodic {
            interval,
            remaining: 0.,
            phase: 0.,
            previous_position: position,
        })
    }
}
impl Particle {
    /// Normal attached preview excludes contact motion and child programs at
    /// retirement; neither may silently become a position-only particle.
    pub fn attachment_profile(&self) -> Result<(), &'static str> {
        if self.retirement.is_some()
            || self.operands.iter().any(|op| {
                matches!(
                    op,
                    Operand::Gravity(_) | Operand::Gravities(_) | Operand::Side(_)
                )
            })
        {
            return Err("attached particle motion or retirement backend pending");
        }
        Ok(())
    }
    pub fn project(image: &Image, label: &str) -> Result<Self, &'static str> {
        let (_, entry) = image.find(label).ok_or("emitted particle absent")?;
        let p = image
            .program(entry.offset)
            .map_err(|_| "emitted particle invalid")?;
        let r = &p.records;
        if r.len() < 5
            || r[0].opcode != 3
            || r[1].opcode != 6
            || r[r.len() - 2].opcode != 7
            || r[r.len() - 1].opcode != 2
        {
            return Err("emitted particle outside finite detached profile");
        }
        let class = word(&r[0], 1)? as i32;
        if class < 0 {
            return Err("invalid emitted class");
        }
        let mut operands = Vec::new();
        let mut retirement = None;
        let mut tracer = None;
        let records = &r[2..r.len() - 2];
        let mut cursor = 0;
        while let Some(record) = records.get(cursor) {
            cursor += 1;
            if record.opcode == 0x20 {
                if tracer.is_some() {
                    return Err("duplicate emitted particle tracer block");
                }
                let block = records
                    .get(cursor - 1..cursor + 3)
                    .ok_or("incomplete emitted particle tracer block")?;
                if block.iter().map(|r| r.opcode).collect::<Vec<_>>() != [0x20, 0x22, 0x25, 0x26] {
                    return Err("emitted particle tracer block outside checked order");
                }
                let colors = values::<6>(&block[0])?;
                let timing = values::<2>(&block[1])?;
                let configured = crate::effect_projectile::Tracer {
                    colors: [
                        [colors[0], colors[1], colors[2]],
                        [colors[3], colors[4], colors[5]],
                    ],
                    lifetime: timing[0],
                    fade_start: timing[1],
                    half_width: values::<1>(&block[2])?[0],
                    min_interval: values::<1>(&block[3])?[0],
                };
                if configured.fade_start >= configured.lifetime
                    || configured
                        .configured_words()
                        .iter()
                        .any(|v| !f32::from_bits(*v).is_finite())
                {
                    return Err("emitted particle tracer timing outside finite descending profile");
                }
                tracer = Some(configured);
                cursor += 3;
                continue;
            }
            operands.push(match record.opcode {
                0x32 => Operand::Spin(values(record)?),
                0x1b => Operand::FixedSize(values::<1>(record)?[0]),
                0x1c => Operand::Size(ranges::<1>(record)?[0]),
                9 => Operand::Sizes(ranges(record)?),
                4 => Operand::Lifetime(values(record)?),
                5 => Operand::Lifetimes(ranges(record)?),
                0xe => Operand::Gravity(values::<1>(record)?[0]),
                0xf => Operand::Gravities(ranges::<1>(record)?[0]),
                0x80 => Operand::Side(ranges::<1>(record)?[0]),
                0xd if retirement.is_none() => {
                    let label = name(record)?;
                    if record.bytes.len() != label.len() + 2 {
                        return Err("particle retirement framing invalid");
                    }
                    retirement = Some(label);
                    continue;
                }
                _ => return Err("emitted particle command outside checked profile"),
            });
        }
        if !operands
            .iter()
            .any(|o| matches!(o, Operand::Lifetime(_) | Operand::Lifetimes(_)))
        {
            return Err("emitted particle finite lifetime absent");
        }
        let result = Self {
            name: label.into(),
            offset: entry.offset,
            class,
            retirement,
            tracer,
            size_configured: operands.iter().any(|o| {
                matches!(
                    o,
                    Operand::FixedSize(_) | Operand::Size(_) | Operand::Sizes(_)
                )
            }),
            operands,
        };
        result.create(&mut 1)?;
        Ok(result)
    }
    pub fn create(&self, seed: &mut u32) -> Result<Configuration, &'static str> {
        self.prepare(seed);
        self.activate(seed)
    }
    pub fn prepare(&self, seed: &mut u32) {
        // Most float operands sample in named skip traversal. Opcode80 instead
        // checks skip mode before invoking its RNG helper.
        for operand in &self.operands {
            if !matches!(operand, Operand::Side(_)) {
                operand.sample(seed);
            }
        }
    }
    pub fn activate(&self, seed: &mut u32) -> Result<Configuration, &'static str> {
        let mut c = Configuration {
            angular_velocity: None,
            class: self.class,
            visual: crate::effect_visual::Configuration {
                lifetime: -1.,
                fade_in: 0.,
                fade_out_start: 0.,
                start_size: 1.,
                end_size: 1.,
            },
            gravity_mass: 1.,
            side_speed: None,
            physics_enabled: false,
        };
        for operand in &self.operands {
            let v = operand.sample(seed);
            match operand {
                Operand::Spin(_) => c.angular_velocity = Some([v[1], v[0], v[2]]),
                Operand::FixedSize(size) => {
                    c.visual.start_size = *size;
                    c.visual.end_size = *size;
                }
                Operand::Size(_) => {
                    c.visual.start_size = v[0];
                    c.visual.end_size = v[0];
                }
                Operand::Sizes(_) => {
                    c.visual.start_size = v[0];
                    c.visual.end_size = v[1];
                }
                Operand::Lifetime(_) | Operand::Lifetimes(_) => {
                    c.visual.lifetime = v[0];
                    c.visual.fade_in = v[1];
                    c.visual.fade_out_start = v[2];
                }
                Operand::Gravity(_) | Operand::Gravities(_) => {
                    c.gravity_mass = v[0];
                    c.physics_enabled = true;
                }
                Operand::Side(_) => c.side_speed = Some(v[0]),
            }
        }
        if c.visual.start_size <= 0. || c.visual.end_size <= 0. {
            return Err("invalid emitted particle size");
        }
        crate::effect_visual::Visual::new(c.visual)?;
        Ok(c)
    }
}

impl Recipe {
    /// Preview allocation reserve derived from source duration, positive minimum
    /// interval and the longest descendant chain. Exhaustion is an error, never
    /// silent particle loss; this is not the native global allocation policy.
    pub fn reserve(&self, source_seconds: f32) -> Result<usize, &'static str> {
        let mut lifetime = 0_f32;
        for particle in &self.particles {
            let longest = particle
                .operands
                .iter()
                .filter_map(|o| match o {
                    Operand::Lifetime(v) => Some(v[0]),
                    Operand::Lifetimes(v) => Some(v[0][0].max(v[0][1])),
                    _ => None,
                })
                .fold(0_f32, f32::max);
            lifetime += longest;
        }
        let seconds = source_seconds.max(lifetime);
        let count =
            (f64::from(seconds) / f64::from(self.interval[0].min(self.interval[1]))).ceil() + 2.;
        if !source_seconds.is_finite() || source_seconds < 0. || !count.is_finite() || count > 512.
        {
            return Err("periodic preview allocation reserve outside budget");
        }
        Ok(count as usize)
    }
}
impl Operand {
    fn sample(&self, seed: &mut u32) -> Vec<f32> {
        match self {
            Self::Spin(v) => v.to_vec(),
            Self::Size(r) | Self::Gravities(r) | Self::Side(r) => {
                vec![crate::timed_effect::sample(seed, *r)]
            }
            Self::Sizes(r) => r
                .iter()
                .map(|r| crate::timed_effect::sample(seed, *r))
                .collect(),
            Self::Lifetimes(r) => r
                .iter()
                .map(|r| crate::timed_effect::sample(seed, *r))
                .collect(),
            Self::Lifetime(v) => v.to_vec(),
            Self::Gravity(v) | Self::FixedSize(v) => vec![*v],
        }
    }
}

/// Own-matrix initial velocity only: -Y forward plus an optional normalized +X
/// contribution. Preserve the native store after normalization and final sum.
pub fn initial_velocity(
    matrix: crate::procedural_aim::Matrix,
    forward: f32,
    side: Option<f32>,
) -> Result<[f32; 3], &'static str> {
    let base = crate::effect_projectile::initial_velocity(matrix, forward)?;
    let Some(side) = side else {
        return Ok(base);
    };
    if !side.is_finite() {
        return Err("nonfinite emitted side speed");
    }
    let mut axis = matrix;
    for (to, from) in [(1, 0), (5, 4), (9, 8)] {
        axis.values[to] = -matrix.values[from];
    }
    let normalized = crate::effect_projectile::initial_velocity(axis, 1.)?;
    Ok(std::array::from_fn(|i| {
        (f64::from(normalized[i]) * f64::from(side) + f64::from(base[i])) as f32
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn partial_or_repeated_light_blocks_remain_refused() {
        let mut block = vec![0x20];
        for value in [1_f32, 0., 0., 0., 1., 0.] {
            block.extend(value.to_le_bytes());
        }
        block.push(0x22);
        for value in [100_f32, 1.] {
            block.extend(value.to_le_bytes());
        }
        block.push(0x25);
        block.extend(0.01_f32.to_le_bytes());
        block.push(0x26);
        block.extend(0.5_f32.to_le_bytes());
        for suffix in [&block[..25], &block[..34], &block[..39]] {
            let mut body = particle(None);
            body.truncate(body.len() - 2);
            body.extend(suffix);
            body.extend([7, 2]);
            assert!(super::Particle::project(&image(&[("test", body)]), "test").is_err());
        }
        let mut body = particle(None);
        body.truncate(body.len() - 2);
        body.extend(&block);
        body.extend(&block);
        body.extend([7, 2]);
        assert!(super::Particle::project(&image(&[("test", body)]), "test").is_err());
    }
    fn image(programs: &[(&str, Vec<u8>)]) -> giants_formats::effect_image::Image {
        let mut payload = Vec::new();
        let mut offset = programs.iter().map(|(n, _)| n.len() + 5).sum::<usize>();
        for (name, body) in programs {
            payload.extend(name.as_bytes());
            payload.push(0);
            payload.extend((offset as u32).to_le_bytes());
            offset += body.len();
        }
        for (_, body) in programs {
            payload.extend(body);
        }
        let mut bytes = 1u32.to_le_bytes().to_vec();
        bytes.extend((payload.len() as u32).to_le_bytes());
        bytes.extend((programs.len() as u32).to_le_bytes());
        bytes.extend(payload);
        giants_formats::effect_image::parse(&bytes).unwrap()
    }
    fn emitter(interval: f32) -> Vec<u8> {
        let mut b = vec![3];
        b.extend(900u32.to_le_bytes());
        b.extend([6, 0x21, 0x1e]);
        b.extend(b"child\0");
        b.extend(interval.to_le_bytes());
        b.extend(interval.to_le_bytes());
        b.extend([7, 2]);
        b
    }
    fn particle(retire: Option<&str>) -> Vec<u8> {
        let mut b = vec![3];
        b.extend(901u32.to_le_bytes());
        b.extend([6, 4]);
        for value in [1f32, 0., 0.5] {
            b.extend(value.to_le_bytes());
        }
        if let Some(name) = retire {
            b.push(0xd);
            b.extend(name.as_bytes());
            b.push(0);
        }
        b.extend([7, 2]);
        b
    }
    #[test]
    fn cyclic_retirement_and_nonpositive_interval_are_refused() {
        let cyclic = image(&[
            ("emitter", emitter(0.5)),
            ("child", particle(Some("child"))),
        ]);
        assert!(super::Recipe::project(&cyclic, "emitter").is_err());
        let invalid = image(&[("emitter", emitter(0.)), ("child", particle(None))]);
        assert!(super::Recipe::project(&invalid, "emitter").is_err());
    }
    #[test]
    fn fixed_service_chain_drains_after_source_stops_without_interval_rng() {
        let source = image(&[("child", particle(Some("tail"))), ("tail", particle(None))]);
        let recipe = super::Recipe::from_child(&source, 900, "child".into(), [1.; 2]).unwrap();
        assert_eq!(recipe.particles.len(), 2);
        let reserve = recipe.reserve(0.).unwrap();
        let mut runtime =
            crate::effect_emitter_runtime::Runtime::fixed(std::sync::Arc::new(recipe), reserve)
                .unwrap();
        let mut seed = 1;
        let mut events = Vec::new();
        for (dt, emitting) in [
            (0.25, true),
            (0.25, true),
            (0.25, false),
            (0.5, false),
            (0.5, false),
            (0.25, false),
            (0.5, false),
            (0.5, false),
        ] {
            events.extend(
                runtime
                    .advance_source(dt, [10., 20., 30.], emitting, &mut seed)
                    .unwrap(),
            );
        }
        assert_eq!(seed, 1);
        let activated = events
            .iter()
            .filter(|e| e.kind == "activated")
            .collect::<Vec<_>>();
        assert_eq!(activated.len(), 2);
        assert_eq!(activated[0].particle, 0);
        assert_eq!(activated[1].particle, 1);
        assert_eq!(activated[0].position, activated[1].position);
        assert_eq!(events.iter().filter(|e| e.kind == "expired").count(), 2);
        assert!(runtime.presentation(0, 0).unwrap().expired);
        assert!(runtime.presentation(1, 0).unwrap().expired);
    }
    #[test]
    fn initial_cache_is_supplied_and_invalid_position_does_not_consume_randomness() {
        let source = image(&[("emitter", emitter(0.5)), ("child", particle(None))]);
        let recipe = super::Recipe::project(&source, "emitter").unwrap();
        let mut seed = 1;
        assert!(recipe.create(&mut seed, [f32::NAN, 0., 0.]).is_err());
        assert_eq!(seed, 1);
        let state = recipe.create(&mut seed, [12.5, -40., 3.25]).unwrap();
        assert_eq!(state.previous_position, [12.5, -40., 3.25]);
        assert_eq!(state.remaining, 0.);
        assert_eq!(state.phase, 0.);
        assert_eq!(
            state.advance(0.01, [12.5, -40., 3.25]).unwrap().positions,
            [[12.5, -40., 3.25]]
        );
    }
}
