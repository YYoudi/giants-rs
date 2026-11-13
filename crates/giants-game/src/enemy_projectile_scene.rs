//! The ordinary partial scene retains declaration identities independently of
//! Session actors. Registry keys are query-local identities, never actor serials.
//! A scene does not certify complete native population or damage availability.
use crate::{projectile_world::Object, resident_states::Id, static_geometry::SourceKey};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Actor(Id),
    Declaration(SourceKey),
}
#[derive(Clone)]
pub struct Entry {
    pub target: Target,
    pub object: Object,
}
/// Existing controller registry: prepared declaration geometry and stable keys
/// for admitted NPCs. Current animated packets must be obtained separately.
pub struct Registry {
    pub declarations: Vec<Entry>,
    pub actors: BTreeMap<Id, u32>,
}
pub struct Scene {
    pub(crate) objects: Vec<Object>,
    pub(crate) identities: BTreeMap<u32, Target>,
    pub(crate) keys: BTreeMap<Target, u32>,
}
impl Scene {
    /// Preserve supplied registry order. The reserved shot key cannot alias a
    /// scene entry; a declaration and an actor with equal numbers remain distinct.
    pub fn new(entries: Vec<Entry>) -> Result<Self, String> {
        let mut objects = Vec::new();
        let mut identities = BTreeMap::new();
        let mut keys = BTreeMap::new();
        for entry in entries {
            if entry.object.id == 0
                || entry.object.id == u32::MAX
                || identities.insert(entry.object.id, entry.target).is_some()
                || keys.insert(entry.target, entry.object.id).is_some()
            {
                return Err("projectile scene duplicate or reserved identity".into());
            }
            objects.push(entry.object);
        }
        let present: BTreeSet<_> = identities.keys().copied().collect();
        if objects
            .iter()
            .any(|o| o.parent != 0 && !present.contains(&o.parent))
        {
            return Err("projectile scene parent absent from the actual registry".into());
        }
        Ok(Self {
            objects,
            identities,
            keys,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(key: u32, source: SourceKey, x: f32) -> Entry {
        Entry {
            target: Target::Declaration(source),
            object: Object {
                id: key,
                parent: 0,
                active: 0,
                personality: 0x20000,
                bounds: crate::volume_queries::Bounds {
                    min: [x - 0.1, -2., -2.],
                    max: [x + 0.1, 2., 2.],
                },
                mesh: crate::object_segment::Mesh::new(
                    key,
                    vec![[[x, -2., -2.], [x, 2., -2.], [x, 0., 2.]]],
                )
                .unwrap(),
            },
        }
    }
    #[test]
    fn declaration_scopes_keep_distinct_targets_and_shared_query_scratch_order() {
        let scene = Scene::new(vec![
            entry(2, (false, 0), 3.),
            entry(0x40000002, (true, 0), 7.),
        ])
        .unwrap();
        let hit =
            crate::projectile_world::query(u32::MAX, 9, [0.; 3], [10., 0., 0.], &scene.objects)
                .unwrap();
        assert_eq!(
            scene.identities[&hit.selected.unwrap()],
            Target::Declaration((false, 0))
        );
        // The native query retains the last attempted scratch point, separately
        // from nearest-target identity. Scene wiring must not "correct" it.
        assert_eq!(hit.point, [7., 0., 0.]);
        assert_eq!(
            scene.objects.iter().map(|o| o.id).collect::<Vec<_>>(),
            [2, 0x40000002]
        );
    }
    #[test]
    fn ambiguous_registry_and_missing_parent_cannot_be_admitted() {
        assert!(Scene::new(vec![entry(2, (false, 0), 3.), entry(2, (true, 0), 7.)]).is_err());
        assert!(Scene::new(vec![entry(2, (false, 0), 3.), entry(3, (false, 0), 7.)]).is_err());
        assert!(Scene::new(vec![entry(u32::MAX, (false, 0), 3.)]).is_err());
        let mut missing = entry(2, (false, 0), 3.);
        missing.object.parent = 99;
        assert!(Scene::new(vec![missing]).is_err());
    }
}
