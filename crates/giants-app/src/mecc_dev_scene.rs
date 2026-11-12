//! Explicit Mecc development scene. Never acknowledges campaign readiness.
use bevy::{asset::RenderAssetUsages, input::mouse::AccumulatedMouseMotion, prelude::*};
use giants_game::{arithmetic::Precision, mecc_motion::Motion};
use std::{collections::BTreeMap, path::Path};
mod collision_replay;
mod mission_contacts;
#[cfg(test)]
mod route_control;

struct Clip {
    name: String,
    animation: giants_formats::anm::Animation,
    tree: String,
    duration: f32,
    initial_positions: Vec<[f32; 3]>,
    changed: bool,
}
#[derive(Component)]
pub struct BodyDraw;
#[derive(Component)]
struct JetGauge(bool);
#[derive(Component)]
struct JetLabel;

pub(crate) fn camera_view(eye: Vec3, forward: Vec3, up: Vec3) -> Mat4 {
    let z = forward.normalize();
    // Source cameras reflect the forward axis after inversion of their
    // right-handed animation basis. Preserve that negative determinant.
    let x = z.cross(up).normalize();
    let y = x.cross(z);
    Mat4::from_cols(
        Vec4::new(x.x, y.x, z.x, 0.),
        Vec4::new(x.y, y.y, z.y, 0.),
        Vec4::new(x.z, y.z, z.z, 0.),
        Vec4::new(-x.dot(eye), -y.dot(eye), -z.dot(eye), 1.),
    )
}

