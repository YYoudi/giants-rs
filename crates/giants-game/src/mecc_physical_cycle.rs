//! Persistent Mecc physical subcycle with an acquired pre-gravity ANM boundary.
//! Full primary/secondary, jet, damage and global scheduling remain separate.
//! Physical completion never sets the native primary-completed flag40.
use crate::{actor_fields::Fields, arithmetic::Precision};
use giants_formats::{global_data::GlobalData, weapons::Catalog};
use std::sync::Arc;

#[derive(Clone)]
pub struct Resources {
    specification: Arc<crate::specification::Prepared>,
    radius: crate::player_collision_radius::Bounds,
}
impl std::fmt::Debug for Resources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeccPhysicalResources")
            .field("radius", &self.radius)
            .finish()
    }
}
impl Resources {
    pub fn load(
        source: &crate::scene_resources::SceneResources,
        specification: Arc<crate::specification::Prepared>,
    ) -> Result<Self, String> {
        if specification.class != 2
            || specification
                .metadata
                .selection
                .animations
                .iter()
                .all(|a| a.slot != 1)
        {
            return Err("physical Mecc requires acquired class2 resources and animation1".into());
        }
        let resources = crate::resident_environment::Resources::load(source, &specification)?;
        let bounds = resources.bounds(1)?;
        Ok(Self {
            specification,
            radius: crate::player_collision_radius::Bounds {
                primary: bounds.primary,
                animation_one: bounds.radius_auxiliary,
            },
        })
    }
    pub(crate) fn belongs_to(&self, specification: &Arc<crate::specification::Prepared>) -> bool {
        Arc::ptr_eq(&self.specification, specification)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Orientation {
    /// Signed heading/look commands; velocities are produced by the helper.
    pub controls: [i8; 2],
    pub control_factor: f32,
    pub skip_heading: bool,
    pub controlled_actor: bool,
}
pub struct Environment<'a> {
    pub number: u64,
    pub generation: u32,
    pub globals: &'a GlobalData,
    pub catalog: &'a Catalog,
    pub global_seconds: f32,
    pub areas: &'a [crate::object_time::SlowArea],
    /// Forward and side signed bytes. Heading/pitch and button services remain
    /// separate primary phases, not silently interpreted here.
    pub controls: [i8; 2],
    /// Independent native header D4 angle, not the Euler angle at CC.
    /// Its control/camera producer is supplied by the caller.
    pub facing_pitch: f32,
    /// Real common heading/look phase in the acquired normal primary. The
    /// legacy independent facing override must stay zero when this is present.
    pub orientation: Option<Orientation>,
    pub actor_key: u32,
    pub objects: &'a [crate::player_world_collision::Object],
}
#[derive(Clone, Debug)]
pub struct View {
    pub number: u64,
    pub local_seconds: f32,
    pub velocity: [f32; 3],
    pub previous_velocity: [f32; 3],
    pub model_rates: [f32; 3],
    /// Four rates copied by the primary wrapper before the owned producers.
    pub previous_model_rates: [f32; 4],
    pub angular_rate: f32,
    pub look_pitch: f32,
    pub commands: [u8; 8],
    pub floor: f32,
    pub plane: [f32; 4],
    pub radius: f32,
    pub hit: u32,
    pub support: u32,
    pub support_part: u32,
    pub support_triangle: u32,
    pub affine: Option<crate::actor_affine::Cache>,
}
#[derive(Clone, Debug)]
pub(crate) struct Owned {
    pub view: View,
    pub failure: Option<String>,
    resources: Resources,
    actor_key: Option<u32>,
    channel_number: u64,
    channel_state: Option<crate::animation_channels::Channels>,
    pub(crate) channel_projection: Option<ChannelProjection>,
    pub(crate) event_number: u64,
    pub(crate) interleaved: bool,
    pub(crate) secondary: bool,
    pub(crate) jet: bool,
    pub(crate) jet_buttons: u32,
    pub(crate) countdowns: bool,
    pub(crate) periodic: bool,
    pub(crate) collection: bool,
    pub(crate) primary: bool,
    pub(crate) fire: bool,
    pub(crate) orientation: bool,
    pub(crate) world_history: Option<u64>,
    pub(crate) world_stopped: bool,
}
/// Channel projection and its optional gated reference snapshots. ANM events,
/// further procedural/vertex consumers and secondary writes remain required.
#[derive(Clone, Debug)]
pub struct ChannelProjection {
    pub first: crate::animation_channels::Channels,
    pub second: crate::animation_channels::Channels,
    pub selected: Option<crate::animation_channels::Request>,
    pub first_references: Option<Vec<crate::procedural_aim::Matrix>>,
    pub second_references: Option<Vec<crate::procedural_aim::Matrix>>,
    /// Header cache stamps for actual interleaved passes. Numeric-only
    /// projections do not execute the allocator and leave these absent.
    pub first_stamp: Option<u32>,
    pub second_stamp: Option<u32>,
    pub first_primary_flags: Option<u32>,
    pub(crate) first_processed: bool,
    pub(crate) second_processed: bool,
}
pub struct InterleavedProjection {
    pub fire: Option<crate::mecc_fire_primary::Output>,
    pub physical: View,
    pub channels: ChannelProjection,
    pub event_visits: u32,
    pub first_physics: [u32; 15],
    pub secondary: Option<SecondaryProjection>,
    pub jet: Option<crate::mecc_jet_owned::Projection>,
}
pub struct SecondaryProjection {
    pub context: u32,
    pub words: [u32; 8],
    pub weapon_commit: bool,
}
pub(crate) struct Output {
    pub fields: Fields,
    pub payload: Vec<u8>,
}
fn word(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}
fn put(data: &mut [u8], at: usize, v: u32) {
    data[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
impl Owned {
    pub(crate) fn belongs_to(&self, specification: &Arc<crate::specification::Prepared>) -> bool {
        self.resources.belongs_to(specification)
    }
    pub fn fresh(resources: Resources, fields: &Fields, payload: &[u8]) -> Result<Self, String> {
        if fields.class != 2
            || fields.configuration[20] != 2
            || fields.scale <= 0.
            || payload.len() != 524
            || fields.object_flags & 0x1c000 != 0
            || [0x98, 0x9c, 0xa0, 0xa4, 0xa8, 0xb0]
                .into_iter()
                .any(|o| word(payload, o) != 0)
        {
            return Err("physical cycle requires unmoved initialized cold Mecc".into());
        }
        Ok(Self {
            resources,
            actor_key: None,
            channel_number: 0,
            channel_state: None,
            channel_projection: None,
            event_number: 0,
            interleaved: false,
            secondary: false,
            jet: false,
            jet_buttons: 0,
            countdowns: false,
            periodic: false,
            collection: false,
            primary: false,
            fire: false,
            orientation: false,
            world_history: None,
            world_stopped: false,
            failure: None,
            view: View {
                number: 0,
                local_seconds: 0.,
                velocity: [0.; 3],
                previous_velocity: [0.; 3],
                model_rates: [0.; 3],
                previous_model_rates: [0.; 4],
                angular_rate: 0.,
                look_pitch: 0.,
                commands: [0; 8],
                floor: 0.,
                plane: [0.; 4],
                radius: 0.,
                hit: 0,
                support: 0,
                support_part: 0,
                support_triangle: 0,
                affine: None,
            },
        })
    }
    pub fn advance(
        &mut self,
        fields: &Fields,
        payload: &[u8],
        e: Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
    ) -> Result<Output, String> {
        self.advance_with_tail(fields, payload, e, ground, |_, _, _| Ok(()))
    }
    pub(crate) fn advance_with_tail(
        &mut self,
        fields: &Fields,
        payload: &[u8],
        e: Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        tail: impl FnMut(&mut Fields, &[u8], &View) -> Result<(), String>,
    ) -> Result<Output, String> {
        if self.failure.is_some() || self.view.number.checked_add(1) != Some(e.number) {
            return Err("physical cycle failed or step is not contiguous".into());
        }
        let mut next = self.clone();
        match next.run(fields, payload, e, ground, tail) {
            Ok(output) => {
                *self = next;
                Ok(output)
            }
            Err(error) => {
                self.failure = Some(error.clone());
                Err(error)
            }
        }
    }
    fn run(
        &mut self,
        fields: &Fields,
        payload: &[u8],
        e: Environment<'_>,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
        mut tail: impl FnMut(&mut Fields, &[u8], &View) -> Result<(), String>,
    ) -> Result<Output, String> {
        if !e.global_seconds.is_finite()
            || e.global_seconds < 0.
            || e.actor_key == 0
            || self.actor_key.is_some_and(|key| key != e.actor_key)
            || fields.object_flags & 0x1c000 != 0
            || fields.control_flags
                & !(if self.secondary {
                    crate::mecc_secondary::control_mask(self.fire)
                } else {
                    0
                })
                != 0
            || e.objects
                .iter()
                .any(|o| o.fields.id == 0 || o.response_flags & 0x10 != 0)
            || !e.areas.iter().all(|a| {
                a.center.into_iter().all(f32::is_finite)
                    && a.radius_squared.is_finite()
                    && a.radius_squared >= 0.
                    && a.factor.is_finite()
                    && a.factor >= 0.
            })
        {
            return Err(
                "Mecc physical environment outside acquired living unparented profile".into(),
            );
        }
        self.actor_key = Some(e.actor_key);
        let mut f = fields.clone();
        if let Some(number) = self.world_history {
            if number != e.number {
                return Err("Mecc world history does not belong to this iteration".into());
            }
        } else {
            f.previous = f.current;
            f.current = f.position;
            f.object_flags &= !0x40;
        }
        // The common world token already cleared40 with prepared history.
        // Standalone cycles perform that prefix above. Mecc47c5d1 then forces
        // placement even while stationary; this is not primary completion.
        f.object_flags |= 0x10000000;
        let prepared = crate::mecc_physics::prepare(
            payload,
            true,
            f.configuration,
            crate::mecc_physics::Refresh {
                generation: e.generation,
                globals: e.globals,
                catalog: e.catalog,
                response_flags: f.response_flags,
                control_flags: f.control_flags,
            },
        )?;
        f.configuration = prepared.configuration;
        let mut body = prepared.payload;
        if let Some(input) = e.orientation {
            if !self.primary || e.facing_pitch != 0. || f.object_flags & 0x1000000 != 0 {
                return Err("Mecc orientation requires the normal primary without external look or AI target".into());
            }
            let p = |i| f32::from_bits(f.configuration[i]);
            let oriented = crate::actor_common_motion::orient(
                crate::actor_common_motion::State {
                    position: f.position,
                    heading: f.angles[0],
                    look_pitch: self.view.look_pitch,
                    rates: [
                        self.view.model_rates[0],
                        self.view.model_rates[1],
                        self.view.model_rates[2],
                        self.view.angular_rate,
                    ],
                    previous_rates: self.view.previous_model_rates,
                    active: f.object_flags,
                    personality: f.configuration[14],
                    state: f.configuration[20],
                    context: 0,
                    parent_present: false,
                    controls: [
                        input.controls[0],
                        input.controls[1],
                        e.controls[0],
                        e.controls[1],
                    ],
                    target_heading: 0.,
                    target_position: [0.; 2],
                },
                &crate::actor_common_motion::Environment {
                    parameters: crate::actor_common_motion::Parameters {
                        acceleration: p(0),
                        speed: p(1),
                        braking: p(4),
                        attenuation: p(3),
                        turn_acceleration: p(5),
                        turn_speed: p(6),
                        turn_braking: p(7),
                        gravity: p(10),
                    },
                    global_seconds: e.global_seconds,
                    areas: e.areas,
                    control_factor: input.control_factor,
                    running_factor: 1.,
                    skip_heading: input.skip_heading,
                    controlled_actor: input.controlled_actor,
                },
            )?;
            f.angles[0] = oriented.heading;
            self.view.angular_rate = oriented.rates[3];
            self.view.previous_model_rates = oriented.previous_rates;
            self.view.look_pitch = oriented.look_pitch;
        } else {
            self.view.look_pitch = e.facing_pitch;
        }
        let mut p = prepared.physics.words();
        let pf = |i| f32::from_bits(p[i]);
        let seconds = |position| {
            crate::object_time::seconds(
                e.global_seconds,
                Some(crate::object_time::Sample { owner: 0, position }),
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
        self.view.previous_velocity = motion.previous_velocity;
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
        let model = crate::mecc_model_motion::derive(
            crate::mecc_model_motion::Input {
                current: f.position,
                previous: f.current,
                velocity: c.velocity,
                matrix: affine.matrix,
                scale: f.scale,
                seconds: seconds(f.position),
                controls: e.controls,
                minimums: [
                    e.globals.scalar(0x65c).map_err(|e| e.to_string())?,
                    e.globals.scalar(0x660).map_err(|e| e.to_string())?,
                ],
            },
            true,
            Precision::Double53,
        )?;
        // Actual first ANM boundary: motion, placement, affine and model rates
        // have completed; gravity and control force have not run yet.
        let mut before_tail = p;
        let direction = crate::facing_direction::direction(
            f.angles[0],
            self.view.look_pitch,
            Precision::Double53,
        )?;
        for (index, component) in direction.into_iter().enumerate() {
            put(&mut body, index * 4, component.to_bits());
        }
        before_tail[0] = c.physics_flags;
        before_tail[5] = motion.stored_dt.to_bits();
        before_tail[6..9].copy_from_slice(&[
            motion.force[0].to_bits(),
            motion.force[1].to_bits(),
            placed.force_z.to_bits(),
        ]);
        before_tail[9] = model.world_speed.to_bits();
        before_tail[11] = motion.force_time_limit.to_bits();
        for (i, value) in before_tail.into_iter().enumerate() {
            put(&mut body, 0x84 + i * 4, value);
        }
        self.view = View {
            number: e.number,
            local_seconds: seconds(f.position) as f32,
            velocity: c.velocity,
            previous_velocity: motion.previous_velocity,
            model_rates: model.model,
            previous_model_rates: self.view.previous_model_rates,
            angular_rate: self.view.angular_rate,
            look_pitch: self.view.look_pitch,
            commands: self.view.commands,
            floor: c.floor,
            plane: c.plane,
            radius: c.radius,
            hit: c.hit,
            support: c.support,
            support_part: c.support_part,
            support_triangle: c.support_triangle,
            affine: Some(affine),
        };
        tail(&mut f, &body, &self.view)?;
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
                project_contact: e.controls == [0, 0] && self.jet_buttons & 0x400 == 0,
            },
            Precision::Double53,
        )?;
        let params = crate::mecc_control_state::resolve(
            crate::mecc_control_state::Input {
                position_z: f.position[2],
                reference_height: f32::from_bits(f.configuration[17]),
                range: f32::from_bits(f.configuration[22]),
                scale: f.scale,
                attenuation: f32::from_bits(f.configuration[3]),
                gain: f32::from_bits(f.configuration[0]),
                speed: f32::from_bits(f.configuration[1]),
                slope_power: f32::from_bits(f.configuration[4]),
                flags: f.object_flags,
            },
            Precision::Double53,
        )?;
        let desired = crate::mecc_control_target::apply(
            crate::mecc_control_target::Input {
                matrix: affine.matrix,
                scale: f.scale,
                controls: e.controls,
                speeds: params.speeds,
                normal: c.plane[..3].try_into().unwrap(),
                physics_flags: c.physics_flags,
                eligible: c.support != 0,
                override_contact: params.override_contact,
            },
            Precision::Double53,
        )?;
        let (force, cap) = if let Some(desired_velocity) = desired {
            let control = crate::mecc_control_force::apply(
                crate::mecc_control_force::Input {
                    desired_velocity,
                    velocity: c.velocity,
                    force: gravity.force,
                    gain: params.gain,
                    slope_power: params.slope_power,
                    mass: pf(2),
                    force_time_limit: gravity.force_time_limit,
                },
                Precision::Double53,
            )?;
            (control.force, control.force_time_limit)
        } else {
            (gravity.force, gravity.force_time_limit)
        };
        p[0] = c.physics_flags;
        p[5] = motion.stored_dt.to_bits();
        p[9] = model.world_speed.to_bits();
        p[11] = cap.to_bits();
        for (i, v) in force.into_iter().enumerate() {
            p[6 + i] = v.to_bits();
        }
        for (i, v) in p.into_iter().enumerate() {
            put(&mut body, 0x84 + i * 4, v);
        }
        self.view = View {
            number: e.number,
            local_seconds: seconds(f.position) as f32,
            velocity: c.velocity,
            previous_velocity: motion.previous_velocity,
            model_rates: model.model,
            previous_model_rates: self.view.previous_model_rates,
            angular_rate: self.view.angular_rate,
            look_pitch: self.view.look_pitch,
            commands: self.view.commands,
            floor: c.floor,
            plane: c.plane,
            radius: c.radius,
            hit: c.hit,
            support: c.support,
            support_part: c.support_part,
            support_triangle: c.support_triangle,
            affine: Some(affine),
        };
        Ok(Output {
            fields: f,
            payload: body,
        })
    }
    pub(crate) fn project_channels(
        &mut self,
        fields: &Fields,
        channels: crate::animation_channels::Channels,
        globals: &GlobalData,
    ) -> Result<ChannelProjection, String> {
        if self.interleaved || self.failure.is_some()
            || self.channel_number.checked_add(1) != Some(self.view.number)
            || self
                .channel_state
                .is_some_and(|previous| previous != channels)
            || fields.action != 0
            || fields.control_flags != 0
            // Source-template dirty bit31 is not consumed by this numeric
            // projection. Its real primary producer/cleanup remains separate.
            || fields.response_flags & !0x8000_3000 != 0
            || channels.iter().any(|channel| channel[6] != 0 || channel[7] != 0)
        {
            return Err(format!(
                "normal channel projection requires contiguous owned physical state: clock={} physical={} action={:x} control={:x} response={:x} changed={}",
                self.channel_number,
                self.view.number,
                fields.action,
                fields.control_flags,
                fields.response_flags,
                self.channel_state
                    .is_some_and(|previous| previous != channels)
            ));
        }
        let mut durations = vec![None; 256];
        for (&slot, descriptor) in &self.resources.specification.animations {
            durations[usize::from(slot)] = Some(descriptor.template.duration);
        }
        let mut first = channels;
        let tick = crate::animation_channels::advance(
            &mut first,
            &durations,
            self.view.model_rates,
            self.view.local_seconds,
            Precision::Double53,
        )?;
        if tick.missing != 0 {
            return Err("normal channel descriptor absent".into());
        }
        let first_processed = tick.processed != 0;
        let selection = crate::mecc_locomotion::select(
            crate::mecc_locomotion::Input {
                model: self.view.model_rates,
                position_z: fields.position[2],
                floor: self.view.floor,
                center_offset: f32::from_bits(fields.configuration[17]),
                scale: fields.scale,
                supported: self.view.support != 0,
            },
            crate::mecc_locomotion::Parameters::from_globals(globals)
                .map_err(|error| error.to_string())?,
            Precision::Double53,
        )?;
        let selected = (first[0][0] != u32::from(selection.clip)).then_some(
            crate::animation_channels::Request {
                slot: selection.clip,
                flags: selection.flags,
                speed_source: selection.speed_source,
                reference_speed: selection.reference_speed,
                blend_seconds: f32::from_bits(0x3e2a_aaab),
            },
        );
        let mut second = first;
        if let Some(request) = selected {
            crate::animation_channels::select_primary(
                &mut second,
                request,
                crate::animation_channels::Blending::Disabled,
            )?;
        }
        let tick = crate::animation_channels::advance(
            &mut second,
            &durations,
            self.view.model_rates,
            0.,
            Precision::Double53,
        )?;
        if tick.missing != 0 {
            return Err("selected normal channel descriptor absent".into());
        }
        self.channel_number = self.view.number;
        self.channel_state = Some(second);
        Ok(ChannelProjection {
            first,
            second,
            selected,
            first_references: None,
            second_references: None,
            first_stamp: None,
            second_stamp: None,
            first_primary_flags: None,
            first_processed,
            second_processed: tick.processed != 0,
        })
    }
    pub(crate) fn accept_interleaved(&mut self, projection: ChannelProjection) {
        self.channel_number = self.view.number;
        self.channel_state = Some(projection.second);
        self.event_number = self.view.number;
        self.channel_projection = Some(projection);
        self.interleaved = true;
    }
}
