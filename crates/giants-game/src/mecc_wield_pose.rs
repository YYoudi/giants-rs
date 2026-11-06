//! Owned normal-hand ANM/reference/world-pose service for inspection.
//! The composing caller supplies phase order/time. This does not certify full
//! child history/finalization, global world order or the subjective-view parent.
use crate::{
    procedural_aim::Matrix,
    resident_states::{Id, Session},
};

#[derive(Clone, Copy)]
pub struct Pose {
    pub child: Id,
    pub binding: crate::mecc_wield::Binding,
    pub world: Matrix,
    channels: crate::animation_channels::Channels,
}
impl Pose {
    /// Read the frozen world cache with the same current child references.
    /// Parent motion does not silently recompute this prior cache.
    pub fn point(&self, session: &Session, reference: i32) -> Result<[f32; 3], String> {
        validate_child(session, self.child)?;
        let actor = session.get(self.child).unwrap();
        if session.wield_binding(self.child) != Some(self.binding)
            || actor.channels != Some(self.channels)
        {
            return Err("held pose cache binding/channels are stale".into());
        }
        let fields = actor.construction_fields.as_ref().unwrap();
        let references = session
            .current_reference_matrices(self.child)?
            .iter()
            .map(|m| m.values)
            .collect::<Vec<_>>();
        crate::object_reference::point(
            &crate::object_reference::Object {
                position: fields.position,
                angles: fields.angles,
                scale: fields.scale,
                flags: 0x20,
                cached: self.world.values,
                references: &references,
            },
            reference,
        )
        .map_err(str::to_owned)
    }
}

fn validate_child(session: &Session, child: Id) -> Result<crate::mecc_wield::Binding, String> {
    let actor = session
        .get(child)
        .ok_or("held pose actor absent or foreign")?;
    let binding = session
        .wield_binding(child)
        .ok_or("held pose binding absent")?;
    let fields = actor
        .construction_fields
        .as_ref()
        .ok_or("held pose fields absent")?;
    if actor.class != 21
        || actor.seed.storage.state() != 0
        || binding.context != 0
        || !matches!(binding.reference, 0 | 1)
        || fields.object_flags & !(0x20 | 0x40 | 0x800 | 0x40000000) != 0x20400000
        || fields.response_flags & 0x02009000 != 0
        || actor.channels != Some(fields.channels)
        || session.wield_failure(binding.parent).is_some()
        || !session
            .wield_list()?
            .iter()
            .any(|entry| entry.id == child && entry.parent == Some(binding.parent))
    {
        return Err("held pose requires the acquired live normal pistol binding".into());
    }
    Ok(binding)
}

pub fn sample(session: &Session, child: Id) -> Result<Pose, String> {
    let binding = validate_child(session, child)?;
    let actor = session.get(child).unwrap();
    let fields = actor.construction_fields.as_ref().unwrap();
    let parent = session
        .get(binding.parent)
        .ok_or("held pose parent absent")?;
    if parent.class != 2
        || session
            .mecc_physical_cycle_failure(binding.parent)
            .is_some()
    {
        return Err("held pose parent outside unfailed Mecc profile".into());
    }
    let parent_fields = parent
        .construction_fields
        .as_ref()
        .ok_or("held pose parent fields absent")?;
    let cached = session.actor_current_affine(binding.parent)?;
    let parent_refs = session.current_reference_matrices(binding.parent)?;
    let parent_ref = if parent_refs.is_empty() {
        Matrix::identity()
    } else {
        *parent_refs
            .get(binding.reference as usize)
            .ok_or("held parent reference absent")?
    };
    let child_ref = *session
        .current_reference_matrices(child)?
        .first()
        .ok_or("held handle reference absent")?;
    let anchor = Matrix {
        flags: cached.flags,
        values: cached.matrix,
    }
    .mul(parent_ref)?;
    let mut world = crate::effect_attachment::normal_matrix(
        anchor,
        fields.scale,
        parent_fields.scale,
        child_ref,
    )?;
    world.values[11] =
        (f64::from(world.values[11]) + f64::from(f32::from_bits(fields.configuration[17]))) as f32;
    if !world.values.iter().all(|v| v.is_finite()) {
        return Err("held world pose is nonfinite".into());
    }
    Ok(Pose {
        child,
        binding,
        world,
        channels: actor.channels.ok_or("held channels absent")?,
    })
}

pub fn advance_animation(
    session: &mut Session,
    child: Id,
    seconds: f32,
    epoch: u32,
) -> Result<(), String> {
    if epoch == 0 || !seconds.is_finite() || seconds < 0. {
        return Err("held animation requires nonzero epoch and finite nonnegative time".into());
    }
    validate_child(session, child)?;
    let spec = session
        .specification(child)
        .ok_or("held specification absent")?;
    if !spec.needs.is_empty()
        || spec
            .metadata
            .events
            .iter()
            .any(|track| !track.events.is_empty())
    {
        return Err(
            "held animation requires complete resources and no unprovided event services".into(),
        );
    }
    let mut durations = vec![None; 256];
    for (&slot, descriptor) in &spec.animations {
        durations[slot as usize] = Some(descriptor.template.duration);
    }
    let tick = session.advance_channels(
        child,
        &durations,
        [0.; 3],
        seconds,
        crate::arithmetic::Precision::Double53,
    )?;
    if tick.missing != 0 {
        return Err("held animation descriptor absent".into());
    }
    session.mirror_wield_child_channels(child)?;
    session.update_current_references_at(child, epoch)?;
    Ok(())
}

