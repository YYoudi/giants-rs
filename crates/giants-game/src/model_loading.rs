//! CPU model preparation with an optional source skeleton and explicit normal policy.
//! Texture lookup/upload, actor creation and live model selection remain separate.
use giants_formats::{Error, gbs, skn};

pub struct Members {
    pub geometry: String,
    pub skeleton: String,
}
impl Members {
    pub fn from_stem(stem: &str) -> Result<Self, Error> {
        if stem.is_empty()
            || stem.len() > 127
            || !stem.is_ascii()
            || stem
                .bytes()
                .any(|b| b < 32 || matches!(b, b'/' | b'\\' | b':'))
        {
            return Err(Error("unsupported model resource stem"));
        }
        Ok(Self {
            geometry: format!("{stem}.gbs"),
            skeleton: format!("{stem}.skn"),
        })
    }
}
#[derive(Debug)]
pub enum Need {
    SkeletonVertexCount { geometry: usize, skeleton: usize },
    SkeletonGroupCount { geometry: usize, skeleton: usize },
}
pub struct Part {
    pub colors: [u32; 4],
    pub centroid: [f32; 3],
}
pub struct Prepared {
    pub mesh: gbs::Mesh,
    pub skin: Option<skn::Skin>,
    pub keep_normal_tables: bool,
    pub vertex_capacity: usize,
    pub aggregate_flags: u32,
    pub triangle_count: usize,
    pub groups: Vec<[u32; 9]>,
    pub parts: Vec<Part>,
    pub needs: Vec<Need>,
}

/// Loaded material color; the source byte's unused high component is ignored.
/// The first two component stores and the wide third component are distinct.
pub fn loaded_color(source: u32) -> u32 {
    let raw = source.to_le_bytes();
    let scale = f64::from(f32::from_bits(0x3b808081));
    let r = f64::from((f64::from(raw[0]) * scale) as f32).min(1.);
    let g = f64::from((f64::from(raw[1]) * scale) as f32).min(1.);
    let b = (f64::from(raw[2]) * scale).min(1.);
    let alpha = ((b * b + g * g) + r * r).sqrt().min(1.);
    ((r * 255.).trunc() as u32)
        | (((g * 255.).trunc() as u32) << 8)
        | (((b * 255.).trunc() as u32) << 16)
        | (((alpha * 255.).trunc() as u32) << 24)
}

impl Prepared {
    pub fn decode(
        geometry: &[u8],
        skeleton: Option<&[u8]>,
        keep_normals: bool,
    ) -> Result<Self, Error> {
        let mesh = gbs::parse(geometry)?;
        let skin = skeleton.map(skn::parse).transpose()?;
        Self::from_decoded(mesh, skin, keep_normals)
    }
    pub fn from_decoded(
        mesh: gbs::Mesh,
        skin: Option<skn::Skin>,
        keep_normals: bool,
    ) -> Result<Self, Error> {
        // In the bounded 6/7 format, the nonempty render table has normal
        // indices exactly when the source's normal-table bit is present.
        let keep_normal_tables = keep_normals && !mesh.normal_indices.is_empty();
        let mut groups: Vec<[u32; 9]> = mesh
            .groups
            .iter()
            .map(|raw| {
                let mut group = [0; 9];
                group[..5].copy_from_slice(raw);
                group
            })
            .collect();
        let mut aggregate_flags = 0;
        let mut triangle_count = 0;
        let mut parts = Vec::with_capacity(mesh.subobjects.len());
        for (index, part) in mesh.subobjects.iter().enumerate() {
            if part.vertex_count == 0 {
                return Err(Error("empty model part centroid unsupported"));
            }
            let mut centroid = [0_f32; 3];
            for &vertex in &mesh.indices[part.vertex_start..part.vertex_start + part.vertex_count] {
                for (sum, value) in centroid.iter_mut().zip(mesh.positions[usize::from(vertex)]) {
                    *sum = (f64::from(*sum) + f64::from(value)) as f32;
                }
            }
            let reciprocal = 1. / part.vertex_count as f64;
            centroid = centroid.map(|v| (f64::from(v) * reciprocal) as f32);
            let group = &mut groups[part.group];
            group[6] = group[6]
                .checked_add(part.triangles.len() as u32)
                .ok_or(Error("model group triangle overflow"))?;
            if group[8] == 0 {
                group[7] = index as u32;
            }
            group[8] += 1;
            aggregate_flags |= part.material.flags;
            triangle_count += part.triangles.len();
            parts.push(Part {
                colors: part.material.colors.map(loaded_color),
                centroid,
            });
        }
        let mut base = 0_u32;
        for group in &mut groups {
            group[5] = base;
            base = base
                .checked_add(group[6])
                .ok_or(Error("model group prefix overflow"))?;
        }
        let mut vertex_capacity = mesh.positions.len();
        let mut needs = Vec::new();
        if let Some(skin) = &skin {
            let count = skin.objects.iter().map(|o| o.vertex_count).sum::<usize>();
            if count != mesh.positions.len() {
                needs.push(Need::SkeletonVertexCount {
                    geometry: mesh.positions.len(),
                    skeleton: count,
                });
            }
            if skin.objects.len() != mesh.groups.len() {
                needs.push(Need::SkeletonGroupCount {
                    geometry: mesh.groups.len(),
                    skeleton: skin.objects.len(),
                });
            }
            vertex_capacity = vertex_capacity.max(count);
        }
        Ok(Self {
            mesh,
            skin,
            keep_normal_tables,
            vertex_capacity,
            aggregate_flags,
            triangle_count,
            groups,
            parts,
            needs,
        })
    }
    /// An expanded native position buffer has not been populated by this loader.
    /// Keep source geometry intact; do not invent positions for the extra capacity.
    pub fn initial_positions_available(&self) -> bool {
        self.vertex_capacity == self.mesh.positions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resource_stems_cannot_escape_the_resource_namespace() {
        let names = Members::from_stem("synthetic_l0").unwrap();
        assert_eq!(names.geometry, "synthetic_l0.gbs");
        assert_eq!(names.skeleton, "synthetic_l0.skn");
        for name in ["", "../outside", "C:\\outside", "a/b"] {
            assert!(Members::from_stem(name).is_err());
        }
    }
}
