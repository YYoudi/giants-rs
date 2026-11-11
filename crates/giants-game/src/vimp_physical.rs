//! Owned ordinary Vimp physical phase, ending before its animation selector.
//! The primary seals its time inputs; a successful physical phase is not a tick.
use crate::{
    actor_fields::Fields,
    arithmetic::Precision,
    vimp_wander::{put, scalar, word},
};

#[derive(Clone, Debug)]
pub struct View {
    pub velocity: [f32; 3],
    pub previous_velocity: [f32; 3],
    pub model_rates: [f32; 3],
    pub previous_model_rates: [f32; 3],
    pub floor: f32,
    pub plane: [f32; 4],
    pub affine: Option<crate::actor_affine::Cache>,
    pub physics_consumed: bool,
    pub animation_consumed: bool,
    pub ground_queries: u32,
    pub failure: Option<String>,
}
#[derive(Clone, Debug)]
pub(crate) struct Owned {
    pub view: View,
    seconds: f32,
    pub animation_seconds: f32,
    areas: Vec<crate::object_time::SlowArea>,
    pub partial: Option<(Fields, Vec<u8>)>,
}
impl Owned {
    pub(crate) fn new(
        tick: &crate::vimp_wander::Tick<'_>,
        position: [f32; 3],
        context: u32,
    ) -> Self {
        Self {
            view: View {
                velocity: [0.; 3],
                previous_velocity: [0.; 3],
                model_rates: [0.; 3],
                previous_model_rates: [0.; 3],
                floor: 0.,
                plane: [0.; 4],
                affine: None,
                physics_consumed: false,
                animation_consumed: false,
                ground_queries: 0,
                failure: None,
            },
            seconds: tick.global_delta,
            animation_seconds: crate::object_time::seconds(
                tick.global_delta,
                Some(crate::object_time::Sample {
                    owner: context,
                    position,
                }),
                None,
                tick.slow_areas,
            ) as f32,
            areas: tick.slow_areas.to_vec(),
            partial: None,
        }
    }
    pub(crate) fn rearm(
        &mut self,
        tick: &crate::vimp_wander::Tick<'_>,
        position: [f32; 3],
        context: u32,
    ) {
        self.seconds = tick.global_delta;
        self.animation_seconds = crate::object_time::seconds(
            tick.global_delta,
            Some(crate::object_time::Sample {
                owner: context,
                position,
            }),
            None,
            tick.slow_areas,
        ) as f32;
        self.areas = tick.slow_areas.to_vec();
        self.view.physics_consumed = false;
        self.view.animation_consumed = false;
        self.view.ground_queries = 0;
        self.partial = None;
    }
    pub(crate) fn advance(
        &mut self,
        mut fields: Fields,
        mut body: Vec<u8>,
        vimp: &mut crate::vimp_actor::Cold,
        globals: &giants_formats::global_data::GlobalData,
        ground: impl FnMut([f32; 2]) -> Result<crate::physical_ground::Ground, &'static str>,
    ) -> Result<(Fields, Vec<u8>), String> {
        if self.view.physics_consumed
            || self.view.failure.is_some()
            || !vimp.wander_pending
            || !vimp.initialized
            || !matches!(vimp.mode, 1..=4 | 7)
            || fields.class != 8
            || body.len() != 216
        {
            return Err("Vimp physical phase absent, consumed or quarantined".into());
        }
        let precision = Precision::Double53;
        let seconds = |position| {
            crate::object_time::seconds(
                self.seconds,
                Some(crate::object_time::Sample {
                    owner: vimp.context,
                    position,
                }),
                None,
                &self.areas,
            )
        };
        let physics: [f32; 15] = std::array::from_fn(|i| scalar(&body, i * 4));
        let p = |i: usize| physics[i];
        let motion = crate::mecc_motion::integrate_with_time(
            crate::mecc_motion::Motion {
                position: fields.position,
                velocity: self.view.velocity,
                previous_velocity: self.view.previous_velocity,
                force: [p(6), p(7), p(8)],
                mass: p(2),
                speed_limit: p(10),
                force_time_limit: p(11),
                stored_dt: p(5),
            },
            seconds(fields.position),
            false,
            false,
            precision,
        )?;
        // Unlike the Mecc caller, this actual Vimp path has no9000-unit limiter.
        fields.position = motion.position;
        // Native sets40 here, before placement/ANM. The owned seal remains set.
        fields.object_flags |= 0x40;
        fields.configuration[14] |= 0x400;
        self.view.velocity = motion.velocity;
        self.view.previous_velocity = motion.previous_velocity;
        put(&mut body, 0x14, motion.stored_dt.to_bits());
        put(&mut body, 0x2c, motion.force_time_limit.to_bits());
        for (i, value) in motion.force.into_iter().enumerate() {
            put(&mut body, 0x18 + i * 4, value.to_bits());
        }
        fields.object_flags &= 0xf7e7_ffff;
        if fields.object_flags & 0x1000_0000 != 0 || fields.position != fields.current {
            fields.object_flags &= !0x1000_0000;
            vimp.contact_hit = 0;
            let flags = word(&body, 0) & 0xffff_ff00;
            put(&mut body, 0, flags);
        }
        self.partial = Some((fields.clone(), body.clone()));
        let placed = crate::player_world_collision::resolve_for_actor(
            1,
            crate::player_world_collision::Input {
                state: crate::player_mesh_contact::State {
                    current: fields.position,
                    previous: fields.current,
                    velocity: motion.velocity,
                    scale: fields.scale,
                    radius: vimp.radius,
                    height: f32::from_bits(fields.configuration[23]),
                    floor: self.view.floor,
                    plane: self.view.plane,
                    physics_flags: word(&body, 0),
                    hit: vimp.contact_hit,
                    support: 0,
                    support_part: 0,
                    support_triangle: 0,
                    restitution: p(3),
                    retention: p(4),
                },
                object_flags: fields.object_flags,
                collision_flags: fields.configuration[14],
                ground_offset: f32::from_bits(fields.configuration[17]),
                center_offset: f32::from_bits(fields.configuration[22]),
                gravity_mass: p(1),
                force_z: motion.force[2],
            },
            // The source400 gate skips the entire object walk, including radius
            // refresh. No empty-world assumption or substituted contact result.
            &[],
            ground,
            |_| Err("Vimp object walk unexpectedly entered"),
        )?;
        // Publish the real placement result before later profile/affine/model
        // failures. Re-arming history must never replace it with a guessed zero.
        vimp.contact_hit = placed.state.hit;
        if placed.mesh_responses != 0
            || placed.collision_events != 0
            || placed.state.hit != 0
            || placed.state.support != 0
        {
            return Err("Vimp physical object contact outside the acquired source gate".into());
        }
        let c = placed.state;
        fields.position = c.current;
        fields.current = c.previous;
        self.view.velocity = c.velocity;
        self.view.floor = c.floor;
        self.view.plane = c.plane;
        put(&mut body, 0, c.physics_flags);
        put(&mut body, 0x20, placed.force_z.to_bits());
        self.partial = Some((fields.clone(), body.clone()));
        let (flags, affine) = crate::actor_affine::update(
            crate::uncached_pose::Pose {
                position: fields.position,
                angles: fields.angles,
                scale: fields.scale,
            },
            placed.object_flags,
            self.view.affine,
            precision,
        )?;
        fields.object_flags = flags;
        self.view.affine = Some(affine);
        self.partial = Some((fields.clone(), body.clone()));
        let controls = [vimp.commands[2] as i8, vimp.commands[3] as i8];
        let model = crate::mecc_model_motion::derive(
            crate::mecc_model_motion::Input {
                current: fields.position,
                previous: fields.current,
                velocity: c.velocity,
                matrix: affine.matrix,
                scale: fields.scale,
                seconds: seconds(fields.position),
                controls,
                minimums: [globals.scalar(0x724).map_err(|e| e.to_string())?, 0.],
            },
            true,
            precision,
        )?;
        self.view.model_rates = model.model;
        put(&mut body, 0x24, model.world_speed.to_bits());
        self.partial = Some((fields.clone(), body.clone()));
        let gravity = crate::mecc_gravity::apply(
            crate::mecc_gravity::Input {
                force: [motion.force[0], motion.force[1], placed.force_z],
                velocity: c.velocity,
                normal: c.plane[..3].try_into().unwrap(),
                gravity_mass: p(1),
                mass: p(2),
                friction_limit: p(12),
                damping_xy: p(13),
                damping_z: p(14),
                force_time_limit: motion.force_time_limit,
                object_flags: fields.object_flags,
                physics_flags: c.physics_flags,
                project_contact: true,
            },
            precision,
        )?;
        for (i, value) in gravity.force.into_iter().enumerate() {
            put(&mut body, 0x18 + i * 4, value.to_bits());
        }
        put(&mut body, 0x2c, gravity.force_time_limit.to_bits());
        self.partial = Some((fields.clone(), body.clone()));
        let (force, cap) = if fields.object_flags & 0x8000_0000 == 0 {
            let desired_velocity = crate::mecc_control_target::apply(
                crate::mecc_control_target::Input {
                    matrix: affine.matrix,
                    scale: fields.scale,
                    controls,
                    speeds: [scalar(&body, 0x68); 2],
                    normal: c.plane[..3].try_into().unwrap(),
                    physics_flags: c.physics_flags,
                    eligible: false,
                    override_contact: true,
                },
                precision,
            )?
            .ok_or("Vimp forced target absent")?;
            let control = crate::mecc_control_force::apply(
                crate::mecc_control_force::Input {
                    desired_velocity,
                    velocity: c.velocity,
                    force: gravity.force,
                    gain: scalar(&body, 0x6c),
                    slope_power: scalar(&body, 0x70),
                    mass: p(2),
                    force_time_limit: gravity.force_time_limit,
                },
                precision,
            )?;
            (control.force, control.force_time_limit)
        } else {
            (gravity.force, gravity.force_time_limit)
        };
        put(&mut body, 0, c.physics_flags);
        put(&mut body, 0x14, motion.stored_dt.to_bits());
        put(&mut body, 0x24, model.world_speed.to_bits());
        put(&mut body, 0x2c, cap.to_bits());
        for (i, value) in force.into_iter().enumerate() {
            put(&mut body, 0x18 + i * 4, value.to_bits());
        }
        self.view = View {
            velocity: c.velocity,
            previous_velocity: motion.previous_velocity,
            model_rates: model.model,
            previous_model_rates: self.view.previous_model_rates,
            floor: c.floor,
            plane: c.plane,
            affine: Some(affine),
            physics_consumed: true,
            animation_consumed: false,
            ground_queries: placed.ground_queries,
            failure: None,
        };
        self.partial = None;
        Ok((fields, body))
    }
}
