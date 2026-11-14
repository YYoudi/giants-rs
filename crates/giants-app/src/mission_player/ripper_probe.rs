//! Guarded supplied-input GPU diagnostic. Ordinary Ripper admission, AI,
//! projectile simulation and rendering never depend on this flag.
use super::*;
pub(crate) fn enabled() -> bool {
    std::env::var("GIANTS_MISSION_RIPPER_PROBE").as_deref() == Ok("1")
}
fn flights() -> Result<u64, String> {
    std::env::var("GIANTS_MISSION_RIPPER_CAPTURE_FLIGHTS").map_or(Ok(80), |s| {
        s.parse()
            .map_err(|_| "invalid Ripper capture revision".into())
    })
}
fn check(
    replay: &str,
    world: &str,
    mission: Option<usize>,
    muted: bool,
    owned: bool,
    vimp: bool,
    flights: u64,
) -> Result<(), String> {
    if replay != "showcase-mecc"
        || world != "Story2"
        || mission != Some(1)
        || !muted
        || owned
        || vimp
        || !(1..=120).contains(&flights)
    {
        return Err("Ripper input probe requires muted showcase-mecc / Story2 mission1, the normal controller, no competing NPC/objective probe and capture revision1..120".into());
    }
    Ok(())
}
pub(crate) fn validate(app: &App) -> Result<(), String> {
    if !enabled() {
        return Ok(());
    }
    target_class()?;
    check(
        &std::env::var("GIANTS_MISSION_REPLAY").unwrap_or_default(),
        &std::env::var("GIANTS_SHOWCASE_WORLD").unwrap_or_default(),
        std::env::var("GIANTS_SHOWCASE_MISSION")
            .ok()
            .and_then(|s| s.parse().ok()),
        app.world()
            .contains_resource::<crate::cinematic_audio::MutedTransport>(),
        std::env::var_os("GIANTS_MISSION_MECC_BACKEND").is_some(),
        crate::frontend_scene::vimp_interaction_probe()
            || crate::frontend_scene::objective_interaction_probe(),
        flights()?,
    )
}
fn target_class() -> Result<Option<i32>, String> {
    match std::env::var("GIANTS_MISSION_RIPPER_TARGET_CLASS")
        .ok()
        .as_deref()
    {
        None => Ok(None),
        Some("73") => Ok(Some(73)),
        Some("230") => Ok(Some(230)),
        _ => Err("Ripper diagnostic target class must be an actual class73 or class230".into()),
    }
}
pub(super) struct Input {
    pub axes: Vec2,
    pub thrust: bool,
    pub boost: bool,
    pub fire: bool,
    pub capture: bool,
    pub flight_revision: u64,
    pub impact_step: Option<u64>,
}
/// Pilot input memory only. The ordinary controller still owns fuel, altitude,
/// physics and sound. Separate release/hold thresholds prevent a supplied RMB
/// command oscillating every frame at one numeric boundary.
#[derive(Default)]
pub(super) struct Navigation {
    recharging: bool,
    climbing: bool,
    chase_actor: Option<giants_game::resident_states::Id>,
    chase_origin: Option<[f32; 3]>,
    chase_first_shot: Option<u32>,
    previous_target_mode: Option<u32>,
    retreat_completed: bool,
    route: Option<super::route_probe::Route>,
    route_actor: Option<giants_game::resident_states::Id>,
    approach_done: bool,
    escape: super::route_probe::Escape,
    close_origin: Option<[f32; 3]>,
    damage_seen_at: Option<u32>,
    lethal_continuation_reported: bool,
}
impl Navigation {
    fn thrust(&mut self, distance: f32, relative_height: f32, fuel: f32) -> bool {
        if fuel <= 10. {
            self.recharging = true;
        } else if fuel >= 90. {
            self.recharging = false;
        }
        if distance <= 120. || relative_height >= 80. {
            self.climbing = false;
        } else if relative_height <= 40. {
            self.climbing = true;
        }
        self.climbing && !self.recharging
    }
}
pub(super) fn input(
    state: &mut crate::mecc_dev_scene::State,
    navigation: &mut Navigation,
    runtime: &mut Runtime,
    ticket: u64,
    step: u32,
    terrain: &giants_formats::gti::Terrain,
) -> Result<Input, String> {
    let position = state.mission_snapshot().0;
    let snapshot = runtime.ripper_input_snapshot(ticket, position, target_class()?)?;
    let retreat = std::env::var("GIANTS_MISSION_RIPPER_CAPTURE_RETREAT").as_deref() == Ok("1");
    let close = std::env::var("GIANTS_MISSION_RIPPER_CAPTURE_CLASSIC_CLOSE").as_deref() == Ok("1");
    let hit = std::env::var("GIANTS_MISSION_RIPPER_CAPTURE_HIT").as_deref() == Ok("1");
    let pistol = std::env::var("GIANTS_MISSION_RIPPER_CAPTURE_PISTOL").as_deref() == Ok("1");
    let lethal_boundary =
        std::env::var("GIANTS_MISSION_RIPPER_CAPTURE_LETHAL_BOUNDARY").as_deref() == Ok("1");
    let pistol = pistol || lethal_boundary;
    let routed = retreat || close || hit || pistol;
    let chase =
        routed || std::env::var("GIANTS_MISSION_RIPPER_CAPTURE_CHASE").as_deref() == Ok("1");
    if chase && navigation.chase_actor != snapshot.target_actor {
        navigation.chase_actor = snapshot.target_actor;
        navigation.chase_origin = snapshot.target_position;
        navigation.chase_first_shot = None;
        navigation.previous_target_mode = None;
        navigation.retreat_completed = false;
        navigation.close_origin = None;
    }
    if close {
        if snapshot.target_mode == Some(41) {
            if navigation.close_origin.is_none() {
                navigation.close_origin = snapshot.target_position;
            }
        } else {
            navigation.close_origin = None;
        }
    }
    if retreat {
        if navigation.previous_target_mode == Some(43) && snapshot.target_mode == Some(31) {
            navigation.retreat_completed = true;
            println!(
                "mission_ripper_retreat_observed ticket={ticket} step={step} owner={:?} from_mode=43 to_mode=31 source=current_owned_actor forced_pose=false",
                snapshot.target_actor
            );
        }
        navigation.previous_target_mode = snapshot.target_mode;
    }
    if chase
        && snapshot.projectile_source.is_some()
        && snapshot.projectile_source == snapshot.target_actor
    {
        navigation.chase_first_shot.get_or_insert(step);
    }
    let chase_moved = navigation
        .chase_origin
        .zip(snapshot.target_position)
        .is_some_and(|(a, b)| (a[0] - b[0]).hypot(a[1] - b[1]) >= 20.);
    let matching_injury = snapshot.mecc_damage.is_some_and(|d| d > 0.)
        && snapshot
            .mecc_remaining
            .zip(snapshot.mecc_limit)
            .is_some_and(|(remaining, limit)| {
                remaining > 0.
                    && remaining.round() < limit.round()
                    && remaining.to_bits() == state.remaining_health().to_bits()
            });
    if hit && matching_injury {
        if navigation.damage_seen_at.is_none() {
            navigation.damage_seen_at = Some(step);
            println!(
                "mission_ripper_injury_observed ticket={ticket} step={step} canonical_damage={:?} canonical_remaining={:?} controller_remaining={} actual_response_matched=true probe_wrote_damage=false os_input=false",
                snapshot.mecc_damage,
                snapshot.mecc_remaining,
                state.remaining_health()
            );
        }
    } else {
        navigation.damage_seen_at = None;
    }
    let mut input = Input {
        axes: Vec2::ZERO,
        thrust: false,
        boost: false,
        fire: false,
        // Read-only endpoints over ordinary consumers. No damage, impact or
        // availability is supplied by selecting a diagnostic capture target.
        capture: if lethal_boundary {
            snapshot
                .unsupported_lethal
                .is_some_and(|(_, _, at, completed)| {
                    completed.is_some_and(|completed| completed >= at + 240)
                })
                && snapshot.pistol_target_active
                && snapshot
                    .pistol_target_health
                    .is_some_and(|(damage, limit)| damage > 0. && damage < limit)
        } else if pistol {
            snapshot
                .pistol_hit
                .is_some_and(|(_, _, at)| u64::from(step) >= at + 30)
                && snapshot
                    .pistol_target_health
                    .is_some_and(|(damage, limit)| damage > 0. && damage < limit)
                && snapshot.pistol_target_active
                && snapshot.blood_quads > 0
        } else if close {
            snapshot.target_mode == Some(41)
                && snapshot.target_has_launch
                && navigation
                    .close_origin
                    .zip(snapshot.target_position)
                    .is_some_and(|(a, b)| (a[0] - b[0]).hypot(a[1] - b[1]) >= 20.)
        } else if retreat {
            navigation.retreat_completed
        } else if hit {
            // Hit navigation also chases; give its stricter endpoint priority.
            // Allow actual player/HUD publication after the late enemy callback.
            matching_injury
                && navigation
                    .damage_seen_at
                    .is_some_and(|at| step.saturating_sub(at) >= 240)
        } else if chase {
            chase_moved
                && snapshot.flight_revision >= flights()?
                && snapshot.projectile_source == snapshot.target_actor
        } else if std::env::var("GIANTS_MISSION_RIPPER_CAPTURE_CONTINUATION").as_deref() == Ok("1")
        {
            snapshot
                .quarantine_step
                .is_some_and(|at| u64::from(step) >= at + 120)
                && snapshot.flight_revision >= flights()?
        } else if std::env::var("GIANTS_MISSION_RIPPER_CAPTURE_IMPACT").as_deref() == Ok("1") {
            snapshot
                .impact
                .is_some_and(|(_, at)| u64::from(step) >= at + 16)
        } else {
            snapshot.flight_revision >= flights()?
        },
        flight_revision: snapshot.flight_revision,
        impact_step: snapshot.impact.map(|(_, at)| at),
    };
    if lethal_boundary && input.capture && !navigation.lethal_continuation_reported {
        navigation.lethal_continuation_reported = true;
        println!(
            "mission_ripper_unsupported_lethal_continued ticket={ticket} step={step} observation={:?} health={:?} active={} source=ordinary_completed_primary no_death_claim=true os_input=false",
            snapshot.unsupported_lethal,
            snapshot.pistol_target_health,
            snapshot.pistol_target_active
        );
    }
    if let Some(target) = snapshot.target {
        let dx = target[0] - position[0];
        let dy = target[1] - position[1];
        let distance = dx.hypot(dy);
        input.fire = pistol
            && if lethal_boundary {
                snapshot.unsupported_lethal.is_none()
            } else {
                snapshot.pistol_hit.is_none()
            }
            && distance < 180.;
        let waypoint = if routed && !navigation.approach_done && distance > 250. {
            if navigation.route_actor != snapshot.target_actor {
                navigation.route = Some(super::route_probe::Route::prepare(
                    terrain,
                    position,
                    snapshot.target_position.unwrap_or(target),
                    0.35,
                )?);
                navigation.route_actor = snapshot.target_actor;
            }
            navigation
                .route
                .as_mut()
                .and_then(|route| route.next(position))
        } else {
            navigation.approach_done |= routed;
            None
        };
        let camera = state.subjective_inputs().0;
        let aim = if let Some(waypoint) = waypoint {
            waypoint
        } else if chase
            || std::env::var("GIANTS_MISSION_RIPPER_CAPTURE_CONTINUATION").as_deref() == Ok("1")
        {
            [target[0], target[1], target[2] + 8.]
        } else {
            snapshot
                .projectile
                .or(snapshot.impact.map(|(position, _)| position))
                .unwrap_or([target[0], target[1], target[2] + 8.])
        };
        let desired = (aim[0] - position[0]).atan2(-(aim[1] - position[1]));
        let horizontal = (aim[0] - camera.position[0]).hypot(aim[1] - camera.position[1]);
        let pitch = (camera.position[2] - aim[2]).atan2(horizontal.max(1.));
        state.mission_look(
            Vec2::new(
                (camera.angles[0].to_radians() - desired + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI,
                pitch + camera.angles[1].to_radians(),
            ),
            false,
        );
        if distance > 5. {
            input.axes = Vec2::Y;
        }
        // Supplied player retreat; never an NPC position or mode write.
        // Injury observation must actually approach contact. The earlier
        // pursuit capture deliberately stops the player at range; reusing
        // that hold prevents the normal source close attack from being tried.
        if chase
            && !hit
            && !pistol
            && let Some(first) = navigation.chase_first_shot
        {
            if step < first + 240 {
                input.axes = Vec2::NEG_Y;
            } else if distance < 200. {
                input.axes = Vec2::ZERO;
            }
        }
        let previous = (navigation.recharging, navigation.climbing);
        let fuel = state.mission_snapshot().1;
        // Input pilot only: use the actual local terrain clearance. A distant
        // low target cannot justify walking into the departure ridge forever.
        let clearance = position[2] - snapshot.floor_height;
        if let Some(waypoint) = waypoint {
            let (axes, thrust, boost) =
                navigation
                    .escape
                    .commands(position, waypoint, fuel, &mut navigation.recharging);
            input.axes = Vec2::from_array(axes);
            input.thrust = thrust;
            input.boost = boost;
        } else {
            input.thrust = navigation.thrust(distance, clearance, fuel);
            input.boost = input.thrust && step.is_multiple_of(360);
        }
        if previous != (navigation.recharging, navigation.climbing) || step.is_multiple_of(240) {
            println!(
                "mission_ripper_supplied_jet ticket={ticket} step={step} fuel={fuel} relative_height={} terrain_clearance={clearance} held={} recharging={} climbing={} player={position:?} waypoint={waypoint:?} supplied_input_only=true os_input=false",
                position[2] - target[2],
                input.thrust,
                navigation.recharging,
                navigation.climbing
            );
        }
    }
    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn supplied_jet_releases_until_refuelled_and_holds_across_hover_boundary() {
        let mut navigation = Navigation::default();
        assert!(navigation.thrust(300., 30., 100.));
        for height in [59.9, 60.1, 59.9, 70.] {
            assert!(navigation.thrust(300., height, 50.));
        }
        assert!(!navigation.thrust(300., 80., 50.));
        assert!(!navigation.thrust(300., 60., 50.));
        assert!(navigation.thrust(300., 40., 50.));
        assert!(!navigation.thrust(300., 30., 10.));
        for fuel in [10.1, 20., 89.9] {
            assert!(!navigation.thrust(300., 30., fuel));
        }
        assert!(navigation.thrust(300., 30., 90.));
        assert!(!navigation.thrust(120., 30., 100.));
    }
    #[test]
    fn probe_refuses_other_transports_missions_and_controller_backends() {
        assert!(check("showcase-mecc", "Story2", Some(1), true, false, false, 80).is_ok());
        assert!(check("showcase-mecc", "Story2", Some(1), false, false, false, 80).is_err());
        assert!(check("showcase-mecc", "Story2", Some(0), true, false, false, 80).is_err());
        assert!(check("showcase-mecc", "Story2", Some(1), true, true, false, 80).is_err());
        assert!(check("showcase-mecc", "Story2", Some(1), true, false, true, 80).is_err());
        assert!(check("showcase-mecc", "Story2", Some(1), true, false, false, 121).is_err());
        assert!(check("showcase-mecc", "Story2", Some(1), true, false, false, 0).is_err());
    }
}
