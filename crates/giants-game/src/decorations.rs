//! Source placements for two measured static models: well and shrub.
use std::path::Path;
pub struct Decoration {
    pub class: i32,
    pub source_offset: usize,
    pub mesh: giants_formats::gbs::Mesh,
    pub matrix: [f32; 16],
    pub textures: Vec<crate::LogoTexture>,
    pub alpha: std::collections::BTreeSet<String>,
}
pub fn load(
    root: &Path,
    world: &crate::WorldAsset,
) -> Result<Vec<Decoration>, Box<dyn std::error::Error>> {
    let mission = crate::props::first_mission(root, world)?;
    load_placements(root, &mission.placements.iter().collect::<Vec<_>>())
}
pub fn load_planned(
    root: &Path,
    plan: &crate::instances::Plan,
) -> Result<Vec<Decoration>, Box<dyn std::error::Error>> {
    load_placements(
        root,
        &plan
            .entries
            .iter()
            .map(|entry| &entry.serialized)
            .collect::<Vec<_>>(),
    )
}
fn load_placements(
    root: &Path,
    declarations: &[&giants_formats::mission::Placement],
) -> Result<Vec<Decoration>, Box<dyn std::error::Error>> {
    let mut result = Vec::new();
    for (class, stem) in [(516i32, "xx_smartie_stuff"), (992, "xx_plants")] {
        let placements: Vec<_> = declarations
            .iter()
            .copied()
            .filter(|p| p.object_type == class as u32)
            .collect();
        if placements.is_empty() {
            continue;
        }
        for p in placements {
            let catalog = crate::read_asset(root.join("Bin").join(format!("{stem}.bin")))?;
            let model = giants_formats::cast::first_model(&catalog, class)?;
            let scale = match p.scale {
                Some(scale) => scale,
                None => giants_formats::cast::class_scale(&catalog, class)?,
            };
            // The unrotated unit-scale well retains the native initialized
            // identity matrix, including positive zero words.
            let [pitch, roll] = p.pitch_roll.unwrap_or([0., 0.]);
            let matrix = if [p.yaw, pitch, roll] == [0.; 3] && scale == 1. {
                [
                    1.,
                    0.,
                    0.,
                    p.position[0],
                    0.,
                    1.,
                    0.,
                    p.position[1],
                    0.,
                    0.,
                    1.,
                    p.position[2],
                    0.,
                    0.,
                    0.,
                    1.,
                ]
            } else {
                crate::object_transform::euler(p.position, [p.yaw, pitch, roll], scale)?
            };
            let data = crate::read_asset(root.join("Bin").join(format!("{stem}.gzp")))?;
            let archive = giants_formats::gzp::Archive::parse(&data)?;
            let mesh = giants_formats::gbs::parse(&archive.read(&format!("{model}.gbs"))?)?;
            if mesh.subobjects.iter().any(|s| {
                !matches!(s.material.flags, 0x40000020 | 0xc0000020) || s.material.falloff != 0.
            }) {
                return Err("Unmeasured decoration material".into());
            }
            let data = crate::read_asset(root.join("Bin/w_Story1.gzp"))?;
            let fallback = giants_formats::gzp::Archive::parse(&data)?;
            let wall_data = crate::read_asset(root.join("Bin/xx_bigwalls.gzp"))?;
            let walls = giants_formats::gzp::Archive::parse(&wall_data)?;
            let names: std::collections::BTreeSet<_> = mesh
                .subobjects
                .iter()
                .map(|s| s.material.texture.clone())
                .collect();
            let mut textures = Vec::new();
            let mut alpha = std::collections::BTreeSet::new();
            for name in names {
                let member = format!("{name}.tga");
                let source = if archive
                    .entries()
                    .iter()
                    .any(|e| e.name.eq_ignore_ascii_case(&member))
                {
                    &archive
                } else if walls
                    .entries()
                    .iter()
                    .any(|e| e.name.eq_ignore_ascii_case(&member))
                {
                    &walls
                } else {
                    &fallback
                };
                let image = giants_formats::tga::decode(&source.read(&member)?)?;
                let (words, rgba) = if let Some(a) = &image.alpha {
                    alpha.insert(name.to_ascii_lowercase());
                    let words = giants_formats::pixels::pack_argb4444(&image.pixels, a)?;
                    let rgba = giants_formats::pixels::rgba8_argb4444(&words);
                    (words, rgba)
                } else {
                    let words = giants_formats::pixels::pack_rgb565(&image.pixels)?;
                    let rgba = giants_formats::pixels::rgba8(&words);
                    (words, rgba)
                };
                textures.push(crate::LogoTexture {
                    name,
                    width: image.width,
                    height: image.height,
                    words,
                    rgba,
                });
            }
            result.push(Decoration {
                class,
                source_offset: p.source_offset,
                mesh,
                matrix,
                textures,
                alpha,
            });
        }
    }
    Ok(result)
}
