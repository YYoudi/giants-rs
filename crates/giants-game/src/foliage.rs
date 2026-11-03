//! Source mission/world palms and canopy; native initial random phase is unresolved.
use giants_formats::{anm::Animation, cast, gbs::Mesh, gzp::Archive, skn::Skin};
use std::{collections::BTreeMap, path::Path};
pub struct Model {
    pub mesh: Mesh,
    pub skin: Option<Skin>,
    pub sort_flags: u32,
    pub sort_points: Vec<[f32; 3]>,
}
impl Model {
    fn new(mesh: Mesh, skin: Option<Skin>, sort_flags: u32) -> Result<Self, String> {
        let sort_points = (0..mesh.subobjects.len())
            .map(|part| {
                crate::render_sort::centroid(&mesh, part, crate::arithmetic::Precision::Double53)
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            mesh,
            skin,
            sort_flags,
            sort_points,
        })
    }
}
pub struct Instance {
    pub class: u32,
    pub from_world: bool,
    pub source_offset: usize,
    pub position: [f32; 3],
    pub scale: f32,
    pub matrix: [f32; 16],
}
pub struct Foliage {
    pub palms: Vec<Model>,
    pub canopy: Model,
    pub instances: Vec<Instance>,
    pub textures: Vec<crate::LogoTexture>,
    pub animation: Animation,
    pub rotations: BTreeMap<usize, [f32; 4]>,
    pub thresholds: Vec<f32>,
}
impl Foliage {
    fn primary_model(&self, instance: &Instance) -> &Model {
        if instance.class == 1045 {
            &self.canopy
        } else {
            &self.palms[0]
        }
    }
    /// Bevy sorting proxy only; this never replaces the geometry's world matrix.
    pub fn sort_point(
        &self,
        instance: &Instance,
        part: usize,
        camera: &crate::animation::Camera,
    ) -> [f32; 3] {
        let model = self.primary_model(instance);
        if model.sort_flags & 0x10 != 0 {
            let first = model.mesh.subobjects[part]
                .name
                .as_bytes()
                .first()
                .copied()
                .unwrap_or(0);
            let priority = f64::from(i32::from(first as i8) - 65);
            let forward = [camera.view[2], camera.view[6], camera.view[10]];
            let norm = forward.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
            return std::array::from_fn(|i| {
                (f64::from(instance.position[i]) - f64::from(forward[i]) * priority / norm) as f32
            });
        }
        crate::render_sort::world_point(
            model.sort_points[part],
            instance.position,
            crate::render_sort::Transform::Cached(instance.matrix),
            crate::arithmetic::Precision::Double53,
        )
        .expect("Measured foliage sort transform")
    }
    pub fn sort_depth(
        &self,
        instance: &Instance,
        part: usize,
        camera: &crate::animation::Camera,
    ) -> f32 {
        let model = self.primary_model(instance);
        let clip = crate::camera_matrix::compose(
            camera.view,
            Some(camera.projection),
            crate::arithmetic::Precision::Double53,
        );
        crate::render_sort::key(
            crate::render_sort::Input {
                spec_flags: model.sort_flags,
                first_name_byte: model.mesh.subobjects[part]
                    .name
                    .as_bytes()
                    .first()
                    .copied()
                    .unwrap_or(0),
                part_point: model.sort_points[part],
                position: instance.position,
                scale: instance.scale,
                transform: crate::render_sort::Transform::Cached(instance.matrix),
                clip_w: [clip[3], clip[7], clip[11], clip[15]],
            },
            crate::arithmetic::Precision::Double53,
        )
        .expect("Measured foliage sort key")
    }
    pub fn load(
        root: &Path,
        world: &crate::WorldAsset,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let plan = crate::instances::Plan::load_first(root, world)?;
        Self::load_planned(root, world, &plan)
    }
    pub fn load_planned(
        root: &Path,
        world: &crate::WorldAsset,
        plan: &crate::instances::Plan,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let sort_flags = |class| -> Result<u32, Box<dyn std::error::Error>> {
            let binding = plan
                .resources()
                .classes
                .models
                .first_selection(class)
                .ok_or("Foliage class model declaration missing")?
                .as_ref()
                .map_err(Clone::clone)?;
            Ok(crate::render_sort::declared_name_flags(
                &binding.selection.non_model_commands,
            ))
        };
        let palm_sort_flags = sort_flags(102)?;
        let canopy_sort_flags = sort_flags(1045)?;
        let catalog = crate::read_asset(root.join("Bin/xx_ptree.bin"))?;
        let bytes = crate::read_asset(root.join("Bin/xx_ptree.gzp"))?;
        let archive = Archive::parse(&bytes)?;
        let animation = giants_formats::anm::parse(&archive.read("ptree_bob_wind.anm")?)?;
        let rotations = ["palm_02", "palm_03"]
            .into_iter()
            .map(|name| Ok((animation.default_node(name)?.offset, [0.; 4])))
            .collect::<Result<_, giants_formats::Error>>()?;
        let references = cast::model_prefix(&catalog, 102)?;
        if references.len() != 8 {
            return Err("Unmeasured palm detail catalog".into());
        }
        let mut palms = Vec::new();
        let mut thresholds = Vec::new();
        for reference in references {
            let name = reference.name.ok_or("Unnamed palm model")?;
            let angle = f32::from_bits(reference.payload[0]);
            let distance = f32::from_bits(reference.payload[1]);
            if !angle.is_finite() || !distance.is_finite() || distance < 0. {
                return Err("Invalid palm detail payload".into());
            }
            let angle = if angle == 0. { 60. } else { angle };
            thresholds.push(
                ((f64::from(angle) * f64::from(0.008726646f32)).sin() * f64::from(distance)) as f32,
            );
            palms.push(Model::new(
                giants_formats::gbs::parse(&archive.read(&format!("{name}.gbs"))?)?,
                Some(giants_formats::skn::parse(
                    &archive.read(&format!("{name}.skn"))?,
                )?),
                palm_sort_flags,
            )?);
        }
        let name = cast::first_model(&catalog, 1045)?;
        let canopy = Model::new(
            giants_formats::gbs::parse(&archive.read(&format!("{name}.gbs"))?)?,
            None,
            canopy_sort_flags,
        )?;
        let mut instances = Vec::new();
        for p in plan
            .entries
            .iter()
            .filter(|p| matches!(p.serialized.object_type, 102 | 1045))
        {
            let input = p.source.as_ref().map_err(Clone::clone)?;
            let scale = input.scale;
            let mut position = input.position;
            if input.class == 1045 {
                // The three observed canopy placements share a flat ground cell.
                let t = &world.terrain;
                let cell = [
                    (position[0] - t.origin[0]) / t.spacing,
                    (position[1] - t.origin[1]) / t.spacing,
                ];
                if cell.iter().any(|v| !v.is_finite() || *v < 0.)
                    || cell[0] >= (t.width - 1) as f32
                    || cell[1] >= (t.height - 1) as f32
                {
                    return Err("Canopy outside terrain".into());
                }
                let [x, y] = cell.map(|v| v.floor() as usize);
                let heights = [
                    t.heights[y * t.width + x],
                    t.heights[y * t.width + x + 1],
                    t.heights[(y + 1) * t.width + x],
                    t.heights[(y + 1) * t.width + x + 1],
                ];
                if heights.iter().any(|h| h.to_bits() != heights[0].to_bits())
                    || (position[2] - heights[0]).abs() >= 0.1
                {
                    return Err("Canopy ground outside measured flat snap".into());
                }
                position[2] = heights[0];
            }
            let mut matrix = crate::object_transform::euler(position, [input.yaw, 0., 0.], scale)?;
            // Native yaw-only foliage starts its unused rotation axes at +0.
            for index in [0, 1, 2, 4, 5, 6, 8, 9, 10] {
                if matrix[index] == 0. {
                    matrix[index] = 0.;
                }
            }
            instances.push(Instance {
                class: input.class as u32,
                from_world: false,
                source_offset: input.source_offset,
                position,
                scale,
                matrix,
            });
        }
        // Consume the same measured palm capability from world declarations.
        // The retained native instances establish zero inherited pitch/roll for
        // this profile; no omitted angle is defaulted by the generic reader.
        for entry in plan
            .world_entries
            .iter()
            .filter(|p| p.declaration.object_type == 102)
        {
            let input = entry.class_input()?;
            let matrix = entry.authored_matrix([0., 0.])?;
            instances.push(Instance {
                class: entry.declaration.object_type,
                from_world: true,
                source_offset: entry.declaration.source_offset,
                position: entry.declaration.position,
                scale: input.scale,
                matrix,
            });
        }
        let shared_bytes = crate::read_asset(root.join("Bin/w_Story1.gzp"))?;
        let shared = Archive::parse(&shared_bytes)?;
        let mut textures = Vec::new();
        let mut names = std::collections::BTreeSet::new();
        for model in palms.iter().chain(std::iter::once(&canopy)) {
            for sub in &model.mesh.subobjects {
                if !matches!(sub.material.flags, 0x40000000 | 0x40000010 | 0x40000020)
                    || sub.material.falloff != 0.
                {
                    return Err("Unmeasured foliage material".into());
                }
                names.insert(sub.material.texture.clone());
            }
        }
        for name in names {
            let member = format!("{name}.tga");
            let source = if archive
                .entries()
                .iter()
                .any(|e| e.name.eq_ignore_ascii_case(&member))
            {
                &archive
            } else {
                &shared
            };
            let image = giants_formats::tga::decode(&source.read(&member)?)?;
            let (words, rgba) = if let Some(alpha) = image.alpha {
                let words = giants_formats::pixels::pack_argb4444(&image.pixels, &alpha)?;
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
        Ok(Self {
            palms,
            canopy,
            instances,
            textures,
            animation,
            rotations,
            thresholds,
        })
    }
    pub fn positions(
        &self,
        detail: usize,
        tick: i32,
    ) -> Result<Vec<[f32; 3]>, giants_formats::Error> {
        let model = self
            .palms
            .get(detail)
            .ok_or(giants_formats::Error("Palm detail outside catalog"))?;
        crate::skinning::positions_with_post_rotations(
            model
                .skin
                .as_ref()
                .ok_or(giants_formats::Error("Palm skin missing"))?,
            &self.animation,
            "Default Tree",
            tick,
            &self.rotations,
        )
    }
    pub fn all_positions(&self, tick: i32) -> Result<Vec<Vec<[f32; 3]>>, giants_formats::Error> {
        let matrices = crate::animation::Evaluator {
            animation: &self.animation,
        }
        .post_rotation_matrices(tick, &self.rotations)?;
        self.palms
            .iter()
            .map(|model| {
                crate::skinning::positions_with_matrices(
                    model
                        .skin
                        .as_ref()
                        .ok_or(giants_formats::Error("Palm skin missing"))?,
                    &self.animation,
                    "Default Tree",
                    &matrices,
                )
            })
            .collect()
    }
    /// Measured quality profile: distance thresholds and camera half-FOV sine.
    pub fn detail(&self, instance: &Instance, eye: [f32; 3], camera_scale: f32) -> usize {
        let distance = (0..3)
            .map(|i| (f64::from(instance.position[i]) - f64::from(eye[i])).powi(2))
            .sum::<f64>()
            .sqrt();
        let metric = distance * f64::from(camera_scale) / f64::from(instance.scale);
        self.thresholds
            .iter()
            .enumerate()
            .skip(1)
            .take_while(|(_, v)| **v > 0. && metric >= f64::from(**v))
            .last()
            .map_or(0, |(i, _)| i)
    }
}
