//! Ordered construction of mixed attached/detached effect objects and waits.
//! This is a construction ledger, not a service, motion or rendering backend.
//! Service configuration remains explicit, including its source RNG consumption.
//! Prototype PS2 `fx_processOpCodes__FP6FxDataiP6OBJECT` is an unknown PC
//! attribution search route. The guard-building continuation preserves pending
//! source15/source47 consumers; parsing their stores does not execute callbacks.
use giants_formats::{effect_image::Image, effect_program::Record};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    Detached,
    Attached {
        opaque: u32,
        reference: u32,
    },
    Immediate {
        parent: usize,
        reference: u32,
        own_reference: u32,
        /// Source70 copies the parent context frame; source10 passes no frame.
        inherit_context: bool,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceKind {
    FixedEmission,
    RangedEmission,
    /// Source opcode28 names another FX program; active emission is unbound.
    Source28,
    /// Source opcode57 names another FX program; active emission is unbound.
    Source57,
}
#[derive(Clone, Debug)]
pub struct Service {
    pub kind: ServiceKind,
    pub record: Record,
    /// Raw CRT sample for the unimplemented service, not an acknowledgement.
    pub random_sample: Option<u32>,
}
#[derive(Clone, Debug)]
pub struct EmissionConfiguration {
    pub effect: String,
    pub interval: f32,
    /// Source words passed to the native factory. Geometry interpretation and
    /// subsequent reference selection remain a separate backend.
    pub selectors: [[u32; 2]; 2],
    pub source_flag8: bool,
    pub source_flag9: bool,
}
impl Service {
    /// Bounds for preview capacity, not a new interval sample. Descending source
    /// ranges are retained; callers select the minimum for a conservative pool.
    pub fn interval_bounds(&self) -> Result<[f32; 2], &'static str> {
        service(self.kind, &self.record)?;
        let end = self.record.bytes[1..]
            .iter()
            .position(|b| *b == 0)
            .ok_or("service label absent")?
            + 2;
        let low = f32::from_bits(word(&self.record, end)?);
        let high = if self.kind == ServiceKind::FixedEmission {
            low
        } else {
            f32::from_bits(word(&self.record, end + 4)?)
        };
        if [low, high].into_iter().any(|v| !v.is_finite() || v < 0.) || low.max(high) <= 0. {
            return Err("service interval bounds outside nonnegative profile");
        }
        Ok([low, high])
    }
    pub fn emission_configuration(&self) -> Result<EmissionConfiguration, &'static str> {
        service(self.kind, &self.record)?;
        let end = self.record.bytes[1..]
            .iter()
            .position(|b| *b == 0)
            .ok_or("service label absent")?
            + 1;
        let effect = std::str::from_utf8(&self.record.bytes[1..end])
            .map_err(|_| "service label invalid")?
            .to_owned();
        let start = end + 1;
        let low = f32::from_bits(word(&self.record, start)?);
        let interval = if self.kind == ServiceKind::FixedEmission {
            if self.random_sample.is_some() {
                return Err("fixed service consumed interval RNG");
            }
            low
        } else {
            let high = f32::from_bits(word(&self.record, start + 4)?);
            let sample = self.random_sample.ok_or("service interval sample absent")?;
            if sample > 32767 || !high.is_finite() {
                return Err("invalid service interval sample/range");
            }
            ((f64::from(sample) * (f64::from(high) - f64::from(low))) * (1. / 32768.)
                + f64::from(low)) as f32
        };
        if !low.is_finite() || !interval.is_finite() || interval <= 0. {
            return Err("service interval outside positive profile");
        }
        let mut selectors = [[u32::MAX; 2]; 2];
        if matches!(self.kind, ServiceKind::Source28 | ServiceKind::Source57) {
            selectors[0] = [
                word(&self.record, start + 8)?,
                word(&self.record, start + 12)?,
            ];
        }
        if self.kind == ServiceKind::Source57 {
            // Source pair order is reversed at the factory boundary.
            selectors[1] = selectors[0];
            selectors[0] = [
                word(&self.record, start + 16)?,
                word(&self.record, start + 20)?,
            ];
        }
        Ok(EmissionConfiguration {
            effect,
            interval,
            selectors,
            source_flag8: self.kind == ServiceKind::Source57,
            source_flag9: false,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Construction {
    pub class: i32,
    pub origin: Origin,
    pub hidden: bool,
    pub size: Option<[f32; 2]>,
    /// Keep the source curve selector; no curve is invented by this planner.
    pub size_opcode: Option<u8>,
    /// None means the program supplied no duration, not a fabricated timeout.
    pub lifetime: Option<[f32; 3]>,
    pub height: f32,
    /// Direct constructor displacement, distinct from the persistent height field.
    pub offset_z: f32,
    pub spin: [f32; 3],
    pub direction_requested: bool,
    pub speed: f32,
    pub gravity: f32,
    pub damping: [f32; 2],
    pub bounce: f32,
    /// Presence of source command19, independent of its numeric coefficient.
    pub contact_requested: bool,
    /// Sampled source79 displacement. Requires a real supplied surface profile.
    pub surface_offset: Option<f32>,
    pub physics_enabled: bool,
    pub services: Vec<Service>,
    pub context_sounds: Vec<ContextSoundNeed>,
    /// Native object callback configuration only; no callback is executed here.
    pub object_services: ObjectServiceNeeds,
}
#[derive(Clone, Debug, Default)]
pub struct ObjectServiceNeeds {
    /// Source-ordered renderer setup, not executed by this construction ledger.
    /// Keep repeated commands: scroll may replace an earlier setup, whereas
    /// vertical fade appends segments. A consumer must bind the actual services.
    pub render_commands: Vec<RenderCommand>,
    /// Source49 registration; native association, resume and cleanup are pending.
    pub logged_context: bool,
    pub source16: bool,
    /// Acquired control-word request, without its active callback consumer.
    pub source15: bool,
    /// Raw source47 operand, including zero; active response remains unbound.
    pub source47: Option<u32>,
    pub source5a: Option<DeferredEffectBinding>,
    /// Named program configured for retirement; no callback is acknowledged.
    pub retirement: Option<RetirementBinding>,
}
impl ObjectServiceNeeds {
    pub fn pending_count(&self) -> usize {
        self.render_commands.len()
            + usize::from(self.logged_context)
            + usize::from(self.source16)
            + usize::from(self.source15)
            + usize::from(self.source47.is_some())
            + usize::from(self.source5a.is_some())
            + usize::from(self.retirement.is_some())
    }
}
/// Prototype PS2 search keys `fx_setupUVScrolling` and `fx_setupVerticleFade`
/// have unknown PC attribution. Operands here are from the PC source program;
/// no prototype signature, layout or rendering completion is imported.
#[derive(Clone, Debug, PartialEq)]
pub enum RenderCommand {
    UvScroll([f32; 2]),
    /// The five PC operands in source order. Multi-segment callback ownership
    /// and vertex-buffer application are separate from command retention.
    VerticalFade([f32; 5]),
}
#[derive(Clone, Debug)]
pub struct RetirementBinding {
    pub effect: String,
    pub program_offset: u32,
}
#[derive(Clone, Debug)]
pub struct DeferredEffectBinding {
    pub effect: String,
    pub parameter: u32,
    pub program_offset: u32,
}
/// Selected context service; never an acknowledgement of audio playback.
#[derive(Clone, Debug)]
pub struct ContextSoundNeed {
    pub record_offset: usize,
    pub id: u32,
}
#[derive(Clone, Debug)]
struct ContextSound {
    record_offset: usize,
    range: [u32; 2],
}
impl ContextSound {
    fn project(record: &Record) -> Result<Self, &'static str> {
        let range = [word(record, 1)?, word(record, 5)?];
        if range[0] > range[1] || range[1] > i32::MAX as u32 {
            return Err("scene context sound range invalid");
        }
        Ok(Self {
            record_offset: record.offset,
            range,
        })
    }
    fn select(&self, seed: &mut u32) -> ContextSoundNeed {
        let sample = random(seed);
        ContextSoundNeed {
            record_offset: self.record_offset,
            id: self.range[0]
                + ((u64::from(sample) * u64::from(self.range[1] - self.range[0] + 1)) >> 15) as u32,
        }
    }
}
#[derive(Clone, Debug)]
enum Op {
    Render(RenderCommand),
    LogContext,
    Size(u8, [f32; 2]),
    Lifetime([f32; 3]),
    RandomLifetime([[f32; 2]; 3]),
    RandomSizes([[f32; 2]; 2]),
    RandomCurvedSizes([[f32; 2]; 2]),
    RandomSpin([[f32; 2]; 3]),
    RandomSize([f32; 2]),
    Source15,
    Source47(u32),
    Source16,
    Source5a(DeferredEffectBinding),
    RandomGravity([f32; 2]),
    RandomOffsetZ([f32; 2]),
    Height(f32),
    Spin([f32; 3]),
    Speed([f32; 2]),
    Gravity(f32),
    Damping([f32; 2]),
    Bounce(f32),
    Retirement(RetirementBinding),
    SurfaceOffset([f32; 2]),
    Hidden,
    Service(ServiceKind, Record),
    Child {
        reference: u32,
        own_reference: u32,
        inherit_context: bool,
        /// Original named creation can return no context for an absent entry.
        /// Retain the command operands without inventing an object or RNG draw.
        program: Option<Box<Plan>>,
    },
}
#[derive(Clone, Debug)]
struct Object {
    class: i32,
    origin: Origin,
    ops: Vec<Op>,
    before_sounds: Vec<ContextSound>,
}
#[derive(Clone, Debug)]
struct Phase {
    objects: Vec<Object>,
    wait: Option<f32>,
    external_wait: bool,
}
#[derive(Clone, Debug)]
pub struct Plan {
    phases: Vec<Phase>,
}
/// A named skip traversal has happened; activation consumes this token once.
pub struct Queued {
    plan: std::sync::Arc<Plan>,
}
impl Queued {
    pub fn activate(
        self,
        seed: &mut u32,
    ) -> Result<(std::sync::Arc<Plan>, Vec<Construction>, Cursor), &'static str> {
        let (objects, cursor) = self.plan.activate(seed)?;
        Ok((self.plan, objects, cursor))
    }
}
#[derive(Clone, Debug)]
pub struct Cursor {
    pub next: usize,
    pub remaining: Option<f32>,
    /// Suspended independently of elapsed time. No native resume is invented.
    pub external_wait: bool,
}

fn word(r: &Record, at: usize) -> Result<u32, &'static str> {
    Ok(u32::from_le_bytes(
        r.bytes
            .get(at..at + 4)
            .ok_or("scene operand truncated")?
            .try_into()
            .map_err(|_| "scene word invalid")?,
    ))
}
fn values<const N: usize>(r: &Record) -> Result<[f32; N], &'static str> {
    let mut out = [0.; N];
    for (i, value) in out.iter_mut().enumerate() {
        *value = f32::from_bits(word(r, 1 + i * 4)?);
        if !value.is_finite() {
            return Err("nonfinite scene operand");
        }
    }
    Ok(out)
}
fn ranges<const N: usize>(r: &Record) -> Result<[[f32; 2]; N], &'static str> {
    let mut out = [[0.; 2]; N];
    for (i, range) in out.iter_mut().enumerate() {
        *range = [
            f32::from_bits(word(r, 1 + i * 8)?),
            f32::from_bits(word(r, 5 + i * 8)?),
        ];
        // The original float sampler also accepts descending endpoints.
        if range.iter().any(|v| !v.is_finite()) {
            return Err("nonfinite scene range");
        }
    }
    Ok(out)
}
fn service(kind: ServiceKind, r: &Record) -> Result<Op, &'static str> {
    let opcode = match kind {
        ServiceKind::FixedEmission => 0x1d,
        ServiceKind::RangedEmission => 0x1e,
        ServiceKind::Source28 => 0x28,
        ServiceKind::Source57 => 0x57,
    };
    if r.opcode != opcode || r.bytes.first() != Some(&opcode) {
        return Err("scene service kind/opcode mismatch");
    }
    // Framing is checked by effect_program; keep the source operands intact.
    let end = r.bytes[1..]
        .iter()
        .position(|b| *b == 0)
        .ok_or("scene service name absent")?
        + 1;
    if end == 1 || !r.bytes[1..end].is_ascii() {
        return Err("invalid scene service name");
    }
    let words = match kind {
        ServiceKind::FixedEmission => 1,
        ServiceKind::RangedEmission => 2,
        ServiceKind::Source28 => 4,
        ServiceKind::Source57 => 6,
    };
    if r.bytes.len() != end + 1 + words * 4 {
        return Err("scene service framing mismatch");
    }
    Ok(Op::Service(kind, r.clone()))
}
fn random(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(214013).wrapping_add(2531011);
    (*seed >> 16) & 32767
}
impl Plan {
    pub fn project(image: &Image, name: &str) -> Result<Self, &'static str> {
        Self::project_inner(image, name, &mut Vec::new(), &mut 0)
    }
    fn project_inner(
        image: &Image,
        name: &str,
        stack: &mut Vec<String>,
        records_seen: &mut usize,
    ) -> Result<Self, &'static str> {
        if stack.len() >= 8 || stack.iter().any(|s| s.eq_ignore_ascii_case(name)) {
            return Err("scene immediate child cycle or depth budget");
        }
        let (_, entry) = image.find(name).ok_or("scene program absent")?;
        let program = image
            .program(entry.offset)
            .map_err(|_| "scene framing invalid")?;
        *records_seen += program.records.len();
        if *records_seen > 32768 {
            return Err("scene expanded program budget");
        }
        stack.push(name.into());
        let mut phases = Vec::new();
        let mut objects = Vec::new();
        let mut at = 0;
        let mut before_sounds = Vec::new();
        loop {
            let r = program.records.get(at).ok_or("scene terminal absent")?;
            if r.opcode == 0x2b {
                before_sounds.push(ContextSound::project(r)?);
                at += 1;
                continue;
            }
            if matches!(r.opcode, 2 | 0x0b | 0x4a) {
                if !before_sounds.is_empty() {
                    return Err("trailing context sound backend pending");
                }
                if objects.is_empty() || phases.len() >= 16 {
                    return Err("empty scene phase or phase budget");
                }
                let wait = if r.opcode == 0x0b {
                    let [seconds] = values(r)?;
                    if seconds < 0. {
                        return Err("negative scene wait");
                    }
                    Some(seconds)
                } else {
                    None
                };
                phases.push(Phase {
                    objects: std::mem::take(&mut objects),
                    wait,
                    external_wait: r.opcode == 0x4a,
                });
                at += 1;
                if r.opcode == 0x4a {
                    // The continuation backend is not available. Admit only a
                    // checked terminal after the hold; never skip real work.
                    if program.records.get(at).map(|r| r.opcode) != Some(2) {
                        return Err("external context resume continuation pending");
                    }
                    at += 1;
                    break;
                }
                if r.opcode == 2 {
                    break;
                }
                continue;
            }
            let (class, origin) = match r.opcode {
                3 => (word(r, 1)? as i32, Origin::Detached),
                0x24 => (
                    word(r, 5)? as i32,
                    Origin::Attached {
                        opaque: word(r, 1)?,
                        reference: word(r, 9)?,
                    },
                ),
                _ => return Err("scene context command outside construction profile"),
            };
            if class < 0 || objects.len() >= 64 {
                return Err("scene class or object budget");
            }
            at += 1;
            while let Some(record) = program.records.get(at).filter(|r| r.opcode == 0x2b) {
                before_sounds.push(ContextSound::project(record)?);
                at += 1;
            }
            if program.records.get(at).map(|r| r.opcode) != Some(6) {
                return Err("scene object setup absent");
            }
            at += 1;
            let mut ops = Vec::new();
            loop {
                let r = program.records.get(at).ok_or("scene object end absent")?;
                at += 1;
                if r.opcode != 7 && ops.iter().any(|op| matches!(op, Op::SurfaceOffset(_))) {
                    return Err("surface setup must be the final checked object operation");
                }
                let op = match r.opcode {
                    7 => break,
                    0x40 => Op::Render(RenderCommand::UvScroll(values(r)?)),
                    0x46 => Op::Render(RenderCommand::VerticalFade(values(r)?)),
                    5 => Op::RandomLifetime(ranges(r)?),
                    9 => Op::RandomSizes(ranges(r)?),
                    0x35 => Op::RandomCurvedSizes(ranges(r)?),
                    0x33 => Op::RandomSpin(ranges(r)?),
                    0x15 => Op::Source15,
                    0x47 => Op::Source47(word(r, 1)?),
                    0x1c => Op::RandomSize(ranges::<1>(r)?[0]),
                    0x16 => Op::Source16,
                    0x5a => {
                        let end = r.bytes[1..]
                            .iter()
                            .position(|b| *b == 0)
                            .ok_or("scene deferred effect name absent")?
                            + 1;
                        let label = std::str::from_utf8(&r.bytes[1..end])
                            .map_err(|_| "scene deferred effect name invalid")?;
                        let (_, entry) = image.find(label).ok_or("scene deferred effect absent")?;
                        Op::Source5a(DeferredEffectBinding {
                            effect: label.into(),
                            parameter: word(r, end + 1)?,
                            program_offset: entry.offset,
                        })
                    }
                    0xf => Op::RandomGravity(ranges::<1>(r)?[0]),
                    0x18 => {
                        if origin != Origin::Detached
                            || ops
                                .iter()
                                .any(|op| matches!(op, Op::Height(_) | Op::RandomOffsetZ(_)))
                        {
                            return Err(
                                "scene offset requires one detached displacement without height",
                            );
                        }
                        Op::RandomOffsetZ(ranges::<1>(r)?[0])
                    }
                    4 => {
                        let v = values(r)?;
                        if v[0] <= 0. || v[1] < 0. || v[2] < 0. {
                            return Err("scene duration outside positive profile");
                        }
                        Op::Lifetime(v)
                    }
                    0x1b | 8 | 0x34 => {
                        let v = if r.opcode == 0x1b {
                            [values::<1>(r)?[0]; 2]
                        } else {
                            values(r)?
                        };
                        if v.iter().any(|v| *v <= 0.) {
                            return Err("scene size outside positive profile");
                        }
                        Op::Size(r.opcode, v)
                    }
                    0x17 => {
                        if ops.iter().any(|op| matches!(op, Op::RandomOffsetZ(_))) {
                            return Err("scene persistent height with direct displacement pending");
                        }
                        Op::Height(values::<1>(r)?[0])
                    }
                    0x32 => {
                        let [x, y, z] = values(r)?;
                        Op::Spin([y, x, z])
                    }
                    0x12 => Op::Speed(values(r)?),
                    0xe => Op::Gravity(values::<1>(r)?[0]),
                    0x13 => Op::Damping(values(r)?),
                    0x19 => Op::Bounce(values::<1>(r)?[0]),
                    0x49 => {
                        if ops.iter().any(|op| matches!(op, Op::LogContext)) {
                            return Err("repeated context registration pending");
                        }
                        Op::LogContext
                    }
                    0x79 => {
                        if origin != Origin::Detached {
                            return Err("surface alignment requires detached object profile");
                        }
                        Op::SurfaceOffset(ranges::<1>(r)?[0])
                    }
                    0xd => {
                        let label = r
                            .bytes
                            .get(1..r.bytes.len() - 1)
                            .ok_or("scene retirement name absent")?;
                        if r.bytes.last() != Some(&0)
                            || label.is_empty()
                            || !label.is_ascii()
                            || label.contains(&0)
                        {
                            return Err("scene retirement name invalid");
                        }
                        let label = std::str::from_utf8(label)
                            .map_err(|_| "scene retirement name invalid")?;
                        let (_, entry) =
                            image.find(label).ok_or("scene retirement program absent")?;
                        Op::Retirement(RetirementBinding {
                            effect: label.into(),
                            program_offset: entry.offset,
                        })
                    }
                    0x21 => Op::Hidden,
                    0x1d => service(ServiceKind::FixedEmission, r)?,
                    0x1e => service(ServiceKind::RangedEmission, r)?,
                    0x28 => service(ServiceKind::Source28, r)?,
                    0x57 => service(ServiceKind::Source57, r)?,
                    0x10 | 0x70 => {
                        let end = r.bytes[5..]
                            .iter()
                            .position(|b| *b == 0)
                            .ok_or("scene child name absent")?
                            + 5;
                        let label = std::str::from_utf8(&r.bytes[5..end])
                            .map_err(|_| "scene child name invalid")?;
                        let child = if image.find(label).is_some() {
                            Some(Self::project_inner(image, label, stack, records_seen)?)
                        } else {
                            None
                        };
                        if child.as_ref().is_some_and(|child| {
                            child.phases.len() != 1
                                || child.phases[0].objects.len() != 1
                                || child.phases[0].external_wait
                        }) {
                            return Err("immediate child requires one constructed object");
                        }
                        if child.as_ref().is_some_and(|child| {
                            child.phases[0].objects[0]
                                .ops
                                .iter()
                                .any(|op| matches!(op, Op::RandomOffsetZ(_)))
                        }) {
                            return Err("immediate child direct displacement pending");
                        }
                        Op::Child {
                            reference: word(r, 1)?,
                            own_reference: word(r, end + 1)?,
                            inherit_context: r.opcode == 0x70,
                            program: child.map(Box::new),
                        }
                    }
                    _ => return Err("scene object command outside construction profile"),
                };
                ops.push(op);
            }
            objects.push(Object {
                class,
                origin,
                ops,
                before_sounds: std::mem::take(&mut before_sounds),
            });
        }
        stack.pop();
        if at != program.records.len() {
            return Err("scene records after terminal");
        }
        if phases
            .iter()
            .any(|p| p.objects.iter().map(Object::count).sum::<usize>() > 512)
        {
            return Err("scene expanded construction budget");
        }
        Ok(Self { phases })
    }
    /// Named preparation visits the whole context but does not instantiate its
    /// future objects or recurse into immediate children before their creation.
    fn prepare(&self, seed: &mut u32) {
        for phase in &self.phases {
            for obj in &phase.objects {
                for _ in &obj.before_sounds {
                    random(seed);
                }
                for op in &obj.ops {
                    op.preparation(seed);
                }
            }
        }
    }
    pub fn start(&self, seed: &mut u32) -> Result<(Vec<Construction>, Cursor), &'static str> {
        self.prepare(seed);
        self.activate(seed)
    }
    pub fn requires_external_resume(&self) -> bool {
        self.phases.iter().any(|phase| phase.external_wait)
    }
    pub fn queue(self: &std::sync::Arc<Self>, seed: &mut u32) -> Queued {
        self.prepare(seed);
        Queued { plan: self.clone() }
    }
    /// Upper lifetime bounds in the same flattened construction order as start.
    /// None remains unbounded; callers must not invent a request-count reserve.
    pub fn lifetime_bounds(&self) -> Vec<Vec<Option<f32>>> {
        fn append(object: &Object, out: &mut Vec<Option<f32>>) {
            let mut bound = None;
            for op in &object.ops {
                match op {
                    Op::Lifetime(v) => bound = Some(v[0]),
                    Op::RandomLifetime(v) => bound = Some(v[0][0].max(v[0][1])),
                    _ => {}
                }
            }
            out.push(bound);
            for op in &object.ops {
                if let Op::Child {
                    program: Some(program),
                    ..
                } = op
                {
                    append(&program.phases[0].objects[0], out);
                }
            }
        }
        self.phases
            .iter()
            .map(|phase| {
                let mut out = Vec::new();
                for object in &phase.objects {
                    append(object, &mut out);
                }
                out
            })
            .collect()
    }
    pub(crate) fn activate(
        &self,
        seed: &mut u32,
    ) -> Result<(Vec<Construction>, Cursor), &'static str> {
        let first = self.phases.first().ok_or("empty scene plan")?;
        Ok((
            first.create(seed)?,
            Cursor {
                next: 1,
                remaining: first.wait,
                external_wait: first.external_wait,
            },
        ))
    }
}
impl Op {
    fn preparation(&self, seed: &mut u32) {
        let additional = match self {
            Self::RandomLifetime(_) | Self::RandomSpin(_) => 3,
            Self::RandomSizes(_) | Self::RandomCurvedSizes(_) => 2,
            Self::RandomGravity(_)
            | Self::RandomOffsetZ(_)
            | Self::RandomSize(_)
            | Self::SurfaceOffset(_) => 1,
            _ => 0,
        };
        for _ in 0..additional {
            random(seed);
        }
        if matches!(
            self,
            Self::Speed(_)
                | Self::Service(
                    ServiceKind::RangedEmission | ServiceKind::Source28 | ServiceKind::Source57,
                    _
                )
        ) {
            random(seed);
        }
    }
}
impl Phase {
    fn create(&self, seed: &mut u32) -> Result<Vec<Construction>, &'static str> {
        let mut out = Vec::new();
        for obj in &self.objects {
            obj.create(seed, None, &mut out)?;
        }
        Ok(out)
    }
}
impl Object {
    fn count(&self) -> usize {
        1 + self
            .ops
            .iter()
            .filter_map(|op| match op {
                Op::Child {
                    program: Some(program),
                    ..
                } => Some(program.phases[0].objects[0].count()),
                _ => None,
            })
            .sum::<usize>()
    }
    fn create(
        &self,
        seed: &mut u32,
        origin: Option<Origin>,
        out: &mut Vec<Construction>,
    ) -> Result<(), &'static str> {
        if out.len() >= 512 {
            return Err("scene construction budget");
        }
        let index = out.len();
        let direction_requested =
            !matches!(origin.as_ref().unwrap_or(&self.origin), Origin::Detached);
        let mut obj = Construction {
            class: self.class,
            origin: origin.unwrap_or_else(|| self.origin.clone()),
            hidden: false,
            size: None,
            size_opcode: None,
            lifetime: None,
            height: 0.,
            offset_z: 0.,
            spin: [0.; 3],
            direction_requested,
            speed: 0.,
            gravity: 1.,
            damping: [0.; 2],
            bounce: 0.,
            contact_requested: false,
            surface_offset: None,
            physics_enabled: false,
            services: Vec::new(),
            context_sounds: self
                .before_sounds
                .iter()
                .map(|sound| sound.select(seed))
                .collect(),
            object_services: ObjectServiceNeeds::default(),
        };
        out.push(obj.clone());
        for op in &self.ops {
            match op {
                Op::Render(command) => obj.object_services.render_commands.push(command.clone()),
                Op::RandomLifetime(ranges) => {
                    let v = ranges.map(|r| crate::timed_effect::sample(seed, r));
                    if v[0] <= 0. || v[1] < 0. || v[2] < 0. {
                        return Err("scene sampled duration outside positive profile");
                    }
                    obj.lifetime = Some(v);
                }
                Op::RandomSizes(ranges) => {
                    let v = ranges.map(|r| crate::timed_effect::sample(seed, r));
                    if v.iter().any(|v| *v <= 0.) {
                        return Err("scene sampled size outside positive profile");
                    }
                    obj.size = Some(v);
                    obj.size_opcode = Some(9);
                }
                Op::RandomCurvedSizes(ranges) => {
                    let v = ranges.map(|r| crate::timed_effect::sample(seed, r));
                    if v.iter().any(|v| *v <= 0.) || v[1] < v[0] {
                        return Err("scene sampled curve outside finite increasing-size profile");
                    }
                    obj.size = Some(v);
                    obj.size_opcode = Some(0x35);
                }
                Op::RandomSpin(ranges) => {
                    let [x, y, z] = ranges.map(|r| crate::timed_effect::sample(seed, r));
                    obj.spin = [y, x, z];
                }
                Op::RandomSize(range) => {
                    let size = crate::timed_effect::sample(seed, *range);
                    if size <= 0. {
                        return Err("scene sampled size outside positive profile");
                    }
                    obj.size = Some([size; 2]);
                    obj.size_opcode = Some(0x1c);
                }
                Op::Source16 => obj.object_services.source16 = true,
                Op::Source15 => obj.object_services.source15 = true,
                Op::Source47(value) => obj.object_services.source47 = Some(*value),
                Op::Source5a(binding) => obj.object_services.source5a = Some(binding.clone()),
                Op::RandomGravity(range) => {
                    obj.gravity = crate::timed_effect::sample(seed, *range);
                    obj.physics_enabled = true;
                }
                Op::RandomOffsetZ(range) => {
                    obj.offset_z = crate::timed_effect::sample(seed, *range)
                }
                Op::Size(op, size) => {
                    if size.iter().any(|v| *v <= 0.) {
                        return Err("scene size outside positive profile");
                    }
                    obj.size = Some(*size);
                    obj.size_opcode = Some(*op);
                }
                Op::Lifetime(v) => {
                    if v[0] <= 0. || v[1] < 0. || v[2] < 0. {
                        return Err("scene duration outside positive profile");
                    }
                    obj.lifetime = Some(*v);
                }
                Op::Height(v) => obj.height = *v,
                Op::Spin(v) => {
                    obj.spin = *v;
                }
                Op::Speed(range) => {
                    obj.speed = crate::timed_effect::sample(seed, *range);
                    obj.physics_enabled = true;
                }
                Op::Gravity(v) => {
                    obj.gravity = *v;
                    obj.physics_enabled = true;
                }
                Op::Hidden => obj.hidden = true,
                Op::LogContext => obj.object_services.logged_context = true,
                Op::Damping(v) => obj.damping = *v,
                Op::Bounce(v) => {
                    obj.bounce = *v;
                    obj.contact_requested = true;
                }
                Op::SurfaceOffset(range) => {
                    obj.surface_offset = Some(crate::timed_effect::sample(seed, *range))
                }
                Op::Retirement(binding) => obj.object_services.retirement = Some(binding.clone()),
                Op::Service(kind, r) => {
                    let sample = if *kind == ServiceKind::FixedEmission {
                        None
                    } else {
                        Some(random(seed))
                    };
                    obj.services.push(Service {
                        kind: *kind,
                        record: r.clone(),
                        random_sample: sample,
                    });
                }
                Op::Child {
                    reference,
                    own_reference,
                    inherit_context,
                    program,
                } => {
                    if let Some(program) = program {
                        program.prepare(seed);
                        program.phases[0].objects[0].create(
                            seed,
                            Some(Origin::Immediate {
                                parent: index,
                                reference: *reference,
                                own_reference: *own_reference,
                                inherit_context: *inherit_context,
                            }),
                            out,
                        )?;
                    }
                }
            }
        }
        out[index] = obj;
        Ok(())
    }
}
impl Cursor {
    pub fn advance(
        &mut self,
        plan: &Plan,
        dt: f32,
        seed: &mut u32,
    ) -> Result<Option<Vec<Construction>>, &'static str> {
        if !dt.is_finite() || dt < 0. {
            return Err("invalid scene delta");
        }
        let Some(remaining) = self.remaining else {
            return Ok(None);
        };
        let next_remaining = remaining - dt;
        if next_remaining > 0. {
            self.remaining = Some(next_remaining);
            return Ok(None);
        }
        let phase = plan.phases.get(self.next).ok_or("scene phase absent")?;
        let objects = phase.create(seed)?;
        self.next += 1;
        self.remaining = phase.wait;
        self.external_wait = phase.external_wait;
        Ok(Some(objects))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn absent_immediate_entries_preserve_parent_operations_and_random_order() {
        let mut baseline = object(1);
        baseline.push(0x18);
        baseline.extend(2_f32.to_le_bytes());
        baseline.extend(4_f32.to_le_bytes());
        baseline.extend([7, 2]);
        let baseline_image = image(&[("parent", baseline.clone())]);
        let plan = super::Plan::project(&baseline_image, "parent").unwrap();
        let mut expected_seed = 17;
        let (expected, _) = plan.start(&mut expected_seed).unwrap();
        for opcode in [0x10, 0x70] {
            let mut parent = object(1);
            parent.push(opcode);
            parent.extend(3_u32.to_le_bytes());
            parent.extend(b"absent-child\0");
            parent.extend(4_u32.to_le_bytes());
            parent.extend(&baseline[object(1).len()..]);
            let source = image(&[("parent", parent.clone())]);
            let actual = super::Plan::project(&source, "parent").unwrap();
            let mut seed = 17;
            let (objects, _) = actual.start(&mut seed).unwrap();
            assert_eq!(objects.len(), 1);
            assert_eq!(objects[0].class, 1);
            assert_eq!(
                objects[0].offset_z.to_bits(),
                expected[0].offset_z.to_bits()
            );
            assert_eq!(seed, expected_seed);
            assert_eq!(actual.lifetime_bounds(), plan.lifetime_bounds());
            assert!(super::Plan::project(&source, "absent-child").is_err());
            // An existing, unsupported child remains an error. Only a missing
            // entry follows the original no-created-context branch.
            let unsupported = image(&[("parent", parent), ("absent-child", vec![0x0c, 2])]);
            assert!(super::Plan::project(&unsupported, "parent").is_err());
        }
    }
    #[test]
    fn immediate_frame_inheritance_preserves_factory_mode_and_rng_order() {
        let child = vec![3, 2, 0, 0, 0, 6, 7, 2];
        let make = |opcode| {
            let mut parent = object(1);
            parent.push(opcode);
            parent.extend(3u32.to_le_bytes());
            parent.extend(b"child\0");
            parent.extend(4u32.to_le_bytes());
            parent.extend([7, 2]);
            image(&[("parent", parent), ("child", child.clone())])
        };
        let mut seeds = [17; 2];
        for (index, opcode) in [0x10, 0x70].into_iter().enumerate() {
            let plan = super::Plan::project(&make(opcode), "parent").unwrap();
            let (objects, cursor) = plan.start(&mut seeds[index]).unwrap();
            assert!(cursor.remaining.is_none());
            assert_eq!(objects.len(), 2);
            assert_eq!(
                objects[1].origin,
                super::Origin::Immediate {
                    parent: 0,
                    reference: 3,
                    own_reference: 4,
                    inherit_context: opcode == 0x70,
                }
            );
        }
        assert_eq!(seeds[0], seeds[1]);
    }
    fn image(programs: &[(&str, Vec<u8>)]) -> giants_formats::effect_image::Image {
        let size: usize = programs.iter().map(|(n, _)| n.len() + 5).sum();
        let mut table = Vec::new();
        let mut bodies: Vec<u8> = Vec::new();
        for (name, code) in programs {
            table.extend(name.as_bytes());
            table.push(0);
            table.extend(((size + bodies.len()) as u32).to_le_bytes());
            bodies.extend(code);
        }
        table.extend(bodies);
        let mut raw = 1u32.to_le_bytes().to_vec();
        raw.extend((table.len() as u32).to_le_bytes());
        raw.extend((programs.len() as u32).to_le_bytes());
        raw.extend(table);
        giants_formats::effect_image::parse(&raw).unwrap()
    }
    fn object(class: u32) -> Vec<u8> {
        let mut b = vec![3];
        b.extend(class.to_le_bytes());
        b.push(6);
        b
    }
    #[test]
    fn zero_bounce_request_is_distinct_from_absence_and_never_reports_a_hit() {
        let mut configured = object(913);
        configured.push(0x19);
        configured.extend(0f32.to_le_bytes());
        configured.extend([7, 2]);
        let mut absent = object(913);
        absent.extend([7, 2]);
        let source = image(&[("configured", configured), ("absent", absent)]);
        for (name, expected) in [("configured", true), ("absent", false)] {
            let plan = super::Plan::project(&source, name).unwrap();
            let (objects, _) = plan.start(&mut 1).unwrap();
            assert_eq!(objects[0].bounce, 0.);
            assert_eq!(objects[0].contact_requested, expected);
            assert!(!objects[0].physics_enabled);
        }
    }
    #[test]
    fn renderer_setup_keeps_source_order_and_needs_without_consuming_random() {
        let mut source = object(900);
        let expected = [
            super::RenderCommand::UvScroll([0.25, -0.5]),
            super::RenderCommand::VerticalFade([1., 0., 0., 0., 0.7]),
            super::RenderCommand::UvScroll([-0.75, 0.5]),
            super::RenderCommand::VerticalFade([0., 0., 1., 0., 0.2]),
        ];
        for command in &expected {
            match command {
                super::RenderCommand::UvScroll(values) => {
                    source.push(0x40);
                    for value in values {
                        source.extend(value.to_le_bytes());
                    }
                }
                super::RenderCommand::VerticalFade(values) => {
                    source.push(0x46);
                    for value in values {
                        source.extend(value.to_le_bytes());
                    }
                }
            }
        }
        source.extend([7, 2]);
        let plan =
            super::Plan::project(&image(&[("render_setup", source)]), "render_setup").unwrap();
        let mut random = 17;
        let (objects, _) = plan.activate(&mut random).unwrap();
        assert_eq!(random, 17);
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].object_services.render_commands, expected);
        assert_eq!(objects[0].object_services.pending_count(), 4);
    }
    #[test]
    fn guard_control_requests_stay_pending_including_the_zero_attribute() {
        let mut bytes = object(913);
        bytes.extend([0x15, 0x47]);
        bytes.extend(0u32.to_le_bytes());
        bytes.extend([7, 2]);
        let plan = super::Plan::project(&image(&[("guard", bytes)]), "guard").unwrap();
        let mut seed = 17;
        let (objects, _) = plan.start(&mut seed).unwrap();
        assert_eq!(seed, 17);
        let needs = &objects[0].object_services;
        assert!(needs.source15);
        assert_eq!(needs.source47, Some(0));
        assert_eq!(needs.pending_count(), 2);
    }
    #[test]
    fn external_hold_keeps_hidden_logged_object_and_never_expires_from_elapsed_time() {
        let mut b = object(887);
        b.extend([0x21, 0x49, 7, 0x4a, 2]);
        let plan = super::Plan::project(&image(&[("held", b)]), "held").unwrap();
        let mut seed = 1;
        let (objects, mut cursor) = plan.start(&mut seed).unwrap();
        assert!(plan.requires_external_resume() && cursor.external_wait);
        assert!(objects[0].hidden && objects[0].object_services.logged_context);
        let before = seed;
        assert!(cursor.advance(&plan, 1000., &mut seed).unwrap().is_none());
        assert!(cursor.external_wait);
        assert_eq!(seed, before);
    }
    #[test]
    fn registered_object_multiplicity_and_unexecuted_resume_continuation_are_refused() {
        let mut repeated = object(887);
        repeated.extend([0x49, 0x49, 7, 2]);
        assert!(super::Plan::project(&image(&[("repeat", repeated)]), "repeat").is_err());
        let mut continued = object(887);
        continued.extend([0x49, 7, 0x4a]);
        continued.extend(object(827));
        continued.extend([7, 2]);
        assert!(super::Plan::project(&image(&[("continue", continued)]), "continue").is_err());
    }
    #[test]
    fn repeated_or_followed_surface_operations_are_not_collapsed_into_one_callback() {
        let mut b = object(938);
        for _ in 0..2 {
            b.push(0x79);
            b.extend(0.1f32.to_le_bytes());
            b.extend(0.3f32.to_le_bytes());
        }
        b.extend([7, 2]);
        assert!(super::Plan::project(&image(&[("surface", b)]), "surface").is_err());
    }
    #[test]
    fn debris_coefficients_and_retirement_remain_configuration_without_child_creation() {
        let mut body = object(900);
        body.push(0x13);
        for value in [0.25f32, 0.5] {
            body.extend(value.to_le_bytes());
        }
        body.push(0x19);
        body.extend(0.75f32.to_le_bytes());
        body.push(0xd);
        body.extend(b"retired\0");
        body.extend([7, 2]);
        let source = image(&[("debris", body.clone()), ("retired", vec![2])]);
        let plan = super::Plan::project(&source, "debris").unwrap();
        let mut seed = 42;
        let (objects, _) = plan.start(&mut seed).unwrap();
        assert_eq!(seed, 42);
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].damping, [0.25, 0.5]);
        assert_eq!(objects[0].bounce, 0.75);
        assert_eq!(objects[0].object_services.pending_count(), 1);
        let binding = objects[0].object_services.retirement.as_ref().unwrap();
        assert_eq!(binding.effect, "retired");
        assert_eq!(
            binding.program_offset,
            source.find("retired").unwrap().1.offset
        );
        assert!(super::Plan::project(&image(&[("debris", body)]), "debris").is_err());
    }
    #[test]
    fn queued_named_preparation_and_activation_preserve_rng_and_bounds() {
        let mut b = object(900);
        b.push(5);
        for value in [1f32, 4., 0., 0., 0.5, 1.] {
            b.extend(value.to_le_bytes());
        }
        b.extend([7, 2]);
        let plan =
            std::sync::Arc::new(super::Plan::project(&image(&[("child", b)]), "child").unwrap());
        let mut combined_seed = 42;
        let (combined, _) = plan.start(&mut combined_seed).unwrap();
        let mut queued_seed = 42;
        let queued = plan.queue(&mut queued_seed);
        assert_ne!(queued_seed, combined_seed);
        let (_, activated, _) = queued.activate(&mut queued_seed).unwrap();
        assert_eq!(queued_seed, combined_seed);
        assert_eq!(activated[0].lifetime, combined[0].lifetime);
        assert_eq!(plan.lifetime_bounds(), vec![vec![Some(4.)]]);
    }
    #[test]
    fn deferred_binding_resolves_without_constructing_a_child_and_size_uses_one_sample() {
        let mut parent = object(916);
        parent.push(0x1c);
        parent.extend(1f32.to_le_bytes());
        parent.extend(2f32.to_le_bytes());
        parent.extend([0x16, 0x5a]);
        parent.extend(b"drop\0");
        parent.extend(20u32.to_le_bytes());
        parent.extend([7, 2]);
        let mut child = object(938);
        child.extend([0x79]);
        child.extend(0.1f32.to_le_bytes());
        child.extend(0.3f32.to_le_bytes());
        child.extend([7, 2]);
        let image = image(&[("blood", parent.clone()), ("drop", child)]);
        let plan = super::Plan::project(&image, "blood").unwrap();
        let mut seed = 1;
        let (objects, _) = plan.start(&mut seed).unwrap();
        assert_eq!(objects.len(), 1);
        let object = &objects[0];
        let mut expected_seed = 1;
        super::random(&mut expected_seed); // Named preparation.
        let expected = crate::timed_effect::sample(&mut expected_seed, [1., 2.]);
        assert_eq!(seed, expected_seed);
        assert_eq!(object.size, Some([expected; 2]));
        assert_eq!(object.size_opcode, Some(0x1c));
        assert_eq!(object.object_services.pending_count(), 2);
        let binding = object.object_services.source5a.as_ref().unwrap();
        assert_eq!(binding.effect, "drop");
        assert_eq!(binding.parameter, 20);
        assert_eq!(binding.program_offset, image.find("drop").unwrap().1.offset);
        let drop_plan = super::Plan::project(&image, "drop").unwrap();
        let (drops, _) = drop_plan.start(&mut seed).unwrap();
        assert!(drops[0].surface_offset.is_some());
        assert!(super::Plan::project(&self::image(&[("blood", parent)]), "blood").is_err());
    }
    #[test]
    fn context_sound_selection_waits_for_activation_and_keeps_future_skip_order() {
        let mut first = vec![0x2b];
        first.extend(42u32.to_le_bytes());
        first.extend(42u32.to_le_bytes());
        first.extend(object(900));
        first.extend([7, 0x0b]);
        first.extend(0.1f32.to_le_bytes());
        first.push(3);
        first.extend(901u32.to_le_bytes());
        first.push(0x2b);
        first.extend(10u32.to_le_bytes());
        first.extend(20u32.to_le_bytes());
        first.extend([6, 7, 2]);
        let plan = std::sync::Arc::new(
            super::Plan::project(&image(&[("sound", first)]), "sound").unwrap(),
        );
        let mut seed = 1;
        let token = plan.queue(&mut seed);
        let after_skip = seed;
        let (_, objects, mut cursor) = token.activate(&mut seed).unwrap();
        assert_ne!(seed, after_skip);
        assert_eq!(objects[0].context_sounds[0].id, 42);
        assert!(cursor.advance(&plan, 0., &mut seed).unwrap().is_none());
        let next = cursor.advance(&plan, 0.1, &mut seed).unwrap().unwrap();
        assert_eq!(next[0].context_sounds[0].id, 18);
        let mut trailing = object(900);
        trailing.extend([7, 0x2b]);
        trailing.extend(42u32.to_le_bytes());
        trailing.extend(42u32.to_le_bytes());
        trailing.push(2);
        assert!(super::Plan::project(&image(&[("trailing", trailing)]), "trailing").is_err());
    }
    #[test]
    fn sampled_displacement_is_not_the_persistent_height_and_keeps_skip_rng() {
        let mut b = object(900);
        b.push(0x18);
        b.extend(2f32.to_le_bytes());
        b.extend(2f32.to_le_bytes());
        b.extend([7, 2]);
        let image = image(&[("offset", b)]);
        let plan = super::Plan::project(&image, "offset").unwrap();
        let mut seed = 1;
        let (objects, _) = plan.start(&mut seed).unwrap();
        assert_eq!(objects[0].offset_z, 2.);
        assert_eq!(objects[0].height, 0.);
        let mut expected = 1;
        super::random(&mut expected);
        super::random(&mut expected);
        assert_eq!(seed, expected);
        for extra in [0x18, 0x17] {
            let mut b = object(900);
            b.push(0x18);
            b.extend(2f32.to_le_bytes());
            b.extend(2f32.to_le_bytes());
            b.push(extra);
            b.extend(3f32.to_le_bytes());
            if extra == 0x18 {
                b.extend(4f32.to_le_bytes());
            }
            b.extend([7, 2]);
            assert!(super::Plan::project(&self::image(&[("offset", b)]), "offset").is_err());
        }
        let mut parent = object(901);
        parent.push(0x10);
        parent.extend(0u32.to_le_bytes());
        parent.extend(b"offset\0");
        parent.extend(0u32.to_le_bytes());
        parent.extend([7, 2]);
        let mut child = object(902);
        child.push(0x18);
        child.extend(2f32.to_le_bytes());
        child.extend(2f32.to_le_bytes());
        child.extend([7, 2]);
        assert!(
            super::Plan::project(
                &self::image(&[("parent", parent), ("offset", child)]),
                "parent"
            )
            .is_err()
        );
    }
    #[test]
    fn service_factory_pairs_and_context_mode_are_not_conflated() {
        let mut bytes = vec![0x57];
        bytes.extend(b"q\0");
        bytes.extend(0.1f32.to_le_bytes());
        bytes.extend(0.2f32.to_le_bytes());
        for word in [11u32, 22, 33, 44] {
            bytes.extend(word.to_le_bytes());
        }
        let s = super::Service {
            kind: super::ServiceKind::Source57,
            record: giants_formats::effect_program::Record {
                offset: 0,
                opcode: 0x57,
                bytes,
            },
            random_sample: Some(0),
        };
        let c = s.emission_configuration().unwrap();
        assert_eq!(c.effect, "q");
        assert_eq!(c.interval, 0.1);
        assert_eq!(c.selectors, [[33, 44], [11, 22]]);
        assert!(c.source_flag8);
        assert!(!c.source_flag9);
        let mut fixed = vec![0x1d];
        fixed.extend(b"q\0");
        fixed.extend(0.1f32.to_le_bytes());
        let mut s = super::Service {
            kind: super::ServiceKind::FixedEmission,
            record: giants_formats::effect_program::Record {
                offset: 0,
                opcode: 0x1d,
                bytes: fixed,
            },
            random_sample: None,
        };
        let c = s.emission_configuration().unwrap();
        assert_eq!(c.selectors, [[u32::MAX; 2]; 2]);
        assert!(!c.source_flag8);
        assert!(!c.source_flag9);
        s.random_sample = Some(0);
        assert!(s.emission_configuration().is_err());
    }
    #[test]
    fn invalid_service_kind_range_and_missing_samples_stay_refused() {
        let mut bytes = vec![0x1e];
        bytes.extend(b"q\0");
        bytes.extend(0.1f32.to_le_bytes());
        bytes.extend(0.2f32.to_le_bytes());
        let mut s = super::Service {
            kind: super::ServiceKind::RangedEmission,
            record: giants_formats::effect_program::Record {
                offset: 0,
                opcode: 0x1e,
                bytes,
            },
            random_sample: None,
        };
        assert!(s.emission_configuration().is_err());
        s.random_sample = Some(32768);
        assert!(s.emission_configuration().is_err());
        s.random_sample = Some(0);
        s.record.opcode = 0x57;
        assert!(s.emission_configuration().is_err());
        s.record.opcode = 0x1e;
        s.record.bytes[3..7].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(s.emission_configuration().is_err());
        s.record.bytes.clear();
        assert!(s.emission_configuration().is_err());
    }
    #[test]
    fn missing_duration_stays_unconfigured_across_waits() {
        let mut b = object(1);
        b.push(7);
        b.push(0x0b);
        b.extend(1f32.to_le_bytes());
        b.extend(object(2));
        b.extend([7, 2]);
        let p = super::Plan::project(&image(&[("p", b)]), "p").unwrap();
        let mut seed = 9;
        let (first, mut cursor) = p.start(&mut seed).unwrap();
        assert!(first[0].lifetime.is_none());
        assert!(cursor.advance(&p, 0.5, &mut seed).unwrap().is_none());
        let last = cursor.advance(&p, 0.5, &mut seed).unwrap().unwrap();
        assert_eq!(last[0].class, 2);
        assert!(last[0].lifetime.is_none());
        assert!(cursor.remaining.is_none());
        assert_eq!(seed, 9);
    }
    #[test]
    fn future_service_preparation_does_not_create_future_objects_or_children() {
        let mut child = object(2);
        child.push(0x12);
        child.extend(2f32.to_le_bytes());
        child.extend(3f32.to_le_bytes());
        child.extend([7, 2]);
        let mut b = object(1);
        b.push(7);
        b.push(0xb);
        b.extend(1f32.to_le_bytes());
        b.extend(object(3));
        b.push(0x10);
        b.extend(7u32.to_le_bytes());
        b.extend(b"child\0");
        b.extend(0u32.to_le_bytes());
        b.push(0x28);
        b.extend(b"frames\0");
        for _ in 0..4 {
            b.extend(0f32.to_le_bytes());
        }
        b.extend([7, 2]);
        let p = super::Plan::project(&image(&[("p", b), ("child", child)]), "p").unwrap();
        let mut seed = 1;
        let (first, mut cursor) = p.start(&mut seed).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(seed, 2745024);
        let snapshot = (cursor.next, cursor.remaining, seed);
        assert!(cursor.advance(&p, f32::NAN, &mut seed).is_err());
        assert_eq!(snapshot, (cursor.next, cursor.remaining, seed));
        let last = cursor.advance(&p, 1., &mut seed).unwrap().unwrap();
        assert_eq!(last.len(), 2);
        assert_eq!(last[0].services.len(), 1);
        assert_eq!(
            last[1].origin,
            super::Origin::Immediate {
                parent: 0,
                reference: 7,
                own_reference: 0,
                inherit_context: false,
            }
        );
        let mut expected = 1u32;
        for _ in 0..4 {
            expected = expected.wrapping_mul(214013).wrapping_add(2531011);
        }
        assert_eq!(seed, expected);
    }
    #[test]
    fn cycles_unknown_commands_and_invalid_future_fields_fail_before_rng_changes() {
        let mut cycle = object(1);
        cycle.push(0x10);
        cycle.extend(0u32.to_le_bytes());
        cycle.extend(b"p\0");
        cycle.extend(0u32.to_le_bytes());
        cycle.extend([7, 2]);
        assert!(super::Plan::project(&image(&[("p", cycle)]), "p").is_err());
        for op in [0x1b, 4] {
            let mut b = object(1);
            b.push(op);
            b.extend((-1f32).to_le_bytes());
            if op == 4 {
                b.extend([0; 8]);
            }
            b.extend([7, 2]);
            assert!(super::Plan::project(&image(&[("p", b)]), "p").is_err());
        }
        let mut b = object(1);
        b.extend([0x48, 7, 2]);
        assert!(super::Plan::project(&image(&[("p", b)]), "p").is_err());
    }
}
