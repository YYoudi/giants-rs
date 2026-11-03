//! Ordered collision triangles from a decoded mesh and an explicit source pose.
//! Part masking, skinning, live state and collider eligibility remain upstream.
use crate::{arithmetic::Precision, collision_world::SourcePose};
use giants_formats::gbs::Mesh;

pub fn world_triangles(
    mesh: &Mesh,
    pose: SourcePose,
    precision: Precision,
) -> Result<Vec<[[f32; 3]; 3]>, &'static str> {
    let positions = mesh
        .positions
        .iter()
        .map(|p| pose.transform(*p, precision))
        .collect::<Result<Vec<_>, _>>()?;
    let mut triangles = Vec::new();
    for part in &mesh.subobjects {
        if triangles.len() + part.triangles.len() > 1_000_000 {
            return Err("collision triangle budget exceeded");
        }
        for triangle in &part.triangles {
            let mut points = [[0.; 3]; 3];
            for (point, index) in points.iter_mut().zip(triangle) {
                let vertex = *mesh
                    .indices
                    .get(*index as usize)
                    .ok_or("collision source index absent")? as usize;
                *point = *positions
                    .get(vertex)
                    .ok_or("collision source position absent")?;
            }
            triangles.push(points);
        }
    }
    Ok(triangles)
}
