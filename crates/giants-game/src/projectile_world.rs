//! State6 list projection over supplied posed geometry and current object fields.
//! Live list order, animation and impact services remain outside this query.
use crate::{object_segment, volume_queries::Bounds};
pub struct Object {
    pub id: u32,
    pub parent: u32,
    pub active: u32,
    pub personality: u32,
    pub bounds: Bounds,
    pub mesh: object_segment::Mesh,
}
#[derive(Debug)]
pub struct Query {
    pub selected: Option<u32>,
    /// Shared scratch output: may differ from the selected object's hit point.
    pub point: [f32; 3],
    pub tag: u32,
    pub distance_sq: f32,
}
pub fn query(
    projectile: u32,
    owner: u32,
    start: [f32; 3],
    end: [f32; 3],
    objects: &[Object],
) -> Result<Query, &'static str> {
    let mut result = Query {
        selected: None,
        point: end,
        tag: 0,
        distance_sq: 0.,
    };
    for object in objects {
        if !object_segment::projectile_candidate(
            projectile,
            owner,
            object.id,
            object.parent,
            object.active,
            object.personality,
        ) || !object_segment::bounds_into(
            start,
            end,
            object.bounds.min,
            object.bounds.max,
            &mut result.point,
        )? {
            continue;
        }
        result.tag = 0;
        if object.personality & 0x1000000 == 0 {
            let Some(point) = object.mesh.hit(start, end)? else {
                continue;
            };
            result.point = point;
        }
        let delta =
            std::array::from_fn::<_, 3, _>(|i| f64::from(result.point[i]) - f64::from(start[i]));
        let distance = (delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2];
        if result.selected.is_none() || f64::from(result.distance_sq) > distance {
            result.selected = Some(object.id);
            result.distance_sq = distance as f32;
        }
    }
    Ok(result)
}
