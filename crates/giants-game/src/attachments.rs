//! First-flick rigid attachments. Their ANM reference is independent of the director.
use crate::animation::Evaluator;
use giants_formats::{Error, anm::Animation, cast, gbs, gzp::Archive};
use std::path::Path;

pub struct Attachment {
    pub slot: usize,
    pub parent_slot: usize,
    pub parent_tree: String,
    pub parent_node: String,
    pub child_node: String,
    pub model: String,
    pub mesh: gbs::Mesh,
    pub animation: Animation,
    pub scale: f32,
    pub parent_scale: f32,
    pub archive_name: &'static str,
}
fn flags(parent: u32, local: u32) -> u32 {
    let union = parent | local;
    (union ^ ((union ^ parent ^ local) & 8))
        | if parent & 2 != 0 && local & 1 != 0 {
            4
        } else {
            0
        }
}
/// Scalar matrix composition with a float store after each output component.
pub fn multiply(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    std::array::from_fn(|i| {
        (0..4)
            .map(|k| f64::from(a[i / 4 * 4 + k]) * f64::from(b[k * 4 + i % 4]))
            .sum::<f64>() as f32
    })
}
pub(crate) fn inverse(m: [f32; 16], flags: u32) -> Result<[f32; 16], Error> {
    if flags & 6 != 0 {
        return Err(Error(
            "scaled attachment reference inverse is not yet measured",
        ));
    }
    let mut result = [0.; 16];
    result[15] = 1.;
    for r in 0..3 {
        for c in 0..3 {
            result[r * 4 + c] = if flags & 15 == 0 {
                if r == c { 1. } else { 0. }
            } else {
                m[c * 4 + r]
            };
        }
    }
    if flags & 16 != 0 {
        for r in 0..3 {
            result[r * 4 + 3] = -((f64::from(result[r * 4 + 1]) * f64::from(m[7])
                + f64::from(result[r * 4]) * f64::from(m[3]))
                + f64::from(result[r * 4 + 2]) * f64::from(m[11]))
                as f32;
        }
    }
    Ok(result)
}
impl Attachment {
    pub fn textures(
        &self,
        root: &Path,
    ) -> Result<Vec<crate::LogoTexture>, Box<dyn std::error::Error>> {
        let bytes = crate::read_asset(root.join("Bin").join(self.archive_name))?;
        let archive = Archive::parse(&bytes)?;
        let fallback_bytes = crate::read_asset(root.join("Bin/xx_bigwalls.gzp"))?;
        let fallback = Archive::parse(&fallback_bytes)?;
        let mut result = Vec::new();
        let names: std::collections::BTreeSet<_> = self
            .mesh
            .subobjects
            .iter()
            .map(|s| s.material.texture.clone())
            .collect();
        for name in names {
            let member = format!("{name}.tga");
            let source = if archive
                .entries()
                .iter()
                .any(|e| e.name.eq_ignore_ascii_case(&member))
            {
                &archive
            } else {
                &fallback
            };
            let image = giants_formats::tga::decode(&source.read(&member)?)?;
            let words = giants_formats::pixels::pack_rgb565(&image.pixels)?;
            let rgba = giants_formats::pixels::rgba8(&words);
            result.push(crate::LogoTexture {
                name,
                width: image.width,
                height: image.height,
                words,
                rgba,
            });
        }
        Ok(result)
    }
    pub fn matrix(&self, director: &Animation, tick: i32) -> Result<([f32; 16], u32), Error> {
        binding_matrix(
            Binding {
                parent_tree: Some(&self.parent_tree),
                parent_node: &self.parent_node,
                child_node: Some(&self.child_node),
                scale: self.scale,
                parent_scale: self.parent_scale,
            },
            director,
            Some(&self.animation),
            tick,
        )
    }
}
pub struct Binding<'a> {
    pub parent_tree: Option<&'a str>,
    pub parent_node: &'a str,
    pub child_node: Option<&'a str>,
    pub scale: f32,
    pub parent_scale: f32,
}
/// Initial matrix for an unnamed actor without parent, handles or local offset.
/// Resource animation coordinates belong to its geometry, not a director tree.
pub fn unparented_matrix(scale: f32) -> Result<([f32; 16], u32), Error> {
    if !scale.is_finite() || scale <= 0. {
        return Err(Error("unparented scale outside positive finite profile"));
    }
    Ok((
        [
            scale, 0., 0., 0., 0., scale, 0., 0., 0., 0., scale, 0., 0., 0., 0., 1.,
        ],
        2,
    ))
}
/// Source attachment profile: director parent, relative declared scale and
/// optional static child-handle inverse. Dynamic child timing is not inferred.
pub fn binding_matrix(
    binding: Binding<'_>,
    director: &Animation,
    animation: Option<&Animation>,
    tick: i32,
) -> Result<([f32; 16], u32), Error> {
    let child = if let Some(name) = binding.child_node {
        let animation = animation.ok_or(Error("attachment child animation missing"))?;
        let node = animation.default_node(name)?.offset;
        Some(crate::animation_references::node_matrix(
            animation, node, 0,
        )?)
    } else {
        None
    };
    binding_matrix_with_reference(binding, director, tick, child)
}
/// Compose the same source link with a caller-evaluated child reference.
/// Its clock, descriptor selection and enabled gate belong to the caller.
pub fn binding_matrix_with_reference(
    binding: Binding<'_>,
    director: &Animation,
    tick: i32,
    child: Option<crate::procedural_aim::Matrix>,
) -> Result<([f32; 16], u32), Error> {
    if binding.child_node.is_some() != child.is_some() {
        return Err(Error(
            "attachment child reference input differs from binding",
        ));
    }
    let parent_node = director
        .named_node(binding.parent_tree, binding.parent_node)?
        .offset;
    let parent = Evaluator {
        animation: director,
    };
    let ratio = binding.scale / binding.parent_scale;
    if !ratio.is_finite() || ratio <= 0. {
        return Err(Error(
            "attachment scale ratio outside positive finite profile",
        ));
    }
    let scale = [
        ratio, 0., 0., 0., 0., ratio, 0., 0., 0., 0., ratio, 0., 0., 0., 0., 1.,
    ];
    let matrix = multiply(parent.matrix(parent_node, tick)?, scale);
    let parent_flags = flags(parent.matrix_flags(parent_node, tick)?, 2);
    if let Some(child) = child {
        let child_flags = child.flags;
        Ok((
            multiply(matrix, inverse(child.values, child_flags)?),
            flags(parent_flags, child_flags),
        ))
    } else {
        Ok((matrix, parent_flags))
    }
}
pub fn load_first(root: &Path) -> Result<Vec<Attachment>, Box<dyn std::error::Error>> {
    load_sequence(root, "story4_T_square_2")
}
pub fn load_sequence(
    root: &Path,
    sequence: &str,
) -> Result<Vec<Attachment>, Box<dyn std::error::Error>> {
    load_sequence_profile(root, sequence, false)
}
pub fn load_sequence_preview(
    root: &Path,
    sequence: &str,
) -> Result<Vec<Attachment>, Box<dyn std::error::Error>> {
    load_sequence_profile(root, sequence, true)
}
fn load_sequence_profile(
    root: &Path,
    sequence: &str,
    preview: bool,
) -> Result<Vec<Attachment>, Box<dyn std::error::Error>> {
    let flk = crate::read_asset(crate::flick::sequence_path(root, sequence)?)?;
    let actors = cast::first_flick(&flk)?;
    let mut result = Vec::new();
    let links = if preview {
        cast::preview_attachments_for_classes(&flk, &[665, 21, 240, 669])?
    } else {
        cast::first_attachments(&flk)?
    };
    for link in links {
        let declaration = &actors[link.slot];
        if preview && !matches!(declaration.object_type, 665 | 21 | 240 | 669) {
            eprintln!(
                "cinematic_preview_omission sequence={sequence} attachment_slot={} class={} attachment_unimplemented=true",
                declaration.slot, declaration.object_type
            );
            continue;
        }
        let (catalog, archive_name) = match declaration.object_type {
            665 => ("xx_smborjoyzee_flick.bin", "xx_smborjoyzee.gzp"),
            21 => ("xx_mecc_flick.bin", "xx_mecc_flick.gzp"),
            240 => ("xx_reaperman.bin", "xx_reaperman.gzp"),
            669 => ("xx_reaper_flick.bin", "xx_raik.gzp"),
            _ => {
                return Err(format!(
                    "Unmeasured attachment class {} at slot {} in {sequence}",
                    declaration.object_type, declaration.slot
                )
                .into());
            }
        };
        let catalog = crate::read_asset(root.join("Bin").join(catalog))?;
        let (model, animation_name, scale) =
            cast::attachment_resource(&catalog, declaration.object_type)?;
        let parent = &actors[link.parent_slot];
        let parent_scale = cast::class_scale(&catalog, parent.object_type)?;
        let bytes = crate::read_asset(root.join("Bin").join(archive_name))?;
        let archive = Archive::parse(&bytes)?;
        let mut mesh = gbs::parse(&archive.read(&format!("{model}.gbs"))?)?;
        let animation = giants_formats::anm::parse(&archive.read(&animation_name)?)?;
        // The measured sword submits its rigid bind pose, not the rounded GBS
        // positions. Other attachment classes retain their separately measured path.
        if declaration.object_type == 669 {
            let skin = giants_formats::skn::parse(&archive.read(&format!("{model}.skn"))?)?;
            let tree = animation
                .trees
                .iter()
                .find(|t| t.offset == animation.default_tree)
                .ok_or(Error("missing sword bind tree"))?;
            let positions = crate::skinning::positions(&skin, &animation, &tree.name, 0)?;
            if positions.len() != mesh.positions.len() {
                return Err(Error("sword skin/mesh vertex count mismatch").into());
            }
            mesh.positions = positions;
        }
        result.push(Attachment {
            slot: link.slot,
            parent_slot: link.parent_slot,
            parent_tree: parent
                .name
                .clone()
                .ok_or("Attachment parent has no director tree")?,
            parent_node: link.parent_node,
            child_node: link.child_node,
            model,
            mesh,
            animation,
            scale,
            parent_scale,
            archive_name,
        });
    }
    Ok(result)
}
