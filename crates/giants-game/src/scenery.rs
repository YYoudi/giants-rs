//! Measured static class1053 in the first Story4 cinematic mission.
use std::path::Path;

pub struct Scenery {
    pub source_offset: usize,
    pub model: String,
    pub mesh: giants_formats::gbs::Mesh,
    pub normals: Vec<[f32; 3]>,
    pub textures: Vec<crate::LogoTexture>,
    pub alpha_textures: std::collections::BTreeSet<String>,
    pub material_passes: Vec<crate::render_parts::Pass>,
}

pub fn load_first(
    root: &Path,
    world: &crate::WorldAsset,
) -> Result<Scenery, Box<dyn std::error::Error>> {
    let plan = crate::instances::Plan::load_first(root, world)?;
    load_planned(&plan)
}
pub fn load_planned(plan: &crate::instances::Plan) -> Result<Scenery, Box<dyn std::error::Error>> {
    let instance = plan.unique_class(1053)?;
    let input = instance.source.as_ref().map_err(Clone::clone)?;
    // The measured scenery profile submits authored world coordinates unchanged.
    if input.position != [0.; 3] || input.yaw != 0. || input.scale != 1. {
        return Err("Static scenery outside measured identity profile".into());
    }
    let geometry = instance.geometry(
        0,
        crate::scene_resources::TextureEncoding::Rgb565OrArgb4444,
        crate::instances::Adjustment::Authored,
    )?;
    let source = geometry.model.ok_or("Static scenery model slot is empty")?;
    let material_passes = source.material_passes();
    if material_passes.contains(&crate::render_parts::Pass::ObjectOrMaterial) {
        return Err("Static scenery outside measured material routing profile".into());
    }
    let model = source.name;
    if model != "story4" {
        return Err("Static scenery outside measured model profile".into());
    }
    let mesh = source.mesh;
    if mesh.subobjects.len() != 18
        || mesh
            .subobjects
            .iter()
            .any(|s| !matches!(s.material.flags, 0x40000020 | 0x40000000))
    {
        return Err("Unmeasured static scenery material profile".into());
    }
    let normals = mesh.render_normals()?;
    let textures = source.textures;
    let alpha_textures = source.alpha_textures;
    Ok(Scenery {
        source_offset: input.source_offset,
        model,
        mesh,
        normals,
        textures,
        alpha_textures,
        material_passes,
    })
}

/// The native material upload uses truncated, saturated RGB vector length as alpha.
pub fn material_color(word: u32) -> [f32; 4] {
    let rgb = std::array::from_fn::<_, 3, _>(|i| (word >> (i * 8)) & 255);
    let alpha = ((rgb.iter().map(|v| v * v).sum::<u32>() as f64).sqrt() as u32).min(255);
    [rgb[0], rgb[1], rgb[2], alpha].map(|v| v as f32 * (1. / 255.))
}
