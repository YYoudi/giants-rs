//! Ordinary partial-controller presentation. Session retains constructor identities;
//! these render clocks do not acknowledge an owned primary or native world tick.
use giants_game::{
    mecc_jet_presentation::Frame,
    procedural_aim::Matrix,
    resident_states::{Id, Session},
    specification::Prepared,
};
use std::sync::Arc;

pub(crate) struct Parent<'a> {
    pub animation: &'a giants_formats::anm::Animation,
    pub seconds: f32,
    pub world: [f32; 16],
    pub scale: f32,
    pub flags: u32,
    pub boost: bool,
}
struct Child {
    id: Id,
    spec: Arc<Prepared>,
}
pub(crate) struct Presentation {
    parent: Id,
    parent_spec: Arc<Prepared>,
    children: Vec<Child>,
    last_step: u64,
    failed: bool,
    stamp: giants_game::animation_epoch::Stamp,
}
fn reference(
    spec: &Prepared,
    animation: &giants_formats::anm::Animation,
    slot: i32,
    tick: i32,
) -> Result<Matrix, String> {
    if (slot as u8 as i8) < 0 {
        return Ok(Matrix::identity());
    }
    let raw = spec
        .metadata
        .references
        .bind(animation)
        .map_err(|e| e.to_string())?
        .evaluate(slot as usize, tick)
        .map_err(|e| e.to_string())?
        .initial_object_matrix()
        .map_err(|e| e.to_string())?;
    giants_game::animation_links::apply(slot as u8, raw, &spec.corrections)
        .map_err(|e| e.to_string())
}
impl Presentation {
    pub(crate) fn new(session: &Session, parent: Id) -> Result<Self, String> {
        let initialized = session
            .mecc_special(parent)
            .ok_or("normal jet constructor absent")?;
        if !initialized.player_fields {
            return Err("normal jet player fields absent".into());
        }
        let parent_spec = Arc::clone(
            session
                .specification(parent)
                .ok_or("normal jet parent spec absent")?,
        );
        let mut children = Vec::new();
        for (id, class) in initialized.children.into_iter().zip([84, 85, 781]) {
            let actor = session.get(id).ok_or("normal jet child absent")?;
            let binding = session
                .mecc_special_child(id)
                .ok_or("normal jet binding absent")?;
            if actor.class != class
                || actor.seed.storage.state() != 0
                || binding.parent != parent
                || binding.binding_word != 2
                || binding.context != 0
            {
                return Err("normal jet constructor outside acquired attachment profile".into());
            }
            actor
                .construction_fields
                .as_ref()
                .ok_or("normal jet fields absent")?;
            let spec = Arc::clone(
                session
                    .specification(id)
                    .ok_or("normal jet child spec absent")?,
            );
            children.push(Child { id, spec });
        }
        Ok(Self {
            parent,
            parent_spec,
            children,
            last_step: 0,
            failed: false,
            stamp: giants_game::animation_epoch::Stamp::default(),
        })
    }
    pub(crate) fn advance(
        &mut self,
        session: &mut Session,
        step: u64,
        seconds: f32,
        parent: Parent<'_>,
    ) -> Result<Vec<Frame>, String> {
        if self.failed {
            return Err("normal jet presentation quarantined".into());
        }
        let result = self.advance_inner(session, step, seconds, parent);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn advance_inner(
        &mut self,
        session: &mut Session,
        step: u64,
        seconds: f32,
        parent: Parent<'_>,
    ) -> Result<Vec<Frame>, String> {
        let motion = session.controlled_mecc_motion(self.parent, step)?;
        let world = giants_game::object_transform::euler(
            motion.frame.position,
            motion.frame.angles,
            motion.frame.scale,
        )?;
        if self.last_step.checked_add(1) != Some(step)
            || !seconds.is_finite()
            || seconds < 0.
            || motion.frame.scale.to_bits() != parent.scale.to_bits()
            || world.map(f32::to_bits) != parent.world.map(f32::to_bits)
            || session.mecc_special(self.parent).is_none_or(|s| {
                s.children.as_slice() != self.children.iter().map(|c| c.id).collect::<Vec<_>>()
            })
        {
            return Err("normal jet requires current ordinary motion and constructor".into());
        }
        self.last_step = step;
        let tick =
            giants_game::model_pose::seconds_tick(parent.animation.timeline, parent.seconds)?;
        let parent_reference = reference(&self.parent_spec, parent.animation, 2, tick)?;
        let parent_world = Matrix {
            flags: 1
                | 16
                | if parent.scale.to_bits() != 1f32.to_bits() {
                    2
                } else {
                    0
                },
            values: parent.world,
        };
        let mut frames = Vec::new();
        for child in &self.children {
            session.prepare_passive_world_history(child.id, step - 1)?;
        }
        for child in &mut self.children {
            let callback = session.advance_mecc_child_after_history(
                child.id,
                giants_game::passive_frame::Frame {
                    number: step - 1,
                    global_seconds: seconds,
                    areas: &[],
                    controlled: Some(self.parent),
                },
                &mut self.stamp,
            )?;
            if callback.outcome != giants_game::actor_world_tick::Outcome::Updated
                || !callback.born.is_empty()
            {
                return Err("normal jet child callback did not complete".into());
            }
            let epoch = session
                .get(child.id)
                .ok_or("normal jet child retired")?
                .mecc_animation_stamp;
            session.prepare_actor_positions(child.id, 0, epoch, false)?;
            let positions = session
                .actor_model_positions(child.id, 0)?
                .ok_or("normal jet current position buffer absent")?
                .to_vec();
            let actor = session.get(child.id).ok_or("normal jet child retired")?;
            let binding = session
                .mecc_special_child(child.id)
                .ok_or("normal jet binding retired")?;
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("normal jet fields absent")?;
            if binding.parent != self.parent
                || binding.binding_word != 2
                || binding.context != 0
                || actor.class != child.spec.class
                || actor.seed.storage.state() != 0
                || actor.channels != Some(fields.channels)
                || session
                    .specification(child.id)
                    .is_none_or(|s| !Arc::ptr_eq(s, &child.spec))
            {
                return Err("normal jet child changed outside its presentation owner".into());
            }
            let channels = actor.channels.ok_or("normal jet child channels absent")?;
            if channels[1..].iter().any(|c| c[0] != 254) {
                return Err("normal jet needs acquired single-channel animation".into());
            }
            let descriptor = child
                .spec
                .animations
                .get(&(channels[0][0] as u8))
                .ok_or("normal jet clip absent")?;
            let tick = giants_game::model_pose::seconds_tick(
                descriptor.animation.timeline,
                f32::from_bits(channels[0][1]),
            )?;
            let mut world = giants_game::effect_attachment::initial_child_matrix(
                parent_world,
                parent_reference,
                reference(
                    &child.spec,
                    &descriptor.animation,
                    binding.reference_word,
                    tick,
                )?,
                fields.scale,
                parent.scale,
            )?;
            world.values[11] = (f64::from(world.values[11])
                + f64::from(f32::from_bits(fields.configuration[17])))
                as f32;
            if !world.values.iter().all(|v| v.is_finite()) {
                return Err("normal jet matrix nonfinite".into());
            }
            frames.push(Frame {
                pose: giants_game::mecc_special::ChildPose {
                    id: child.id,
                    class: actor.class,
                    hidden: giants_game::mecc_special::child_hidden(
                        actor.class,
                        parent.flags,
                        parent.boost,
                    ),
                    world,
                },
                positions,
                personality: fields.configuration[14],
                scale: fields.scale,
                angle: fields.angles[0],
            });
        }
        Ok(frames)
    }
}
