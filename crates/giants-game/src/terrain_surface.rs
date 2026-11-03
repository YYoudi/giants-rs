//! Initial, unlit two-texture terrain for the debug hardware rendering profile.
//! Images use the controlled unsigned RGBA conversion with decoded row order.
//! Software fallback, overlays, dynamic shadow/color updates and device resets
//! are separate. Successful CPU preparation is not mission readiness.
use giants_formats::{gzp::Archive, world::WorldConfig};

pub struct Level {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
pub struct Layer {
    pub name: String,
    pub reciprocal: f32,
    pub derived_from_base: bool,
    pub levels: Vec<Level>,
}
pub struct Prepared {
    pub layers: [Option<Layer>; 3],
    pub direction_colors: Vec<u32>,
    pub context_direction: [f32; 3],
    /// Emulate the acquired hardware unlit profile's second color channel.
    /// This is a chosen reference execution profile, not Vulkan/DXGI detection.
    pub secondary_color: bool,
}

pub fn prepare_debug_hardware(
    archive: &Archive<'_>,
    terrain: &giants_formats::gti::Terrain,
    config: &WorldConfig,
) -> Result<Option<Prepared>, Box<dyn std::error::Error>> {
    prepare_debug_hardware_with(|name| Ok(archive.read(name)?), terrain, config)
}

/// Shared preparation with an explicit resource namespace. The archive-scoped
/// controller above remains available for isolated material comparisons.
pub fn prepare_debug_hardware_with(
    read: impl Fn(&str) -> Result<Vec<u8>, Box<dyn std::error::Error>>,
    terrain: &giants_formats::gti::Terrain,
    config: &WorldConfig,
) -> Result<Option<Prepared>, Box<dyn std::error::Error>> {
    if config
        .prefix
        .additional_materials
        .iter()
        .all(Option::is_none)
    {
        return Ok(None);
    }
    let context_direction = crate::lighting::terrain_context_direction(config)?;
    let direction = crate::terrain_direction::prepare_declared(terrain, config, context_direction)?;
    let mut layers = Vec::new();
    for (base, additional) in config
        .prefix
        .materials
        .iter()
        .zip(&config.prefix.additional_materials)
    {
        let Some(additional) = additional else {
            layers.push(None);
            continue;
        };
        let source =
            giants_formats::tga::decode_typed(&read(&format!("{}.tga", additional.name))?)?;
        let [width, height] = source.dimensions();
        if width != height || !width.is_power_of_two() || !(2..=512).contains(&width) {
            return Err("terrain additional image outside controlled square profile".into());
        }
        let mut levels = Vec::new();
        let mut size = width;
        loop {
            let (image, _) = crate::bump_image::terrain_normal_map_level(
                &source,
                false,
                crate::bump_image::ByteRounding::NearestEven,
                [size, size],
            )?;
            let rgba = image
                .pixels
                .as_chunks::<3>()
                .0
                .iter()
                .flat_map(|p| [p[0], p[1], p[2], 255])
                .collect();
            levels.push(Level {
                width: size,
                height: size,
                rgba,
            });
            if size == 1 {
                break;
            }
            size /= 2;
        }
        layers.push(Some(Layer {
            name: additional.name.clone(),
            reciprocal: additional.reciprocal,
            derived_from_base: base
                .as_ref()
                .is_some_and(|b| b.name.eq_ignore_ascii_case(&additional.name)),
            levels,
        }));
    }
    Ok(Some(Prepared {
        layers: layers
            .try_into()
            .map_err(|_| "three terrain material slots required")?,
        direction_colors: direction.colors,
        context_direction,
        secondary_color: true,
    }))
}
