//! Bounded ordinary replacement owner, distinct from a declared mission record.
//! Prototype PS2 `object_allocate_type`, `object_copy_personality`,
//! `object_copy_activedata`, `object_copy_posdirinfo` and
//! `object_bbox_to_world_bbox` remain unknown PC attribution search routes.
//! Constructor channels/header, placement history and bounds are separate
//! acquired projections. Source opaque-word semantics and native world/list
//! scheduling are not fabricated; logical lineage retains the previous alias.
use super::*;
use giants_game::{scene_resources::SceneResources, static_geometry};

pub(super) struct Template {
    pub(super) fields: Fields,
    mesh: giants_formats::gbs::Mesh,
    slot: u32,
    bounds: giants_formats::world_bounds::Resolved,
    instance: static_geometry::Instance,
    assets: BTreeMap<usize, Arc<static_geometry::Asset>>,
    tree_class: Option<Arc<Class>>,
}
pub(super) struct Placed {
    template: Arc<Template>,
    pub(super) fields: Fields,
    pub(super) object: Option<u32>,
    pub(super) serial: u64,
    pub(super) replaces: u32,
    pub(super) alias: static_geometry::SourceKey,
    pub(super) contacts: u64,
    tree: Option<[u32; 19]>,
    matrix: [f32; 16],
    bounds: [f32; 6],
    triangles: Vec<[[f32; 3]; 3]>,
    identities: Vec<(u32, u32)>,
}
impl Template {
    pub(super) fn load(source: &SceneResources, class: i32) -> Result<Arc<Self>, String> {
        // These source classes have independent constructor/header controls.
        // Do not relax all callbackless ANM declarations into this cohort.
        if !matches!(class, 664 | 1127 | 1129) {
            return Err("replacement constructor requires an acquired class profile".into());
        }
        let specification = giants_game::specification::Prepared::load_registered(source, class)?;
        let fields = Fields::load(source, class, &specification)?;
        let cold = giants_game::collider_init::cold_projection(source, class)
            .map_err(|e| e.to_string())?;
        let tree_class = if class == 664 {
            Some(trees::class(source, 664)?)
        } else {
            None
        };
        if cold.initial.state != if class == 664 { 60 } else { 0 }
            || cold.initial.usage != [0; 2]
            || cold.initial.fields.physical
            || fields.object_flags != 0
            || fields.control_flags != 0
            || fields.response_flags != 0
            || fields.configuration[14] != if class == 664 { 0x20042 } else { 0 }
            || fields.channels[0][0] != if class == 664 { 254 } else { 1 }
            || fields.channels[1..].iter().any(|c| c[0] != 254)
        {
            return Err("replacement needs its actual callback/physics/role constructor".into());
        }
        let selection = source
            .collision_model_mesh(class)
            .map_err(|e| e.to_string())?;
        let (name, mesh) = selection
            .geometry
            .ok_or("replacement collision resource absent")?;
        if source
            .archives
            .members
            .contains_key(&format!("{}.skn", name.to_ascii_lowercase()))
        {
            return Err("replacement collision needs its skinned pose owner".into());
        }
        let slot = cold.initial.fields.animation_slot;
        if slot != fields.channels[0][0] {
            return Err("replacement channel/bounds slot differs".into());
        }
        let bounds = source
            .collision_resource_bounds(
                class,
                Some(u8::try_from(slot).map_err(|_| "replacement slot overflow")?),
            )
            .map_err(|e| e.to_string())?;
        let mut scene = static_geometry::Scene::constructed_rigid_draw(
            source,
            (false, 0),
            class,
            giants_game::procedural_aim::Matrix::identity().values,
            fields.scale,
        )?;
        let instance = scene.instances.remove(0);
        let assets = scene
            .assets
            .into_iter()
            .map(|((_, slot), asset)| (slot, Arc::new(asset)))
            .collect();
        Ok(Arc::new(Self {
            fields,
            mesh,
            slot,
            bounds,
            instance,
            assets,
            tree_class,
        }))
    }
    pub(super) fn place(
        self: &Arc<Self>,
        old: &Fields,
        old_tree: Option<[u32; 19]>,
        alias: static_geometry::SourceKey,
        replaces: u32,
        serial: u64,
    ) -> Result<Placed, String> {
        let tree = self
            .tree_class
            .as_ref()
            .map(|class| {
                let mut payload = class.tree.ok_or("replacement tree payload absent")?;
                let old = old_tree.ok_or("tree replacement needs source state60 payload")?;
                // Native shared shot copies exactly fifteen words, preserving the
                // new class's replacement/effect/restriction prefix.
                payload[4..].copy_from_slice(&old[4..]);
                if payload[9] & 4 != 0 {
                    return Err("falling replacement needs its acquired pose owner".into());
                }
                Ok::<_, String>(payload)
            })
            .transpose()?;
        let mut fields = self.fields.clone();
        // Acquired caller stores only yaw/scale and an opaque source word.
        // The latter has no invented native meaning here: retain logical
        // source/old-object lineage separately from the fresh collision ID.
        fields.angles[0] = old.angles[0];
        fields.scale = old.scale;
        fields.previous = fields.current;
        fields.position = old.position;
        fields.current = old.position;
        if !fields.scale.is_finite() || fields.scale <= 0. {
            return Err("replacement scale invalid".into());
        }
        let pose = giants_game::uncached_pose::Pose {
            position: fields.position,
            angles: fields.angles,
            scale: fields.scale,
        };
        let affine = giants_game::uncached_pose::matrix(pose, Precision::Double53)?;
        let mut matrix = giants_game::procedural_aim::Matrix::identity().values;
        matrix[..12].copy_from_slice(&affine);
        // Bounds production is separate from placement. The debug placement's
        // optional final helper is a no-op, not a completed bbox service.
        let bounds = giants_game::collision_bounds::uncached(
            Some(giants_game::collision_bounds::Models {
                primary: self.bounds.primary,
                auxiliary: self.bounds.animation,
                slot: self.slot,
            }),
            pose,
            Precision::Double53,
        )?;
        let source_pose = SourcePose {
            position: fields.position,
            basis: [
                affine[0], affine[1], affine[2], affine[4], affine[5], affine[6], affine[8],
                affine[9], affine[10],
            ],
        };
        let triangles = giants_game::collision_geometry::world_triangles(
            &self.mesh,
            source_pose,
            Precision::Double53,
        )?;
        let identities = self
            .mesh
            .subobjects
            .iter()
            .enumerate()
            .flat_map(|(part, sub)| {
                (0..sub.triangles.len()).map(move |triangle| (part as u32, triangle as u32 + 1))
            })
            .collect();
        Ok(Placed {
            template: self.clone(),
            fields,
            object: None,
            serial,
            replaces,
            alias,
            contacts: 0,
            tree,
            matrix,
            bounds,
            triangles,
            identities,
        })
    }
}
impl Placed {
    pub(super) fn projectile(
        &self,
        id: u32,
    ) -> Result<Option<giants_game::projectile_world::Object>, String> {
        if self.fields.configuration[14] & 0x20000 == 0 {
            return Ok(None);
        }
        Ok(Some(giants_game::projectile_world::Object {
            id,
            parent: 0,
            active: self.fields.object_flags,
            personality: self.fields.configuration[14],
            bounds: giants_game::volume_queries::Bounds {
                min: self.bounds[..3].try_into().unwrap(),
                max: self.bounds[3..].try_into().unwrap(),
            },
            mesh: giants_game::object_segment::Mesh::new(id, self.triangles.clone())?,
        }))
    }
    pub(super) fn damage_owner(
        &self,
        lineage: giants_game::declared_residents::Key,
    ) -> Option<Owner> {
        let class = self.template.tree_class.clone()?;
        Some(Owner {
            physical: true,
            environment: false,
            contacts: 0,
            key: self.alias,
            declaration: lineage,
            fields: self.fields.clone(),
            damage: House {
                damage: 0.,
                limit: f32::from_bits(self.fields.configuration[12]),
            },
            class,
            pose: SourcePose {
                basis: [
                    self.matrix[0],
                    self.matrix[1],
                    self.matrix[2],
                    self.matrix[4],
                    self.matrix[5],
                    self.matrix[6],
                    self.matrix[8],
                    self.matrix[9],
                    self.matrix[10],
                ],
                position: self.fields.position,
            },
            fraction: 1f32.to_bits(),
            tree: self.tree,
        })
    }
    pub(super) fn player(
        &self,
        id: u32,
    ) -> Result<giants_game::player_world_collision::Object, String> {
        if self.object.is_some() || id == self.replaces {
            return Err("replacement identity reused or already published".into());
        }
        Ok(giants_game::player_world_collision::Object {
            fields: giants_game::mecc_collision_candidates::Object {
                id,
                parent: 0,
                has_model: true,
                object_flags: self.fields.object_flags,
                collision_flags: self.fields.configuration[14],
                bounds: giants_game::volume_queries::Bounds {
                    min: self.bounds[..3].try_into().unwrap(),
                    max: self.bounds[3..].try_into().unwrap(),
                },
            },
            response_flags: self.fields.response_flags,
            triangles: self
                .triangles
                .iter()
                .zip(&self.identities)
                .map(
                    |(vertices, (part, ordinal))| giants_game::player_mesh_contact::Triangle {
                        vertices: *vertices,
                        part: *part,
                        ordinal: *ordinal,
                    },
                )
                .collect(),
        })
    }
    pub(super) fn draw(
        &self,
        detail: Option<([f32; 3], f32)>,
    ) -> Result<crate::mission_runtime::NpcDraw, String> {
        if self.object.is_none() {
            return Err("replacement draw awaits its physical world owner".into());
        }
        let (eye, factor) = detail.ok_or("replacement needs current source LOD camera")?;
        let slot = self.template.instance.table.select_positioned(
            giants_game::lod::PositionInput {
                object_position: self.fields.position,
                camera_position: eye,
                offset: 0.,
                camera_factor: factor,
                object_scale: self.fields.scale,
                quality: 2,
                forced: -1,
                allow_missing: false,
            },
            Precision::Double53,
        )?;
        let asset = self
            .template
            .assets
            .get(&usize::try_from(slot).map_err(|_| "replacement negative LOD")?)
            .ok_or("replacement selected LOD absent")?
            .clone();
        Ok(crate::mission_runtime::NpcDraw {
            rigid: Some(crate::mission_runtime::NpcRigidProfile {
                asset: asset.clone(),
                matrix: self.matrix,
            }),
            class: self.fields.class,
            id: crate::mission_runtime::NpcDrawOwner::BuildingReplacement(self.serial),
            geometry: asset.model.clone(),
            positions: asset.model.mesh.positions.clone(),
            frame: 0,
            transparency: 0.,
            center: self.fields.position,
            texture: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; supplied hits/time and CPU world/draw ownership, no GPU/OS/audio-sink acceptance"]
    fn installed_guard_damage_replaces_independent_player_owner_and_retires_full_graph() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_BUILDING_ROOT").unwrap());
        let menu = giants_game::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            giants_game::mission_loading::prepare(&root, menu.request("Story2", 0).unwrap())
                .unwrap();
        let world = packet.world_asset(&root).unwrap();
        let mut state =
            crate::mecc_dev_scene::State::load_prepared(&root, &world, &packet).unwrap();
        let (&old_id, owner) = state
            .buildings
            .owners
            .iter()
            .find(|(_, o)| o.fields.class == 1050)
            .unwrap();
        let point = owner.fields.position;
        let limit = owner.damage.limit;
        let old_fields = owner.fields.clone();
        let persistent_before = state.mission_colliders().len();
        let projectile_before = state.projectile_objects.len();
        let hit = |serial, damage| giants_game::mecc_pistol::WorldHit {
            object: old_id,
            serial,
            point,
            damage,
            older: point,
            motion: giants_game::damage_impulse::Motion {
                current: point,
                previous: point,
            },
        };
        let mut random = giants_game::audio_parameters::Random(17);
        let first = state
            .buildings
            .hit(91, hit(1, limit * 0.26), point, 1, &mut random)
            .unwrap()
            .unwrap();
        assert!(first.update.geometry.is_some());
        assert!(first.replacement.is_none());
        state.apply_building_hit(&first).unwrap();
        let last = state
            .buildings
            .hit(91, hit(2, limit), point, 1, &mut random)
            .unwrap()
            .unwrap();
        assert!(last.update.geometry.is_none());
        assert!(last.triangles.is_empty());
        assert!(last.replacement.is_some());
        assert!(!last.sounds.is_empty());
        state.apply_building_hit(&last).unwrap();
        let replacement = &state.buildings.replacements[last.replacement.unwrap()];
        let new_id = replacement.object.unwrap();
        assert_ne!(old_id, new_id);
        assert_eq!(replacement.fields.class, 1127);
        assert_eq!(replacement.fields.position, point);
        assert_eq!(replacement.fields.angles[0], old_fields.angles[0]);
        assert_eq!(replacement.fields.scale, old_fields.scale);
        assert_eq!(
            replacement.fields.configuration,
            replacement.template.fields.configuration
        );
        assert!(
            state
                .mission_colliders()
                .iter()
                .any(|o| o.fields.id == new_id && !o.triangles.is_empty())
        );
        assert!(!state.projectile_objects.iter().any(|o| o.id == new_id));
        assert_eq!(state.projectile_objects.len(), projectile_before);
        assert_ne!(
            state
                .projectile_objects
                .iter()
                .find(|o| o.id == old_id)
                .unwrap()
                .active
                & 0x8000,
            0
        );
        assert_eq!(state.mission_colliders().len(), persistent_before + 1);
        state.set_mission_contacts(91, vec![]).unwrap();
        assert_eq!(state.mission_colliders().len(), persistent_before + 1);
        assert!(
            state
                .mission_colliders()
                .iter()
                .any(|o| o.fields.id == new_id)
        );
        assert!(state.buildings.observes_contact(new_id));
        let view = bevy::prelude::Mat4::IDENTITY.to_cols_array();
        let detail = Some(([point[0], point[1] - 200., point[2] + 30.], 1.));
        let draws = state.buildings.draws(view, detail).unwrap();
        assert_eq!(
            draws
                .iter()
                .filter(|d| matches!(
                    d.id,
                    crate::mission_runtime::NpcDrawOwner::BuildingReplacement(_)
                ))
                .count(),
            1
        );
        assert_eq!(
            draws
                .iter()
                .map(|d| d.id)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            draws.len()
        );
        let before = (random.0, state.buildings.effects.births);
        assert!(
            state
                .buildings
                .hit(91, hit(3, limit), point, 1, &mut random)
                .unwrap()
                .is_none()
        );
        assert_eq!((random.0, state.buildings.effects.births), before);
        for sample in 0..3600 {
            state
                .buildings
                .effects
                .advance(
                    crate::mission_building_effects::Frame {
                        ticket: 91,
                        sample,
                        delta: 1. / 60.,
                        areas: &[],
                        world: &world,
                    },
                    &mut random,
                )
                .unwrap();
            if state.buildings.effects.active() == 0 {
                break;
            }
        }
        assert_eq!(state.buildings.effects.active(), 0);
        assert_eq!(
            state.buildings.effects.births,
            state.buildings.effects.retired
        );
        let draws = state.buildings.draws(view, detail).unwrap();
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].class, 1127);
    }
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; supplied inheritance profile, no runtime acceptance"]
    fn installed_replacement_preserves_class_defaults_and_refuses_unacquired_classes() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_BUILDING_ROOT").unwrap());
        let source =
            giants_game::scene_resources::SceneResources::load(&root, "w_Story2.bin", "English")
                .unwrap();
        for class in [1127, 1129] {
            let template = Template::load(&source, class).unwrap();
            let mut old = template.fields.clone();
            old.position = [10., 20., 30.];
            old.angles = [45., 10., -5.];
            old.scale = 2.5;
            old.object_flags = 0x400;
            old.response_flags = 0x1000;
            old.configuration[12] = 999f32.to_bits();
            let r = template.place(&old, None, (false, 123), 77, 1).unwrap();
            assert_eq!(
                r.fields.angles,
                [45., template.fields.angles[1], template.fields.angles[2]]
            );
            assert_eq!(r.fields.previous, template.fields.current);
            assert_eq!(r.fields.object_flags, 0);
            assert_eq!(r.fields.response_flags, 0);
            assert_eq!(
                r.fields.configuration[12],
                template.fields.configuration[12]
            );
            assert!(r.player(77).is_err());
            assert!(r.player(78).is_ok());
            assert!(r.draw(Some(([0.; 3], 1.))).is_err());
        }
        assert!(Template::load(&source, 1050).is_err());
        assert!(Template::load(&source, 593).is_err());
    }
}
