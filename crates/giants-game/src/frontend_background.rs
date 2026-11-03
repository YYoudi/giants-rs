//! Frontend presentation preview from the intro director and its logo binding.
//! This is not a campaign VM or a validated normal-playback clock.
use crate::animation::{Animation, Camera, Evaluator};
use std::path::Path;

pub struct Segment {
    pub animation: Animation,
    pub next: Option<String>,
    pub camera: String,
    pub anchor: String,
    pub socket: String,
    pub class: i32,
}
pub struct Background {
    pub world: std::sync::Arc<crate::WorldAsset>,
    pub segments: Vec<Segment>,
    pub logo: crate::LogoAsset,
    pub textures: Vec<crate::LogoTexture>,
    pub logo_animation: Animation,
    pub scale: f32,
    pub sun: Sun,
    pub lenses: Vec<Lens>,
}
pub struct Lens {
    pub class: u32,
    pub mesh: giants_formats::gbs::Mesh,
    pub texture: giants_formats::tga::RgbImage,
    /// Serialized order: exponents/opacity/responses/scale/wave/rotation.
    pub values: [f32; 10],
}
fn lenses(
    root: &Path,
    world: &crate::WorldAsset,
    registry: &crate::catalog_registry::WorldClassRegistry,
) -> Result<Vec<Lens>, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(root.join("Bin/w_intro_island.bin"))?;
    let mut result = Vec::new();
    for &(at, opcode) in &world.config.opcodes {
        if !matches!(opcode, 119 | 120) {
            continue;
        }
        let record = bytes.get(at..at + 49).ok_or("truncated intro lens")?;
        if record[0] != opcode {
            return Err("intro lens command mismatch".into());
        }
        let class = u32::from_le_bytes(record[1..5].try_into()?);
        if !matches!((opcode, class), (119, 1102) | (120, 1130)) {
            return Err("intro lens outside acquired class/pass profile".into());
        }
        if u32::from_le_bytes(record[5..9].try_into()?) != 0 {
            return Err("unsupported intro lens mode".into());
        }
        let mut values = [0.; 10];
        for (v, raw) in values.iter_mut().zip(record[9..].as_chunks::<4>().0) {
            *v = f32::from_le_bytes(*raw);
        }
        if values.iter().any(|v| !v.is_finite()) || values[..9].iter().any(|v| *v < 0.) {
            return Err("unsupported intro lens scalar".into());
        }
        let definition = registry
            .first_registered(class as i32)
            .ok_or("intro lens class")?;
        let models = definition.models.as_ref().map_err(|e| e.to_string())?;
        if models.len() != 1 {
            return Err("intro lens model count".into());
        }
        let name = models[0].name.as_ref().ok_or("intro lens model name")?;
        let archive_bytes = std::fs::read(definition.catalog.with_extension("gzp"))?;
        let archive = giants_formats::gzp::Archive::parse(&archive_bytes)?;
        let mesh = giants_formats::gbs::parse(&archive.read(&format!("{name}.gbs"))?)?;
        if mesh.subobjects.len() != 1 {
            return Err("intro lens part count".into());
        }
        let texture = giants_formats::tga::decode(
            &archive.read(&format!("{}.tga", mesh.subobjects[0].material.texture))?,
        )?;
        result.push(Lens {
            class,
            values,
            mesh,
            texture,
        });
    }
    Ok(result)
}
pub struct Sun {
    pub mesh: giants_formats::gbs::Mesh,
    pub texture: giants_formats::tga::RgbImage,
    pub scale: f32,
}
impl Sun {
    fn load(
        root: &Path,
        world: &crate::WorldAsset,
        registry: &crate::catalog_registry::WorldClassRegistry,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let name = &world
            .config
            .sun_glare
            .as_ref()
            .ok_or("intro solar effect missing")?
            .effect;
        let effects =
            giants_formats::effect_image::parse(&std::fs::read(root.join("Bin/fx.bin"))?)?;
        let effect = effects
            .find(name)
            .ok_or("intro solar effect symbol missing")?
            .1;
        let program = effects.program(effect.offset)?;
        if program.records.iter().map(|r| r.opcode).collect::<Vec<_>>() != [3, 6, 27, 73, 7, 74, 2]
        {
            return Err("intro solar disk outside acquired program".into());
        }
        let class = i32::from_le_bytes(program.records[0].bytes[1..5].try_into()?);
        let scale = f32::from_le_bytes(program.records[2].bytes[1..5].try_into()?);
        if !scale.is_finite() || scale <= 0. {
            return Err("invalid solar disk scale".into());
        }
        let definition = registry
            .first_registered(class)
            .ok_or("solar disk class missing")?;
        let models = definition.models.as_ref().map_err(|e| e.to_string())?;
        if models.len() != 1 {
            return Err("solar disk requires one model".into());
        }
        let model = models[0].name.as_ref().ok_or("solar model name missing")?;
        let bytes = std::fs::read(definition.catalog.with_extension("gzp"))?;
        let archive = giants_formats::gzp::Archive::parse(&bytes)?;
        let mesh = giants_formats::gbs::parse(&archive.read(&format!("{model}.gbs"))?)?;
        if mesh.subobjects.len() != 1 || mesh.subobjects[0].material.flags != 0x40000000 {
            return Err("solar disk requires acquired opaque texture pass".into());
        }
        let texture = giants_formats::tga::decode(
            &archive.read(&format!("{}.tga", mesh.subobjects[0].material.texture))?,
        )?;
        Ok(Self {
            mesh,
            texture,
            scale,
        })
    }
    /// Camera-facing source basis measured in the menu's opaque disk draw.
    pub fn matrix(&self, view: [f32; 16], position: [f32; 3]) -> [f32; 16] {
        let mut result = [0.; 16];
        for i in 0..3 {
            result[i] = view[i * 4] * self.scale;
            result[4 + i] = -view[i * 4 + 2] * self.scale;
            result[8 + i] = view[i * 4 + 1] * self.scale;
            result[12 + i] = position[i];
        }
        result[15] = 1.;
        result
    }
}
fn label(
    bytes: &[u8],
    at: u32,
    pool: std::ops::Range<usize>,
) -> Result<String, Box<dyn std::error::Error>> {
    if !pool.contains(&(at as usize)) {
        return Err("intro label outside pool".into());
    }
    let raw = bytes
        .get(at as usize..pool.end)
        .ok_or("intro label offset")?;
    let end = raw
        .iter()
        .take(128)
        .position(|b| *b == 0)
        .ok_or("intro label end")?;
    if end == 0 || !raw[..end].is_ascii() {
        return Err("intro label profile".into());
    }
    Ok(std::str::from_utf8(&raw[..end])?.to_owned())
}
impl Segment {
    fn load(root: &Path, name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let bytes = std::fs::read(crate::flick::sequence_path(root, name)?)?;
        let metadata = giants_formats::flick::discover_dependencies(&bytes)?;
        if metadata.commands.iter().filter(|c| c.opcode == 3).count() != 1
            || metadata.commands.iter().filter(|c| c.opcode == 4).count() != 1
        {
            return Err("intro presentation requires a single binding and camera".into());
        }
        let end = metadata
            .commands
            .last()
            .ok_or("intro commands missing")?
            .offset
            + 8;
        let pool = end..metadata.animation_region_offset;
        let camera = metadata
            .commands
            .iter()
            .find(|c| c.opcode == 4)
            .ok_or("intro camera request")?;
        let binding = metadata
            .commands
            .iter()
            .find(|c| c.opcode == 3)
            .ok_or("intro logo binding")?;
        if camera.words.len() != 3 || binding.words.len() != 4 {
            return Err("intro presentation declaration shape".into());
        }
        let actor = metadata
            .actors
            .iter()
            .find(|a| a.slot == binding.words[2] as usize)
            .ok_or("intro bound actor")?;
        let result = Self {
            animation: giants_formats::anm::inline_flk(&bytes)?,
            next: metadata.next,
            camera: label(&bytes, camera.words[2], pool.clone())?,
            anchor: label(&bytes, binding.words[1], pool.clone())?,
            socket: label(&bytes, binding.words[3], pool)?,
            class: actor.object_type,
        };
        // Only the single director/default-tree binding is supported here.
        let carrier = metadata
            .actors
            .iter()
            .find(|a| a.slot == binding.words[0] as usize)
            .ok_or("intro carrier slot")?;
        if carrier.object_type != -1
            || carrier.director != camera.words[0] as i32
            || camera.words[1] != 0
        {
            return Err("intro presentation requires another director/tree".into());
        }
        result.camera(result.animation.timeline[0])?;
        if result.animation.timeline[0] >= result.animation.timeline[1] {
            return Err("intro presentation requires a positive timeline span".into());
        }
        Ok(result)
    }
    pub fn camera(&self, tick: i32) -> Result<Camera, giants_formats::Error> {
        Evaluator {
            animation: &self.animation,
        }
        .camera_named_in_context(
            None,
            &self.camera,
            tick,
            crate::cinematic_viewport::Context {
                extent: [1920, 1080],
                fraction: 1.,
            },
        )
    }
    pub fn anchor(&self, tick: i32) -> Result<[f32; 16], giants_formats::Error> {
        let node = self.animation.default_node(&self.anchor)?;
        Evaluator {
            animation: &self.animation,
        }
        .matrix(node.offset, tick)
    }
}

