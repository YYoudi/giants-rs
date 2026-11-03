//! Source-position derivatives with respect to serialized UVs. The measured
//! preparation independently normalizes accumulated fields; it does not build
//! an orthogonal tangent frame or establish the subsequent graphics consumer.
pub struct Basis {
    pub vertex_start: usize,
    pub du: Vec<[f32; 3]>,
    pub dv: Vec<[f32; 3]>,
}

/// Bounded non-degenerate profile, including the measured intermediate stores.
pub fn triangle(
    positions: [[f32; 3]; 3],
    uv: [[f32; 2]; 3],
) -> Result<[[f32; 3]; 2], &'static str> {
    if !positions
        .iter()
        .flatten()
        .chain(uv.iter().flatten())
        .all(|v| v.is_finite())
    {
        return Err("non-finite UV gradient input");
    }
    let [a, b, c] = positions.map(|p| p.map(f64::from));
    let uv = uv.map(|p| p.map(f64::from));
    let du1 = uv[1][0] - uv[0][0];
    let dv1 = uv[1][1] - uv[0][1];
    let du2 = f64::from((uv[2][0] - uv[0][0]) as f32);
    let dv2 = uv[2][1] - uv[0][1];
    let determinant = (dv2 * du1 - du2 * dv1) as f32;
    if !determinant.is_finite() || f64::from(determinant).abs() <= 1e-12 {
        return Err("UV gradient determinant outside non-degenerate profile");
    }
    let inverse = 1. / f64::from(determinant);
    let mut result = [[0.; 3]; 2];
    for axis in 0..3 {
        let edge1 = b[axis] - a[axis];
        let edge2 = f64::from((c[axis] - a[axis]) as f32);
        let numerator_u = (edge2 * dv1 - dv2 * edge1) as f32;
        let numerator_v = edge1 * du2 - edge2 * du1;
        result[0][axis] = (-f64::from(numerator_u) * inverse) as f32;
        result[1][axis] = (-numerator_v * inverse) as f32;
    }
    if !result.iter().flatten().all(|v| v.is_finite()) {
        return Err("non-finite UV gradient output");
    }
    Ok(result)
}

/// Render-vertex fields for one source part, in original triangle order.
/// Degenerate UV behavior depends on native caller state and is rejected here.
pub fn prepare(
    mesh: &giants_formats::gbs::Mesh,
    positions: &[[f32; 3]],
    part: usize,
) -> Result<Basis, &'static str> {
    if positions.len() != mesh.positions.len() {
        return Err("UV basis position count differs from mesh");
    }
    let part = mesh
        .subobjects
        .get(part)
        .ok_or("UV basis part out of bounds")?;
    let end = part
        .vertex_start
        .checked_add(part.vertex_count)
        .ok_or("UV basis vertex range overflow")?;
    if end > mesh.indices.len() || end > mesh.uv.len() || part.triangles.is_empty() {
        return Err("UV basis vertex range or triangle list outside profile");
    }
    let mut du = vec![[0.; 3]; part.vertex_count];
    let mut dv = du.clone();
    for indices in &part.triangles {
        let mut p = [[0.; 3]; 3];
        let mut uv = [[0.; 2]; 3];
        for (corner, &index) in indices.iter().enumerate() {
            let index = usize::from(index);
            if !(part.vertex_start..end).contains(&index) {
                return Err("UV basis triangle outside part range");
            }
            p[corner] = *positions
                .get(usize::from(mesh.indices[index]))
                .ok_or("UV basis source position out of bounds")?;
            uv[corner] = mesh.uv[index];
        }
        let gradients = triangle(p, uv)?;
        for &index in indices {
            let index = usize::from(index) - part.vertex_start;
            for axis in 0..3 {
                du[index][axis] += gradients[0][axis];
                dv[index][axis] += gradients[1][axis];
            }
        }
    }
    for value in du.iter_mut().chain(dv.iter_mut()) {
        *value = crate::body_pipeline::normalize_prepared(*value)?;
    }
    Ok(Basis {
        vertex_start: part.vertex_start,
        du,
        dv,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mirrored_uvs_reverse_the_corresponding_derivative() {
        let p = [[0., 0., 0.], [2., 0., 0.], [0., 3., 0.]];
        let basis = triangle(p, [[0., 0.], [-1., 0.], [0., 1.]]).unwrap();
        assert_eq!(basis, [[-2., 0., 0.], [0., 3., 0.]]);
    }
    #[test]
    fn missing_uv_area_and_non_finite_inputs_remain_explicit() {
        assert!(triangle([[0.; 3]; 3], [[0.; 2]; 3]).is_err());
        assert!(triangle([[f32::NAN; 3]; 3], [[0., 0.], [1., 0.], [0., 1.]]).is_err());
    }
}
