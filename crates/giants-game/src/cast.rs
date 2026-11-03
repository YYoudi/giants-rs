//! Measured first Story4 cast: declarations, body catalogs and rigid skins.
use std::{collections::BTreeMap, path::Path, sync::Arc};
pub struct Body {
    pub model: String,
    pub mesh: giants_formats::gbs::Mesh,
    pub skin: giants_formats::skn::Skin,
    pub textures: Vec<crate::LogoTexture>,
    pub alpha_textures: std::collections::BTreeSet<String>,
    pub source: Option<BodySource>,
}
/// Selected source identities, distinct from a live model or renderer decision.
pub struct BodySource {
    pub slot: usize,
    pub class_catalog: std::path::PathBuf,
    pub spec_catalog: std::path::PathBuf,
    pub mesh: crate::catalog_registry::Location,
    pub skin: crate::catalog_registry::Location,
    pub textures: BTreeMap<String, crate::catalog_registry::Location>,
    pub secondary_textures: BTreeMap<String, crate::scene_resources::ImageRequest>,
}
/// Load through the common, native-checked class and archive lookup rules.
/// Slot and upload encoding remain explicit inspection policies, not live LOD.
pub fn load_registered_body(
    resources: &crate::scene_resources::SceneResources,
    class: i32,
    slot: usize,
) -> Result<Body, Box<dyn std::error::Error>> {
    let binding = resources
        .classes
        .models
        .first_selection(class)
        .ok_or("class not registered")?
        .as_ref()
        .map_err(Clone::clone)?;
    let model = resources
        .model_slot_for_body(
            class,
            slot,
            crate::scene_resources::TextureEncoding::Rgb565OrArgb4444,
        )?
        .ok_or("selected body slot has no geometry")?;
    let mesh_member = format!("{}.gbs", model.name);
    let skin_member = format!("{}.skn", model.name);
    let skin = giants_formats::skn::parse(
        &resources
            .archives
            .read_bin(&skin_member, &resources.language)?,
    )?;
    let prepared = crate::model_loading::Prepared::from_decoded(model.mesh, Some(skin), true)?;
    if !prepared.needs.is_empty() {
        return Err("registered body skin/mesh groups or vertex counts differ".into());
    }
    let skin = prepared
        .skin
        .ok_or("registered skinned body has no skeleton")?;
    let mut textures = BTreeMap::new();
    for texture in &model.textures {
        textures.insert(
            texture.name.clone(),
            resources
                .archives
                .select_bin(&format!("{}.tga", texture.name), &resources.language)?
                .clone(),
        );
    }
    let mut secondary_textures = BTreeMap::new();
    for part in &prepared.mesh.subobjects {
        let name = &part.material.bump_texture;
        if !name.is_empty() {
            secondary_textures
                .entry(name.to_ascii_lowercase())
                .or_insert_with(|| resources.image_request(name));
        }
    }
    let source = BodySource {
        slot,
        class_catalog: binding.class_catalog.clone(),
        spec_catalog: binding.spec_catalog.clone(),
        mesh: resources
            .archives
            .select_bin(&mesh_member, &resources.language)?
            .clone(),
        skin: resources
            .archives
            .select_bin(&skin_member, &resources.language)?
            .clone(),
        textures,
        secondary_textures,
    };
    Ok(Body {
        model: model.name,
        mesh: prepared.mesh,
        skin,
        textures: model.textures,
        alpha_textures: model.alpha_textures,
        source: Some(source),
    })
}