impl Background {
    pub fn load(root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut segments = Vec::new();
        let mut name = "intro_island_1".to_owned();
        let mut seen = std::collections::BTreeSet::new();
        while seen.insert(name.clone()) {
            if segments.len() == 8 {
                return Err("intro presentation chain budget".into());
            }
            let segment = Segment::load(root, &name)?;
            let next = segment.next.clone();
            segments.push(segment);
            match next {
                Some(next) => name = next,
                None => break,
            }
        }
        let classes = crate::catalog_registry::WorldClassRegistry::load(
            &root.join("Bin"),
            "w_intro_island.bin",
        )?;
        let class = classes
            .first_registered(segments[0].class)
            .ok_or("intro logo class")?;
        let model = class.models.as_ref().map_err(|e| e.to_string())?;
        if !model.iter().any(|m| {
            m.name
                .as_ref()
                .is_some_and(|n| n.eq_ignore_ascii_case("Giants_logo_3D"))
        }) {
            return Err("intro bound class is not the supported logo".into());
        }
        let scale = *class.scale.as_ref().map_err(|e| e.to_string())?;
        let archive_bytes = std::fs::read(root.join("Bin/xx_giants_logo_3d.gzp"))?;
        let archive = giants_formats::gzp::Archive::parse(&archive_bytes)?;
        let logo_animation = giants_formats::anm::parse(&archive.read("Giants_logo_3D.anm")?)?;
        for segment in &segments {
            if segment.class != segments[0].class {
                return Err("intro logo class changes".into());
            }
            logo_animation.default_node(&segment.socket)?;
        }
        let world = std::sync::Arc::new(crate::load_world(root, "intro")?);
        let sun = Sun::load(root, &world, &classes)?;
        let lenses = lenses(root, &world, &classes)?;
        Ok(Self {
            world,
            sun,
            lenses,
            logo: crate::load_logo(root)?,
            textures: crate::load_logo_textures(root)?,
            segments,
            logo_animation,
            scale,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::label;
    #[test]
    fn intro_labels_cannot_point_into_commands_or_animation() {
        let bytes = b"command\0camera\0animation\0";
        assert_eq!(label(bytes, 8, 8..15).unwrap(), "camera");
        assert!(label(bytes, 0, 8..15).is_err());
        assert!(label(bytes, 15, 8..15).is_err());
        assert!(label(bytes, 8, 8..14).is_err());
        assert!(label(bytes, 14, 8..15).is_err());
    }
}
