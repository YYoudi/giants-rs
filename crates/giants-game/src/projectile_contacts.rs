//! D state77 mesh-contact list with explicit identities, root fields and poses.
//! Native list production, animated mesh freshness and hit dispatch are separate.
use crate::{object_lists::DirectFlags, object_segment::Mesh, volume_queries::Bounds};

#[derive(Clone, Copy, Debug)]
pub struct Actor<K> {
    pub id: K,
    pub parent: Option<K>,
    pub class: i32,
    pub state: u32,
    pub context: u32,
    /// Opaque native header E4 word, used by the class805 exception.
    pub type_word: u32,
    pub flags: DirectFlags,
}

#[cfg(test)]
mod tests {
    use super::*;
    const ENV: Environment = Environment {
        mode: 0,
        override_flag: 0,
    };
    fn object(id: u32, parent: Option<u32>, x: f32) -> Object<'static, u32> {
        Object {
            actor: Actor {
                id,
                parent,
                class: 1,
                state: 0,
                context: 2,
                type_word: 0,
                flags: DirectFlags {
                    active: 0,
                    personality: 0x1020000,
                },
            },
            bounds: Bounds {
                min: [x, -1., -1.],
                max: [x + 1., 1., 1.],
            },
            mesh: None,
        }
    }
    fn shot(source: Option<u32>) -> Shot<u32> {
        Shot {
            id: 10,
            source,
            context: 1,
            start: [0.; 3],
            end: [10., 0., 0.],
        }
    }
    #[test]
    fn root_walk_preserves_child_gate_and_rejects_missing_or_cycles() {
        let mut objects = [object(1, Some(2), 2.), object(2, None, 3.)];
        objects[1].actor.context = 1;
        objects[1].actor.state = 74;
        objects[1].actor.flags = DirectFlags {
            active: u32::MAX,
            personality: 0,
        };
        assert!(
            Frame::new(&objects, &[1])
                .unwrap()
                .accepts(1, 1, ENV)
                .unwrap()
        );
        objects[1].actor.parent = Some(1);
        assert!(
            Frame::new(&objects, &[1])
                .unwrap()
                .accepts(1, 1, ENV)
                .is_err()
        );
        objects[0].actor.parent = Some(99);
        assert!(
            Frame::new(&objects, &[1])
                .unwrap()
                .accepts(1, 1, ENV)
                .is_err()
        );
        objects[0].actor.flags.personality = 0;
        assert!(
            !Frame::new(&objects, &[1])
                .unwrap()
                .accepts(1, 1, ENV)
                .unwrap()
        );
    }
    #[test]
    fn selection_and_shared_point_are_separate_and_ties_keep_first() {
        let objects = [
            object(1, None, 2.),
            object(2, None, 8.),
            object(3, None, 2.),
        ];
        let result = Frame::new(&objects, &[1, 2, 3, 2])
            .unwrap()
            .query(shot(Some(99)), ENV)
            .unwrap();
        assert_eq!(result.selected, Some(1));
        assert_eq!(result.distance_sq, Some(4.));
        assert_eq!(result.point, [8., 0., 0.]);
        let result = Frame::new(&objects, &[1, 2])
            .unwrap()
            .query(shot(None), ENV)
            .unwrap();
        assert_eq!(result.selected, None);
        assert_eq!(result.distance_sq, None);
        assert_eq!(result.tag, None);
    }
    #[test]
    fn required_mesh_is_not_replaced_by_bounds() {
        let mut objects = [object(1, None, 2.)];
        objects[0].actor.flags.personality &= !0x1000000;
        assert!(
            Frame::new(&objects, &[1])
                .unwrap()
                .query(shot(Some(99)), ENV)
                .is_err()
        );
    }
    #[test]
    fn root_context_exception_requires_exact_mode_and_type_word() {
        let mut actor = object(1, None, 2.).actor;
        actor.context = 1;
        actor.flags.personality |= 1;
        assert!(!accepts(&actor, &actor, 1, ENV));
        assert!(!accepts(
            &actor,
            &actor,
            1,
            Environment {
                mode: 3,
                override_flag: 1
            }
        ));
        assert!(accepts(
            &actor,
            &actor,
            1,
            Environment {
                mode: 4,
                override_flag: 1
            }
        ));
        actor.class = 805;
        actor.type_word = 1;
        assert!(!accepts(&actor, &actor, 1, ENV));
        actor.type_word = 0;
        assert!(accepts(&actor, &actor, 1, ENV));
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Environment {
    /// Value consumed by the original mode predicate; only4 takes its branch.
    pub mode: u32,
    /// Nonzero enables the root-personality-bit1 exception in that mode.
    pub override_flag: u32,
}
#[derive(Clone, Copy)]
pub struct Object<'a, K> {
    pub actor: Actor<K>,
    pub bounds: Bounds,
    /// World-posed, unskinned collision geometry. None is allowed for nonmembers
    /// and box-mode objects; a required mesh is never replaced by its bounds.
    pub mesh: Option<&'a Mesh>,
}
pub struct Shot<K> {
    pub id: K,
    pub source: Option<K>,
    pub context: u32,
    pub start: [f32; 3],
    pub end: [f32; 3],
}
#[derive(Clone, Debug)]
pub struct Query<K> {
    pub selected: Option<K>,
    /// Shared scratch point, which can belong to a later rejected candidate.
    pub point: [f32; 3],
    /// Undefined native scratch words without a selection are represented as
    /// None, not as an invented zero distance or tag.
    pub distance_sq: Option<f32>,
    pub tag: Option<u32>,
}
/// Complete acquired D4b2b10 predicate once the current root has been resolved.
/// Initial eligibility is checked on the child; root flags are not reapplied.
pub fn accepts<K>(target: &Actor<K>, root: &Actor<K>, context: u32, env: Environment) -> bool {
    target.flags.personality & 0x20000 != 0
        && target.flags.active & 0x4000_c000 == 0
        && ((env.mode == 4 && env.override_flag != 0 && root.flags.personality & 1 != 0)
            || matches!(root.state, 74 | 83 | 44 | 95)
            || root.context != context
            || (root.class == 805 && root.type_word != 1))
}
/// Ordered IDs are a supplied membership snapshot; do not infer native list
/// completeness or sort/deduplicate that order. The registry also holds parents.
pub struct Frame<'a, K> {
    registry: &'a [Object<'a, K>],
    order: &'a [K],
}
impl<'a, K: Copy + Eq> Frame<'a, K> {
    pub(crate) fn members(&self) -> impl Iterator<Item = K> + '_ {
        self.order.iter().copied()
    }
    pub(crate) fn actors(&self) -> impl Iterator<Item = &Actor<K>> {
        self.registry.iter().map(|object| &object.actor)
    }
    pub fn new(registry: &'a [Object<'a, K>], order: &'a [K]) -> Result<Self, &'static str> {
        for (i, object) in registry.iter().enumerate() {
            if registry[..i]
                .iter()
                .any(|other| other.actor.id == object.actor.id)
            {
                return Err("duplicate projectile contact registry identity");
            }
        }
        let frame = Self { registry, order };
        if order.iter().any(|id| frame.object(*id).is_none()) {
            return Err("projectile contact list identity absent from registry");
        }
        Ok(frame)
    }
    fn object(&self, id: K) -> Option<&Object<'a, K>> {
        self.registry.iter().find(|object| object.actor.id == id)
    }
    fn root(&self, mut actor: Actor<K>) -> Result<Actor<K>, &'static str> {
        for _ in 0..=self.registry.len() {
            let Some(parent) = actor.parent else {
                return Ok(actor);
            };
            actor = self
                .object(parent)
                .ok_or("projectile contact parent absent")?
                .actor;
        }
        Err("cyclic projectile contact parents")
    }
    pub fn accepts(&self, id: K, context: u32, env: Environment) -> Result<bool, &'static str> {
        let actor = self
            .object(id)
            .ok_or("contact filter identity absent")?
            .actor;
        if actor.flags.personality & 0x20000 == 0 || actor.flags.active & 0x4000_c000 != 0 {
            return Ok(false);
        }
        Ok(accepts(&actor, &self.root(actor)?, context, env))
    }
    pub fn query(&self, shot: Shot<K>, env: Environment) -> Result<Query<K>, &'static str> {
        if !shot.start.into_iter().chain(shot.end).all(f32::is_finite) {
            return Err("non-finite projectile contact segment");
        }
        let mut result = Query {
            selected: None,
            point: shot.end,
            distance_sq: None,
            tag: None,
        };
        for &id in self.order {
            let object = self.object(id).expect("validated membership");
            let actor = object.actor;
            // Equality applies even to the two null identities (None == None).
            if actor.id == shot.id || Some(actor.id) == shot.source || actor.parent == shot.source {
                continue;
            }
            if !self.accepts(actor.id, shot.context, env)?
                || !crate::object_segment::bounds_into(
                    shot.start,
                    shot.end,
                    object.bounds.min,
                    object.bounds.max,
                    &mut result.point,
                )?
            {
                continue;
            }
            if actor.flags.personality & 0x1000000 == 0 {
                let mesh = object
                    .mesh
                    .ok_or("current posed projectile contact mesh unavailable")?;
                mesh.validate_current_flags(actor.flags.active)?;
                let Some(point) = mesh.hit(shot.start, shot.end)? else {
                    continue;
                };
                result.point = point;
            }
            let delta = std::array::from_fn::<_, 3, _>(|i| {
                f64::from(result.point[i]) - f64::from(shot.start[i])
            });
            let distance = (delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2];
            if result
                .distance_sq
                .is_none_or(|old| f64::from(old) > distance)
            {
                if !distance.is_finite() || distance > f64::from(f32::MAX) {
                    return Err("projectile contact distance outside finite profile");
                }
                result.selected = Some(actor.id);
                result.distance_sq = Some(distance as f32);
                // The full D mesh wrapper writes zero with an output point,
                // including the controlled skinned dedicated-model path.
                result.tag = Some(0);
            }
        }
        Ok(result)
    }
}
