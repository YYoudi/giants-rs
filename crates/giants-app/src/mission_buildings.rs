//! Ordinary house and bounded state101 scenery owners. Prototype PS2
//! `house_process`, `house_shot`, `house_handle_destruction`, `deathfx_logic`
//! and `deathfx_shot` remain unknown PC attribution search routes.
//! Other building families retain their collisions without a guessed callback.
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
}
struct Owner {
    contacts: u64,
    key: (bool, usize),
    fields: Fields,
    damage: House,
    class: Arc<Class>,
    pose: SourcePose,
    fraction: u32,
}
pub(crate) struct World {
    owners: BTreeMap<u32, Owner>,
    thresholds: Thresholds,
    quality: f32,
    lite_distance: f32,
    pub(crate) effects: crate::mission_building_effects::Runtime,
    failure: Option<String>,
}
pub(crate) struct HitResult {
    pub object: u32,
    pub active: u32,
    pub triangles: Vec<[[f32; 3]; 3]>,
    pub identities: Vec<(u32, u32)>,
    pub update: Update,
    pub sounds: Vec<crate::mission_audio::Event>,
}
impl World {
    pub(crate) fn contains(&self, object: u32) -> bool {
        self.owners.contains_key(&object)
    }
    pub(crate) fn note_contact(&mut self, object: u32) {
        if let Some(owner) = self.owners.get_mut(&object) {
            owner.contacts += 1;
            if owner.contacts == 1 {
                println!(
                    "mission_building_player_contact object={object} class={} actual_narrow_phase=true",
                    owner.fields.class
                );
            }
        }
    }
    pub(crate) fn probe_contacts(&self, object: u32) -> u64 {
        self.owners.get(&object).map_or(0, |o| o.contacts)
    }
    pub(crate) fn probe_target(
        &self,
        position: [f32; 3],
        selected: Option<u32>,
    ) -> Option<(u32, [f32; 3])> {
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
        colliders: &giants_game::declared_mesh_scene::Prepared,
        globals: &giants_formats::global_data::GlobalData,
    ) -> Result<Self, String> {
        let source = plan.resources();
        let thresholds = Thresholds::load(globals)?;
        let mut classes = BTreeMap::<i32, Arc<Class>>::new();
        let mut owners = BTreeMap::new();
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
                    if word(0) > 0 || record.class == 0x2a5 || word(3) != 0 {
                        return Err(
                            "scenery replacement/special/weapon owner remains required".into()
                        );
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
            owners,
            thresholds,
            quality: globals.scalar(0x13e0).map_err(|e| e.to_string())?,
            lite_distance: globals.scalar(0x1570).map_err(|e| e.to_string())?,
            effects: crate::mission_building_effects::Runtime::new(source.clone()),
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
                    death_fx::Request::SpecialDeath | death_fx::Request::Replace { .. } => {
                        return Err("scenery replacement/special service is not bound".into());
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
        }))
    }
    pub(crate) fn draws(
        &self,
        view: [f32; 16],
    ) -> Result<Vec<crate::mission_runtime::NpcDraw>, String> {
        self.effects.draws(view)
    }
}

#[cfg(test)]
mod tests {
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
                .advance(91, sample, 1. / 120., &[], &world)
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
                .advance(91, sample, 1. / 60., &[], &world)
                .unwrap();
            for draw in state
                .buildings
                .draws(bevy::prelude::Mat4::IDENTITY.to_cols_array())
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
