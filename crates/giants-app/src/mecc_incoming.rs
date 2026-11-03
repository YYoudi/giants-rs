//! Explicit stationary test attack driver, not native AI or mission scripting.
use bevy::prelude::*;
use giants_game::{
    arithmetic::Precision,
    straight_projectile::{Event, Shot},
};
#[derive(Resource)]
pub(crate) struct Incoming {
    pub shots: Vec<Shot>,
    pub enabled: bool,
    definition: giants_formats::weapons::Projectile,
    cooldown: f32,
    remainder: f64,
    tick: u32,
    fired: u32,
    hits: u32,
    replay: bool,
    pose: Option<u32>,
    done: bool,
}
impl Incoming {
    pub(crate) fn fresh(&self) -> bool {
        self.shots.is_empty()
            && !self.enabled
            && self.fired == 0
            && self.hits == 0
            && self.cooldown == 0.
    }
    fn reset(&mut self) {
        self.shots.clear();
        self.enabled = false;
        self.cooldown = 0.;
        self.fired = 0;
        self.hits = 0;
    }
    fn step(
        &mut self,
        actor: &mut crate::mecc_dev_scene::State,
        object: &crate::mecc_target::Target,
        terrain: &giants_formats::gti::Terrain,
        held: bool,
    ) -> Result<(), &'static str> {
        let dt = 1. / 120.;
        self.cooldown =
            giants_game::mecc_fire::advance_clock(self.cooldown, dt, Precision::Double53)?;
        let (fire, next) = giants_game::mecc_fire::request(
            self.cooldown,
            self.definition.interval,
            self.definition.flags,
            true,
            false,
            held && actor.alive() && object.enabled && object.health.alive(),
            Precision::Double53,
        )?;
        self.cooldown = next;
        let geometry = if actor.alive() && (fire || !self.shots.is_empty()) {
            Some(actor.projectile_geometry()?)
        } else {
            None
        };
        if fire {
            let aim = geometry.as_ref().expect("live prepared player").center();
            self.shots
                .push(Shot::new(&self.definition, object.aim, aim)?);
            self.fired += 1;
            println!(
                "mecc_dev_incoming shot={} origin=development_target_center ai=false",
                self.fired
            );
        }
        let mut live = Vec::new();
        for mut shot in self.shots.drain(..) {
            let start = shot.motion.position;
            let event = shot.step(terrain, dt)?;
            let hit = if actor.alive() {
                geometry
                    .as_ref()
                    .map(|g| g.hit(start, shot.motion.position))
                    .transpose()?
                    .flatten()
                    .is_some()
            } else {
                false
            };
            if hit {
                self.hits += 1;
                actor.receive_hit(self.definition.direct_damage)?;
                continue;
            }
            if matches!(event, Event::Flying) {
                live.push(shot);
            }
        }
        self.shots = live;
        if !actor.alive() {
            self.shots.clear();
        }
        Ok(())
    }
}
pub(crate) fn load(root: &std::path::Path) -> Result<Incoming, Box<dyn std::error::Error>> {
    let catalogue = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin"))?,
        giants_formats::weapons::Profile::Debug,
    )?;
    let definition = catalogue
        .projectiles
        .get(&17)
        .ok_or("test pistol absent")?
        .clone();
    let mode = std::env::var("GIANTS_MECC_DEV_REPLAY").ok();
    let replay = mode.as_deref().is_some_and(|m| m.starts_with("player-"));
    Ok(Incoming {
        shots: Vec::new(),
        enabled: false,
        definition,
        cooldown: 0.,
        remainder: 0.,
        tick: 0,
        fired: 0,
        hits: 0,
        replay,
        pose: match mode.as_deref() {
            Some("player-hit-pose") => Some(300),
            Some("player-dead-pose") => Some(900),
            _ => None,
        },
        done: false,
    })
}
pub fn install(app: &mut App, root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    app.insert_resource(load(root)?).add_systems(
        Update,
        update
            .after(crate::mecc_combat::CombatUpdate)
            .before(crate::source_view::ViewUpdate)
            .run_if(crate::mecc_session::playing),
    );
    Ok(())
}
#[allow(clippy::too_many_arguments)]
fn update(
    mut incoming: ResMut<Incoming>,
    mut actor: ResMut<crate::mecc_dev_scene::State>,
    mut combat: ResMut<crate::mecc_combat::Combat>,
    mut object: ResMut<crate::mecc_target::Target>,
    mut scene: ResMut<crate::world_scene::Scene>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    mut exit: MessageWriter<AppExit>,
) {
    let focused = windows.iter().any(|w| w.focused);
    if focused && keys.just_pressed(KeyCode::F5) {
        incoming.reset();
    }
    if focused && keys.just_pressed(KeyCode::F8) && actor.alive() {
        incoming.enabled = !incoming.enabled;
    }
    if !incoming.done {
        incoming.remainder += f64::from(time.delta_secs().min(0.1));
        while incoming.remainder >= 1. / 120. {
            if incoming.replay {
                incoming.enabled = (60..550).contains(&incoming.tick);
                if incoming.tick == 900 {
                    let passed = incoming.fired == 9
                        && incoming.hits == 9
                        && actor.deaths == 1
                        && !actor.alive();
                    println!(
                        "mecc_dev_player_death_check passed={passed} hits={} deaths={} hp={}",
                        incoming.hits,
                        actor.deaths,
                        actor.remaining_health()
                    );
                    if !passed {
                        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                        return;
                    }
                }
                if incoming.tick == 960 && incoming.pose.is_none() {
                    incoming.reset();
                    actor.reset_pose();
                    combat.reset();
                    object.reset();
                    println!(
                        "mecc_dev_player_restart passed={} hp={} incoming={}",
                        actor.alive()
                            && actor.remaining_health() == 100.
                            && actor.deaths == 0
                            && incoming.shots.is_empty(),
                        actor.remaining_health(),
                        incoming.shots.len()
                    );
                }
                if incoming.tick == 1080 || incoming.pose == Some(incoming.tick) {
                    incoming.done = true;
                    println!(
                        "mecc_dev_player_hit_finished passed=true steps={} hp={} pose={} owner_validation=false",
                        incoming.tick,
                        actor.remaining_health(),
                        incoming.pose.is_some()
                    );
                    break;
                }
            }
            let held = incoming.enabled;
            if let Err(error) = incoming.step(&mut actor, &object, &scene.asset.terrain, held) {
                error!("mecc_dev_incoming_failed={error}");
                exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                return;
            }
            incoming.tick += 1;
            incoming.remainder -= 1. / 120.;
        }
    }
    if incoming.replay {
        scene.cinematic_capture_ready = incoming.done;
    }
}
