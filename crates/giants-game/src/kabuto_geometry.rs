//! Owned auxiliary geometry used by the acquired Kabuto cold initializer.
//! This prepares vertices/topology only; material generation, owner registration,
//! deformation callbacks and rendering remain separate phases.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}
#[derive(Clone, Debug)]
pub struct Geometry {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}
/// Acquired Double53 profile. Normals and texture coordinates are retained as
/// generated; this does not replace them with recomputed smooth surface normals.
pub fn prepare(segments: u16, rings: u16, radius: f32) -> Result<Geometry, String> {
    let n = usize::from(segments);
    let m = usize::from(rings);
    let count = n * m + 1;
    if n < 3 || m < 2 || count > 65536 || !radius.is_finite() || radius <= 0. {
        return Err("Kabuto auxiliary geometry outside bounded positive profile".into());
    }
    let mut vertices = Vec::with_capacity(count);
    let mut indices = Vec::with_capacity(3 * n * (2 * m - 1));
    vertices.push(Vertex {
        position: [0., 0., 1.],
        normal: [0., 0., 1.],
        uv: [0., 0.],
    });
    let mut height = 1.;
    let step = 1. / (m - 1) as f64;
    for ring in 0..m {
        for segment in 0..n {
            let theta = (segment as f64 * f64::from(std::f32::consts::TAU)) / n as f64;
            let angle = -theta;
            // The cosine is stored before multiplying the radius. Sine is not.
            let x = (angle.sin() * f64::from(radius)) as f32;
            let y = (f64::from(angle.cos() as f32) * f64::from(radius)) as f32;
            let z = height as f32;
            let u = (theta * f64::from(std::f32::consts::FRAC_1_PI * 0.5)) as f32;
            let mut v = (1. - height) as f32;
            if ring == 0 {
                v = (f64::from(v) + 0.03125) as f32;
            }
            if ring == m - 1 {
                v = (f64::from(v) - 0.03125) as f32;
            }
            let gain = (1. - f64::from(z)).max(f64::from(0.1f32));
            vertices.push(Vertex {
                position: [
                    (f64::from(x) * gain) as f32,
                    (f64::from(y) * gain) as f32,
                    z,
                ],
                normal: [0., 0., 1.],
                uv: [u, v],
            });
            if ring == 0 {
                indices.extend([
                    0,
                    if segment == 0 {
                        n as u16
                    } else {
                        segment as u16
                    },
                    (segment + 1) as u16,
                ]);
            } else {
                let previous = if segment == 0 {
                    ring * n
                } else {
                    (ring - 1) * n + segment
                };
                let current = if segment == 0 {
                    (ring + 1) * n
                } else {
                    ring * n + segment
                };
                indices.extend([
                    previous as u16,
                    current as u16,
                    (ring * n + segment + 1) as u16,
                    previous as u16,
                    (ring * n + segment + 1) as u16,
                    ((ring - 1) * n + segment + 1) as u16,
                ]);
            }
        }
        // Keep the accumulated wide height; deriving it afresh from the index
        // loses the acquired residual at the last ring for some ring counts.
        height -= step;
    }
    Ok(Geometry { vertices, indices })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_fan_seam_normals_uv_offsets_and_signed_zero() {
        let g = prepare(4, 2, 1.).unwrap();
        assert_eq!(g.vertices.len(), 9);
        assert_eq!(g.indices.len(), 36);
        assert_eq!(&g.indices[..6], &[0, 4, 1, 0, 1, 2]);
        assert_eq!(&g.indices[12..18], &[4, 8, 5, 4, 5, 1]);
        assert_eq!(g.vertices[1].position[0].to_bits(), (-0f32).to_bits());
        assert_eq!(g.vertices[1].uv, [0., 0.03125]);
        assert_eq!(g.vertices[5].uv, [0., 0.96875]);
        assert!(g.vertices.iter().all(|v| v.normal == [0., 0., 1.]));
    }
    #[test]
    fn accumulated_height_and_index_budget_are_retained() {
        let g = prepare(5, 4, 2.).unwrap();
        assert!(g.vertices.last().unwrap().position[2] > 0.);
        assert!(g.indices.iter().all(|&i| usize::from(i) < g.vertices.len()));
        assert!(prepare(2, 2, 1.).is_err());
        assert!(prepare(4, 1, 1.).is_err());
        assert!(prepare(u16::MAX, u16::MAX, 1.).is_err());
        assert!(prepare(4, 2, f32::NAN).is_err());
    }
}
