//! Ordinary partial-controller presentation. Session retains constructor identities;
//! these render clocks do not acknowledge an owned primary or native world tick.
use giants_game::{
    animation_channels::{self, Channels},
    arithmetic::Precision,
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
    fields: giants_game::actor_fields::Fields,
    channels: Channels,
    spec: Arc<Prepared>,
    durations: Vec<Option<f32>>,
}
pub(crate) struct Presentation {
    parent: Id,
    parent_spec: Arc<Prepared>,
    children: Vec<Child>,
    last_step: u64,
    failed: bool,
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
            let fields = actor
                .construction_fields
                .as_ref()
                .ok_or("normal jet fields absent")?
                .clone();
            let spec = Arc::clone(
                session
                    .specification(id)
                    .ok_or("normal jet child spec absent")?,
            );
            let durations = (0..256)
                .map(|slot| {
                    spec.animations
                        .get(&(slot as u8))
                        .map(|d| d.template.duration)
                })
                .collect();
            children.push(Child {
                id,
                channels: fields.channels,
                fields,
                spec,
                durations,
            });
        }
        Ok(Self {
            parent,
            parent_spec,
            children,
            last_step: 0,
            failed: false,
        })
    }
    pub(crate) fn advance(
        &mut self,
        session: &Session,
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
        session: &Session,
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
        for child in &mut self.children {
            let actor = session.get(child.id).ok_or("normal jet child retired")?;
            let binding = session
                .mecc_special_child(child.id)
                .ok_or("normal jet binding retired")?;
            if binding.parent != self.parent
                || binding.binding_word != 2
                || binding.context != 0
                || actor.class != child.fields.class
                || actor.seed.storage.state() != 0
                || actor.channels != Some(child.fields.channels)
                || actor.construction_fields.as_ref().is_none_or(|f| {
                    f.channels != child.fields.channels
                        || f.scale.to_bits() != child.fields.scale.to_bits()
                        || f.configuration != child.fields.configuration
                        || f.object_flags != child.fields.object_flags
                        || f.angles.map(f32::to_bits) != child.fields.angles.map(f32::to_bits)
                })
            {
                return Err("normal jet child changed outside its presentation owner".into());
            }
            let advanced = animation_channels::advance(
                &mut child.channels,
                &child.durations,
                [0.; 3],
                seconds,
                Precision::Double53,
            )?;
            if advanced.missing != 0 || child.channels[1..].iter().any(|c| c[0] != 254) {
                return Err("normal jet needs acquired single-channel animation".into());
            }
            let descriptor = child
                .spec
                .animations
                .get(&(child.channels[0][0] as u8))
                .ok_or("normal jet clip absent")?;
            let tick = giants_game::model_pose::seconds_tick(
                descriptor.animation.timeline,
                f32::from_bits(child.channels[0][1]),
            )?;
            let model = child
                .spec
                .models
                .first()
                .and_then(Option::as_ref)
                .ok_or("normal jet model absent")?;
            let positions = if let Some(skin) = &model.prepared.skin {
                let tree = descriptor
                    .animation
                    .trees
                    .iter()
                    .find(|t| t.offset == descriptor.animation.default_tree)
                    .ok_or("normal jet tree absent")?;
                giants_game::skinning::positions(skin, &descriptor.animation, &tree.name, tick)
                    .map_err(|e| e.to_string())?
            } else {
                model.prepared.mesh.positions.clone()
            };
            let mut world = giants_game::effect_attachment::initial_child_matrix(
                parent_world,
                parent_reference,
                reference(
                    &child.spec,
                    &descriptor.animation,
                    binding.reference_word,
                    tick,
                )?,
                child.fields.scale,
                parent.scale,
            )?;
            world.values[11] = (f64::from(world.values[11])
                + f64::from(f32::from_bits(child.fields.configuration[17])))
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
                personality: child.fields.configuration[14],
                scale: child.fields.scale,
                angle: child.fields.angles[0],
            });
        }
        Ok(frames)
    }
}