pub struct BodyRequest {
    pub offset: usize,
    pub slot: usize,
    pub class: i32,
    pub tree: Option<String>,
    pub result: Result<&'static str, String>,
}
pub struct RegisteredCast {
    pub actors: Vec<Actor>,
    pub requests: Vec<BodyRequest>,
}
/// One cache shared across a source chain. A failed body remains attributable.
pub struct RegisteredBodies<'a> {
    resources: &'a crate::scene_resources::SceneResources,
    bodies: BTreeMap<i32, Result<Arc<Body>, String>>,
}
impl<'a> RegisteredBodies<'a> {
    pub fn new(resources: &'a crate::scene_resources::SceneResources) -> Self {
        Self {
            resources,
            bodies: BTreeMap::new(),
        }
    }
    pub fn sequence(&mut self, bytes: &[u8]) -> Result<RegisteredCast, giants_formats::Error> {
        let metadata = giants_formats::flick::discover_dependencies(bytes)?;
        let animation = giants_formats::anm::inline_flk(bytes)?;
        let mut actors = Vec::new();
        let mut requests = Vec::new();
        for declaration in metadata.actors {
            let offset = metadata
                .commands
                .iter()
                .find(|command| {
                    command.opcode == 2 && command.words.get(2) == Some(&(declaration.slot as u32))
                })
                .ok_or(giants_formats::Error("actor declaration origin missing"))?
                .offset;
            let result = if declaration.object_type < 0 {
                Ok("non-body role")
            } else if declaration.name.is_none() {
                Ok("unnamed declaration; attachment/effect binding pending")
            } else if declaration.director != 0 {
                Err("named body director outside supported binding profile".into())
            } else {
                match self
                    .bodies
                    .entry(declaration.object_type)
                    .or_insert_with(|| {
                        load_registered_body(self.resources, declaration.object_type, 0)
                            .map(Arc::new)
                            .map_err(|e| e.to_string())
                    }) {
                    Ok(body) => {
                        let mut template = crate::animation_template::Template::prepare_inline(
                            &animation,
                            &[crate::animation_template::Model::Skinned(&body.skin)],
                            declaration.name.as_deref(),
                            crate::arithmetic::Precision::Double53,
                        )?;
                        if !template.needs.is_empty() {
                            return Err(giants_formats::Error(
                                "inline body binding has unresolved nodes",
                            ));
                        }
                        actors.push(Actor {
                            slot: declaration.slot,
                            object_type: declaration.object_type,
                            tree: declaration.name.clone().unwrap(),
                            body: Arc::clone(body),
                            inline_binding: template.bindings.remove(0),
                        });
                        Ok("body decoded; pose/material/visibility not verified")
                    }
                    Err(error) => Err(error.clone()),
                }
            };
            requests.push(BodyRequest {
                offset,
                slot: declaration.slot,
                class: declaration.object_type,
                tree: declaration.name,
                result,
            });
        }
        Ok(RegisteredCast { actors, requests })
    }
}
pub struct Actor {
    pub slot: usize,
    pub object_type: i32,
    pub tree: String,
    pub body: Arc<Body>,
    pub inline_binding: Option<crate::animation_template::Binding>,
}
impl Actor {
    pub fn positions(
        &self,
        animation: &giants_formats::anm::Animation,
        tick: i32,
    ) -> Result<Vec<[f32; 3]>, giants_formats::Error> {
        match &self.inline_binding {
            Some(binding) => {
                crate::skinning::positions_bound(&self.body.skin, animation, binding, tick)
            }
            None => crate::skinning::positions(&self.body.skin, animation, &self.tree, tick),
        }
    }
}
pub fn load_first(root: &Path) -> Result<Vec<Actor>, Box<dyn std::error::Error>> {
    load_sequence(root, "story4_T_square_2")
}
pub fn load_sequence(
    root: &Path,
    sequence: &str,
) -> Result<Vec<Actor>, Box<dyn std::error::Error>> {
    load_sequence_profile(root, sequence, false)
}
pub fn load_sequence_preview(
    root: &Path,
    sequence: &str,
) -> Result<Vec<Actor>, Box<dyn std::error::Error>> {
    load_sequence_profile(root, sequence, true)
}
fn load_sequence_profile(
    root: &Path,
    sequence: &str,
    preview: bool,
) -> Result<Vec<Actor>, Box<dyn std::error::Error>> {
    let declarations = giants_formats::cast::first_flick(&crate::read_asset(
        crate::flick::sequence_path(root, sequence)?,
    )?)?;
    let mut bodies = BTreeMap::new();
    let mut actors = Vec::new();
    for declaration in declarations {
        let Some(tree) = declaration.name else {
            continue;
        };
        if declaration.director != 0 {
            return Err("Named actor outside first director profile".into());
        }
        if preview && !matches!(declaration.object_type, 2 | 3 | 215 | 583 | 584 | 666) {
            eprintln!(
                "cinematic_preview_omission sequence={sequence} actor_slot={} class={} body_unimplemented=true",
                declaration.slot, declaration.object_type
            );
            continue;
        }
        let body = if let Some(body) = bodies.get(&declaration.object_type) {
            Arc::clone(body)
        } else {
            let (catalog, archive) = match declaration.object_type {
                2 => ("xx_mecc_flick.bin", "xx_mecc_flick.gzp"),
                584 => ("xx_smborjoyzee_flick.bin", "xx_smborjoyzee.gzp"),
                583 => ("xx_smtimmy.bin", "xx_smtimmy.gzp"),
                3 => ("xx_reaper_flick.bin", "xx_reaper_flick.gzp"),
                666 => ("xx_rpqueen_flick.bin", "xx_rpqueen.gzp"),
                215 => ("xx_reaperman.bin", "xx_reaperman.gzp"),
                _ => {
                    return Err(format!(
                        "Unmeasured body catalog type {} at slot {} in {sequence}",
                        declaration.object_type, declaration.slot
                    )
                    .into());
                }
            };
            let model = giants_formats::cast::first_model(
                &crate::read_asset(root.join("Bin").join(catalog))?,
                declaration.object_type,
            )?;
            let bytes = crate::read_asset(root.join("Bin").join(archive))?;
            let archive = giants_formats::gzp::Archive::parse(&bytes)?;
            let mesh = giants_formats::gbs::parse(&archive.read(&format!("{model}.gbs"))?)?;
            let skin = giants_formats::skn::parse(&archive.read(&format!("{model}.skn"))?)?;
            if skin.objects.iter().map(|o| o.vertex_count).sum::<usize>() != mesh.positions.len() {
                return Err("Actor skin/mesh vertex count mismatch".into());
            }
            let fallback_name = match declaration.object_type {
                2 => "xx_lander.gzp",
                3 | 666 | 215 => "xx_raik.gzp",
                _ => "tx_lev1.gzp",
            };
            let fallback_bytes = crate::read_asset(root.join("Bin").join(fallback_name))?;
            let fallback = giants_formats::gzp::Archive::parse(&fallback_bytes)?;
            // Measured class3 correction textures precede its local archive.
            // Other patch archives and general archive ordering remain unmeasured.
            let patch_path = root.join("Bin/arpfix.gzp");
            let patch_bytes = if declaration.object_type == 3 && patch_path.exists() {
                Some(crate::read_asset(patch_path)?)
            } else {
                None
            };
            let patch = patch_bytes
                .as_deref()
                .map(giants_formats::gzp::Archive::parse)
                .transpose()?;
            let mut textures = Vec::new();
            let mut alpha_textures = std::collections::BTreeSet::new();
            let names: std::collections::BTreeSet<_> = mesh
                .subobjects
                .iter()
                .map(|s| s.material.texture.clone())
                .collect();
            for name in names {
                let member = format!("{name}.tga");
                let correction = patch.as_ref().filter(|p| {
                    matches!(name.to_ascii_lowercase().as_str(), "rp_front" | "rp_back")
                        && p.entries()
                            .iter()
                            .any(|e| e.name.eq_ignore_ascii_case(&member))
                });
                let source = if let Some(patch) = correction {
                    patch
                } else if archive
                    .entries()
                    .iter()
                    .any(|e| e.name.eq_ignore_ascii_case(&member))
                {
                    &archive
                } else {
                    &fallback
                };
                let image = giants_formats::tga::decode(&source.read(&member)?)?;
                let (words, rgba) = if let Some(alpha) = &image.alpha {
                    alpha_textures.insert(name.to_ascii_lowercase());
                    let words = giants_formats::pixels::pack_argb4444(&image.pixels, alpha)?;
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
            let body = Arc::new(Body {
                model,
                mesh,
                skin,
                textures,
                alpha_textures,
                source: None,
            });
            bodies.insert(declaration.object_type, Arc::clone(&body));
            body
        };
        actors.push(Actor {
            inline_binding: None,
            slot: declaration.slot,
            object_type: declaration.object_type,
            tree,
            body,
        });
    }
    Ok(actors)
}
