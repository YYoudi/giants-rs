use super::*;
use crate::{
    declared_residents::{self, Scope},
    resident_states::Session,
};

fn time() -> Time<'static> {
    Time {
        seconds: 1. / 120.,
        areas: &[],
        control_factor: 1.,
        running_factor: 1.,
        skip_heading: false,
        controlled_actor: false,
    }
}

#[test]
#[ignore = "requires GIANTS_BUILDING_ROOT; actual pub resources/terrain, supplied empty neighbours and failure services; no normal mission/GPU/input acceptance"]
fn installed_pub_body_retains_current_collision_and_failed_phase_ownership() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_BUILDING_ROOT").unwrap());
    let menu = crate::mission_selection::MissionMenu::parse(
        &std::fs::read(root.join("Bin/w_Story1.bin")).unwrap(),
        0,
        true,
    )
    .unwrap();
    let packet =
        crate::mission_loading::prepare(&root, menu.request("Story1", 0).unwrap()).unwrap();
    let plan = packet.instance_plan(&root, "English").unwrap();
    let world = packet.world_asset(&root).unwrap();
    let terrain = &world.terrain;
    let query = crate::terrain_queries::StaticTerrainQuery::new(
        terrain,
        1. / terrain.spacing,
        Precision::Double53,
    )
    .unwrap();
    let mut session = Session::new(422);
    let report = declared_residents::construct_acquired(&plan, &mut session);
    let record = report.records.iter().find(|r| r.class == Some(53)).unwrap();
    let id = *record.result.as_ref().unwrap();
    let key = (record.key.scope == Scope::World, record.key.index);
    let collision_id = (u32::try_from(key.1).unwrap() + 2) | if key.0 { 0x4000_0000 } else { 0 };
    let ground = |p: [f32; 3]| {
        let s = query.sample([p[0], p[1]])?;
        Ok((s.height, s.plane))
    };
    session
        .prepare_ordinary_building_body(id, collision_id, plan.resources(), ground)
        .unwrap();
    assert!(
        session
            .prepare_ordinary_building_body(id, collision_id, plan.resources(), ground)
            .is_err()
    );
    assert!(Session::new(423).ordinary_building_body(id).is_err());
    let first = session.ordinary_building_player_mesh(id).unwrap();
    assert_eq!(first.fields.id, collision_id);
    assert!(!first.triangles.is_empty());
    assert!(
        first
            .triangles
            .iter()
            .all(|t| t.ordinal != 0 && t.vertices.into_iter().flatten().all(f32::is_finite))
    );
    let initial_channels = session.get(id).unwrap().channels;
    for number in 0..12 {
        let slope = f32::from_bits(
            session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .configuration[11],
        );
        let mut services = crate::actor_environment::FinalizationServices {
            actor: collision_id,
            state: 7,
            slope_parameter: slope,
            physics: None,
            skip_object_responses: false,
            objects: &[],
            ground: |p| {
                crate::physical_ground::Ground::from_sample(query.sample(p)?, terrain.width)
            },
            platform: |p| crate::platform_queries::sample_prepared(p, &[], Precision::Double53),
            seconds: |_| f64::from(1_f32 / 120.),
            release: |identity: Option<u32>| {
                if identity.is_none() {
                    Ok(())
                } else {
                    Err("unowned footprint".into())
                }
            },
            statistics: None,
        };
        session
            .advance_ordinary_building_body(id, number, time(), &mut services)
            .unwrap();
        assert!(
            session
                .advance_ordinary_building_body(id, number, time(), &mut services)
                .is_err()
        );
        let body = session.ordinary_building_body(id).unwrap();
        let fields = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        assert_eq!(body.steps(), number + 1);
        assert_eq!(
            body.fields().current.map(f32::to_bits),
            fields.position.map(f32::to_bits)
        );
        assert_eq!(
            body.affine().unwrap().matrix.map(f32::to_bits),
            session
                .actor_current_affine(id)
                .unwrap()
                .matrix
                .map(f32::to_bits)
        );
        let mesh = session.ordinary_building_player_mesh(id).unwrap();
        assert_eq!(mesh.fields.bounds.min, body.fields().bounds[..3]);
        assert_eq!(mesh.fields.bounds.max, body.fields().bounds[3..]);
        assert_eq!(mesh.triangles.len(), first.triangles.len());
        assert_eq!(session.get(id).unwrap().channels, initial_channels);
        assert!(session.actor_world_tick(id).is_err());
    }
    struct Missing;
    impl actor_finalization::Services for Missing {
        fn release_footprint(&mut self, id: Option<u32>) -> Result<(), String> {
            assert!(id.is_none());
            Ok(())
        }
        fn environment(&mut self, state: &mut actor_finalization::State) -> Result<(), String> {
            state.fields.floor = 123.;
            Err("controlled missing environment after phase entry".into())
        }
    }
    assert!(
        session
            .advance_ordinary_building_body(id, 12, time(), &mut Missing)
            .is_err()
    );
    assert_eq!(
        session.ordinary_building_body(id).unwrap().fields().floor,
        123.
    );
    assert!(
        session
            .ordinary_building_body(id)
            .unwrap()
            .failure()
            .is_some()
    );
    assert!(
        session
            .ordinary_building_body(id)
            .unwrap()
            .affine()
            .is_err()
    );
    assert!(session.ordinary_building_player_mesh(id).is_err());
    assert!(session.actor_current_affine(id).is_err());
    assert!(
        session
            .advance_ordinary_building_body(id, 12, time(), &mut Missing)
            .is_err()
    );
    session.clear();
    assert!(session.ordinary_building_body(id).is_err());
    let fresh = declared_residents::construct_acquired(&plan, &mut session);
    let next = *fresh
        .records
        .iter()
        .find(|r| r.class == Some(53))
        .unwrap()
        .result
        .as_ref()
        .unwrap();
    session
        .prepare_ordinary_building_body(next, collision_id, plan.resources(), ground)
        .unwrap();
    assert_eq!(session.ordinary_building_body(next).unwrap().steps(), 0);
    assert_eq!(
        session
            .ordinary_building_player_mesh(next)
            .unwrap()
            .triangles
            .len(),
        first.triangles.len()
    );
    let select = |session: &mut Session, owner| {
        session
            .select_primary(
                owner,
                crate::animation_channels::Request {
                    slot: 0,
                    flags: 0,
                    speed_source: -1,
                    reference_speed: 1.,
                    blend_seconds: 0.,
                },
                crate::animation_channels::Blending::Disabled,
            )
            .unwrap();
    };
    select(&mut session, next);
    assert!(session.ordinary_building_body(next).is_err());
    assert!(session.ordinary_building_player_mesh(next).is_err());
    session.clear();
    let fresh = declared_residents::construct_acquired(&plan, &mut session);
    let next = *fresh
        .records
        .iter()
        .find(|r| r.class == Some(53))
        .unwrap()
        .result
        .as_ref()
        .unwrap();
    select(&mut session, next);
    assert!(
        session
            .prepare_ordinary_building_body(next, collision_id, plan.resources(), ground)
            .is_err()
    );
}
