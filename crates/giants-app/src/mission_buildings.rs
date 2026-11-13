//! Ordinary house and bounded state101 scenery owners. Prototype PS2
//! `house_process`, `house_shot`, `house_handle_destruction`, `deathfx_logic`
//! and `deathfx_shot` remain unknown PC attribution search routes.
//! Other building families retain their collisions without a guessed callback.
mod barracks;
mod no_shot;
mod replacement;
use giants_game::{
    actor_fields::Fields,
    arithmetic::Precision,
    building_damage::{Hit, House, Request, Thresholds},
    building_death_fx as death_fx,
    building_geometry::Stage,
    collision_world::SourcePose,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(bevy::prelude::Message, Clone)]
pub(crate) struct Update {
    pub ticket: u64,
    pub key: (bool, usize),
    pub class: i32,
    pub geometry: Option<Arc<Stage>>,
}
enum Policy {
    House,
    Scenery {
        configuration: death_fx::Configuration,
        effects: BTreeMap<usize, giants_game::model_registry::StateBinding>,
    },
}
enum Reaction {
    House(Request),
    Scenery(death_fx::Request),
}
struct Class {
    policy: Policy,
    fields: Fields,
    stages: BTreeMap<u32, Arc<Stage>>,
    replacement: Option<Arc<replacement::Template>>,
}
struct Owner {
    physical: bool,
    environment: bool,
    contacts: u64,
    key: (bool, usize),
    fields: Fields,
    damage: House,
    class: Arc<Class>,
    pose: SourcePose,
    fraction: u32,
}
pub(crate) struct World {
    pub(crate) barracks_initial: barracks::Initial,
    owners: BTreeMap<u32, Owner>,
    no_shot: no_shot::Surfaces,
    replacements: Vec<replacement::Placed>,
    thresholds: Thresholds,
    quality: f32,
    lite_distance: f32,
    pub(crate) effects: crate::mission_building_effects::Runtime,
    failure: Option<String>,
}
pub(crate) struct HitResult {
    pub physical: bool,
    pub environment: bool,
    pub object: u32,
    pub active: u32,
    pub triangles: Vec<[[f32; 3]; 3]>,
    pub identities: Vec<(u32, u32)>,
    pub update: Update,
    pub sounds: Vec<crate::mission_audio::Event>,
    pub replacement: Option<usize>,
}
impl World {
    /// Actual narrow contacts may belong to damage owners or admitted no-shot
    /// surfaces. Observation does not assign either a damage callback.
    pub(crate) fn observes_contact(&self, object: u32) -> bool {
        self.owners.contains_key(&object)
            || self.no_shot.physical(object)
            || self.replacements.iter().any(|r| r.object == Some(object))
    }
    pub(crate) fn note_contact(&mut self, object: u32, support: bool) {
        self.no_shot.note_contact(object, support);
        if let Some(replacement) = self
            .replacements
            .iter_mut()
            .find(|r| r.object == Some(object))
        {
            replacement.contacts += 1;
            if replacement.contacts == 1 {
                println!(
                    "mission_building_replacement_contact object={object} class={} actual_narrow_phase=true kind={}",
                    replacement.fields.class,
                    if support { "floor-support" } else { "blocking" }
                );
            }
        }
        if let Some(owner) = self.owners.get_mut(&object) {
            owner.contacts += 1;
            if owner.contacts == 1 {
                println!(
                    "mission_building_player_contact object={object} class={} actual_narrow_phase=true kind={}",
                    owner.fields.class,
                    if support { "floor-support" } else { "blocking" }
                );
            }
        }
    }
    pub(crate) fn probe_requires_contact(&self, object: u32) -> bool {
        self.owners.get(&object).is_some_and(|o| o.physical) || self.no_shot.physical(object)
    }
    /// Input fixture only: approach the Story2 small tower through the nearby
    /// entrance's declared origin. This returns a command destination, never an
    /// actor placement, native navigation service or mission condition.
    pub(crate) fn probe_entry_waypoint(&self, object: u32, position: [f32; 3]) -> Option<[f32; 3]> {
        if self.owners.get(&object)?.fields.class != 515 {
            return None;
        }
        self.owners
            .values()
            .filter(|o| o.fields.class == 544 && o.physical && o.fields.object_flags & 0x8000 == 0)
            .min_by(|a, b| {
                let d = |o: &Owner| {
                    (o.fields.position[0] - position[0]).hypot(o.fields.position[1] - position[1])
                };
                d(a).total_cmp(&d(b))
            })
            .map(|o| o.fields.position)
    }
    pub(crate) fn probe_contacts(&self, object: u32) -> u64 {
        self.owners
            .get(&object)
            .map_or_else(|| self.no_shot.contacts(object), |o| o.contacts)
    }
    pub(crate) fn probe_no_shot_hits(&self, object: u32) -> u32 {
        self.no_shot.hits(object)
    }
    pub(crate) fn verify_no_shot(
        &self,
        object: u32,
        players: &[giants_game::player_world_collision::Object],
        projectiles: &[giants_game::projectile_world::Object],
    ) -> Result<(), String> {
        self.no_shot.verify(object, players, projectiles)
    }
    pub(crate) fn probe_target(
        &self,
        position: [f32; 3],
        selected: Option<u32>,
    ) -> Option<(u32, [f32; 3])> {
        if std::env::var("GIANTS_MISSION_BUILDING_CAPTURE").as_deref() == Ok("unchanged") {
            return self.no_shot.target(position, selected);
        }
        let (&id, owner) = if let Some(id) = selected {
            self.owners.get_key_value(&id)?
        } else {
            self.owners
                .iter()
                .filter(|(_, o)| {
                    o.damage.limit > 0.
                        && std::env::var("GIANTS_MISSION_BUILDING_CLASS")
                            .ok()
                            .and_then(|c| c.parse::<i32>().ok())
                            .map_or_else(
                                || matches!(o.class.policy, Policy::House),
                                |c| o.fields.class == c,
                            )
                })
                .min_by(|(_, a), (_, b)| {
                    let d = |o: &Owner| {
                        (o.fields.position[0] - position[0])
                            .hypot(o.fields.position[1] - position[1])
                    };
                    d(a).total_cmp(&d(b))
                })?
        };
        if owner.fields.object_flags & 0x8000 != 0 {
            return None;
        }
        let stage = owner.class.stages.get(&owner.fraction)?;
        let triangles = stage.placed(owner.pose).ok()?;
        let point = triangles
            .iter()
            .map(|t| {
                std::array::from_fn::<_, 3, _>(|i| {
                    (f64::from(t[0][i]) + f64::from(t[1][i]) + f64::from(t[2][i])) as f32 / 3.
                })
            })
            .max_by(|a, b| a[2].total_cmp(&b[2]))?;
        Some((id, point))
    }
    pub(crate) fn prepare(
        plan: &giants_game::instances::Plan,
        physical: &giants_game::declared_mesh_scene::Prepared,
        environment: &giants_game::declared_mesh_scene::Prepared,
        colliders: &giants_game::declared_mesh_scene::Prepared,
        globals: &giants_formats::global_data::GlobalData,
        terrain: &giants_formats::gti::Terrain,
    ) -> Result<Self, String> {
        let source = plan.resources();
        let thresholds = Thresholds::load(globals)?;
        let mut classes = BTreeMap::<i32, Arc<Class>>::new();
        let mut owners = BTreeMap::new();
        let mut effect_runtime = crate::mission_building_effects::Runtime::new(
            source.clone(),
            Arc::new(terrain.clone()),
        );
        for (record, object) in colliders.records.iter().zip(&colliders.objects) {
            if !matches!(record.instance.state, 61 | 101) {
                continue;
            }
            let prepared = (|| -> Result<Arc<Class>, String> {
                if let Some(c) = classes.get(&record.class) {
                    return Ok(c.clone());
                }
                let specification =
                    giants_game::specification::Prepared::load_registered(source, record.class)?;
                let fields = Fields::load(source, record.class, &specification)?;
                let policy = if record.instance.state == 61 {
                    Policy::House
                } else {
                    let seed = source
                        .class_state_seed(record.class)
                        .map_err(|e| e.to_string())?;
                    let data = seed.storage.data().ok_or("scenery state payload absent")?;
                    let word =
                        |i: usize| i32::from_le_bytes(data[i * 4..i * 4 + 4].try_into().unwrap());
                    let effects = source
                        .class_state_bindings_for(
                            record.class,
                            giants_game::state_storage::Profile::Debug,
                        )?
                        .into_iter()
                        .map(|b| (b.offset, b))
                        .collect::<BTreeMap<_, _>>();
                    for index in [1, 2] {
                        if word(index) != 0 && !effects.contains_key(&(index * 4)) {
                            return Err(
                                "scenery inline effect pointer needs its captured resource".into(),
                            );
                        }
                    }
                    if record.class == 0x2a5 || word(3) != 0 {
                        return Err("scenery special/weapon owner remains required".into());
                    }
                    for effect in effects.values() {
                        effect.target.as_ref().map_err(Clone::clone)?;
                    }
                    Policy::Scenery {
                        configuration: death_fx::Configuration {
                            class: record.class,
                            limit: f32::from_bits(fields.configuration[12]),
                            replacement: word(0),
                            death_effect: effects.contains_key(&4),
                            hit_effect: effects.contains_key(&8),
                            restricted: word(3) != 0,
                        },
                        effects,
                    }
                };
                let replacement = if let Policy::Scenery {
                    configuration,
                    effects,
                } = &policy
                    && configuration.replacement > 0
                {
                    let template = replacement::Template::load(source, configuration.replacement)?;
                    for effect in effects.values() {
                        effect_runtime.preflight_offset(effect)?;
                    }
                    Some(template)
                } else {
                    None
                };
                let mut stages = BTreeMap::new();
                for fraction in [1., 0.].into_iter().chain(
                    thresholds.0[..3]
                        .iter()
                        .map(|v| (f64::from(*v) * f64::from(0.01_f32)) as f32),
                ) {
                    stages.insert(
                        fraction.to_bits(),
                        Arc::new(Stage::health(source, record.class, fraction)?),
                    );
                }
                let c = Arc::new(Class {
                    policy,
                    fields,
                    stages,
                    replacement,
                });
                classes.insert(record.class, c.clone());
                Ok(c)
            })();
            let class = match prepared {
                Ok(c) => c,
                Err(reason) => {
                    println!(
                        "mission_building_need key={:?} class={} reason={reason:?}",
                        record.key, record.class
                    );
                    continue;
                }
            };
            let source_offset = if record.key.0 {
                plan.world_entries[record.key.1].declaration.source_offset
            } else {
                plan.entries
                    .iter()
                    .find(|e| e.index == record.key.1)
                    .ok_or("building source identity absent")?
                    .serialized
                    .source_offset
            };
            let f = record.instance.fields;
            let mut fields = class.fields.instantiate();
            fields.position = f.current;
            fields.current = f.current;
            fields.previous = f.previous;
            fields.angles = f.angles;
            fields.scale = f.scale;
            fields.object_flags = f.object_flags;
            fields.response_flags = f.response_flags;
            // The acquired first house callback resets its own damage and
            // selects channel1. This is a house owner, not a full native actor.
            if record.instance.state == 61 {
                fields.control_flags |= 1;
            }
            let damage = House {
                damage: 0.,
                limit: f32::from_bits(fields.configuration[12]),
            };
            let fraction = damage.animation_fraction(thresholds)?.to_bits();
            let m = giants_game::uncached_pose::matrix(
                giants_game::uncached_pose::Pose {
                    position: f.current,
                    angles: f.angles,
                    scale: f.scale,
                },
                Precision::Double53,
            )?;
            owners.insert(
                object.fields.id,
                Owner {
                    physical: physical
                        .objects
                        .iter()
                        .any(|o| o.fields.id == object.fields.id),
                    environment: environment
                        .objects
                        .iter()
                        .any(|o| o.fields.id == object.fields.id),
                    contacts: 0,
                    key: (record.key.0, source_offset),
                    fields,
                    damage,
                    class,
                    fraction,
                    pose: SourcePose {
                        basis: [m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]],
                        position: f.current,
                    },
                },
            );
        }
        println!(
            "mission_buildings_prepared owners={} classes={} callbacks=source_house_and_bounded_state101 other_families=false",
            owners.len(),
            classes.len()
        );
        Ok(Self {
            barracks_initial: barracks::Initial::prepare(plan, globals)?,
            owners,
            no_shot: no_shot::Surfaces::prepare(plan, physical, colliders),
            replacements: vec![],
            thresholds,
            quality: globals.scalar(0x13e0).map_err(|e| e.to_string())?,
            lite_distance: globals.scalar(0x1570).map_err(|e| e.to_string())?,
            effects: effect_runtime,
            failure: None,
        })
    }
    pub(crate) fn hit(
        &mut self,
        ticket: u64,
        hit: giants_game::mecc_pistol::WorldHit,
        camera: [f32; 3],
        residents: usize,
        random: &mut giants_game::audio_parameters::Random,
    ) -> Result<Option<HitResult>, String> {
        if self.no_shot.note_hit(ticket, &hit) {
            return Ok(None);
        }
        if !self.owners.contains_key(&hit.object) {
            return Ok(None);
        }
        if let Some(e) = &self.failure {
            return Err(e.clone());
        }
        let result = self.hit_inner(ticket, hit, camera, residents, random);
        if let Err(e) = &result {
            self.failure = Some(e.clone());
        }
        result
    }
    fn hit_inner(
        &mut self,
        ticket: u64,
        hit: giants_game::mecc_pistol::WorldHit,
        camera: [f32; 3],
        residents: usize,
        random: &mut giants_game::audio_parameters::Random,
    ) -> Result<Option<HitResult>, String> {
        let count = residents + self.owners.len() + self.effects.active();
        let owner = self.owners.get_mut(&hit.object).unwrap();
        if owner.fields.object_flags & 0x8000 != 0 {
            return Ok(None);
        }
        let input = Hit {
            amount: hit.damage,
            impact: Some(hit.point),
            origin: owner.fields.position,
            response_flags: owner.fields.response_flags,
        };
        // Commit the consumed decision prefix before an external service can
        // fail. A quarantined owner never rewinds damage or composing RNG.
        let requests = match &owner.class.policy {
            Policy::House => {
                let result = owner.damage.hit(input, self.thresholds)?;
                if !result.accepted {
                    return Ok(None);
                }
                owner.damage = result.state;
                result
                    .requests
                    .into_iter()
                    .map(Reaction::House)
                    .collect::<Vec<_>>()
            }
            Policy::Scenery { configuration, .. } => {
                // Admission currently excludes restricted classes: there is no
                // invented projectile actor/state standing in for the pistol.
                let result =
                    configuration.hit(owner.damage.damage, input, None, self.thresholds)?;
                if !result.accepted {
                    return Ok(None);
                }
                owner.damage.damage = result.damage;
                result.requests.into_iter().map(Reaction::Scenery).collect()
            }
        };
        let mut sounds = vec![];
        let mut replacement = None;
        for request in requests {
            match request {
                Reaction::Scenery(request) => match request {
                    death_fx::Request::Effect { slot, point } => {
                        let Policy::Scenery { effects, .. } = &owner.class.policy else {
                            unreachable!()
                        };
                        let offset = match slot {
                            death_fx::EffectSlot::Death => 4,
                            death_fx::EffectSlot::Hit => 8,
                        };
                        let binding = effects
                            .get(&offset)
                            .ok_or("scenery captured effect slot absent")?;
                        sounds.extend(self.effects.emit_offset(ticket, binding, point, random)?);
                    }
                    death_fx::Request::Retire => owner.fields.object_flags |= 0x8000,
                    death_fx::Request::Replace { class } => {
                        let template = owner
                            .class
                            .replacement
                            .as_ref()
                            .ok_or("scenery replacement constructor absent")?;
                        if template.fields.class != class {
                            return Err("replacement source class changed".into());
                        }
                        let index = self.replacements.len();
                        let serial = u64::try_from(index)
                            .map_err(|_| "replacement serial overflow")?
                            .checked_add(1)
                            .ok_or("replacement serial overflow")?;
                        let placed =
                            template.place(&owner.fields, owner.key, hit.object, serial)?;
                        self.replacements.push(placed);
                        replacement = Some(index);
                    }
                    death_fx::Request::SpecialDeath => {
                        return Err("scenery special service is not bound".into());
                    }
                },
                Reaction::House(request) => match request {
                    Request::Stone { kind, point } => sounds.extend(self.effects.emit(
                        ticket,
                        kind,
                        point,
                        giants_game::effect_initialization::Selection {
                            active_objects:
                                u32::try_from(count).map_err(|_| "building FX count overflow")?,
                            quality: self.quality,
                            lite_distance: self.lite_distance,
                            camera,
                            recipient_position: point,
                        },
                        random,
                    )?),
                    Request::Sound { selector, point } => {
                        sounds.push(crate::mission_audio::Event::NpcPosition {
                            sound: selector,
                            position: point,
                        })
                    }
                    Request::MissionDeath => {
                        return Err("building mission-credit mode/count owner is not bound".into());
                    }
                    Request::MarkTerminal => {
                        owner.fields.configuration[15] |= 1;
                        owner.fields.control_flags |= 2;
                    }
                    Request::Retire => owner.fields.object_flags |= 0x8000,
                },
            }
        }
        let fraction = owner.damage.animation_fraction(self.thresholds)?.to_bits();
        let stage = owner
            .class
            .stages
            .get(&fraction)
            .ok_or("building health pose absent")?
            .clone();
        let retired = owner.fields.object_flags & 0x8000 != 0;
        let geometry = (!retired).then(|| stage.clone());
        let triangles = if retired {
            vec![]
        } else {
            stage.placed(owner.pose)?
        };
        owner.fraction = fraction;
        println!(
            "mission_building_hit ticket={ticket} object={} class={} damage={} limit={} fraction={} retired={retired} normal_callback=true native_fidelity=false",
            hit.object,
            owner.fields.class,
            owner.damage.damage,
            owner.damage.limit,
            f32::from_bits(fraction)
        );
        Ok(Some(HitResult {
            physical: owner.physical,
            environment: owner.environment,
            object: hit.object,
            active: owner.fields.object_flags,
            triangles,
            identities: stage.identities.clone(),
            update: Update {
                ticket,
                key: owner.key,
                class: owner.fields.class,
                geometry,
            },
            sounds,
            replacement,
        }))
    }
    pub(crate) fn draws(
        &self,
        view: [f32; 16],
        detail: Option<([f32; 3], f32)>,
    ) -> Result<Vec<crate::mission_runtime::NpcDraw>, String> {
        let mut draws = self.effects.draws(view, detail)?;
        for replacement in &self.replacements {
            draws.push(replacement.draw(detail)?);
        }
        Ok(draws)
    }
    pub(crate) fn probe_replacement(&self, old: u32) -> Option<(u32, [f32; 3], u64)> {
        let r = self.replacements.iter().find(|r| r.replaces == old)?;
        Some((r.object?, r.fields.position, r.contacts))
    }
    pub(crate) fn replacement_contact_ready(&self, old: u32) -> bool {
        self.replacements
            .iter()
            .filter(|r| r.replaces == old)
            .all(|r| r.object.is_some() && r.contacts > 0)
    }
    pub(crate) fn verify_replacement_contact(
        &self,
        old: u32,
        players: &[giants_game::player_world_collision::Object],
        projectiles: &[giants_game::projectile_world::Object],
    ) -> Result<(), String> {
        for r in self.replacements.iter().filter(|r| r.replaces == old) {
            let id = r.object.ok_or("replacement physical owner unpublished")?;
            if id == old
                || r.contacts == 0
                || !players
                    .iter()
                    .any(|p| p.fields.id == id && !p.triangles.is_empty())
                || projectiles.iter().any(|p| p.id == id)
            {
                return Err("replacement actual contact/independent collision roles absent".into());
            }
            println!(
                "mission_building_replacement_verified old_object={old} object={id} class={} contacts={} independent_player_mesh=true projectile=false actual_narrow_phase=true",
                r.fields.class, r.contacts
            );
        }
        Ok(())
    }
    pub(crate) fn replacement_player(
        &self,
        index: usize,
        id: u32,
    ) -> Result<giants_game::player_world_collision::Object, String> {
        self.replacements
            .get(index)
            .ok_or("replacement instance absent")?
            .player(id)
    }
    pub(crate) fn publish_replacement(&mut self, index: usize, id: u32) -> Result<(), String> {
        if self.observes_contact(id) {
            return Err("replacement world identity already used".into());
        }
        let r = self
            .replacements
            .get_mut(index)
            .ok_or("replacement instance absent")?;
        if r.object.is_some() || id == r.replaces {
            return Err("replacement publication repeated/reused".into());
        }
        r.object = Some(id);
        println!(
            "mission_building_replacement object={id} old_object={} class={} alias={:?} serial={} player=true projectile=false environment=false independent_identity=true native_fidelity=false",
            r.replaces, r.fields.class, r.alias, r.serial
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; source entrance hit/geometry only, no GPU or OS input"]
    fn installed_entrance_damage_updates_player_shape_without_inventing_environment_owner() {
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
        let (&id, owner) = state
            .buildings
            .owners
            .iter()
            .find(|(_, o)| o.fields.class == 544)
            .unwrap();
        assert!(owner.fields.shadow_texture.is_some());
        assert!(owner.physical);
        assert!(!owner.environment);
        let point = owner.fields.position;
        let damage = owner.damage.limit * 0.26;
        let before = state
            .projectile_objects
            .iter()
            .find(|o| o.id == id)
            .unwrap()
            .active;
        let mut random = giants_game::audio_parameters::Random(17);
        let result = state
            .buildings
            .hit(
                91,
                giants_game::mecc_pistol::WorldHit {
                    object: id,
                    serial: 1,
                    point,
                    damage,
                    older: point,
                    motion: giants_game::damage_impulse::Motion {
                        current: point,
                        previous: point,
                    },
                },
                point,
                1,
                &mut random,
            )
            .unwrap()
            .unwrap();
        assert!(result.physical);
        assert!(!result.environment);
        assert!(result.update.geometry.is_some());
        assert_eq!(state.buildings.owners[&id].fraction, 0.75_f32.to_bits());
        state.apply_building_hit(&result).unwrap();
        assert_eq!(
            state
                .projectile_objects
                .iter()
                .find(|o| o.id == id)
                .unwrap()
                .active,
            before
        );
        assert!(state.mission_colliders().iter().any(|o| o.fields.id == id));
    }
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; supplied hit, no GPU or OS input"]
    fn installed_fence_damage_updates_player_without_inventing_environment_membership() {
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
        let (&id, owner) = state
            .buildings
            .owners
            .iter()
            .find(|(_, o)| o.fields.class == 512)
            .unwrap();
        assert!(owner.physical);
        assert!(!owner.environment);
        let point = owner.fields.position;
        let limit = owner.damage.limit;
        let count = state.mission_colliders().len();
        assert!(state.mission_colliders().iter().any(|o| o.fields.id == id));
        let mut random = giants_game::audio_parameters::Random(17);
        let hit = giants_game::mecc_pistol::WorldHit {
            object: id,
            serial: 1,
            point,
            damage: limit,
            older: point,
            motion: giants_game::damage_impulse::Motion {
                current: point,
                previous: point,
            },
        };
        let result = state
            .buildings
            .hit(91, hit, point, 1, &mut random)
            .unwrap()
            .unwrap();
        assert!(result.physical);
        assert!(!result.environment);
        assert!(result.update.geometry.is_none());
        assert_eq!(state.buildings.effects.births, 17);
        state.apply_building_hit(&result).unwrap();
        assert_eq!(state.mission_colliders().len(), count);
        assert_ne!(
            state
                .mission_colliders()
                .iter()
                .find(|o| o.fields.id == id)
                .unwrap()
                .fields
                .object_flags
                & 0x8000,
            0
        );
        assert_ne!(
            state
                .projectile_objects
                .iter()
                .find(|o| o.id == id)
                .unwrap()
                .active
                & 0x8000,
            0
        );
        for sample in 0..1200 {
            state
                .buildings
                .effects
                .advance(
                    crate::mission_building_effects::Frame {
                        ticket: 91,
                        sample,
                        delta: 1. / 120.,
                        areas: &[],
                        world: &world,
                    },
                    &mut random,
                )
                .unwrap();
        }
        assert_eq!(state.buildings.effects.active(), 0);
        assert_eq!(state.buildings.effects.retired, 17);
    }
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; supplied state101 hits/clock, no GPU or audio sinks"]
    fn installed_scenery_tower_consumes_captured_wood_stone_effect_and_retires() {
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
        let (&id, owner) = state
            .buildings
            .owners
            .iter()
            .find(|(_, o)| o.fields.class == 515)
            .unwrap();
        let point = owner.fields.position;
        let limit = owner.damage.limit;
        let hit = |amount| giants_game::mecc_pistol::WorldHit {
            object: id,
            serial: 1,
            point,
            damage: amount,
            older: point,
            motion: giants_game::damage_impulse::Motion {
                current: point,
                previous: point,
            },
        };
        let mut random = giants_game::audio_parameters::Random(17);
        let partial = state
            .buildings
            .hit(91, hit(limit * 0.5), point, 1, &mut random)
            .unwrap()
            .unwrap();
        assert_eq!(state.buildings.effects.births, 0);
        assert!(partial.sounds.is_empty());
        assert!(partial.update.geometry.is_some());
        state.apply_building_hit(&partial).unwrap();
        let terminal = state
            .buildings
            .hit(91, hit(limit), point, 1, &mut random)
            .unwrap()
            .unwrap();
        assert_eq!(state.buildings.effects.births, 29);
        assert!(terminal.update.geometry.is_none());
        assert!(terminal.triangles.is_empty());
        assert_ne!(terminal.active & 0x8000, 0);
        state.apply_building_hit(&terminal).unwrap();
        let seed = random.0;
        assert!(
            state
                .buildings
                .hit(91, hit(limit), point, 1, &mut random)
                .unwrap()
                .is_none()
        );
        assert_eq!(random.0, seed);
        for sample in 0..1200 {
            state
                .buildings
                .effects
                .advance(
                    crate::mission_building_effects::Frame {
                        ticket: 91,
                        sample,
                        delta: 1. / 120.,
                        areas: &[],
                        world: &world,
                    },
                    &mut random,
                )
                .unwrap();
        }
        assert_eq!(state.buildings.effects.active(), 0);
        assert_eq!(state.buildings.effects.retired, 29);
    }
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; supplied hits/clock/camera/RNG, no GPU, OS input or audio sinks"]
    fn installed_house_damage_updates_all_contact_owners_and_expires_source_bursts() {
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
        let (&id, owner) = state
            .buildings
            .owners
            .iter()
            .find(|(_, o)| o.fields.class == 590)
            .unwrap();
        let limit = owner.damage.limit;
        let point = owner.fields.position;
        let original = state
            .mission_colliders()
            .iter()
            .find(|o| o.fields.id == id)
            .unwrap()
            .triangles
            .iter()
            .map(|t| t.vertices)
            .collect::<Vec<_>>();
        let hit = |serial, amount| giants_game::mecc_pistol::WorldHit {
            object: id,
            serial,
            point,
            damage: amount,
            older: point,
            motion: giants_game::damage_impulse::Motion {
                current: point,
                previous: point,
            },
        };
        let mut random = giants_game::audio_parameters::Random(17);
        let result = state
            .buildings
            .hit(91, hit(1, limit * 0.3), point, 1, &mut random)
            .unwrap()
            .unwrap();
        assert_ne!(result.triangles, original);
        assert_eq!(state.buildings.effects.births, 25);
        assert_eq!(result.sounds.len(), 2); // Source root cue plus house callback cue.
        state.apply_building_hit(&result).unwrap();
        let placed = state
            .mission_colliders()
            .iter()
            .find(|o| o.fields.id == id)
            .unwrap()
            .triangles
            .iter()
            .map(|t| t.vertices)
            .collect::<Vec<_>>();
        assert_eq!(placed, result.triangles);
        assert!(result.update.geometry.is_some());
        let terminal = state
            .buildings
            .hit(91, hit(2, limit), point, 1, &mut random)
            .unwrap()
            .unwrap();
        assert_ne!(terminal.active & 0x8000, 0);
        assert!(terminal.triangles.is_empty());
        assert!(terminal.update.geometry.is_none());
        assert_eq!(terminal.sounds.len(), 3);
        assert_eq!(state.buildings.effects.births, 25 + 25 + 49);
        state.apply_building_hit(&terminal).unwrap();
        assert!(!giants_game::object_segment::projectile_candidate(
            u32::MAX,
            1,
            id,
            0,
            terminal.active,
            0x20000
        ));
        let before = random.0;
        assert!(
            state
                .buildings
                .hit(91, hit(3, limit), point, 1, &mut random)
                .unwrap()
                .is_none()
        );
        assert_eq!(random.0, before);
        let mut frames = std::collections::BTreeSet::new();
        for sample in 0..1200 {
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
            for draw in state
                .buildings
                .draws(bevy::prelude::Mat4::IDENTITY.to_cols_array(), None)
                .unwrap()
            {
                if let Some(texture) = draw.texture {
                    frames.insert(texture);
                }
            }
        }
        assert!(
            frames.len() > 1,
            "source fireball must advance numbered textures"
        );
        assert_eq!(state.buildings.effects.active(), 0);
        assert_eq!(
            state.buildings.effects.births,
            state.buildings.effects.retired
        );
    }
}
