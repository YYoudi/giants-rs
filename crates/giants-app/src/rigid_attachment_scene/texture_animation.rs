//! Numbered frame resources for the supported unlit burst draw path.
use giants_game::{
    LogoTexture,
    effect_texture_animation::Configuration,
    scene_resources::{ModelAsset, SceneResources},
};

pub(super) fn load(
    configuration: Configuration,
    model: &ModelAsset,
    resources: &SceneResources,
) -> Result<Vec<LogoTexture>, String> {
    let base = &model
        .mesh
        .subobjects
        .first()
        .ok_or("animated texture has no first material")?
        .material
        .texture;
    let names = configuration.numbered_names(base).map_err(str::to_owned)?;
    let alpha = model.alpha_textures.contains(&base.to_ascii_lowercase());
    if model.mesh.subobjects.iter().any(|p| {
        model
            .alpha_textures
            .contains(&p.material.texture.to_ascii_lowercase())
            != alpha
    }) {
        return Err("animated texture spans mixed alpha material profiles".into());
    }
    names
        .into_iter()
        .map(|name| {
            let request = resources.image_request(&name);
            let giants_formats::tga::Image::TrueColor(image) = request.decoded? else {
                return Err("animated texture is not true color".into());
            };
            if image.alpha.is_some() != alpha {
                return Err("animated texture changes source upload profile".into());
            }
            let (words, rgba) = if let Some(alpha) = &image.alpha {
                let words = giants_formats::pixels::pack_argb4444(&image.pixels, alpha)
                    .map_err(|e| e.to_string())?;
                let rgba = giants_formats::pixels::rgba8_argb4444(&words);
                (words, rgba)
            } else {
                let words = giants_formats::pixels::pack_rgb565(&image.pixels)
                    .map_err(|e| e.to_string())?;
                let rgba = giants_formats::pixels::rgba8(&words);
                (words, rgba)
            };
            Ok(LogoTexture {
                name,
                width: image.width,
                height: image.height,
                words,
                rgba,
            })
        })
        .collect()
}
