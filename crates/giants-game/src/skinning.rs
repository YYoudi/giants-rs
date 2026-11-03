//! Rigid SKN deformation with the measured SSE bulk/scalar tail rounding.
use crate::animation::Evaluator;
use giants_formats::{Error, anm::Animation, skn::Skin};
use std::collections::BTreeMap;

pub fn positions(
    skin: &Skin,
    animation: &Animation,
    tree: &str,
    tick: i32,
) -> Result<Vec<[f32; 3]>, Error> {
    positions_inner(skin, animation, tree, tick, None, None, None)
}
/// Consume the node bindings prepared by the actor constructor, rather than
/// resolving bone names again during each pose. The skin identity is checked.
pub fn positions_bound(
    skin: &Skin,
    animation: &Animation,
    binding: &crate::animation_template::Binding,
    tick: i32,
) -> Result<Vec<[f32; 3]>, Error> {
    let tree = animation
        .trees
        .iter()
        .find(|t| t.offset == binding.tree)
        .ok_or(Error("prepared skin tree is absent from animation"))?;
    if binding.objects.len() != skin.objects.len() {
        return Err(Error("prepared skin object count changed"));
    }
    positions_inner(skin, animation, &tree.name, tick, None, None, Some(binding))
}
pub fn positions_bound_with_matrices(
    skin: &Skin,
    animation: &Animation,
    binding: &crate::animation_template::Binding,
    matrices: &BTreeMap<usize, [f32; 16]>,
) -> Result<Vec<[f32; 3]>, Error> {
    let tree = animation
        .trees
        .iter()
        .find(|t| t.offset == binding.tree)
        .ok_or(Error("prepared skin tree is absent from animation"))?;
    if binding.objects.len() != skin.objects.len() {
        return Err(Error("prepared skin object count changed"));
    }
    positions_inner(
        skin,
        animation,
        &tree.name,
        0,
        None,
        Some(matrices),
        Some(binding),
    )
}
/// Rigid deformation with explicit source-selected procedural rotations.
pub fn positions_with_post_rotations(
    skin: &Skin,
    animation: &Animation,
    tree: &str,
    tick: i32,
    rotations: &BTreeMap<usize, [f32; 4]>,
) -> Result<Vec<[f32; 3]>, Error> {
    positions_inner(skin, animation, tree, tick, Some(rotations), None, None)
}
pub fn positions_with_matrices(
    skin: &Skin,
    animation: &Animation,
    tree: &str,
    matrices: &BTreeMap<usize, [f32; 16]>,
) -> Result<Vec<[f32; 3]>, Error> {
    positions_inner(skin, animation, tree, 0, None, Some(matrices), None)
}
fn positions_inner(
    skin: &Skin,
    animation: &Animation,
    tree: &str,
    tick: i32,
    rotations: Option<&BTreeMap<usize, [f32; 4]>>,
    prepared: Option<&BTreeMap<usize, [f32; 16]>>,
    binding: Option<&crate::animation_template::Binding>,
) -> Result<Vec<[f32; 3]>, Error> {
    let tree = animation
        .trees
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(tree))
        .ok_or(Error("missing actor animation tree"))?;
    let evaluator = Evaluator { animation };
    let mut matrices = BTreeMap::new();
    let mut result = Vec::new();
    for (object_index, object) in skin.objects.iter().enumerate() {
        let bound = binding.map(|b| &b.objects[object_index]);
        if bound.is_some_and(|b| {
            b.source != object.offset
                || b.vertex_count != object.vertex_count
                || b.clusters.len() != object.clusters.len()
        }) {
            return Err(Error("prepared skin object identity changed"));
        }
        let base = result.len();
        result.resize(base + object.vertex_count, [0.; 3]);
        for (cluster_index, cluster) in object.clusters.iter().enumerate() {
            let node = if let Some(bound) = bound {
                let selected = &bound.clusters[cluster_index];
                if selected.source != cluster.offset {
                    return Err(Error("prepared skin cluster identity changed"));
                }
                selected
                    .node
                    .ok_or(Error("prepared skin bone unresolved"))?
            } else {
                tree.entries
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(&cluster.bone))
                    .ok_or(Error("skin bone missing from actor tree"))?
                    .1
            };
            if let std::collections::btree_map::Entry::Vacant(entry) = matrices.entry(node) {
                entry.insert(if let Some(prepared) = prepared {
                    *prepared
                        .get(&node)
                        .ok_or(Error("missing prepared skin matrix"))?
                } else if let Some(rotations) = rotations {
                    evaluator.matrix_with_post_rotations(node, tick, rotations)?
                } else {
                    evaluator.matrix(node, tick)?
                });
            }
            let matrix = matrices[&node];
            let bulk = if cluster.vertices.len() > 12 {
                cluster.vertices.len() / 4 * 4
            } else {
                0
            };
            for (i, vertex) in cluster.vertices.iter().enumerate() {
                let [x, y, z] = vertex.position;
                result[base + vertex.index] = std::array::from_fn(|r| {
                    let m = &matrix[r * 4..r * 4 + 4];
                    if i < bulk {
                        let sum = x * m[0] + (y * m[1] + z * m[2]);
                        (f64::from(sum) + f64::from(m[3])) as f32
                    } else {
                        ((f64::from(x) * f64::from(m[0]) + f64::from(z) * f64::from(m[2]))
                            + f64::from(y) * f64::from(m[1])
                            + f64::from(m[3])) as f32
                    }
                });
            }
        }
    }
    if !result.iter().flatten().all(|v| v.is_finite()) {
        return Err(Error("non-finite skinned position"));
    }
    Ok(result)
}