#[derive(Clone)]
pub struct Frame {
    pub number: u64,
    pub epoch: u32,
    pub pose: Pose,
    pub muzzle: [f32; 3],
    pub positions: Vec<[f32; 3]>,
}
pub struct Producer {
    owner: Id,
    number: u64,
    failed: bool,
    frames: Vec<Frame>,
}
impl Producer {
    pub fn new(owner: Id) -> Self {
        Self {
            owner,
            number: 0,
            failed: false,
            frames: Vec::new(),
        }
    }
    pub fn advance(
        &mut self,
        session: &mut Session,
        number: u64,
        seconds: f32,
        epoch: u32,
    ) -> Result<Option<Frame>, String> {
        if self.failed
            || self.number.checked_add(1) != Some(number)
            || epoch == 0
            || !seconds.is_finite()
            || seconds < 0.
            || session.actor_world_tick(self.owner)?.phase()
                != crate::actor_world_tick::Phase::Finished
            || session.mecc_physical_cycle(self.owner)?.number != number
            || session.actor_world_flags(self.owner)? & 0x40 == 0
            || session.mecc_physical_cycle_failure(self.owner).is_some()
        {
            return Err("held pose producer requires the next completed parent frame".into());
        }
        self.number = number;
        let result = (|| {
            let children = session
                .wield_list()?
                .into_iter()
                .filter(|child| child.parent == Some(self.owner) && child.flags & 0x8000 == 0)
                .collect::<Vec<_>>();
            let mut frames = Vec::new();
            for child in children {
                advance_animation(session, child.id, seconds, epoch)?;
                let pose = sample(session, child.id)?;
                let muzzle = pose.point(session, 1)?;
                session.prepare_actor_positions(child.id, 0, epoch, false)?;
                let positions = session
                    .actor_model_positions(child.id, 0)?
                    .ok_or("held child vertices absent")?
                    .to_vec();
                frames.push(Frame {
                    number,
                    epoch,
                    pose,
                    muzzle,
                    positions,
                });
            }
            let right = frames
                .iter()
                .find(|frame| frame.pose.binding.reference == 1)
                .cloned();
            self.frames = frames;
            Ok(right)
        })();
        if result.is_err() {
            self.failed = true;
            self.frames.clear();
        }
        result
    }
    /// For a future firing transaction, only the prior completed world-view
    /// cache is admissible. Other weapons/views still need their own providers.
    pub fn prior_world_point(
        &self,
        session: &Session,
        next: u64,
        definition: &crate::projectile_definition::Definition,
    ) -> Result<(crate::mecc_fire_lock::Binding<Id>, [f32; 3]), String> {
        if self.failed
            || self.number == 0
            || self.number.checked_add(1) != Some(next)
            || definition.id != 17
        {
            return Err("normal pistol needs its prior unfailed owned pose cache".into());
        }
        let binding = session.mecc_fire_lock(self.owner, 17, definition)?;
        let frame = self
            .frames
            .iter()
            .find(|frame| frame.pose.child == binding.parent)
            .ok_or("selected gun has no produced pose cache")?;
        Ok((binding, frame.pose.point(session, binding.reference)?))
    }
    /// Post-world firing consumes this frame's completed held cache. Updating
    /// subjective source references must not recompute its frozen world pose.
    pub fn current_world_point(
        &self,
        session: &Session,
        number: u64,
        definition: &crate::projectile_definition::Definition,
    ) -> Result<(crate::mecc_fire_lock::Binding<Id>, [f32; 3]), String> {
        self.require_current(session, self.owner, number)?;
        if definition.id != 17 {
            return Err("current held muzzle outside normal pistol profile".into());
        }
        let binding = session.mecc_fire_lock(self.owner, 17, definition)?;
        let frame = self
            .frames
            .iter()
            .find(|f| f.pose.child == binding.parent)
            .ok_or("selected gun has no current produced pose cache")?;
        Ok((binding, frame.pose.point(session, binding.reference)?))
    }
    pub(crate) fn require_current(
        &self,
        session: &Session,
        owner: Id,
        number: u64,
    ) -> Result<(), String> {
        if self.failed
            || self.owner != owner
            || self.number == 0
            || self.number != number
            || session.actor_world_tick(owner)?.phase() != crate::actor_world_tick::Phase::Finished
            || session.mecc_physical_cycle(owner)?.number != number
            || session.mecc_physical_cycle_failure(owner).is_some()
        {
            return Err("subjective post requires this source's completed held pass".into());
        }
        Ok(())
    }
}