#[derive(Resource)]
pub struct State {
    collision: giants_game::collision_radius::ColdProfile,
    collision_dimensions: [f32; 2],
    mesh_objects: Vec<giants_game::player_world_collision::Object>,
    static_mesh_objects: usize,
    npc_contact_keys: BTreeMap<giants_game::resident_states::Id, u32>,
    next_contact_key: u32,
    npc_contacts: Vec<mission_contacts::Contact>,
    npc_mesh_responses: u64,
    environment_objects: Vec<giants_game::actor_environment::Collider>,
    pub(crate) projectile_objects: Vec<giants_game::projectile_world::Object>,
    static_projectile_objects: usize,
    pub(crate) projectile_test: Option<([f32; 3], [f32; 3], u32)>,
    mesh_responses: u64,
    pub(crate) rigid_source: std::sync::Arc<giants_game::static_geometry::Scene>,
    collision_walk: Option<collision_replay::Walk>,
    health: giants_game::mecc_damage::State,
    landing_impact: Option<[f32; 3]>,
    impact_parameters: giants_game::mecc_impact::Parameters,
    contact_speed_thresholds: [f32; 2],
    pub(crate) deaths: u32,
    floor_height: f32,
    pub(crate) weapon: giants_game::held_weapon::Weapon,
    pub(crate) body: giants_game::cast::Body,
    positions: Vec<[f32; 3]>,
    owned_pose: Option<giants_game::mecc_presentation::Frame>,
    pub(crate) scale: f32,
    pub(crate) start: Motion,
    motion: Motion,
    yaw: f32,
    pub(crate) initial_yaw: f32,
    pitch: f32,
    parameters: giants_game::mecc_initial::Parameters,
    item_catalog: giants_formats::weapons::Catalog,
    inventory: giants_game::mecc_inventory::Inventory,
    load_response_flags: u32,
    pub(crate) mission_pistol: Option<giants_game::mecc_pistol::Pistol>,
    subjective_weapon_matrix: Option<[f32; 16]>,
    gravity_mass: f32,
    initial_gravity_mass: f32,
    special_mass: f32,
    jet_parameters: giants_game::mecc_jet::Parameters,
    jet: giants_game::mecc_jet::State,
    jet_flags: u32,
    pending_boost: bool,
    thrusting: bool,
    audio_boost_started: bool,
    jet_replay: bool,
    jet_checks: [bool; 5],
    jet_depletions: u64,
    remainder: f64,
    seconds: f64,
    last_log: i32,
    pub(crate) draws: Vec<(usize, Handle<Mesh>)>,
    replay_tick: Option<u32>,
    replay_checks: [bool; 3],
    clips: BTreeMap<u8, Clip>,
    active_clip: u8,
    clock: giants_game::animation_loop::Clock,
    last_pose: Option<(u8, i32)>,
    grounded: bool,
    replay_pose_tick: Option<u32>,
    pose_capture_reported: bool,
    combat_replay: bool,
    first_person: bool,
    initial_first_person: bool,
    last_view: Option<bool>,
    ground_offset: f32,
    ground_offset_source: f32,
    physics_flags: u32,
    support: u32,
    support_part: u32,
    support_triangle: u32,
    footsteps: Option<giants_game::mecc_footsteps::Prepared>,
    foot_cues: Vec<giants_game::mecc_footsteps::Cue>,
    impact_audio: Option<giants_game::mecc_impacts::Prepared>,
    foot_materials: BTreeMap<(u32, u32), String>,
    ground_normal: [f32; 3],
    initial_normal: [f32; 3],
    ground_plane: [f32; 4],
    initial_plane: [f32; 4],
    initial_floor: f32,
    idle_passed: bool,
    contact_flags: u32,
    model_vertical_speed: f32,
    model_motion: giants_game::mecc_model_motion::Output,
    model_speed_minimums: [f32; 2],
    locomotion: giants_game::mecc_locomotion::Parameters,
    descent: giants_game::mecc_locomotion::Descent,
    plane_contacts: u64,
    terrain_edge_transitions: u64,
    ground_queries: u64,
    ground_junctions: u64,
    ground_target_steps: u64,
    ground_target_max_normal_speed: f32,
}
fn v3(p: [f32; 3]) -> Vec3 {
    Vec3::new(p[0], p[1], p[2])
}
impl State {
    pub(crate) fn normal_jet_parent(&self) -> crate::mission_player::jet::Parent<'_> {
        crate::mission_player::jet::Parent {
            animation: &self.clips[&self.active_clip].animation,
            seconds: self.clock.seconds,
            world: self.object_matrix(),
            scale: self.scale,
            flags: self.jet_flags,
            boost: self.jet.boost_active,
        }
    }
    pub(crate) fn owned_jet_frames(&self) -> Option<&[giants_game::mecc_jet_presentation::Frame]> {
        self.owned_pose.as_ref().map(|v| v.children.as_slice())
    }
    pub(crate) fn mission_environment_colliders(
        &self,
    ) -> &[giants_game::actor_environment::Collider] {
        &self.environment_objects
    }
    pub(crate) fn mission_colliders(&self) -> &[giants_game::player_world_collision::Object] {
        &self.mesh_objects
    }
    pub(crate) fn mission_contact_materials(&self) -> &BTreeMap<(u32, u32), String> {
        &self.foot_materials
    }
    pub(crate) fn accept_owned_pose(
        &mut self,
        frame: giants_game::mecc_presentation::Frame,
    ) -> Result<(), String> {
        if self
            .body
            .source
            .as_ref()
            .is_none_or(|s| s.mesh != frame.geometry)
            || frame.positions.len() != self.positions.len()
            || self
                .owned_pose
                .as_ref()
                .is_some_and(|old| old.owner != frame.owner || old.number >= frame.number)
            || self.owned_pose.is_none() && frame.number != 1
            || frame
                .camera_cache
                .as_ref()
                .is_none_or(|c| c.number != frame.number)
            || frame
                .draw_model
                .as_ref()
                .is_some_and(|d| d.source() != frame.owner || d.number() != frame.number)
        {
            return Err(
                "owned presentation is foreign, stale or bound to different body geometry".into(),
            );
        }
        let clip = u8::try_from(frame.channels[0][0])
            .map_err(|_| "owned primary clip outside byte profile")?;
        self.motion.position = frame.position;
        self.motion.velocity = frame.velocity;
        self.motion.mass = frame.mass;
        self.health = frame.health;
        self.jet = frame.jet;
        self.jet_flags = frame.object_flags;
        self.thrusting = frame.object_flags & 0x1000 != 0;
        self.yaw = frame.angles[0].to_radians();
        self.pitch = -frame.look_pitch.to_radians();
        self.scale = frame.scale;
        self.model_motion.model = frame.model_rates[..3].try_into().unwrap();
        self.active_clip = clip;
        self.clock.seconds = f32::from_bits(frame.channels[0][1]);
        self.clock.previous = f32::from_bits(frame.channels[0][2]);
        self.positions = frame.positions.clone();
        self.owned_pose = Some(frame);
        Ok(())
    }
    pub(crate) fn mission_look(&mut self, delta: Vec2, toggle_view: bool) {
        if self.alive() {
            assert!(
                self.owned_pose.is_none() || delta == Vec2::ZERO,
                "owned primary look must come from commands, not development angles"
            );
            if self.owned_pose.is_none() {
                self.yaw = (self.yaw - delta.x).rem_euclid(std::f32::consts::TAU);
                self.pitch = (self.pitch + delta.y).clamp(-1.45, 1.45);
            }
            if toggle_view {
                self.first_person = !self.first_person;
            }
        }
    }
    pub(crate) fn mission_snapshot(&self) -> ([f32; 3], f32, u8, f32, u64) {
        (
            self.motion.position,
            self.jet.fuel,
            self.active_clip,
            self.clock.seconds,
            self.mesh_responses,
        )
    }
    pub(crate) fn mission_physical_snapshot(&self) -> (u32, u32, bool, [f32; 3]) {
        (
            self.physics_flags,
            self.support,
            self.grounded,
            self.motion.velocity,
        )
    }
    pub(crate) fn mission_fire_step(
        &mut self,
        dt: f32,
        held: bool,
        terrain: &giants_formats::gti::Terrain,
    ) -> Result<giants_game::mecc_pistol::Frame, String> {
        if self.owned_pose.is_some() {
            return Err("owned Mecc firing cannot use the development pistol simulation".into());
        }
        let (origin, aim) = self.combat_pose().map_err(|e| e.to_string())?;
        let held = held && self.alive();
        self.mission_pistol
            .as_mut()
            .ok_or("selected mission weapon is not prepared")?
            .step(dt, origin, aim, held, terrain, &self.projectile_objects)
            .map_err(str::to_owned)
    }
    pub(crate) fn impact_sound_ids(&self) -> Vec<u32> {
        self.impact_audio
            .as_ref()
            .map_or(Vec::new(), |p| p.sound_ids().into_iter().collect())
    }
    pub(crate) fn impact_sound(
        &mut self,
        code: u32,
        point: [f32; 3],
    ) -> Option<giants_game::mecc_impacts::Cue> {
        self.impact_audio
            .as_mut()
            .and_then(|p| match p.request(code, point) {
                Ok(cue) => cue,
                Err(error) => {
                    warn!("mission_impact_audio_refused error={error}");
                    None
                }
            })
    }
    pub(crate) fn mission_audio_steps(&mut self) -> Vec<giants_game::mecc_footsteps::Cue> {
        std::mem::take(&mut self.foot_cues)
    }
    pub(crate) fn mission_audio_jet(&self) -> (bool, bool, i32) {
        (
            self.thrusting,
            self.audio_boost_started,
            self.inventory.special,
        )
    }
    pub(crate) fn mission_hud(&self) -> (f32, f32, f32) {
        (self.remaining_health(), self.jet.fuel, self.jet.boost)
    }
    pub(crate) fn take_landing_impact(&mut self) -> Option<[f32; 3]> {
        self.landing_impact.take()
    }
    pub(crate) fn reset_pose(&mut self) {
        if let Some(pistol) = &mut self.mission_pistol {
            pistol.reset();
        }
        self.health = giants_game::mecc_damage::State::cold();
        self.landing_impact = None;
        self.deaths = 0;
        self.motion = self.start;
        self.gravity_mass = self.initial_gravity_mass;
        self.yaw = self.initial_yaw;
        self.pitch = 0.;
        self.active_clip = 1;
        self.clock = Default::default();
        self.last_pose = None;
        self.grounded = true;
        self.first_person = self.initial_first_person;
        self.subjective_weapon_matrix = None;
        self.contact_flags = 0;
        self.model_vertical_speed = 0.;
        self.model_motion = giants_game::mecc_model_motion::Output {
            world_speed: 0.,
            model: [0.; 3],
        };
        self.physics_flags = 0;
        self.support = 0;
        self.support_part = 0;
        self.support_triangle = 0;
        self.foot_cues.clear();
        self.ground_normal = self.initial_normal;
        self.ground_plane = self.initial_plane;
        self.floor_height = self.initial_floor;
        self.jet = Default::default();
        self.jet_flags = 0;
        self.pending_boost = false;
        self.thrusting = false;
    }
    pub(crate) fn load(
        root: &Path,
        world: &giants_game::WorldAsset,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Explicit development selection; no owner profile/progress is modified.
        let catalog = std::fs::read(root.join("Bin").join(&world.catalog))?;
        let menu = giants_game::mission_selection::MissionMenu::parse(&catalog, u32::MAX, true)?;
        let packet = giants_game::mission_loading::prepare(root, menu.request(&world.name, 0)?)?;
        Self::load_prepared(root, world, &packet)
    }
    pub(crate) fn load_prepared(
        root: &Path,
        world: &giants_game::WorldAsset,
        packet: &giants_game::mission_loading::PreparedMission,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::load_profile(root, world, packet, None)
    }
    /// Shared cold controller using the selected mission's established equipment.
    /// Continuity/base loadouts require their own construction rules.
    pub(crate) fn load_mission(
        root: &Path,
        world: &giants_game::WorldAsset,
        packet: &giants_game::mission_loading::PreparedMission,
        initial: &giants_game::player_initial::Prepared,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        if initial.start.kind != 2
            || initial.equipment != giants_game::player_equipment::Initial::non_base(true)
        {
            return Err(
                "mission controller requires measured cold controlled Mecc equipment".into(),
            );
        }
        Self::load_profile(root, world, packet, Some(initial.equipment))
    }
    fn load_profile(
        root: &Path,
        world: &giants_game::WorldAsset,
        packet: &giants_game::mission_loading::PreparedMission,
        equipment: Option<giants_game::player_equipment::Initial>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let replay = if equipment.is_none() {
            std::env::var("GIANTS_MECC_DEV_REPLAY").ok()
        } else {
            None
        };
        if !world.name.eq_ignore_ascii_case(&packet.request.world)
            || !world
                .catalog
                .eq_ignore_ascii_case(&format!("w_{}.bin", packet.request.world))
        {
            return Err("development environment does not match selected packet".into());
        }
        let plan = packet.instance_plan(root, "English")?;
        let resources = std::sync::Arc::clone(plan.resources());
        let mission = packet.mission.as_ref().ok_or("prepared mission missing")?;
        let anchor = giants_game::player_start::declared(
            &plan,
            packet.world.as_ref().ok_or("prepared world missing")?,
            mission,
        )?;
        if anchor.angles[1..].iter().any(|angle| *angle != 0.) {
            return Err("development controller start pitch/roll outside admitted profile".into());
        }
        let collision =
            giants_game::collision_radius::ColdProfile::load(&resources, 2, Precision::Double53)?;
        let body = giants_game::cast::load_registered_body(&resources, 2, 0)?;
        let weapons = giants_formats::weapons::parse_with_profile(
            &std::fs::read(root.join("Bin/weapondefs.bin"))?,
            giants_formats::weapons::Profile::Debug,
        )?;
        let mission_pistol = equipment
            .map(|e| giants_game::mecc_pistol::Pistol::from_initial(e, &weapons))
            .transpose()?;
        let impact_audio = mission_pistol.as_ref().and_then(|p| {
            match giants_game::mecc_impacts::Prepared::load(&resources, &p.definition) {
                Ok(prepared) => Some(prepared),
                Err(error) => {
                    println!("mission_impact_audio_prepared=false error={error}");
                    None
                }
            }
        });
        let selected_icon = equipment.map_or(1, |e| e.weapons[e.selected as usize]);
        let weapon = giants_game::held_weapon::Weapon::load(&resources, &weapons, selected_icon)?;
        if body.mesh.normal_count != 0 {
            return Err("Mecc development body requires checked unlit profile".into());
        }
        let binding = resources
            .classes
            .models
            .first_selection(2)
            .ok_or("Mecc spec missing")?
            .as_ref()
            .map_err(Clone::clone)?;
        let standing = binding
            .selection
            .animations
            .iter()
            .find(|a| a.slot == 1)
            .ok_or("standing slot missing")?;
        let animation = resources.animation(&standing.name)?;
        let tree = animation
            .trees
            .iter()
            .find(|t| t.offset == animation.default_tree)
            .ok_or("standing tree missing")?;
        let positions = giants_game::skinning::positions(
            &body.skin,
            &animation,
            &tree.name,
            animation.timeline[0],
        )?;
        let mut clips = BTreeMap::new();
        for slot in [
            1, 2, 5, 6, 7, 23, 39, 40, 41, 50, 136, 143, 144, 169, 172, 202,
        ] {
            let selected = binding
                .selection
                .animations
                .iter()
                .filter(|a| a.slot == slot)
                .collect::<Vec<_>>();
            if selected.len() != 1 {
                return Err("missing or ambiguous Mecc development clip".into());
            }
            let animation = resources.animation(&selected[0].name)?;
            let tree = animation
                .trees
                .iter()
                .find(|t| t.offset == animation.default_tree)
                .ok_or("Mecc clip default tree missing")?
                .name
                .clone();
            let (duration, _) = giants_game::model_pose::intact_time(animation.timeline)?;
            let initial_positions = giants_game::skinning::positions(
                &body.skin,
                &animation,
                &tree,
                animation.timeline[0],
            )?;
            // Decode interior poses before exposing a partially supported clip.
            for seconds in [duration * 0.25, duration * 0.5, duration * 0.75] {
                let tick = giants_game::model_pose::seconds_tick(animation.timeline, seconds)?;
                giants_game::skinning::positions(&body.skin, &animation, &tree, tick)?;
            }
            println!(
                "mecc_dev_clip slot={slot} name={} duration={duration} timeline={:?} selection=prepared_normal_and_death normal_selector=style_zero_checked",
                selected[0].name, animation.timeline
            );
            clips.insert(
                slot,
                Clip {
                    name: selected[0].name.clone(),
                    animation,
                    tree,
                    duration,
                    initial_positions,
                    changed: false,
                },
            );
        }
        let program = resources
            .classes
            .models
            .first_program(2)
            .ok_or("Mecc class missing")?
            .program
            .as_ref()
            .map_err(Clone::clone)?;
        let scale = program.declared_scale()?;
        let collision_dimensions = program
            .declared_collision_dimensions()?
            .ok_or("Mecc collision dimensions require bound-model preparation")?;
        let ground_offset_source = program.declared_ground_offset()?;
        let ground_offset = ground_offset_source * scale;
        let requested_view = if equipment.is_none() {
            std::env::var("GIANTS_MECC_DEV_VIEW").ok()
        } else {
            None
        };
        let first_person = match requested_view.as_deref() {
            None | Some("third") => false,
            Some("first") => true,
            Some(_) => return Err("unknown Mecc development view".into()),
        };
        let globals =
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn"))?)?;
        let parameters = giants_game::mecc_initial::Parameters::from_globals(&globals)?;
        let locomotion = giants_game::mecc_locomotion::Parameters::from_globals(&globals)?;
        let descent = giants_game::mecc_locomotion::Descent::from_globals(&globals)?;
        let impact_parameters = giants_game::mecc_impact::Parameters::from_globals(&globals)?;
        let contact_speed_thresholds = [globals.scalar(0x664)?, globals.scalar(0x668)?];
        let model_speed_minimums = [globals.scalar(0x65c)?, globals.scalar(0x660)?];
        let jet_parameters = giants_game::mecc_jet::Parameters::from_globals(&globals)?;
        let utility = if let Some(initial) = equipment {
            initial.utility
        } else {
            match std::env::var("GIANTS_MECC_DEV_UTILITY") {
                Ok(value) => value
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<Result<Vec<_>, _>>()?
                    .try_into()
                    .map_err(|_| "GIANTS_MECC_DEV_UTILITY requires three IDs")?,
                Err(std::env::VarError::NotPresent) => [-1; 3],
                Err(error) => return Err(error.into()),
            }
        };
        let inventory = giants_game::mecc_inventory::Inventory {
            utility,
            // The admitted cold state has no fourth weapon. Do not truncate a
            // populated slot when continuity/other loadouts are implemented.
            weapons: equipment.map_or([1, 7, -1], |e| [e.weapons[0], e.weapons[1], e.weapons[2]]),
            special: equipment.map_or(-1, |e| e.special),
        };
        let weights = inventory.weights(&weapons)?;
        let special_mass = globals.scalar(0xacc)?;
        let load = giants_game::mecc_load_mass::calculate(
            giants_game::mecc_load_mass::Input {
                base_mass: parameters.mass,
                base_gravity_mass: parameters.gravity_mass,
                special_mass,
                item_weights: weights.slots,
                special_weight: weights.special,
                response_flags: 0,
                object_flags: 0,
                has_physics: true,
                attached: false,
                previous: [parameters.mass, parameters.gravity_mass],
            },
            Precision::Double53,
        )?;
        if load[0] <= 0. || !load.into_iter().all(f32::is_finite) {
            return Err("invalid development inventory mass".into());
        }
        println!(
            "mecc_dev_inventory utility={:?} weapons={:?} mass={} gravity_mass={} profile=debug_declared_weights campaign_inventory={} carry_visuals=false",
            inventory.utility,
            inventory.weapons,
            load[0],
            load[1],
            equipment.is_some()
        );

        let colliders = giants_game::declared_mesh_scene::prepare(&plan, &world.terrain);
        println!(
            "mecc_dev_colliders objects={} triangles={} needs={} profile=declared_rigid native_list_order=false live_callbacks=false platform_support=false",
            colliders.objects.len(),
            colliders
                .objects
                .iter()
                .map(|o| o.triangles.len())
                .sum::<usize>(),
            colliders.needs.len(),
        );
        for need in &colliders.needs {
            println!(
                "mecc_dev_collider_need key={:?} reason={}",
                need.key, need.reason
            );
        }
        let wanted = colliders
            .records
            .iter()
            .map(|record| {
                if record.key.0 {
                    (
                        true,
                        plan.world_entries[record.key.1].declaration.source_offset,
                    )
                } else {
                    (
                        false,
                        plan.entries
                            .iter()
                            .find(|p| p.index == record.key.1)
                            .expect("prepared mission key")
                            .serialized
                            .source_offset,
                    )
                }
            })
            .collect::<std::collections::BTreeSet<_>>();
        let covered = plan
            .entries
            .iter()
            .map(|p| (false, p.serialized.source_offset))
            .chain(
                plan.world_entries
                    .iter()
                    .map(|p| (true, p.declaration.source_offset)),
            )
            .filter(|key| !wanted.contains(key))
            .collect();
        let rigid_source = std::sync::Arc::new(
            giants_game::static_geometry::Scene::prepare_normal_initial(&plan, &covered),
        );
        let cube = rigid_source
            .assets
            .values()
            .any(|a| a.reflection_descriptors.iter().any(Option::is_some));
        println!(
            "mecc_dev_rigid_visuals instances={} assets={} expected={} cube_required={} initial_pose=debug_normal callbacks=false campaign_ready=false",
            rigid_source.instances.len(),
            rigid_source.assets.len(),
            wanted.len(),
            cube
        );
        for d in rigid_source
            .diagnostics
            .iter()
            .filter(|d| wanted.contains(&d.key))
        {
            println!(
                "mecc_dev_rigid_visual key={:?} class={} result={}",
                d.key, d.class, d.result
            );
        }
        let query = giants_game::terrain_queries::StaticTerrainQuery::new(
            &world.terrain,
            1. / world.terrain.spacing,
            Precision::Double53,
        )?;
        let point = anchor.position;
        let ground = query.sample([point[0], point[1]])?;
        if !ground.in_bounds {
            return Err("development anchor outside terrain".into());
        }
        let start = Motion {
            position: point,
            velocity: [0.; 3],
            previous_velocity: [0.; 3],
            force: [0.; 3],
            mass: load[0],
            speed_limit: parameters.speed_limit,
            force_time_limit: 0.,
            stored_dt: 0.,
        };
        println!(
            "mecc_dev_loaded class=2 model={} parts={} animation={} scale={} anchor_class={} anchor_offset={} position={:?} nominal_mass={} input_pipeline=preview start_selection=initial_marker continuity=false campaign_ready=false",
            body.model,
            body.mesh.subobjects.len(),
            standing.name,
            scale,
            679,
            anchor.marker_offset,
            point,
            start.mass
        );
        println!(
            "mecc_dev_initialized speed={} gain={} slope_power={} friction={} damping_xy={} damping_z={} parameters=cold_mecc force_order=integrate_contact_then_prepare",
            parameters.control_speed,
            parameters.control_gain,
            parameters.slope_power,
            parameters.friction_limit,
            parameters.damping_xy,
            parameters.damping_z
        );
        let eligible_projectiles = colliders
            .objects
            .iter()
            .filter(|o| {
                giants_game::object_segment::projectile_candidate(
                    u32::MAX,
                    1,
                    o.fields.id,
                    o.fields.parent,
                    o.fields.object_flags,
                    o.fields.collision_flags,
                )
            })
            .count();
        println!(
            "mecc_projectile_world objects={} eligible={} list=declared_supplied poses=initial callbacks=false",
            colliders.objects.len(),
            eligible_projectiles
        );
        let footsteps = match giants_game::mecc_footsteps::Prepared::load(&resources) {
            Ok(prepared) => {
                println!(
                    "mecc_footsteps_prepared sounds={} route=local_player_root visual_effects=false rng_phase=preview_seed_one",
                    prepared.sound_ids().len()
                );
                Some(prepared)
            }
            Err(error) => {
                println!("mecc_footsteps_unavailable reason={error:?}");
                None
            }
        };
        let mut foot_materials = BTreeMap::new();
        for (record, object) in colliders.records.iter().zip(&colliders.objects) {
            let selection = resources.collision_model_mesh(record.class)?;
            let (_, mesh) = selection
                .geometry
                .ok_or("footstep contact geometry missing")?;
            for (part, subobject) in mesh.subobjects.iter().enumerate() {
                foot_materials.insert(
                    (object.fields.id, part as u32),
                    subobject.material.texture.clone(),
                );
            }
        }
        // Share the same declared rigid scene with the first-living component.
        // Source class/state and candidate identities are retained; platform/
        // radial and native live-list eligibility remain outside this profile.
        let environment_objects = colliders
            .records
            .iter()
            .zip(&colliders.objects)
            .map(
                |(record, object)| giants_game::actor_environment::Collider {
                    class: record.class as u32,
                    state: record.instance.state,
                    volume: None,
                    mesh: giants_game::mecc_mesh_dispatch::MeshObject {
                        fields: object.fields,
                        triangles: object.triangles.clone(),
                    },
                },
            )
            .collect();
        let static_mesh_objects = colliders.player_objects.len();
        let next_contact_key = colliders
            .player_objects
            .iter()
            .map(|o| o.fields.id)
            .max()
            .unwrap_or(1)
            .checked_add(1)
            .ok_or("collision identity budget")?;
        let mut state = Self {
            collision,
            collision_dimensions,
            projectile_objects: colliders
                .objects
                .iter()
                .map(|o| {
                    Ok(giants_game::projectile_world::Object {
                        id: o.fields.id,
                        parent: o.fields.parent,
                        active: o.fields.object_flags,
                        personality: o.fields.collision_flags,
                        bounds: o.fields.bounds,
                        mesh: giants_game::object_segment::Mesh::new(
                            o.fields.id,
                            o.triangles.clone(),
                        )?,
                    })
                })
                .collect::<Result<Vec<_>, &'static str>>()?,
            static_projectile_objects: colliders.objects.len(),
            projectile_test: None,
            mesh_objects: colliders.player_objects,
            static_mesh_objects,
            npc_contact_keys: BTreeMap::new(),
            next_contact_key,
            npc_contacts: Vec::new(),
            npc_mesh_responses: 0,
            environment_objects,
            mesh_responses: 0,
            rigid_source,
            collision_walk: None,
            health: giants_game::mecc_damage::State::cold(),
            landing_impact: None,
            impact_parameters,
            contact_speed_thresholds,
            deaths: 0,
            floor_height: ground.height,
            weapon,
            body,
            positions,
            owned_pose: None,
            scale,
            start,
            motion: start,
            yaw: anchor.angles[0].to_radians(),
            initial_yaw: anchor.angles[0].to_radians(),
            pitch: if matches!(
                replay.as_deref(),
                Some("fire" | "fire-pose" | "impact-pose")
            ) {
                0.2
            } else {
                0.
            },
            parameters,
            item_catalog: weapons,
            inventory,
            load_response_flags: 0,
            mission_pistol,
            gravity_mass: load[1],
            initial_gravity_mass: load[1],
            special_mass,
            jet_parameters,
            jet: Default::default(),
            jet_flags: 0,
            pending_boost: false,
            thrusting: false,
            audio_boost_started: false,
            jet_replay: matches!(replay.as_deref(), Some("jet" | "jet-pose")),
            jet_checks: [false; 5],
            jet_depletions: 0,
            physics_flags: 0,
            support: 0,
            support_part: 0,
            support_triangle: 0,
            footsteps,
            foot_cues: Vec::new(),
            impact_audio,
            foot_materials,
            ground_normal: ground.plane[..3].try_into().unwrap(),
            initial_normal: ground.plane[..3].try_into().unwrap(),
            ground_plane: ground.plane,
            initial_plane: ground.plane,
            initial_floor: ground.height,
            idle_passed: false,
            ground_offset,
            ground_offset_source,
            contact_flags: 0,
            model_vertical_speed: 0.,
            model_motion: giants_game::mecc_model_motion::Output {
                world_speed: 0.,
                model: [0.; 3],
            },
            model_speed_minimums,
            locomotion,
            descent,
            plane_contacts: 0,
            terrain_edge_transitions: 0,
            ground_queries: 0,
            ground_junctions: 0,
            ground_target_steps: 0,
            ground_target_max_normal_speed: 0.,
            remainder: 0.,
            seconds: 0.,
            last_log: -1,
            draws: Vec::new(),
            replay_tick: match replay.as_deref() {
                None => None,
                Some(
                    "player-hit"
                    | "player-hit-pose"
                    | "player-dead-pose"
                    | "session-menu"
                    | "session-cycles"
                    | "session-cycles-play",
                ) => None,
                Some(
                    "collision-walk" | "collision-free" | "collision-pose" | "smoke" | "stand-pose"
                    | "walk-pose" | "air-pose" | "jet" | "jet-pose" | "fire" | "fire-pose"
                    | "fire-impact-pose" | "damage" | "damage-pose" | "damage-dead-pose"
                    | "world-fire" | "world-fire-pose" | "world-fire-free",
                ) => Some(0),
                Some(_) => return Err("unknown Mecc development replay".into()),
            },
            replay_checks: [false; 3],
            pose_capture_reported: false,
            combat_replay: matches!(
                replay.as_deref(),
                Some(
                    "fire"
                        | "fire-pose"
                        | "fire-impact-pose"
                        | "damage"
                        | "damage-pose"
                        | "damage-dead-pose"
                        | "world-fire"
                        | "world-fire-pose"
                        | "world-fire-free"
                )
            ),
            first_person,
            initial_first_person: first_person,
            subjective_weapon_matrix: None,
            last_view: None,
            clips,
            active_clip: 1,
            clock: Default::default(),
            last_pose: None,
            grounded: true,
            replay_pose_tick: match replay.as_deref() {
                Some("stand-pose") => Some(60),
                Some("walk-pose") => Some(120),
                Some("air-pose") => Some(270),
                Some("jet-pose") => Some(120),
                Some(
                    "fire" | "fire-pose" | "fire-impact-pose" | "damage" | "damage-pose"
                    | "damage-dead-pose" | "world-fire" | "world-fire-pose" | "world-fire-free",
                ) => Some(60),
                _ => None,
            },
        };
        if matches!(
            replay.as_deref(),
            Some("collision-walk" | "collision-free" | "collision-pose")
        ) {
            state.begin_collision_walk(&world.terrain)?;
        }
        if replay.as_ref().is_some_and(|s| s.starts_with("world-fire")) {
            state.begin_world_shot(&world.terrain)?;
        }
        Ok(state)
    }
    fn animate(&mut self, dt: f32) -> Result<(), &'static str> {
        if !self.alive() {
            let selected = giants_game::mecc_death::select(giants_game::mecc_death::Input {
                flags: if self.grounded { 0x80000 } else { 0 },
                current: u32::from(self.active_clip),
                forced: -1,
                z: self.motion.position[2],
                vertical_speed: self.motion.velocity[2],
                ground_offset: self.ground_offset_source,
                scale: self.scale,
                floor: self.floor_height,
            })?;
            if let Some(selected) = selected {
                let slot =
                    u8::try_from(selected).map_err(|_| "death slot outside source byte table")?;
                if !self.clips.contains_key(&slot) {
                    return Err("unprepared native death clip");
                }
                if slot != self.active_clip {
                    println!(
                        "mecc_dev_death_clip slot={slot} contact_flags=development one_shot_hold=development"
                    );
                    self.active_clip = slot;
                    self.clock = Default::default();
                    self.last_pose = None;
                }
            }
            self.clock.previous = self.clock.seconds;
            self.clock.seconds = (f64::from(self.clock.seconds) + f64::from(dt))
                .min(f64::from(self.clips[&self.active_clip].duration))
                as f32;
            return Ok(());
        }
        let input = giants_game::mecc_locomotion::Input {
            model: self.model_motion.model,
            position_z: self.motion.position[2],
            floor: self.floor_height,
            center_offset: self.collision_dimensions[0],
            scale: self.scale,
            supported: self.support != 0,
        };
        let ordinary =
            giants_game::mecc_locomotion::select(input, self.locomotion, Precision::Double53)?;
        let request = self
            .descent
            .request(input, self.motion.velocity[2])?
            .unwrap_or(ordinary);
        let selected = request.clip;
        let rate = request.rate(self.model_motion.model, Precision::Double53)?;
        if selected != self.active_clip {
            println!(
                "mecc_dev_animation_transition from={} to={selected} native_selector=normal_style_zero rate={rate} source={} reference={} special_states_pending=true",
                self.active_clip, request.speed_source, request.reference_speed
            );
            self.active_clip = selected;
            self.clock = Default::default();
            if rate < 0. {
                self.clock.seconds = self.clips[&selected].duration;
            }
            self.last_pose = None;
        }
        self.clock.advance_with_rate(
            self.clips[&selected].duration,
            dt,
            rate,
            Precision::Double53,
        )?;
        if let Some(footsteps) = &mut self.footsteps {
            let window = giants_game::animation_event_dispatch::Window::classify(
                self.clock.previous,
                self.clock.seconds,
                self.clips[&selected].duration,
            )
            .map_err(|_| "invalid footstep animation interval")?;
            let surface = self.foot_materials.get(&(self.support, self.support_part));
            let cues = if self.support != 0 && surface.is_none() {
                Err(giants_formats::Error(
                    "footstep support material is not prepared",
                ))
            } else {
                footsteps.step(
                    selected,
                    window,
                    self.clips[&selected].duration,
                    self.contact_flags & 0x100000 != 0,
                    surface.map(String::as_str),
                )
            };
            match cues {
                Ok(cues) if self.foot_cues.len() + cues.len() <= 256 => self.foot_cues.extend(cues),
                other => {
                    println!(
                        "mecc_footsteps_unavailable runtime=true reason={:?}",
                        other
                            .err()
                            .map(|e| e.to_string())
                            .unwrap_or_else(|| "pending cue budget".into())
                    );
                    self.footsteps = None;
                    self.foot_cues.clear();
                }
            }
        }
        Ok(())
    }

    pub(crate) fn sample_pose(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.owned_pose.is_some() {
            return Ok(());
        }
        let clip = self
            .clips
            .get_mut(&self.active_clip)
            .expect("prepared clip");
        let tick =
            giants_game::model_pose::seconds_tick(clip.animation.timeline, self.clock.seconds)?;
        if self.last_pose == Some((self.active_clip, tick)) {
            return Ok(());
        }
        self.positions =
            giants_game::skinning::positions(&self.body.skin, &clip.animation, &clip.tree, tick)?;
        let changed = self
            .positions
            .iter()
            .zip(&clip.initial_positions)
            .any(|(a, b)| a.iter().zip(b).any(|(a, b)| (a - b).abs() > 0.00001));
        if changed && !clip.changed {
            println!(
                "mecc_dev_pose_changed slot={} name={} tick={tick} native_selector=false",
                self.active_clip, clip.name
            );
            clip.changed = true;
        }
        self.last_pose = Some((self.active_clip, tick));
        Ok(())
    }
    pub(crate) fn camera(&self) -> (crate::validation_camera::ValidationCamera, [f32; 3]) {
        let pose = if let Some(frame) = &self.owned_pose {
            frame.camera
        } else {
            giants_game::mecc_camera::pose(
                self.motion.position,
                self.yaw.to_degrees(),
                -self.pitch.to_degrees(),
                self.first_person,
            )
            .expect("bounded Mecc camera inputs")
        };
        let eye = v3(pose.eye);
        let cache = self
            .owned_pose
            .as_ref()
            .and_then(|f| f.camera_cache.as_ref());
        let view = cache.map_or_else(
            || camera_view(eye, v3(pose.forward), v3(pose.up)),
            |c| Mat4::from_cols_array(&c.view),
        );
        (
            crate::validation_camera::ValidationCamera {
                view: view.to_cols_array(),
                projection: giants_game::mecc_camera::projection(1920. / 1080., 25000.)
                    .expect("normal Mecc lens"),
                viewport: UVec4::new(0, 0, 1920, 1080),
            },
            cache.map_or(eye.to_array(), |c| c.viewport_origin),
        )
    }
    pub(crate) fn subjective_inputs(&self) -> (giants_game::first_person::Camera, f32, [f32; 3]) {
        if let Some(cache) = self
            .owned_pose
            .as_ref()
            .and_then(|f| f.camera_cache.as_ref())
        {
            return (cache.camera, self.scale, self.model_motion.model);
        }
        (
            giants_game::first_person::Camera {
                position: self.camera().1,
                angles: self.owned_pose.as_ref().map_or(
                    [self.yaw.to_degrees(), -self.pitch.to_degrees(), 0.],
                    |frame| [frame.angles[0], frame.look_pitch, 0.],
                ),
            },
            self.scale,
            self.model_motion.model,
        )
    }
    pub(crate) fn set_subjective_weapon(&mut self, matrix: Option<[f32; 16]>) {
        self.subjective_weapon_matrix = matrix;
    }
    /// A draw animation can genuinely have no held object. Preserve stale-pose
    /// errors while letting the render consumer hide its existing submission.
    pub(crate) fn render_weapon_matrix(
        &self,
    ) -> Result<Option<[f32; 16]>, Box<dyn std::error::Error>> {
        if self
            .owned_pose
            .as_ref()
            .is_some_and(|frame| frame.world_weapon.is_none())
        {
            return Ok(None);
        }
        self.weapon_matrix().map(Some)
    }
    pub(crate) fn weapon_matrix(&self) -> Result<[f32; 16], Box<dyn std::error::Error>> {
        if let Some(frame) = &self.owned_pose {
            let weapon = frame
                .world_weapon
                .as_ref()
                .filter(|weapon| weapon.number == frame.number && weapon.epoch == frame.stamp)
                .ok_or("owned world weapon pose absent or stale")?;
            return Ok(weapon.pose.world.values);
        }
        let clip = &self.clips[&self.active_clip];
        let tick =
            giants_game::model_pose::seconds_tick(clip.animation.timeline, self.clock.seconds)?;
        self.weapon.matrix(
            &clip.animation,
            &clip.tree,
            tick,
            self.object_matrix(),
            self.scale,
        )
    }
    pub(crate) fn world_weapon_positions(&self) -> Option<&[[f32; 3]]> {
        self.owned_pose
            .as_ref()?
            .world_weapon
            .as_ref()
            .map(|weapon| weapon.positions.as_slice())
    }
    pub(crate) fn carried_matrix(
        &self,
        actor: &giants_game::carried_actor::Prepared,
        child_seconds: f32,
    ) -> Result<[f32; 16], Box<dyn std::error::Error>> {
        if self.owned_pose.is_some() {
            let tick =
                giants_game::model_pose::seconds_tick(actor.animation.timeline, child_seconds)?;
            let node = actor.animation.default_node(&actor.child_reference)?.offset;
            return self.carried_matrix_from_reference(
                actor,
                giants_game::animation_references::node_matrix(&actor.animation, node, tick)?,
            );
        }
        let clip = &self.clips[&self.active_clip];
        let tick =
            giants_game::model_pose::seconds_tick(clip.animation.timeline, self.clock.seconds)?;
        actor.world_matrix(
            &clip.animation,
            &clip.tree,
            tick,
            self.object_matrix(),
            self.scale,
            child_seconds,
        )
    }
    pub fn combat_pose(&self) -> Result<([f32; 3], [f32; 3]), Box<dyn std::error::Error>> {
        if let Some(frame) = &self.owned_pose {
            if self.first_person {
                return Err(
                    "owned subjective firing requires its own current muzzle producer".into(),
                );
            }
            let weapon = frame
                .world_weapon
                .as_ref()
                .filter(|weapon| weapon.number == frame.number && weapon.epoch == frame.stamp)
                .ok_or("owned aim weapon cache absent or stale")?;
            let aim = frame.world_aim.as_ref().ok_or(
                "owned aim requires its world-view producer;subjective firing remains unbound",
            )?;
            return Ok((weapon.muzzle, aim.point));
        }
        let (camera, eye) = self.camera();
        let direction = Mat4::from_cols_array(&camera.view)
            .inverse()
            .z_axis
            .truncate()
            .normalize();
        let muzzle = if self.alive() {
            let matrix = if self.first_person {
                match self.subjective_weapon_matrix {
                    Some(matrix) => matrix,
                    None => self.weapon_matrix()?,
                }
            } else {
                self.weapon_matrix()?
            };
            self.weapon.muzzle(matrix)?
        } else {
            self.motion.position
        };
        Ok((muzzle, (v3(eye) + direction * 500.).to_array()))
    }
    pub(crate) fn carried_matrix_from_reference(
        &self,
        actor: &giants_game::carried_actor::Prepared,
        child: giants_game::procedural_aim::Matrix,
    ) -> Result<[f32; 16], Box<dyn std::error::Error>> {
        if let Some(frame) = &self.owned_pose {
            let parent = frame
                .references
                .get(giants_game::smartie_carry::MECC_PARENT_REFERENCE as usize)
                .ok_or("owned carrying reference absent")?;
            return actor.world_matrix_from_parent_reference(
                parent.values,
                frame.affine,
                frame.scale,
                child,
            );
        }
        let clip = &self.clips[&self.active_clip];
        let tick =
            giants_game::model_pose::seconds_tick(clip.animation.timeline, self.clock.seconds)?;
        actor.world_matrix_with_child_reference(
            &clip.animation,
            &clip.tree,
            tick,
            self.object_matrix(),
            self.scale,
            child,
        )
    }
    fn object_matrix(&self) -> [f32; 16] {
        if let Some(frame) = &self.owned_pose {
            return frame.affine;
        }
        giants_game::object_transform::euler(
            self.motion.position,
            [self.yaw.to_degrees(), 0., 0.],
            self.scale,
        )
        .expect("finite controlled Mecc pose")
    }
    pub(crate) fn expanded(&self, part: usize) -> Vec<[f32; 3]> {
        let matrix = self.object_matrix();
        self.body.mesh.subobjects[part]
            .triangles
            .iter()
            .flatten()
            .map(|id| {
                let i = usize::from(self.body.mesh.indices[usize::from(*id)]);
                let p = self.positions[i];
                let s = std::array::from_fn::<_, 3, _>(|r| {
                    matrix[r * 4] * p[0]
                        + matrix[r * 4 + 1] * p[1]
                        + matrix[r * 4 + 2] * p[2]
                        + matrix[r * 4 + 3]
                });
                [s[0], s[2], -s[1]]
            })
            .collect()
    }
    pub(crate) fn body_draw_slot(&self) -> Option<usize> {
        self.owned_pose
            .as_ref()
            .and_then(|f| f.draw_model.as_ref())
            .map_or(Some(0), |d| d.positions().map(|_| d.slot()))
    }
    pub(crate) fn body_draw_part_count(&self) -> usize {
        self.owned_pose
            .as_ref()
            .and_then(|f| f.draw_model.as_ref())
            .map_or(self.body.mesh.subobjects.len(), |d| {
                d.model().map_or(0, |m| m.prepared.mesh.subobjects.len())
            })
    }
    pub(crate) fn expanded_draw_model(&self, slot: usize, part: usize) -> Option<Vec<[f32; 3]>> {
        let Some(draw) = self.owned_pose.as_ref().and_then(|f| f.draw_model.as_ref()) else {
            return (slot == 0).then(|| self.expanded(part));
        };
        if slot != draw.slot() {
            return None;
        }
        let mesh = &draw.model()?.prepared.mesh;
        let positions = draw.positions()?;
        let matrix = self.object_matrix();
        Some(
            mesh.subobjects
                .get(part)?
                .triangles
                .iter()
                .flatten()
                .map(|id| {
                    let p = positions[usize::from(mesh.indices[usize::from(*id)])];
                    let s = std::array::from_fn::<_, 3, _>(|r| {
                        matrix[r * 4] * p[0]
                            + matrix[r * 4 + 1] * p[1]
                            + matrix[r * 4 + 2] * p[2]
                            + matrix[r * 4 + 3]
                    });
                    [s[0], s[2], -s[1]]
                })
                .collect(),
        )
    }
    pub(crate) fn standing_positions(&self) -> &[[f32; 3]] {
        &self.clips[&1].initial_positions
    }
    pub(crate) fn alive(&self) -> bool {
        self.health.alive()
    }
    pub(crate) fn first_person_view(&self) -> bool {
        self.first_person
    }
    pub(crate) fn remaining_health(&self) -> f32 {
        self.health.remaining()
    }
    pub(crate) fn receive_hit(&mut self, amount: f32) -> Result<(), &'static str> {
        let hit = self.health.apply(giants_game::mecc_damage::Hit {
            kind: 41,
            amount,
            globally_disabled: false,
            cutscene: false,
            allowed: true,
            shield_percent: 0.,
        })?;
        self.health = hit.state;
        if hit.died {
            self.deaths += 1;
            self.first_person = false;
            self.subjective_weapon_matrix = None;
            self.thrusting = false;
            self.jet_flags = 0;
            self.motion.force = [0.; 3];
        }
        println!(
            "mecc_dev_player_hit amount={amount} remaining={} died={} deaths={} attacker=scripted_development",
            self.health.remaining(),
            hit.died,
            self.deaths
        );
        Ok(())
    }
    pub(crate) fn projectile_geometry(
        &self,
    ) -> Result<giants_game::object_segment::Mesh, &'static str> {
        let triangles = (0..self.body.mesh.subobjects.len())
            .flat_map(|index| {
                let positions = self.expanded(index);
                positions
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|face| std::array::from_fn(|i| [face[i][0], -face[i][2], face[i][1]]))
                    .collect::<Vec<_>>()
            })
            .collect();
        giants_game::object_segment::Mesh::new(2, triangles)
    }
    pub(crate) fn set_carried_load(
        &mut self,
        carried: bool,
    ) -> Result<Option<[f32; 2]>, &'static str> {
        let previous = self.load_response_flags;
        let next = if carried {
            previous | 0x80_0000
        } else {
            previous & !0x80_0000
        };
        if previous == next {
            return Ok(None);
        }
        self.load_response_flags = next;
        if let Err(error) = self.refresh_inventory_mass() {
            self.load_response_flags = previous;
            return Err(error);
        }
        Ok(Some([self.motion.mass, self.gravity_mass]))
    }
    pub(crate) fn carried_load(&self) -> (bool, [f32; 2]) {
        (
            self.load_response_flags & 0x80_0000 != 0,
            [self.motion.mass, self.gravity_mass],
        )
    }
    fn refresh_inventory_mass(&mut self) -> Result<(), &'static str> {
        let weights = self.inventory.weights(&self.item_catalog)?;
        let load = giants_game::mecc_load_mass::calculate(
            giants_game::mecc_load_mass::Input {
                base_mass: self.parameters.mass,
                base_gravity_mass: self.parameters.gravity_mass,
                special_mass: self.special_mass,
                item_weights: weights.slots,
                special_weight: weights.special,
                // Owned pickup response state supplies the measured owner bit;
                // live actor callbacks and other object flags stay upstream.
                response_flags: self.load_response_flags,
                object_flags: 0,
                has_physics: true,
                attached: false,
                previous: [self.motion.mass, self.gravity_mass],
            },
            Precision::Double53,
        )?;
        if load[0] <= 0. || !load.into_iter().all(f32::is_finite) {
            return Err("invalid inventory mass");
        }
        self.motion.mass = load[0];
        self.gravity_mass = load[1];
        Ok(())
    }
    fn prepare_forces(&mut self, axes: Vec2, thrust: bool) -> Result<(), &'static str> {
        let gravity = giants_game::mecc_gravity::apply(
            giants_game::mecc_gravity::Input {
                force: self.motion.force,
                velocity: self.motion.velocity,
                normal: self.ground_normal,
                gravity_mass: self.gravity_mass,
                mass: self.motion.mass,
                friction_limit: self.parameters.friction_limit,
                damping_xy: self.parameters.damping_xy,
                damping_z: self.parameters.damping_z,
                force_time_limit: self.motion.force_time_limit,
                object_flags: 0,
                physics_flags: self.physics_flags,
                project_contact: axes == Vec2::ZERO && !thrust,
            },
            Precision::Double53,
        )?;
        self.motion.force = gravity.force;
        self.motion.force_time_limit = gravity.force_time_limit;
        // Native positive side byte is SideLeft; development axes.x is D-A.
        let controls = [(axes.y * 32.) as i8, (-axes.x * 32.) as i8];
        let desired = giants_game::mecc_control_target::apply(
            giants_game::mecc_control_target::Input {
                matrix: self.object_matrix(),
                scale: self.scale,
                controls,
                speeds: [self.parameters.control_speed; 2],
                normal: self.ground_normal,
                physics_flags: self.physics_flags,
                // Native player contact produces a real object support marker.
                eligible: self.support != 0,
                override_contact: false,
            },
            Precision::Double53,
        )?;
        let Some(desired) = desired else {
            return Ok(());
        };
        if axes != Vec2::ZERO && self.physics_flags & 0x28 != 0 {
            let normal_speed = v3(desired).dot(v3(self.ground_normal)).abs();
            self.ground_target_max_normal_speed =
                self.ground_target_max_normal_speed.max(normal_speed);
            if self.ground_normal[0].hypot(self.ground_normal[1]) > 0.0001 {
                self.ground_target_steps += 1;
            }
        }
        let control = giants_game::mecc_control_force::apply(
            giants_game::mecc_control_force::Input {
                desired_velocity: desired,
                velocity: self.motion.velocity,
                force: self.motion.force,
                gain: self.parameters.control_gain,
                slope_power: self.parameters.slope_power,
                mass: self.motion.mass,
                force_time_limit: self.motion.force_time_limit,
            },
            Precision::Double53,
        )?;
        self.motion.force = control.force;
        self.motion.force_time_limit = control.force_time_limit;
        Ok(())
    }
    fn prepare_jet(
        &mut self,
        dt: f32,
        axes: Vec2,
        thrust: bool,
        boost: bool,
    ) -> Result<(), &'static str> {
        let jet = giants_game::mecc_jet::step_with_ceiling(
            giants_game::mecc_jet::Input {
                state: self.jet,
                matrix: self.object_matrix(),
                force: self.motion.force,
                scale: self.scale,
                height: self.motion.position[2],
                dt,
                controls: [(axes.y * 32.) as i8, (-axes.x * 32.) as i8],
                object_flags: 0,
                motion_flags: self.jet_flags,
                input_flags: if thrust { 0x400 } else { 0 } | if boost { 0x800 } else { 0 },
                physics_flags: self.physics_flags,
                recharge_blocked: false,
                recharge_equipment: false,
                disable_boost: false,
                paused: false,
            },
            self.jet_parameters,
            Precision::Double53,
            // Owner-selected debug reference: full original height parameter.
            giants_game::mecc_jet::HeightCeiling::DebugFull,
        )?;
        self.jet = jet.state;
        self.motion.force = jet.force;
        self.jet_flags = jet.motion_flags;
        self.thrusting = jet.motion_flags & 0x1000 != 0;
        self.audio_boost_started = jet.boost_started;
        if jet.shield_depleted {
            self.jet_depletions += 1;
        }
        if jet.boost_started {
            println!("mecc_dev_jet_boost_started=true effects_pending=true");
        }
        Ok(())
    }
    pub(crate) fn step(
        &mut self,
        dt: f32,
        axes: Vec2,
        terrain: &giants_formats::gti::Terrain,
        thrust: bool,
        boost: bool,
    ) -> Result<giants_game::controlled_mecc::Frame, &'static str> {
        if self.owned_pose.is_some() {
            return Err(
                "owned primary presentation cannot run the development physics a second time",
            );
        }
        // Ordinary player path: integrate prior forces, resolve contact, then
        // prepare gravity/controls and jet force for the following step.
        self.landing_impact = None;
        let was_grounded = self.grounded;
        let axes = if self.alive() { axes } else { Vec2::ZERO };
        let thrust = thrust && self.alive();
        let boost = boost && self.alive();
        self.refresh_inventory_mass()?;
        let previous_position = self.motion.position;
        self.motion = giants_game::mecc_motion::integrate(
            self.motion,
            dt,
            false,
            false,
            Precision::Double53,
        )?;
        self.motion.position =
            giants_game::mecc_motion::limit_position(self.motion.position, Precision::Double53)?;
        let query = giants_game::terrain_queries::StaticTerrainQuery::new(
            terrain,
            1. / terrain.spacing,
            Precision::Double53,
        )?;
        let initial_ground = query.sample([previous_position[0], previous_position[1]])?;
        // Contact consumes the preceding model-speed field. Native derivation
        // occurs after contact, using its corrected current/previous positions.
        let mut npc_responses = 0u64;
        let response = giants_game::player_world_collision::resolve_for_actor_with_responses(
            1,
            giants_game::player_world_collision::Input {
                state: giants_game::player_mesh_contact::State {
                    current: self.motion.position,
                    previous: previous_position,
                    velocity: self.motion.velocity,
                    radius: self.collision.radius,
                    scale: self.scale,
                    height: self.collision_dimensions[1],
                    floor: self.floor_height,
                    plane: self.ground_plane,
                    physics_flags: self.physics_flags,
                    support: self.support,
                    hit: 0,
                    support_part: self.support_part,
                    support_triangle: self.support_triangle,
                    restitution: self.parameters.restitution,
                    retention: self.parameters.retention,
                },
                // The acquired unparented Mecc prefix forces placement even
                // at rest. Vehicle/attached-player behavior remains separate.
                object_flags: self.contact_flags | 0x10000000,
                collision_flags: 0,
                ground_offset: self.ground_offset_source,
                center_offset: self.collision_dimensions[0],
                gravity_mass: self.gravity_mass,
                force_z: self.motion.force[2],
            },
            &self.mesh_objects,
            |point| {
                giants_game::physical_ground::Ground::from_sample(
                    query.sample(point)?,
                    terrain.width,
                )
            },
            |state| {
                giants_game::player_collision_radius::Bounds {
                    primary: self.collision.primary,
                    animation_one: self.collision.auxiliary,
                }
                .refresh(
                    state.radius,
                    state.scale,
                    state.physics_flags,
                    Precision::Double53,
                )
            },
            // Exact PC debug-1.1 event dispatcher 0x004a0990 returns zero for
            // actual absent state2/21/23 table entries (independently executed).
            // This does not acknowledge callbacks for other NPC families.
            |_, _| Ok(false),
            |object, response| {
                if response.retry
                    && response.state.hit == object.fields.id
                    && self
                        .npc_contact_keys
                        .values()
                        .any(|&key| key == object.fields.id)
                {
                    if response.reciprocal_hit {
                        return Err("NPC reciprocal contact requires its peer-reference owner");
                    }
                    npc_responses += 1;
                }
                Ok(())
            },
        )?;
        self.npc_mesh_responses += npc_responses;
        self.mesh_responses += u64::from(response.mesh_responses);
        let resolved = response.state;
        self.collision.radius = resolved.radius;
        self.motion.position = resolved.current;
        self.motion.velocity = resolved.velocity;
        self.motion.force[2] = response.force_z;
        if response.terrain_responses != 0 && response.state.physics_flags & 2 != 0 {
            self.plane_contacts += 1;
        }
        self.physics_flags = response.state.physics_flags;
        self.ground_queries += u64::from(response.ground_queries);
        self.ground_junctions += u64::from(response.junctions);
        self.floor_height = resolved.floor;
        self.ground_plane = resolved.plane;
        self.ground_normal = resolved.plane[..3].try_into().unwrap();
        let final_sample = query.sample([self.motion.position[0], self.motion.position[1]])?;
        if (initial_ground.cell, initial_ground.triangle)
            != (final_sample.cell, final_sample.triangle)
        {
            self.terrain_edge_transitions += 1;
        }
        self.model_motion = giants_game::mecc_model_motion::derive(
            giants_game::mecc_model_motion::Input {
                current: self.motion.position,
                previous: resolved.previous,
                velocity: self.motion.velocity,
                matrix: self.object_matrix(),
                scale: self.scale,
                seconds: f64::from(dt),
                controls: [(axes.y * 32.) as i8, (-axes.x * 32.) as i8],
                minimums: self.model_speed_minimums,
            },
            true,
            Precision::Double53,
        )?;
        self.model_vertical_speed = self.model_motion.model[0];
        self.contact_flags = response.object_flags;
        self.support = resolved.support;
        self.support_part = resolved.support_part;
        self.support_triangle = resolved.support_triangle;
        self.grounded = self.contact_flags & giants_game::ground_contact::CONTACT_BIT != 0;
        self.prepare_forces(axes, thrust)?;
        self.prepare_jet(dt, axes, thrust, boost)?;
        self.animate(dt)?;
        // Bounded ordinary-controller presentation only. On this terrain landing
        // both native contact-selection speed tests are false; health, authority
        // and the full primary's impact/voice providers remain separate.
        if self.mission_pistol.is_some()
            && self.alive()
            && !was_grounded
            && self.grounded
            && response.terrain_responses != 0
            && self.motion.velocity[2] >= 0.
            && self
                .contact_speed_thresholds
                .into_iter()
                .all(|v| v.is_finite() && v > 0.)
            && self.active_clip != 41
            && self.load_response_flags & 0x800000 == 0
            && self.inventory.special == -1
        {
            let reaction = giants_game::mecc_impact::classify(
                self.motion.velocity,
                self.motion.previous_velocity,
                self.impact_parameters,
            )
            .map_err(|_| "ordinary landing classification failed")?;
            if reaction.severity != 0 {
                self.landing_impact = Some(self.motion.position);
            }
        }
        Ok(giants_game::controlled_mecc::Frame {
            before: previous_position,
            position: self.motion.position,
            contact_previous: resolved.previous,
            angles: [self.yaw.to_degrees(), 0., 0.],
            scale: self.scale,
            velocity: self.motion.velocity,
            model_rates: self.model_motion.model,
        })
    }
    fn jet_replay_tick(&mut self, tick: u32) -> bool {
        if tick == 180 {
            self.jet_checks[0] = self.motion.position[2] > self.start.position[2] + 2.
                && self.jet.fuel < 95.
                && self.jet.boost < 1.;
            println!(
                "mecc_dev_jet_lift passed={} height={} fuel={} boost={}",
                self.jet_checks[0],
                self.motion.position[2] - self.start.position[2],
                self.jet.fuel,
                self.jet.boost
            );
        }
        if tick == 780 {
            self.jet_checks[1] = self.jet.fuel >= 99.9 && self.jet.boost >= 0.999 && self.grounded;
            println!(
                "mecc_dev_jet_recharge passed={} fuel={} boost={} grounded={}",
                self.jet_checks[1], self.jet.fuel, self.jet.boost, self.grounded
            );
        }
        if tick == 1260 {
            self.jet_checks[2] = self.jet_depletions > 0 && self.jet.fuel < 10.;
            println!(
                "mecc_dev_jet_exhaust passed={} depletions={} fuel={}",
                self.jet_checks[2], self.jet_depletions, self.jet.fuel
            );
        }
        if tick == 1740 {
            self.jet_checks[3] = self.jet.fuel > 70. && self.grounded;
            self.reset_pose();
            self.jet_checks[4] = self.jet.fuel == 100.
                && self.jet.boost == 1.
                && !self.thrusting
                && self.jet_flags == 0
                && self.motion.force == [0.; 3];
            println!(
                "mecc_dev_jet_land_reset passed={} landing={}",
                self.jet_checks[4], self.jet_checks[3]
            );
        }
        tick == 1800
    }
}
pub fn install(app: &mut App, root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let state = State::load(
        root,
        &app.world().resource::<crate::world_scene::Scene>().asset,
    )?;
    let rigid = crate::static_scene::Scene::from_prepared(
        state.rigid_source.clone(),
        &app.world()
            .resource::<crate::world_scene::Scene>()
            .asset
            .config,
    )?;
    if rigid.needs_cube() {
        return Err("Mecc initial rigid reflection provider is not prepared".into());
    }
    app.insert_resource(rigid);
    let (camera, origin) = state.camera();
    let mut scene = app.world_mut().resource_mut::<crate::world_scene::Scene>();
    scene.validation_camera = Some(camera);
    scene.camera_origin = Some(origin);
    scene.camera_source = "mecc_development_follow";
    scene.cinematic_capture_ready = state.replay_tick.is_none();
    app.insert_resource(state)
        .add_plugins(MaterialPlugin::<crate::actor_scene::Material>::default())
        .add_systems(
            Update,
            update
                .before(crate::source_view::ViewUpdate)
                .run_if(crate::mecc_session::playing),
        )
        .add_systems(
            Update,
            update_hud
                .after(update)
                .run_if(crate::mecc_session::playing),
        );
    app.add_systems(
        Update,
        rigid_view
            .after(update)
            .before(crate::source_view::ViewUpdate)
            .run_if(crate::mecc_session::playing),
    );
    crate::mecc_combat::install(app, root)?;
    crate::mecc_weapon::install(app);
    crate::mecc_incoming::install(app, root)?;
    Ok(())
}
fn rigid_view(
    scene: Res<crate::static_scene::Scene>,
    world: Res<crate::world_scene::Scene>,
    mut visibility: Query<&mut Visibility>,
    mut materials: crate::static_scene::Materials,
) {
    let eye = world.camera_origin.unwrap_or([0.; 3]);
    crate::static_scene::apply_view(
        &scene,
        &world,
        eye,
        1.,
        crate::static_scene::ViewAdmission {
            show: true,
            ordinary: None,
        },
        &mut visibility,
        &mut materials,
    );
}
pub(crate) fn setup(
    mut commands: Commands,
    mut state: ResMut<State>,
    scene: Res<crate::world_scene::Scene>,
    view: Res<crate::source_view::SourceView>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<crate::actor_scene::Material>>,
) {
    println!(
        "mecc_dev_collision_input radius={} bits={:08x} primary={} animation_slot1={:?} profile=world_prepared_resources native_radius_verified=false live_callbacks=false object_dispatch=player_physical_prepared callbacks=unhandled native_trajectory=false",
        state.collision.radius,
        state.collision.radius.to_bits(),
        state.collision.primary_name,
        state.collision.auxiliary_name
    );
    println!(
        "mecc_dev_collision_dimensions center={} height={} scale={} ground_offset_raw={} profile=cold_explicit live_callbacks=false",
        state.collision_dimensions[0],
        state.collision_dimensions[1],
        state.scale,
        state.ground_offset_source
    );
    let mut textures = BTreeMap::new();
    for t in &state.body.textures {
        let mut image = Image::new(
            bevy::render::render_resource::Extent3d {
                width: t.width,
                height: t.height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            t.rgba.clone(),
            bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        );
        image.sampler = crate::logo_material::sampler();
        textures.insert(t.name.to_ascii_lowercase(), images.add(image));
    }
    let mut draws = Vec::new();
    for (index, part) in state.body.mesh.subobjects.iter().enumerate() {
        let ids = part
            .triangles
            .iter()
            .flatten()
            .map(|i| usize::from(*i))
            .collect::<Vec<_>>();
        let colors = ids
            .iter()
            .map(|i| {
                if part.material.flags & 0x20 != 0 {
                    let rgb = state.body.mesh.rgb[*i];
                    [
                        rgb[0] as f32 / 255.,
                        rgb[1] as f32 / 255.,
                        rgb[2] as f32 / 255.,
                        1.,
                    ]
                } else {
                    [1.; 4]
                }
            })
            .collect::<Vec<_>>();
        let mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, state.expanded(index))
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_UV_0,
            ids.iter()
                .map(|i| state.body.mesh.uv[*i])
                .collect::<Vec<_>>(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        let handle = meshes.add(mesh);
        draws.push((index, handle.clone()));
        let alpha = if part.material.flags & 1 != 0 {
            ((1. - part.material.falloff) * 255.) as u8 as f32 / 255.
        } else {
            1.
        };
        let fog = scene.asset.config.fog;
        commands.spawn((
            BodyDraw,
            if state.first_person {
                Visibility::Hidden
            } else {
                Visibility::Visible
            },
            Name::new(format!("Mecc development body / {}", part.name)),
            Mesh3d(handle),
            MeshMaterial3d(
                materials.add(crate::actor_scene::Material::from(
                    crate::actor_scene::Bindings {
                        texture: textures[&part.material.texture.to_ascii_lowercase()].clone(),
                        clip_from_bevy: view.buffer.clone(),
                        validation: Vec4::new(1., 1. / 1920., -1. / 1080., alpha),
                        fog: Vec4::new(fog.start, fog.end, 1., 0.),
                        fog_color: if part.material.flags & 1 != 0 {
                            Vec4::ZERO
                        } else {
                            Vec4::new(
                                fog.color[0] as f32 / 255.,
                                fog.color[1] as f32 / 255.,
                                fog.color[2] as f32 / 255.,
                                0.,
                            )
                        },
                        force_additive: false,
                        blend: state
                            .body
                            .alpha_textures
                            .contains(&part.material.texture.to_ascii_lowercase()),
                    },
                )),
            ),
            bevy::camera::visibility::NoFrustumCulling,
        ));
    }
    state.draws = draws;
    println!(
        "mecc_dev_ready meshes={} controller=development static_standing=false terrain_only=false native_camera=false native_animation_selector=false",
        state.draws.len()
    );
}
#[allow(clippy::too_many_arguments)]
pub fn update(
    mut state: ResMut<State>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mouse: Res<AccumulatedMouseMotion>,
    mut windows: Query<(&mut Window, &mut bevy::window::CursorOptions)>,
    frontend: Option<Res<crate::frontend_scene::Scene>>,
    mut scene: ResMut<crate::world_scene::Scene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut body_visibility: Query<&mut Visibility, With<BodyDraw>>,
    mut materials: crate::world_scene::SceneMaterials,
    mut exit: MessageWriter<AppExit>,
) {
    use crate::mecc_input::{self, Action};
    let controls = frontend.as_deref().map(|s| s.mecc_controls());
    let focused = windows.iter().any(|(w, _)| w.focused);
    let look_ready = windows
        .iter()
        .any(|(w, c)| w.focused && c.grab_mode == bevy::window::CursorGrabMode::Locked);
    for (window, mut cursor) in &mut windows {
        let capture = window.focused && state.replay_tick.is_none();
        cursor.visible = !capture;
        cursor.grab_mode = if capture {
            bevy::window::CursorGrabMode::Locked
        } else {
            bevy::window::CursorGrabMode::None
        };
    }

    if focused && keys.just_pressed(KeyCode::F10) {
        exit.write(AppExit::Success);
    }
    if focused && keys.just_pressed(KeyCode::F5) {
        state.reset_pose();
        state.remainder = 0.;
        println!("mecc_dev_reset=true");
    }
    if focused
        && state.alive()
        && (keys.just_pressed(KeyCode::F9)
            || mecc_input::pressed(controls, Action::View, &keys, &buttons, true))
    {
        state.first_person = !state.first_person;
    }
    if focused && look_ready && state.alive() {
        let gain = mecc_input::mouse_gain(controls);
        let vertical = if controls.is_some_and(|o| o.2) {
            -1.
        } else {
            1.
        };
        state.yaw = (state.yaw - mouse.delta.x * gain).rem_euclid(std::f32::consts::TAU);
        state.pitch = (state.pitch + mouse.delta.y * gain * vertical).clamp(-1.45, 1.45);
    }
    let mut axes = Vec2::ZERO;
    if focused {
        axes = mecc_input::movement_axes(controls, &keys, &buttons);
    }
    state.pending_boost |=
        focused && mecc_input::pressed(controls, Action::Boost, &keys, &buttons, true);
    state.remainder += f64::from(time.delta_secs().min(0.1));
    state.seconds += f64::from(time.delta_secs());
    while state.remainder >= 1. / 120. {
        let mut intent = axes;
        let mut thrust =
            focused && mecc_input::pressed(controls, Action::Thrust, &keys, &buttons, false);
        let mut boost = std::mem::take(&mut state.pending_boost);
        if let Some(tick) = state.replay_tick {
            if tick == 60 && !state.idle_passed && state.collision_walk.is_none() {
                let horizontal_displacement = (state.motion.position[0] - state.start.position[0])
                    .hypot(state.motion.position[1] - state.start.position[1]);
                let horizontal_speed = state.motion.velocity[0].hypot(state.motion.velocity[1]);
                state.idle_passed = horizontal_displacement < 0.01
                    && horizontal_speed < 0.01
                    && state.active_clip == 1;
                println!(
                    "mecc_dev_replay_idle passed={} displacement={horizontal_displacement} speed={horizontal_speed} slot={}",
                    state.idle_passed, state.active_clip
                );
                if !state.idle_passed {
                    exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    return;
                }
            }
            if state.replay_pose_tick.is_some_and(|end| tick >= end) {
                if !state.pose_capture_reported {
                    println!(
                        "mecc_dev_replay_pose_capture tick={tick} slot={} animation_seconds={} owner_validation=false",
                        state.active_clip, state.clock.seconds
                    );
                    state.pose_capture_reported = true;
                }
                if !state.combat_replay {
                    scene.cinematic_capture_ready = true;
                }
                state.remainder = 0.;
                break;
            }
            if state.collision_walk.is_some() {
                thrust = false;
                boost = false;
                match state.collision_walk_tick(tick) {
                    Ok(Some(passed)) => {
                        scene.cinematic_capture_ready = passed;
                        if !passed {
                            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                        }
                        state.remainder = 0.;
                        break;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        error!("mecc_collision_replay_failed={error}");
                        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                        break;
                    }
                }
                intent = match tick {
                    60..=239 => Vec2::Y,
                    240..=359 => -Vec2::Y,
                    _ => Vec2::ZERO,
                };
            } else if state.jet_replay {
                intent = if (60..180).contains(&tick) {
                    Vec2::Y
                } else {
                    Vec2::ZERO
                };
                thrust = (60..180).contains(&tick) || (780..1260).contains(&tick);
                boost = tick == 90;
                if state.jet_replay_tick(tick) {
                    let passed = state.jet_checks.into_iter().all(|v| v)
                        && state.idle_passed
                        && giants_game::mecc_locomotion::AIR_CLIPS
                            .iter()
                            .any(|slot| state.clips[slot].changed);
                    println!(
                        "mecc_dev_jet_finished passed={passed} steps=1800 owner_validation=false native_trajectory_parity=false support_mapping=player_dispatcher"
                    );
                    scene.cinematic_capture_ready = passed;
                    if !passed {
                        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    }
                    state.replay_tick = Some(1801);
                    state.remainder = 0.;
                    break;
                }
                if tick > 1800 {
                    state.remainder = 0.;
                    break;
                }
            } else {
                thrust = false;
                boost = false;
                intent = match tick {
                    60..=179 => Vec2::Y,
                    180..=239 => Vec2::X,
                    _ => Vec2::ZERO,
                };
                if (180..240).contains(&tick) {
                    state.yaw += 0.005;
                }
                if tick == 240 {
                    state.replay_checks[0] =
                        (v3(state.motion.position) - v3(state.start.position)).length() > 1.;
                    println!(
                        "mecc_dev_replay_walk passed={} position={:?}",
                        state.replay_checks[0], state.motion.position
                    );
                    state.motion.position[2] += 6.;
                    state.motion.velocity = [0.; 3];
                }
                if tick == 300 {
                    state.first_person = true;
                }
                if tick == 360 {
                    state.first_person = false;
                }
                if tick == 420 {
                    let terrain = &scene.asset.terrain;
                    let ground = giants_game::terrain_queries::StaticTerrainQuery::new(
                        terrain,
                        1. / terrain.spacing,
                        Precision::Double53,
                    )
                    .and_then(|q| q.sample([state.motion.position[0], state.motion.position[1]]));
                    state.replay_checks[1] = ground.is_ok_and(|g| {
                        let normal_speed: f32 = g.plane[..3]
                            .iter()
                            .zip(state.motion.velocity)
                            .map(|(n, v)| n * v)
                            .sum();
                        (state.motion.position[2] - state.ground_offset - g.height).abs() < 0.1
                            && state.grounded
                            && normal_speed >= -0.05
                    });
                    println!(
                        "mecc_dev_replay_fall passed={} position={:?}",
                        state.replay_checks[1], state.motion.position
                    );
                    state.reset_pose();
                    state.replay_checks[2] = state.motion.position.map(f32::to_bits)
                        == state.start.position.map(f32::to_bits)
                        && state.motion.velocity == [0.; 3]
                        && state.active_clip == 1
                        && state.clock.seconds == 0.
                        && state.first_person == state.initial_first_person;
                    println!("mecc_dev_replay_reset passed={}", state.replay_checks[2]);
                }
                if tick == 480 {
                    let animation_passed = state.clips[&2].changed
                        && giants_game::mecc_locomotion::AIR_CLIPS
                            .iter()
                            .any(|slot| state.clips[slot].changed);
                    println!(
                        "mecc_dev_replay_terrain plane_contacts={} edge_transitions={} ground_queries={} junctions={} physical_ground=checked_static physical_dispatch=player_physical_prepared mesh_objects={} mesh_responses={} height_guard=false native_dispatcher=false",
                        state.plane_contacts,
                        state.terrain_edge_transitions,
                        state.ground_queries,
                        state.ground_junctions,
                        state.mesh_objects.len(),
                        state.mesh_responses
                    );
                    println!(
                        "mecc_dev_replay_animation passed={animation_passed} run_changed={} air_changed={} native_selector=false",
                        state.clips[&2].changed,
                        giants_game::mecc_locomotion::AIR_CLIPS
                            .iter()
                            .any(|slot| state.clips[slot].changed)
                    );
                    let ground_target_passed = state.ground_target_steps > 0
                        && state.ground_target_max_normal_speed < 0.001;
                    println!(
                        "mecc_dev_replay_ground_target passed={ground_target_passed} sloped_steps={} max_normal_speed={} scalar_profile=true eligibility_supplied=true",
                        state.ground_target_steps, state.ground_target_max_normal_speed
                    );
                    let passed = state.replay_checks.into_iter().all(|v| v)
                        && animation_passed
                        && state.idle_passed
                        && ground_target_passed;
                    println!(
                        "mecc_dev_replay_finished passed={passed} steps=480 owner_validation=false native_trajectory_parity=false"
                    );
                    scene.cinematic_capture_ready = passed;
                    if !passed {
                        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
                    }
                    state.replay_tick = Some(481);
                }
                if tick >= 480 {
                    state.remainder = 0.;
                    break;
                }
            }
            state.replay_tick = Some(tick + 1);
        }
        if let Err(error) = state.step(1. / 120., intent, &scene.asset.terrain, thrust, boost) {
            error!("mecc_dev_step_failed={error}");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            break;
        }
        state.remainder -= 1. / 120.;
    }
    if let Err(error) = state.sample_pose() {
        error!("mecc_dev_pose_failed={error}");
        exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        return;
    }
    for mut visibility in &mut body_visibility {
        *visibility = if state.first_person {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    if state.last_view != Some(state.first_person) {
        let mode = if state.first_person { "first" } else { "third" };
        println!(
            "mecc_dev_view mode={mode} body_hidden={} meshes={} native_camera=false",
            state.first_person,
            state.draws.len()
        );
        state.last_view = Some(state.first_person);
    }
    for (index, handle) in &state.draws {
        if let Some(mut mesh) = meshes.get_mut(handle) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, state.expanded(*index));
        }
    }
    let (camera, origin) = state.camera();
    scene.validation_camera = Some(camera);
    scene.camera_origin = Some(origin);
    crate::world_scene::apply_camera(&scene, &mut materials);
    if state.last_log != state.seconds as i32 {
        state.last_log = state.seconds as i32;
        info!(
            "mecc_dev_motion seconds={} position={:?} velocity={:?} axes={:?} grounded={} plane_contacts={} edge_transitions={} height_guard=false",
            state.last_log,
            state.motion.position,
            state.motion.velocity,
            axes,
            state.grounded,
            state.plane_contacts,
            state.terrain_edge_transitions
        );
        for (mut window, _) in &mut windows {
            window.title = "Giants-rs".into();
        }
    }
}

pub(crate) fn setup_hud(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    let path = std::env::var_os("GIANTS_INSPECTION_FONT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(crate::inspection_font_path);
    let font = fonts.add(Font::from_bytes(
        std::fs::read(path).expect("development HUD inspection font"),
    ));
    let camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: 100,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            Msaa::Off,
            IsDefaultUiCamera,
            Name::new("Mecc development HUD camera"),
        ))
        .id();
    let root = commands
        .spawn((
            UiTargetCamera(camera),
            Node {
                position_type: PositionType::Absolute,
                left: px(24.),
                bottom: px(24.),
                width: px(380.),
                padding: UiRect::all(px(14.)),
                row_gap: px(7.),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.04, 0.88)),
        ))
        .id();
    commands.spawn((
        ChildOf(root),
        JetLabel,
        Text::new("MECC DEVELOPMENT LOADOUT"),
        TextFont::from_font_size(FontSize::Px(20.)).with_font(font.clone()),
        TextColor(Color::WHITE),
    ));
    for (boost, color) in [
        (false, Color::srgb(0.1, 0.8, 0.9)),
        (true, Color::srgb(1., 0.6, 0.1)),
    ] {
        let track = commands
            .spawn((
                ChildOf(root),
                Node {
                    width: percent(100.),
                    height: px(12.),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.15, 0.18, 0.2)),
            ))
            .id();
        commands.spawn((
            ChildOf(track),
            JetGauge(boost),
            Node {
                width: percent(100.),
                height: percent(100.),
                ..default()
            },
            BackgroundColor(color),
        ));
    }
    commands.spawn((
        ChildOf(root),
        Text::new("Controls: active profile in Options\nF5: reset   F9: debug view   Esc: menu"),
        TextFont::from_font_size(FontSize::Px(16.)).with_font(font),
        TextColor(Color::srgb(0.8, 0.85, 0.9)),
    ));
}
fn update_hud(
    state: Res<State>,
    mut bars: Query<(&JetGauge, &mut Node)>,
    mut labels: Query<&mut Text, With<JetLabel>>,
) {
    for (g, mut n) in &mut bars {
        n.width = percent(
            if g.0 {
                state.jet.boost * 100.
            } else {
                state.jet.fuel
            }
            .clamp(0., 100.),
        );
    }
    for mut t in &mut labels {
        t.0 = format!(
            "MECC DEV â€” {}\nFuel {:.0}%   Boost {:.0}%   Mass {:.1}",
            if state.thrusting { "JET ON" } else { "JET OFF" },
            state.jet.fuel,
            state.jet.boost * 100.,
            state.motion.mass
        );
    }
}

#[cfg(test)]
mod camera_tests {
    use super::*;
    #[test]
    fn source_basis_keeps_winding_and_projects_the_forward_target() {
        let eye = Vec3::new(20., 30., 8.);
        let view = camera_view(eye, -Vec3::Y, Vec3::Z);
        assert!((view.determinant() + 1.).abs() < 0.00001);
        assert!(view.transform_point3(eye).length() < 0.00001);
        assert_eq!(view.transform_point3(eye - Vec3::Y), Vec3::Z);
        assert_eq!(view.transform_point3(eye - Vec3::X), Vec3::X);
        assert_eq!(view.transform_point3(eye + Vec3::Z), Vec3::Y);
    }
}
