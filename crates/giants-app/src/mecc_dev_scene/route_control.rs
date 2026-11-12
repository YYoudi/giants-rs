//! Installed motion diagnosis only. This never acknowledges campaign completion.
use super::*;
use crate::mission_player::route_probe as route;

#[test]
#[ignore = "Requires GIANTS_ROUTE_CONTROL_ROOT and private GIANTS_ROUTE_CONTROL_POINTS; supplied commands, no GPU or OS input"]
fn installed_round_trip_with_ordinary_motion() {
    let root = std::path::PathBuf::from(std::env::var("GIANTS_ROUTE_CONTROL_ROOT").unwrap());
    let points = std::fs::read_to_string(std::env::var("GIANTS_ROUTE_CONTROL_POINTS").unwrap())
        .unwrap()
        .lines()
        .map(|line| {
            let v = line
                .split_whitespace()
                .map(|v| v.parse::<f32>().unwrap())
                .collect::<Vec<_>>();
            <[f32; 3]>::try_from(v).unwrap()
        })
        .collect::<Vec<_>>();
    assert!(!points.is_empty());
    let bytes = std::fs::read(root.join("Bin/w_Story2.bin")).unwrap();
    let menu = giants_game::mission_selection::MissionMenu::parse(&bytes, 0, true).unwrap();
    let packet =
        giants_game::mission_loading::prepare(&root, menu.request("Story2", 0).unwrap()).unwrap();
    let world = packet.world_asset(&root).unwrap();
    let plan = packet.instance_plan(&root, "English").unwrap();
    let player = giants_game::player_initial::Prepared::load(
        &root,
        &plan,
        packet.world.as_ref().unwrap(),
        packet.mission.as_ref().unwrap(),
    )
    .unwrap();
    let mut actor = State::load_mission(&root, &world, &packet, &player).unwrap();
    let mut steps = 0;
    for goal in points {
        let mut path =
            route::Route::prepare(&world.terrain, actor.motion.position, goal, 0.35).unwrap();
        let mut recharge = false;
        let mut reached = false;
        for tick in 0..120000 {
            let p = actor.motion.position;
            if v3(p).distance(v3(goal)) < 8. {
                reached = true;
                break;
            }
            let target = path.next(p).unwrap();
            let dx = target[0] - p[0];
            let dy = target[1] - p[1];
            actor.mission_look(Vec2::new(actor.yaw - dx.atan2(-dy), 0.), false);
            let (axes, thrust, boost) = route::commands(p, target, actor.jet.fuel, &mut recharge);
            let axes = Vec2::from_array(axes);
            actor
                .step(1. / 120., axes, &world.terrain, thrust, boost)
                .unwrap();
            steps += 1;
            if tick % 1200 == 0 {
                println!(
                    "route_control step={steps} position={p:?} target={target:?} fuel={} boost={} flags={} normal={:?} alive={} axes={axes:?} thrust={thrust}",
                    actor.jet.fuel,
                    actor.jet.boost,
                    actor.physics_flags,
                    actor.ground_normal,
                    actor.alive()
                );
            }
        }
        assert!(
            reached,
            "ordinary commands did not reach private waypoint; position={:?}",
            actor.motion.position
        );
    }
    println!(
        "route_control_completed steps={steps} supplied_commands=true positions_written=false runtime_campaign_proof=false"
    );
}
