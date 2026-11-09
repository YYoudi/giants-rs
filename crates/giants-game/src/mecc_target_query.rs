//! Viewport target selection over explicit ordered actors and current geometry.
//! World admission, context production and collision-pose advancement remain
//! upstream. This is not the state6 projectile-list query.
use crate::{contact_pose::Cached, object_segment::Mesh, volume_queries::Bounds};
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Geometry {
    pub pose: Cached,
    /// Current animation bounds, or the actual native fallback model bounds.
    pub bounds: Bounds,
    /// Dedicated collision geometry in local coordinates.
    pub mesh: Option<Mesh>,
}
pub struct Actor {
    pub parent: Option<u32>,
    pub flags: u32,
    pub personality: u32,
    pub context: Option<u32>,
    pub state: u32,
    pub class: i32,
    pub word_e4: Option<u32>,
    pub model_present: bool,
    pub geometry: Option<Geometry>,
}
#[derive(Clone, Copy)]
pub enum Predicate {
    Default,
    OfflinePistol,
    /// Required caller-specific policy, evaluated before any geometry service.
    Provided,
}
pub struct Input {
    pub current_player: Option<u32>,
    pub context: u32,
    pub predicate: Predicate,
    pub origin: [f32; 3],
    pub direction: [f32; 3],
    pub range: f32,
}
pub struct Output {
    pub actor: Option<u32>,
    pub point: [f32; 3],
}
pub struct LocalOutput {
    pub target: Output,
    pub ray: crate::mecc_aim::Ray,
    pub near_terrain: bool,
}
#[derive(Clone, Copy)]
pub struct Header {
    pub context: u32,
    pub word_e4: Option<u32>,
}
/// Required current providers. Geometry is requested only after metadata filters.
pub trait Services {
    fn predicate(&mut self, _id: u32) -> Result<bool, String> {
        Err("target query predicate provider absent".into())
    }
    fn header(&mut self, root: u32) -> Result<Header, String>;
    fn geometry(&mut self, id: u32) -> Result<Geometry, String>;
    fn mesh(&mut self, id: u32) -> Result<Mesh, String>;
}
struct Snapshot<'a>(&'a BTreeMap<u32, Actor>);
impl Services for Snapshot<'_> {
    fn header(&mut self, id: u32) -> Result<Header, String> {
        let actor = self.0.get(&id).ok_or("target root absent")?;
        Ok(Header {
            context: actor.context.ok_or("target root context provider absent")?,
            word_e4: actor.word_e4,
        })
    }
    fn geometry(&mut self, id: u32) -> Result<Geometry, String> {
        self.0
            .get(&id)
            .and_then(|a| a.geometry.clone())
            .ok_or_else(|| "target current geometry absent".into())
    }
    fn mesh(&mut self, id: u32) -> Result<Mesh, String> {
        self.0
            .get(&id)
            .and_then(|a| a.geometry.as_ref())
            .and_then(|g| g.mesh.clone())
            .ok_or_else(|| "target dedicated collision mesh absent".into())
    }
}
/// Local normal-pistol prepass. The camera ray and viewport segment use
/// distinct snapshots, as in the source. This does not publish actor links.
pub fn local(
    input: Input,
    camera: crate::first_person::Camera,
    terrain: &crate::terrain_queries::StaticTerrainQuery<'_>,
    actors: &BTreeMap<u32, Actor>,
    order: &[u32],
) -> Result<LocalOutput, String> {
    local_with(input, camera, terrain, actors, order, &mut Snapshot(actors))
}
pub fn local_with(
    mut input: Input,
    camera: crate::first_person::Camera,
    terrain: &crate::terrain_queries::StaticTerrainQuery<'_>,
    actors: &BTreeMap<u32, Actor>,
    order: &[u32],
    services: &mut impl Services,
) -> Result<LocalOutput, String> {
    if !matches!(input.predicate, Predicate::OfflinePistol) || input.current_player.is_none() {
        return Err("local prepass requires its current player and normal pistol predicate".into());
    }
    let declaration_range = input.range;
    let ray = crate::mecc_aim::terrain_ray(
        terrain,
        camera.position,
        crate::mecc_aim::direction(camera.angles)?,
        (f64::from(declaration_range) * 2.) as f32,
    )?;
    input.range = ray.distance;
    let target = query_with(input, actors, order, services)?;
    Ok(LocalOutput {
        target,
        near_terrain: ray.distance < declaration_range,
        ray,
    })
}
fn chain(actors: &BTreeMap<u32, Actor>, id: u32) -> Result<Vec<u32>, &'static str> {
    let mut result = Vec::new();
    let mut current = Some(id);
    while let Some(id) = current {
        if id == 0 {
            return Err("zero target identity");
        }
        if result.contains(&id) {
            return Err("target hierarchy cycle");
        }
        let actor = actors.get(&id).ok_or("target ancestor absent")?;
        result.push(id);
        current = actor.parent;
    }
    Ok(result)
}
pub fn query(input: Input, actors: &BTreeMap<u32, Actor>, order: &[u32]) -> Result<Output, String> {
    query_with(input, actors, order, &mut Snapshot(actors))
}
pub fn query_with(
    input: Input,
    actors: &BTreeMap<u32, Actor>,
    order: &[u32],
    services: &mut impl Services,
) -> Result<Output, String> {
    if !input
        .origin
        .into_iter()
        .chain(input.direction)
        .chain([input.range])
        .all(f32::is_finite)
    {
        return Err("target ray outside finite profile".into());
    }
    let range = if input.range == 0. {
        9000.
    } else {
        input.range
    };
    let end = std::array::from_fn(|i| {
        (f64::from(input.direction[i]) * f64::from(range) + f64::from(input.origin[i])) as f32
    });
    if !end.into_iter().all(f32::is_finite) {
        return Err("target ray overflow".into());
    }
    let mut selected = None;
    let mut saved = [0.; 3];
    let mut selected_pose = None;
    let mut distance = 0f32;
    for &id in order {
        let actor = actors.get(&id).ok_or("target list actor absent")?;
        if actor.flags & 0x8000 != 0 || !actor.model_present || Some(id) == input.current_player {
            continue;
        }
        let chain = chain(actors, id)?;
        if chain
            .iter()
            .skip(1)
            .any(|&id| Some(id) == input.current_player)
            || input.current_player.is_none()
        {
            continue;
        }
        if matches!(input.predicate, Predicate::Provided) {
            if !services.predicate(id)? {
                continue;
            }
        } else {
            if actor.personality & 0x20000 == 0 || actor.flags & 0x4000c000 != 0 {
                continue;
            }
            let root = &actors[chain.last().unwrap()];
            let special = matches!(input.predicate, Predicate::OfflinePistol)
                && matches!(root.state, 74 | 83 | 44 | 95);
            let permitted = if special {
                true
            } else {
                let header = services.header(*chain.last().unwrap())?;
                header.context != input.context
                    || matches!(input.predicate, Predicate::OfflinePistol)
                        && root.class == 805
                        && header
                            .word_e4
                            .ok_or("target root word_e4 provider absent")?
                            != 1
            };
            if !permitted {
                continue;
            }
        }
        if actor.flags & 0x20 == 0 {
            return Err("target geometry requires its current affine cache".into());
        }
        let geometry = services.geometry(id)?;
        let start_local = geometry.pose.to_local(input.origin)?;
        let end_local = geometry.pose.to_local(end)?;
        let Some(mut hit) = crate::object_segment::bounds(
            start_local,
            end_local,
            geometry.bounds.min,
            geometry.bounds.max,
        )?
        else {
            continue;
        };
        let distance_to = |p: [f32; 3]| -> Result<f64, &'static str> {
            let world = geometry.pose.to_world(p)?;
            let d: [f64; 3] =
                std::array::from_fn(|i| f64::from(world[i]) - f64::from(input.origin[i]));
            Ok((d[0] * d[0] + d[1] * d[1]) + d[2] * d[2])
        };
        if selected.is_some() && distance_to(hit)? >= f64::from(distance) {
            continue;
        }
        if actor.personality & 0x1000000 == 0 {
            let mesh = services.mesh(id)?;
            let Some(point) = mesh.hit_local(start_local, end_local)? else {
                continue;
            };
            hit = point;
        }
        let next = distance_to(hit)?;
        if selected.is_none() || next < f64::from(distance) {
            selected = Some(id);
            saved = hit;
            selected_pose = Some(geometry.pose);
            distance = next as f32;
        }
    }
    Ok(Output {
        actor: selected,
        point: if selected.is_some() {
            selected_pose.unwrap().to_world(saved)?
        } else {
            end
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> Input {
        Input {
            current_player: Some(2),
            context: 0,
            predicate: Predicate::OfflinePistol,
            origin: [-5., 0., 0.],
            direction: [1., 0., 0.],
            range: 10.,
        }
    }
    fn actor() -> Actor {
        Actor {
            parent: None,
            flags: 0x20,
            personality: 0x20000,
            context: Some(1),
            state: 2,
            class: 73,
            word_e4: Some(0),
            model_present: true,
            geometry: None,
        }
    }
    struct Probe {
        same_context: bool,
        geometry_calls: usize,
    }
    impl Services for Probe {
        fn header(&mut self, _: u32) -> Result<Header, String> {
            Ok(Header {
                context: if self.same_context { 0 } else { 1 },
                word_e4: None,
            })
        }
        fn geometry(&mut self, _: u32) -> Result<Geometry, String> {
            self.geometry_calls += 1;
            let matrix = std::array::from_fn(|i| f32::from(i / 4 == i % 4));
            Ok(Geometry {
                pose: Cached::new(0x20, 0x13, matrix)?,
                bounds: Bounds {
                    min: [100., -1., -1.],
                    max: [101., 1., 1.],
                },
                mesh: None,
            })
        }
        fn mesh(&mut self, _: u32) -> Result<Mesh, String> {
            panic!("filtered/missed model must not prepare its collision buffer")
        }
    }
    #[test]
    fn rejected_root_does_not_resolve_geometry_or_invent_missing_word() {
        let mut actors = BTreeMap::from([(1, actor())]);
        let mut probe = Probe {
            same_context: true,
            geometry_calls: 0,
        };
        assert!(
            query_with(input(), &actors, &[1], &mut probe)
                .unwrap()
                .actor
                .is_none()
        );
        assert_eq!(probe.geometry_calls, 0);
        actors.get_mut(&1).unwrap().class = 805;
        assert_eq!(
            query_with(input(), &actors, &[1], &mut probe).err(),
            Some("target root word_e4 provider absent".into())
        );
        assert_eq!(probe.geometry_calls, 0);
    }
    #[test]
    fn bounds_miss_does_not_prepare_the_collision_model() {
        let actors = BTreeMap::from([(1, actor())]);
        let mut probe = Probe {
            same_context: false,
            geometry_calls: 0,
        };
        assert!(
            query_with(input(), &actors, &[1], &mut probe)
                .unwrap()
                .actor
                .is_none()
        );
        assert_eq!(probe.geometry_calls, 1);
    }
    #[test]
    fn eligible_missing_geometry_is_not_replaced_by_an_actor_center() {
        let actors = BTreeMap::from([(1, actor())]);
        assert_eq!(
            query(input(), &actors, &[1]).err(),
            Some("target current geometry absent".into())
        );
    }
    #[test]
    fn invalid_hierarchy_is_refused_before_geometry_queries() {
        let mut child = actor();
        child.parent = Some(1);
        let mut actors = BTreeMap::from([(1, child)]);
        assert_eq!(
            query(input(), &actors, &[1]).err(),
            Some("target hierarchy cycle".into())
        );
        actors.get_mut(&1).unwrap().parent = Some(99);
        assert_eq!(
            query(input(), &actors, &[1]).err(),
            Some("target ancestor absent".into())
        );
    }
    struct PolicyProbe {
        result: bool,
        calls: Vec<u32>,
    }
    impl Services for PolicyProbe {
        fn predicate(&mut self, id: u32) -> Result<bool, String> {
            self.calls.push(id);
            Ok(self.result)
        }
        fn header(&mut self, _: u32) -> Result<Header, String> {
            panic!("provided policy must not request legacy context headers")
        }
        fn geometry(&mut self, _: u32) -> Result<Geometry, String> {
            Err("reached actual geometry provider".into())
        }
        fn mesh(&mut self, _: u32) -> Result<Mesh, String> {
            panic!("geometry failed before mesh preparation")
        }
    }
    #[test]
    fn provided_policy_bypasses_legacy_filters_and_keeps_common_exclusions() {
        let mut candidate = actor();
        candidate.personality = 0;
        candidate.context = None;
        let mut descendant = actor();
        descendant.parent = Some(2);
        let mut hidden = actor();
        hidden.flags |= 0x8000;
        let actors = BTreeMap::from([(1, candidate), (2, actor()), (3, descendant), (4, hidden)]);
        let mut i = input();
        i.predicate = Predicate::Provided;
        let mut probe = PolicyProbe {
            result: true,
            calls: Vec::new(),
        };
        assert_eq!(
            query_with(i, &actors, &[2, 3, 4, 1], &mut probe).err(),
            Some("reached actual geometry provider".into())
        );
        assert_eq!(probe.calls, [1]);
    }
    #[test]
    fn provided_policy_rejection_precedes_affine_requirement_and_never_defaults() {
        let mut candidate = actor();
        candidate.flags = 0;
        let actors = BTreeMap::from([(1, candidate)]);
        let mut i = input();
        i.predicate = Predicate::Provided;
        let mut probe = PolicyProbe {
            result: false,
            calls: Vec::new(),
        };
        assert!(
            query_with(i, &actors, &[1], &mut probe)
                .unwrap()
                .actor
                .is_none()
        );
        assert_eq!(probe.calls, [1]);
        let mut i = input();
        i.predicate = Predicate::Provided;
        assert_eq!(
            query(i, &actors, &[1]).err(),
            Some("target query predicate provider absent".into())
        );
    }
}
