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
pub(super) struct Input {
    pub axes: Vec2,
    pub thrust: bool,
    pub boost: bool,
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
) -> Result<Input, String> {
    let position = state.mission_snapshot().0;
    let snapshot = runtime.ripper_input_snapshot(ticket, position)?;
    let mut input = Input {
        axes: Vec2::ZERO,
        thrust: false,
        boost: false,
        // Read-only endpoints over ordinary consumers. No damage, impact or
        // availability is supplied by selecting a diagnostic capture target.
        capture: if std::env::var("GIANTS_MISSION_RIPPER_CAPTURE_HIT").as_deref() == Ok("1") {
            snapshot.mecc_damage.is_some_and(|damage| damage > 0.)
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
    if let Some(target) = snapshot.target {
        let dx = target[0] - position[0];
        let dy = target[1] - position[1];
        let distance = dx.hypot(dy);
        let camera = state.subjective_inputs().0;
        let aim = snapshot
            .projectile
            .or(snapshot.impact.map(|(position, _)| position))
            .unwrap_or([target[0], target[1], target[2] + 8.]);
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
        let previous = (navigation.recharging, navigation.climbing);
        let fuel = state.mission_snapshot().1;
        input.thrust = navigation.thrust(distance, position[2] - target[2], fuel);
        if previous != (navigation.recharging, navigation.climbing) || step.is_multiple_of(240) {
            println!(
                "mission_ripper_supplied_jet ticket={ticket} step={step} fuel={fuel} relative_height={} held={} recharging={} climbing={} supplied_input_only=true os_input=false",
                position[2] - target[2],
                input.thrust,
                navigation.recharging,
                navigation.climbing
            );
        }
        input.boost = input.thrust && step.is_multiple_of(360);
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
