//! Owned ordinary Delphi physical/ANM/history composition.
//! Primary selection, immersion, turbo and live mission admission are separate.
use crate::{actor_fields::Fields, arithmetic::Precision, specification::Prepared};
use std::sync::Arc;

#[derive(Clone)]
pub struct Resources {
    pub(crate) specification: Arc<Prepared>,
    radius: crate::player_collision_radius::Bounds,
    pub(crate) materials: Arc<crate::mecc_contact_materials::Catalog>,
    pub(crate) locomotion: Option<crate::delphi_locomotion::Clips>,
    pub(crate) manual_rotation: bool,
    pub(crate) authored_camera: bool,
    pub(crate) post_physical: bool,
    pub(crate) reserve: bool,
    pub(crate) primary_tail: bool,
    pub(crate) proximity_continuation: bool,
    pub(crate) local_target_phase: bool,
}
impl std::fmt::Debug for Resources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DelphiPhysicalResources")
            .field("class", &self.specification.class)
            .field("radius", &self.radius)
            .field("locomotion", &self.locomotion)
            .field("manual_rotation", &self.manual_rotation)
            .field("authored_camera", &self.authored_camera)
            .field("post_physical", &self.post_physical)
            .field("reserve", &self.reserve)
            .field("primary_tail", &self.primary_tail)
            .field("local_target_phase", &self.local_target_phase)
            .finish()
    }
}
impl Resources {
    /// Read the caller-selected external metadata file without launching it.
    /// This prepares the scoped locomotion profile, not a complete controller.
    pub fn load_locomotion(
        source: &crate::scene_resources::SceneResources,
        specification: Arc<Prepared>,
        materials: Arc<crate::mecc_contact_materials::Catalog>,
        executable: &std::path::Path,
    ) -> Result<Self, String> {
        let definitions = crate::delphi_animation_definitions::Definitions::read(executable)?;
        Self::load(source, specification, materials)?
            .with_locomotion(definitions.sword_locomotion()?)
    }
    pub fn load(
        source: &crate::scene_resources::SceneResources,
        specification: Arc<Prepared>,
        materials: Arc<crate::mecc_contact_materials::Catalog>,
    ) -> Result<Self, String> {
        if specification.class != 3 || !specification.animations.contains_key(&1) {
            return Err("Delphi physical resources require acquired class3 and animation1".into());
        }
        let bounds =
            crate::resident_environment::Resources::load(source, &specification)?.bounds(1)?;
        Ok(Self {
            specification,
            materials,
            locomotion: None,
            manual_rotation: false,
            authored_camera: false,
            post_physical: false,
            reserve: false,
            primary_tail: false,
            proximity_continuation: false,
            local_target_phase: false,
            radius: crate::player_collision_radius::Bounds {
                primary: bounds.primary,
                animation_one: bounds.radius_auxiliary,
            },
        })
    }
    pub fn with_locomotion(
        mut self,
        clips: crate::delphi_locomotion::Clips,
    ) -> Result<Self, String> {
        if clips
            .slots()
            .any(|slot| slot >= 254 || !self.specification.animations.contains_key(&slot))
        {
            return Err(
                "Delphi locomotion requires its actual prepared animation descriptors".into(),
            );
        }
        self.locomotion = Some(clips);
        Ok(self)
    }
    /// Freeze the acquired manual-rotation producer into a locomotion cycle.
    pub fn with_manual_rotation(mut self) -> Result<Self, String> {
        if self.locomotion.is_none() {
            return Err("Delphi manual rotation requires its locomotion resources".into());
        }
        self.manual_rotation = true;
        Ok(self)
    }
    /// Freeze the acquired pre-heading camera callback into the controlled
    /// profile. World mode/aspect/target and startup pose remain actual inputs.
    pub fn with_authored_camera(mut self) -> Result<Self, String> {
        if !self.manual_rotation
            || !self
                .specification
                .metadata
                .references
                .names
                .get(10)
                .is_some_and(Option::is_some)
        {
            return Err(
                "Delphi authored camera requires controlled resources and its declared reference"
                    .into(),
            );
        }
        self.authored_camera = true;
        Ok(self)
    }
    /// Freeze the acquired dry post-physical callback into a controlled cycle.
    /// This is not the surrounding full-primary or auxiliary scheduler.
    pub fn with_post_physical(mut self) -> Result<Self, String> {
        if !self.manual_rotation {
            return Err("Delphi post-physical callback requires controlled resources".into());
        }
        self.post_physical = true;
        Ok(self)
    }
    /// Freeze reserve processing after the acquired controlled post callback.
    /// Active movement/immersion and the remaining item callbacks stay separate.
    pub fn with_reserve(mut self) -> Result<Self, String> {
        if !self.post_physical {
            return Err("Delphi reserve requires its post-physical controlled profile".into());
        }
        self.reserve = true;
        Ok(self)
    }
    /// Add acquired item timing/selection and the absent auxiliary-context route.
    /// Eligible interaction actions and nonempty auxiliary resources still refuse.
    pub fn with_primary_tail(mut self) -> Result<Self, String> {
        if !self.reserve {
            return Err("Delphi primary tail requires reserve/post resources".into());
        }
        self.primary_tail = true;
        Ok(self)
    }
    /// Append the acquired proximity continuation to the explicit item-tail
    /// sequence. This does not acquire the enclosing full-primary dispatch.
    pub fn with_proximity_continuation(mut self) -> Result<Self, String> {
        if !self.primary_tail {
            return Err("Delphi proximity requires its prepared item-tail profile".into());
        }
        self.proximity_continuation = true;
        Ok(self)
    }
    /// Require an owned target phase before every physical component, including
    /// the first. This does not provide perception or enable active movement.
    pub fn with_local_target_phase(mut self) -> Result<Self, String> {
        if !self.primary_tail {
            return Err("Delphi local target phase requires its item-tail consumer profile".into());
        }
        self.local_target_phase = true;
        Ok(self)
    }
}
/// Actual per-frame preference/owner boundaries for the manual helpers.
#[derive(Clone, Copy, Debug)]
pub struct ManualRotation {
    pub sensitivity: f32,
    pub local_owner: bool,
    pub paused: bool,
}
pub struct Environment<'a> {
    pub number: u64,
    pub actor_key: u32,
    pub globals: &'a giants_formats::global_data::GlobalData,
    pub global_seconds: f32,
    pub areas: &'a [crate::object_time::SlowArea],
    pub commands: [u8; 8],
    pub launch_blocked: bool,
    pub objects: &'a [crate::player_world_collision::Object],
}
#[derive(Clone, Debug)]
pub struct View {
    pub number: u64,
    /// Acquired zero cold header damage; damage producers are not admitted yet.
    pub accumulated_damage: f32,
    pub velocity: [f32; 3],
    pub previous_velocity: [f32; 3],
    pub model: [f32; 4],
    pub previous_model: [f32; 4],
    /// Independent look angle; this is not body pitch or roll.
    pub look: f32,
    pub floor: f32,
    pub plane: [f32; 4],
    pub radius: f32,
    pub hit: u32,
    pub support: u32,
    pub support_part: u32,
    pub support_triangle: u32,
    pub affine: Option<crate::actor_affine::Cache>,
    /// Actual input word after the optional completed/retained post callback.
    pub post_buttons: Option<u32>,
    /// Current commands after the selected proximity continuation, including
    /// retained partial effects. None means that continuation has not run.
    pub post_commands: Option<[u8; 8]>,
    pub failure: Option<String>,
}
#[derive(Clone, Debug)]
pub(crate) struct Owned {
    pub resources: Resources,
    pub view: View,
    key: Option<u32>,
}
#[derive(Clone)]
pub(crate) struct Output {
    pub fields: Fields,
    pub payload: Vec<u8>,
}
pub(crate) enum Phase<'a> {
    BeforeHeading,
    DryReset {
        fields: &'a mut Fields,
        payload: &'a mut [u8],
        physics_flags: u32,
        damage: f32,
    },
    Animation {
        fields: &'a mut Fields,
        view: &'a View,
        seconds: f32,
    },
}
pub(crate) fn word(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}
fn scalar(data: &[u8], at: usize) -> f32 {
    f32::from_bits(word(data, at))
}
fn put(data: &mut [u8], at: usize, value: u32) {
    data[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
impl Owned {
    pub fn fresh(resources: Resources, fields: &Fields, payload: &[u8]) -> Result<Self, String> {
        if fields.class != 3
            || fields.configuration[20] != 3
            || payload.len() != 4096
            || fields.configuration[14] & 1 == 0
            || fields.object_flags & 0x1c000 != 0
            || [0xd14, 0xd18, 0xd1c, 0xd20]
                .into_iter()
                .any(|at| word(payload, at) != 0)
            || word(payload, 0x68) != u32::MAX
        {
            return Err(
                "Delphi physical cycle requires its unmoved cold player and physics".into(),
            );
        }
        Ok(Self {
            resources,
            key: None,
            view: View {
                number: 0,
                accumulated_damage: 0.,
                velocity: [0.; 3],
                previous_velocity: [0.; 3],
                model: [0.; 4],
                previous_model: [0.; 4],
                look: 0.,
                floor: 0.,
                plane: [0.; 4],
                radius: 0.,
                hit: 0,
                support: 0,
                support_part: 0,
                support_triangle: 0,
                affine: None,
                post_buttons: None,
                post_commands: None,
                failure: None,
            },
        })
    }
    pub fn advance(
        &mut self,
        fields: &Fields,
        payload: &[u8],
        e: Environment<'_>,
        manual: Option<ManualRotation>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        consume: impl FnMut(Phase<'_>) -> Result<(), String>,
    ) -> Result<Output, String> {
        if self.view.failure.is_some() || self.view.number.checked_add(1) != Some(e.number) {
            return Err("Delphi physical step failed or is not contiguous".into());
        }
        let mut next = self.clone();
        match next.run(fields, payload, e, manual, ground, consume) {
            Ok(out) => {
                *self = next;
                Ok(out)
            }
            Err(error) => {
                self.view.failure = Some(error.clone());
                Err(error)
            }
        }
    }
    fn run(
        &mut self,
        fields: &Fields,
        payload: &[u8],
        e: Environment<'_>,
        manual: Option<ManualRotation>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        mut consume: impl FnMut(Phase<'_>) -> Result<(), String>,
    ) -> Result<Output, String> {
        if payload.len() != 4096
            || e.launch_blocked
            || e.actor_key == 0
            || self.key.is_some_and(|k| k != e.actor_key)
            || !e.global_seconds.is_finite()
            || e.global_seconds < 0.
            || fields.object_flags & 0x1c000 != 0
            || fields.control_flags & (0x100 | 0x2000000 | 0x8000) != 0
            || word(payload, 0xe2c) != 0
            || word(payload, 0xf20) != 0
            || word(payload, 0xf24) != 0
        {
            return Err("Delphi physical cycle outside the acquired ordinary dry profile".into());
        }
        self.key = Some(e.actor_key);
        self.view.post_commands = None;
        let mut f = fields.clone();
        let mut body = payload.to_vec();
        f.previous = f.current;
        f.current = f.position;
        f.object_flags |= 0x40;
        let entry = crate::actor_primary_gate::enter(
            f.object_flags,
            f.configuration[14],
            e.commands,
            self.view.model.map(f32::to_bits),
            self.view.previous_model.map(f32::to_bits),
        );
        if !entry.invoke {
            return Err("Delphi physical service requires an entered primary prefix".into());
        }
        f.configuration[14] = entry.binding_flags;
        self.view.previous_model = entry.previous_rates.map(f32::from_bits);
        if self.resources.authored_camera {
            consume(Phase::BeforeHeading)?;
        }
        if let Some(manual) = manual {
            let seconds = crate::object_time::seconds(
                e.global_seconds,
                Some(crate::object_time::Sample {
                    owner: f.dispatch_context,
                    position: f.position,
                }),
                None,
                e.areas,
            );
            let rotation = crate::player_rotation::advance(
                crate::player_rotation::State {
                    heading: f.angles[0],
                    look: self.view.look,
                    rate: self.view.model[3],
                    previous_rate: self.view.previous_model[3],
                },
                crate::player_rotation::Parameters {
                    acceleration: f32::from_bits(f.configuration[5]),
                    maximum_rate: f32::from_bits(f.configuration[6]),
                    deceleration: f32::from_bits(f.configuration[7]),
                },
                crate::player_rotation::Input {
                    turn: e.commands[0] as i8,
                    look: e.commands[1] as i8,
                    seconds,
                    sensitivity: manual.sensitivity,
                    local_owner: manual.local_owner,
                    paused: manual.paused,
                    automatic_heading: f.object_flags & 0x1000000 != 0,
                },
            )?;
            f.angles[0] = rotation.heading;
            self.view.look = rotation.look;
            self.view.model[3] = rotation.rate;
            self.view.previous_model[3] = rotation.previous_rate;
        }
        let jump = crate::delphi_jump::step(
            crate::delphi_jump::State {
                mode: word(&body, 0x5c) as i32,
                active: f.object_flags,
                response: f.response_flags,
                controls: f.control_flags,
                horizontal_rates: [self.view.model[1], self.view.model[2]],
                pitch_roll: [f.angles[1], f.angles[2]],
                position: f.position,
                vertical_speed: self.view.velocity[2],
                hold_seconds: scalar(&body, 0x60),
                stored_height: scalar(&body, 0x48),
                context: [word(&body, 0xf20), word(&body, 0xf24)],
                launch_reset: word(&body, 0xfa4),
                physical_request: scalar(&body, 0xd2c),
                physical_acceleration_z: scalar(&body, 0xd20),
            },
            crate::delphi_jump::Parameters::from_globals(e.globals)?,
            crate::delphi_jump::Tick {
                held: u32::from_le_bytes(e.commands[4..8].try_into().unwrap()) & 0x200 != 0,
                launch_blocked: e.launch_blocked || manual.is_some_and(|m| m.paused),
                global_seconds: e.global_seconds,
                owner: f.dispatch_context,
                areas: e.areas,
            },
        )?;
        f.object_flags = jump.active;
        f.response_flags = jump.response;
        f.control_flags = jump.controls;
        self.view.velocity[2] = jump.vertical_speed;
        for (at, value) in [
            (0x60, jump.hold_seconds.to_bits()),
            (0x48, jump.stored_height.to_bits()),
            (0xfa4, jump.launch_reset),
            (0xd2c, jump.physical_request.to_bits()),
            (0xd20, jump.physical_acceleration_z.to_bits()),
        ] {
            put(&mut body, at, value);
        }
        if manual.is_some() {
            let aligned =
                crate::delphi_body_alignment::advance(crate::delphi_body_alignment::Input {
                    mode: word(&body, 0x5c) as i32,
                    controls: f.control_flags,
                    previous: f.previous,
                    position: f.position,
                    pitch_roll: [f.angles[1], f.angles[2]],
                    seconds: crate::object_time::seconds(
                        e.global_seconds,
                        Some(crate::object_time::Sample {
                            owner: f.dispatch_context,
                            position: f.position,
                        }),
                        None,
                        e.areas,
                    ),
                })?;
            f.angles[1..3].copy_from_slice(&aligned);
        }
        if manual.is_some_and(|m| m.paused) {
            // The physical callback keeps first ANM/history while skipping
            // integration, placement, new model rates and force production.
            // ANM does not by itself request an actor world transform. Keep
            // an absent or older cache until a real consumer requires it.
            self.view.number = e.number;
            let seconds = crate::object_time::seconds(
                e.global_seconds,
                Some(crate::object_time::Sample {
                    owner: f.dispatch_context,
                    position: f.position,
                }),
                None,
                e.areas,
            );
            consume(Phase::Animation {
                fields: &mut f,
                view: &self.view,
                seconds: seconds as f32,
            })?;
            history(&mut body, &mut f, seconds)?;
            return Ok(Output {
                fields: f,
                payload: body,
            });
        }
        let mut p: [u32; 15] = std::array::from_fn(|i| word(&body, 0xd00 + i * 4));
        let pf = |i| f32::from_bits(p[i]);
        let context = f.dispatch_context;
        let seconds = |position| {
            crate::object_time::seconds(
                e.global_seconds,
                Some(crate::object_time::Sample {
                    owner: context,
                    position,
                }),
                None,
                e.areas,
            )
        };
        let motion = crate::mecc_motion::integrate_with_time(
            crate::mecc_motion::Motion {
                position: f.position,
                velocity: self.view.velocity,
                previous_velocity: self.view.previous_velocity,
                force: [pf(6), pf(7), pf(8)],
                mass: pf(2),
                speed_limit: pf(10),
                force_time_limit: pf(11),
                stored_dt: pf(5),
            },
            seconds(f.position),
            false,
            false,
            Precision::Double53,
        )?;
        f.position = crate::mecc_motion::limit_position(motion.position, Precision::Double53)?;
        let placed = crate::player_world_collision::resolve_for_actor(
            e.actor_key,
            crate::player_world_collision::Input {
                state: crate::player_mesh_contact::State {
                    current: f.position,
                    previous: f.current,
                    velocity: motion.velocity,
                    scale: f.scale,
                    radius: self.view.radius,
                    height: f32::from_bits(f.configuration[23]),
                    floor: self.view.floor,
                    plane: self.view.plane,
                    physics_flags: p[0],
                    hit: self.view.hit,
                    support: self.view.support,
                    support_part: self.view.support_part,
                    support_triangle: self.view.support_triangle,
                    restitution: pf(3),
                    retention: pf(4),
                },
                object_flags: f.object_flags,
                collision_flags: f.configuration[14],
                ground_offset: f32::from_bits(f.configuration[17]),
                center_offset: f32::from_bits(f.configuration[22]),
                gravity_mass: pf(1),
                force_z: motion.force[2],
            },
            e.objects,
            ground,
            |s| {
                self.resources.radius.refresh(
                    s.radius,
                    s.scale,
                    s.physics_flags,
                    Precision::Double53,
                )
            },
        )?;
        let c = placed.state;
        if c.physics_flags & 0x40 != 0 {
            return Err(
                "Delphi immersion transition requires its acquired water/effect producer".into(),
            );
        }
        f.position = c.current;
        f.current = c.previous;
        let (flags, affine) = crate::actor_affine::update(
            crate::uncached_pose::Pose {
                position: f.position,
                angles: f.angles,
                scale: f.scale,
            },
            placed.object_flags,
            self.view.affine,
            Precision::Double53,
        )?;
        f.object_flags = flags;
        self.view.number = e.number;
        self.view.velocity = c.velocity;
        self.view.previous_velocity = motion.previous_velocity;
        self.view.floor = c.floor;
        self.view.plane = c.plane;
        self.view.radius = c.radius;
        self.view.hit = c.hit;
        self.view.support = c.support;
        self.view.support_part = c.support_part;
        self.view.support_triangle = c.support_triangle;
        self.view.affine = Some(affine);
        // Delphi's first ANM consumes the preceding model rates. New rates are
        // produced afterwards, before gravity and control forces.
        let animation_seconds = seconds(f.position) as f32;
        consume(Phase::Animation {
            fields: &mut f,
            view: &self.view,
            seconds: animation_seconds,
        })?;
        if self.resources.post_physical {
            consume(Phase::DryReset {
                fields: &mut f,
                payload: &mut body,
                physics_flags: c.physics_flags,
                damage: self.view.accumulated_damage,
            })?;
        }
        let model = crate::mecc_model_motion::derive(
            crate::mecc_model_motion::Input {
                current: f.position,
                previous: f.current,
                velocity: c.velocity,
                matrix: affine.matrix,
                scale: f.scale,
                seconds: seconds(f.position),
                controls: [e.commands[2] as i8, e.commands[3] as i8],
                minimums: [
                    e.globals.scalar(0x724).map_err(|e| e.to_string())?,
                    e.globals.scalar(0x720).map_err(|e| e.to_string())?,
                ],
            },
            true,
            Precision::Double53,
        )?;
        self.view.model[..3].copy_from_slice(&model.model);
        let gravity = crate::mecc_gravity::apply(
            crate::mecc_gravity::Input {
                force: [motion.force[0], motion.force[1], placed.force_z],
                velocity: c.velocity,
                normal: c.plane[..3].try_into().unwrap(),
                gravity_mass: pf(1),
                mass: pf(2),
                friction_limit: pf(12),
                damping_xy: pf(13),
                damping_z: pf(14),
                force_time_limit: motion.force_time_limit,
                object_flags: f.object_flags,
                physics_flags: c.physics_flags,
                project_contact: e.commands[2..4] == [0, 0],
            },
            Precision::Double53,
        )?;
        let speed = e.globals.scalar(0x728).map_err(|e| e.to_string())?;
        let target = crate::mecc_control_target::apply(
            crate::mecc_control_target::Input {
                matrix: affine.matrix,
                scale: f.scale,
                controls: [e.commands[2] as i8, e.commands[3] as i8],
                speeds: [speed; 2],
                normal: c.plane[..3].try_into().unwrap(),
                physics_flags: c.physics_flags,
                eligible: c.support != 0,
                override_contact: f.object_flags & 0x10 != 0,
            },
            Precision::Double53,
        )?;
        let (force, cap) = if let Some(desired_velocity) = target {
            let output = crate::mecc_control_force::apply(
                crate::mecc_control_force::Input {
                    desired_velocity,
                    velocity: c.velocity,
                    force: gravity.force,
                    gain: e.globals.scalar(0x72c).map_err(|e| e.to_string())?,
                    slope_power: e.globals.scalar(0x730).map_err(|e| e.to_string())?,
                    mass: pf(2),
                    force_time_limit: gravity.force_time_limit,
                },
                Precision::Double53,
            )?;
            (output.force, output.force_time_limit)
        } else {
            (gravity.force, gravity.force_time_limit)
        };
        p[0] = c.physics_flags;
        p[5] = motion.stored_dt.to_bits();
        p[6..9].copy_from_slice(&force.map(f32::to_bits));
        p[9] = model.world_speed.to_bits();
        p[11] = cap.to_bits();
        for (i, value) in p.into_iter().enumerate() {
            put(&mut body, 0xd00 + i * 4, value);
        }
        let history_seconds = seconds(f.position);
        history(&mut body, &mut f, history_seconds)?;
        Ok(Output {
            fields: f,
            payload: body,
        })
    }
}
fn history(body: &mut [u8], fields: &mut Fields, seconds: f64) -> Result<(), String> {
    if fields.angles[0] < 0. {
        fields.angles[0] = (f64::from(fields.angles[0]) + 360.) as f32;
    } else if fields.angles[0] >= 360. {
        fields.angles[0] = (f64::from(fields.angles[0]) - 360.) as f32;
    }
    let mut index = word(body, 0x68);
    let record = |fields: &Fields| -> [u32; 9] {
        [
            fields.position[0].to_bits(),
            fields.position[1].to_bits(),
            fields.position[2].to_bits(),
            fields.angles[0].to_bits(),
            fields.channels[0][0],
            fields.channels[0][1],
            fields.channels[0][7],
            fields.channels[2][0],
            fields.channels[2][1],
        ]
    };
    let store = |body: &mut [u8], index: usize| {
        for (i, value) in record(fields).into_iter().enumerate() {
            put(body, 0x6c + index * 40 + i * 4, value);
        }
    };
    if index == u32::MAX {
        for i in 0..80 {
            store(body, i);
            put(body, 0x6c + i * 40 + 36, ((i as f32) * 0.025f32).to_bits());
        }
        index = 79;
    }
    if index >= 80 {
        return Err("Delphi history index outside its owned ring".into());
    }
    let i = index as usize;
    let previous = if i == 0 { 79 } else { i - 1 };
    let old = scalar(body, 0x6c + i * 40 + 36);
    let previous_time = scalar(body, 0x6c + previous * 40 + 36);
    let next = if f64::from(old) - f64::from(previous_time) < f64::from(0.025f32) {
        i
    } else {
        (i + 1) % 80
    };
    store(body, next);
    // The native destination is float32 even though local seconds are wide.
    put(
        body,
        0x6c + next * 40 + 36,
        ((f64::from(old) + seconds) as f32).to_bits(),
    );
    put(body, 0x68, next as u32);
    Ok(())
}
