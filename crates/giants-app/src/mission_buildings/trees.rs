//! Source state60 Mecc/pistol profile. Prototype PS2 `tree_process`,
//! `tree_init` and `deathfx_shot` are unknown PC attribution routes.
//! Heavy-actor felling and native wind/collision scheduling remain separate.
use super::*;
pub(super) fn class(
    source: &giants_game::scene_resources::SceneResources,
    class: i32,
) -> Result<Arc<Class>, String> {
    if !matches!(class, 102 | 664) {
        return Err("tree class needs its acquired resources and replacement".into());
    }
    let seed = source.class_state_seed(class).map_err(|e| e.to_string())?;
    if seed.storage.state() != 60 || !seed.diagnostics.is_empty() {
        return Err("tree state profile differs".into());
    }
    let data = seed.storage.data().ok_or("tree payload absent")?;
    if data.len() != 76 {
        return Err("tree payload size differs".into());
    }
    let payload = std::array::from_fn::<_, 19, _>(|i| {
        u32::from_le_bytes(data[i * 4..i * 4 + 4].try_into().unwrap())
    });
    if payload[3] != 0 || payload[4..].iter().any(|v| *v != 0) {
        return Err("tree requires idle unrestricted source payload".into());
    }
    let specification = giants_game::specification::Prepared::load_registered(source, class)?;
    let fields = Fields::load(source, class, &specification)?;
    let effects = source
        .class_state_bindings_for(class, giants_game::state_storage::Profile::Debug)?
        .into_iter()
        .map(|b| (b.offset, b))
        .collect::<BTreeMap<_, _>>();
    for binding in effects.values() {
        binding.target.as_ref().map_err(Clone::clone)?;
    }
    if effects.keys().any(|k| !matches!(k, 4 | 8)) {
        return Err("tree binding outside shared shot prefix".into());
    }
    let replacement = if class == 102 {
        if payload[0] != 664 || !effects.contains_key(&4) {
            return Err("palm source replacement/effect differs".into());
        }
        Some(replacement::Template::load(source, 664)?)
    } else {
        if payload[..4] != [0; 4] || !effects.is_empty() {
            return Err("stump source reaction differs".into());
        }
        None
    };
    let configuration = death_fx::Configuration {
        class,
        limit: f32::from_bits(fields.configuration[12]),
        replacement: payload[0] as i32,
        death_effect: effects.contains_key(&4),
        hit_effect: effects.contains_key(&8),
        restricted: false,
    };
    let stage = Arc::new(Stage::tree_snapshot(source, class)?);
    Ok(Arc::new(Class {
        policy: Policy::Scenery {
            configuration,
            effects,
        },
        fields,
        stages: [(1f32.to_bits(), stage)].into_iter().collect(),
        replacement,
        tree: Some(payload),
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; supplied hits/time and CPU owners, not GPU/ordinary input acceptance"]
    fn installed_palm_damage_keeps_source_stump_owner_and_independent_hits() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_BUILDING_ROOT").unwrap());
        let menu = giants_game::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            giants_game::mission_loading::prepare(&root, menu.request("Story2", 2).unwrap())
                .unwrap();
        let world = packet.world_asset(&root).unwrap();
        let mut state =
            crate::mecc_dev_scene::State::load_prepared(&root, &world, &packet).unwrap();
        let (&id, owner) = state
            .buildings
            .owners
            .iter()
            .find(|(_, o)| o.fields.class == 102)
            .expect("ordinary declared palm owner");
        assert!(owner.physical);
        assert!(!owner.environment);
        assert_eq!(owner.damage.limit, 10.);
        let point = owner.fields.position;
        let key = owner.key;
        let hit = |object, serial| giants_game::mecc_pistol::WorldHit {
            object,
            serial,
            point,
            damage: 12.,
            older: point,
            motion: giants_game::damage_impulse::Motion {
                current: point,
                previous: point,
            },
        };
        let mut random = giants_game::audio_parameters::Random(17);
        let result = state
            .buildings
            .hit(91, hit(id, 1), point, 1, &mut random)
            .unwrap()
            .unwrap();
        assert_eq!(result.update.key, key);
        assert!(result.update.geometry.is_none());
        assert!(result.triangles.is_empty());
        assert!(!result.sounds.is_empty());
        state.apply_building_hit(&result).unwrap();
        let index = result.replacement.expect("source stump");
        let stump = state.buildings.replacements[index].object.unwrap();
        assert_ne!(stump, id);
        assert_eq!(state.buildings.owners[&stump].fields.class, 664);
        assert_eq!(state.buildings.owners[&stump].damage.limit, 100.);
        assert_eq!(
            state.buildings.owners[&stump].tree.unwrap()[4..],
            state.buildings.owners[&id].tree.unwrap()[4..]
        );
        assert!(
            state
                .projectile_objects
                .iter()
                .any(|o| o.id == stump && o.active & 0x8000 == 0)
        );
        assert!(
            state
                .mission_colliders()
                .iter()
                .any(|o| o.fields.id == stump && !o.triangles.is_empty())
        );
        state.set_mission_contacts(91, vec![]).unwrap();
        assert!(state.projectile_objects.iter().any(|o| o.id == stump));
        for serial in 1..=9 {
            let result = state
                .buildings
                .hit(91, hit(stump, serial), point, 1, &mut random)
                .unwrap()
                .unwrap();
            assert!(result.replacement.is_none());
            assert_eq!(result.update.geometry.is_none(), serial == 9);
            state.apply_building_hit(&result).unwrap();
        }
        assert_ne!(
            state
                .projectile_objects
                .iter()
                .find(|o| o.id == stump)
                .unwrap()
                .active
                & 0x8000,
            0
        );
        assert_ne!(
            state.buildings.replacements[index].fields.object_flags & 0x8000,
            0
        );
        assert!(
            state
                .mission_colliders()
                .iter()
                .find(|o| o.fields.id == stump)
                .unwrap()
                .triangles
                .is_empty()
        );
        for frame in 1..=1200 {
            state
                .buildings
                .effects
                .advance(
                    crate::mission_building_effects::Frame {
                        ticket: 91,
                        sample: frame - 1,
                        delta: 1. / 60.,
                        areas: &[],
                        world: &world,
                    },
                    &mut random,
                )
                .unwrap();
            if state.buildings.effects.active() == 0 {
                println!("tree_effects_expired frame={frame}");
                break;
            }
        }
        assert_eq!(state.buildings.effects.active(), 0);
    }
}
