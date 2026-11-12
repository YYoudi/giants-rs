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
        return Err("Ripper input probe requires muted showcase-mecc / Story2 mission1, the normal controller, no Vimp probe and capture revision1..120".into());
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
        crate::frontend_scene::vimp_interaction_probe(),
        flights()?,
    )
}
pub(super) struct Input {
    pub axes: Vec2,
    pub thrust: bool,
    pub boost: bool,
    pub capture: bool,
    pub flight_revision: u64,
}
pub(super) fn input(
    state: &mut crate::mecc_dev_scene::State,
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
        capture: snapshot.flight_revision >= flights()?,
        flight_revision: snapshot.flight_revision,
    };
    if let Some(target) = snapshot.target {
        let dx = target[0] - position[0];
        let dy = target[1] - position[1];
        let distance = dx.hypot(dy);
        let camera = state.subjective_inputs().0;
        let aim = snapshot
            .projectile
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
        input.thrust =
            distance > 120. && position[2] < target[2] + 60. && state.mission_snapshot().1 > 10.;
        input.boost = input.thrust && step.is_multiple_of(360);
    }
    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;
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
