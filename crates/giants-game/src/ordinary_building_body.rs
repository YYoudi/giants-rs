//! Owned fresh pub movement, ground finalization and current rigid collision.
//! Prototype PS2 `playerobj_normal_logic`, `object_init_usage`,
//! `object_initframe_location` and `object_bbox_to_world_bbox` remain unknown
//! attribution search routes. `object_apply_location` retains the separately
//! documented probable relationship. This phase owner does not acknowledge a
//! complete world update, animation callback, footprint or mission readiness.
use crate::{
    actor_affine::{self, Cache},
    actor_common_motion as motion,
    actor_fields::Fields,
    actor_finalization,
    arithmetic::Precision,
    collider_init::{self, GeometryResources, SourceTransform},
    collision_world::SourcePose,
    resident_motion::Time,
    scene_resources::SceneResources,
};
use std::sync::Arc;
#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct Resources {
    radius: giants_formats::world_bounds::Resolved,
    mesh: Arc<giants_formats::gbs::Mesh>,
}
impl Resources {
    pub fn load(source: &SceneResources) -> Result<Self, String> {
        let binding = source
            .classes
            .models
            .first_selection(53)
            .ok_or("pub specification absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        if !binding.selection.animations.is_empty() {
            return Err("pub rigid body requires its acquired unanimated profile".into());
        }
        let selected = source.collision_model_mesh(53).map_err(|e| e.to_string())?;
        let (name, mesh) = selected.geometry.ok_or("pub collision model absent")?;
        if source
            .archives
            .members
            .contains_key(&format!("{}.skn", name.to_ascii_lowercase()))
        {
            return Err("pub collision requires a separate skin/pose owner".into());
        }
        Ok(Self {
            radius: source
                .collision_resource_bounds(53, Some(1))
                .map_err(|e| e.to_string())?,
            mesh: Arc::new(mesh),
        })
    }
}

#[derive(Debug)]
pub struct Body {
    identity: u32,
    initial_fields: crate::collider_copy::Fields,
    finalization: actor_finalization::State,
    rates: [f32; 4],
    previous_rates: [f32; 4],
    look_pitch: f32,
    resources: Resources,
    stamp: Vec<u32>,
    steps: u64,
    failure: Option<String>,
}
fn stamp(h: &Fields) -> Vec<u32> {
    [
        h.class as u32,
        h.dispatch_context,
        h.object_flags,
        h.response_flags,
        h.control_flags,
        h.action,
        h.scale.to_bits(),
    ]
    .into_iter()
    .chain(h.configuration)
    .chain(
        h.position
            .into_iter()
            .chain(h.current)
            .chain(h.previous)
            .chain(h.angles)
            .map(f32::to_bits),
    )
    .chain(h.channels.into_iter().flatten())
    .collect()
}
impl Body {
    /// Only a fresh acquired class53/state7 constructor supplies the zero rates,
    /// commands, parent, physics, impulse history and footprint used here.
    /// Original dirty-pool controls establish those zeros; this is never a reset
    /// operation for an actor whose callbacks have already started.
    pub(crate) fn initialize(
        identity: u32,
        mut header: Fields,
        resources: Resources,
        cache: Option<Cache>,
        ground: impl FnMut([f32; 3]) -> Result<(f32, [f32; 4]), &'static str>,
    ) -> Result<(Self, Fields), String> {
        if identity == 0
            || header.class != 53
            || header.configuration[20] != 7
            || header.response_flags & 0x8000_0000 == 0
            || header.object_flags & 0x1c000 != 0
            || header.channels[0][0] != 254
            || (header.object_flags & 0x20 != 0) != cache.is_some()
        {
            return Err("pub body requires fresh acquired unparented rigid construction".into());
        }
        let c = header.configuration;
        let mut fields = crate::collider_copy::Fields::zeroed();
        fields.class = 53;
        fields.model = identity;
        fields.current = header.position;
        fields.previous = header.current;
        fields.angles = header.angles;
        fields.scale = header.scale;
        fields.object_flags = header.object_flags;
        fields.response_flags = header.response_flags;
        fields.collision_flags = c[14];
        fields.ground_offset = f32::from_bits(c[17]);
        fields.center_offset = f32::from_bits(c[22]);
        fields.height = f32::from_bits(c[23]);
        fields.packed_parameters = c[25];
        fields.radius_limit = f32::from_bits(c[26]);
        fields.radius_override = f32::from_bits(c[27]);
        fields.animation_slot = 254;
        let output = collider_init::initialize_geometry(
            collider_init::Instance {
                fields,
                state: 7,
                usage: [c[15], c[16]],
                footprint: 0,
            },
            GeometryResources {
                radius: resources.radius.clone(),
                current_slot: 254,
                current_auxiliary: None,
            },
            cache.map_or(SourceTransform::Uncached, |c| {
                SourceTransform::Cached(c.matrix[..12].try_into().unwrap())
            }),
            Precision::Double53,
            ground,
        )?;
        if !output.initialized {
            return Err("pub collision initialization was not consumed".into());
        }
        let initial_fields = output.instance.fields;
        header.object_flags = initial_fields.object_flags;
        header.response_flags = initial_fields.response_flags;
        header.configuration[14] = initial_fields.collision_flags;
        header.configuration[15..17].copy_from_slice(&output.instance.usage);
        let (active, affine) = actor_affine::update(
            crate::uncached_pose::Pose {
                position: header.position,
                angles: header.angles,
                scale: header.scale,
            },
            header.object_flags,
            cache,
            Precision::Double53,
        )?;
        header.object_flags = active;
        let mut current = initial_fields;
        current.object_flags = active;
        let body = Self {
            identity,
            initial_fields,
            resources,
            finalization: actor_finalization::State {
                fields: current,
                previous_physical_velocity: [0.; 3],
                friction: f32::from_bits(c[28]),
                parent_present: false,
                footprint: None,
                affine: Some(affine),
                physical_argument_word: None,
            },
            rates: [0.; 4],
            previous_rates: [0.; 4],
            look_pitch: 0.,
            stamp: stamp(&header),
            steps: 0,
            failure: None,
        };
        Ok((body, header))
    }
    pub fn initialized_fields(&self) -> crate::collider_copy::Fields {
        self.initial_fields
    }
    pub fn collision_identity(&self) -> u32 {
        self.identity
    }
    pub fn fields(&self) -> crate::collider_copy::Fields {
        self.finalization.fields
    }
    pub fn affine(&self) -> Result<Cache, String> {
        if self.failure.is_some() {
            return Err("pub affine phase failed".into());
        }
        self.finalization
            .affine
            .ok_or_else(|| "pub affine was not produced".into())
    }
    pub fn rates(&self) -> [f32; 4] {
        self.rates
    }
    pub fn previous_rates(&self) -> [f32; 4] {
        self.previous_rates
    }
    pub fn physical_history(&self) -> [f32; 3] {
        self.finalization.previous_physical_velocity
    }
    pub fn steps(&self) -> u64 {
        self.steps
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub(crate) fn matches_header(&self, header: &Fields) -> bool {
        self.stamp == stamp(header)
    }

    /// Source zero-command pub primary, followed by the acquired finalizer.
    /// Actual object, terrain, platform and release services remain mandatory.
    /// Errors after history starts retain the partial state and lock replay.
    pub(crate) fn advance(
        &mut self,
        header: &mut Fields,
        time: Time<'_>,
        services: &mut impl actor_finalization::Services,
    ) -> Result<bool, String> {
        if self.failure.is_some() || self.stamp != stamp(header) || time.controlled_actor {
            return Err(
                "pub body requires its current owned phase; failure or foreign/stale header".into(),
            );
        }
        if !time.seconds.is_finite()
            || !time.control_factor.is_finite()
            || !time.running_factor.is_finite()
        {
            return Err("pub body time is not finite".into());
        }
        let result = self.advance_inner(header, time, services);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        self.stamp = stamp(header);
        result
    }
    fn advance_inner(
        &mut self,
        header: &mut Fields,
        time: Time<'_>,
        services: &mut impl actor_finalization::Services,
    ) -> Result<bool, String> {
        header.advance_position_history();
        let entry = crate::actor_primary_gate::enter(
            header.object_flags,
            header.configuration[14],
            [0; 8],
            self.rates.map(f32::to_bits),
            self.previous_rates.map(f32::to_bits),
        );
        header.configuration[14] = entry.binding_flags;
        self.previous_rates = entry.previous_rates.map(f32::from_bits);
        if entry.invoke {
            let p = |i| f32::from_bits(header.configuration[i]);
            let next = motion::step(
                motion::State {
                    position: header.position,
                    heading: header.angles[0],
                    look_pitch: self.look_pitch,
                    rates: self.rates,
                    previous_rates: self.previous_rates,
                    active: header.object_flags,
                    personality: header.configuration[14],
                    state: 7,
                    context: header.dispatch_context,
                    parent_present: false,
                    controls: [0; 4],
                    target_heading: 0.,
                    target_position: [0.; 2],
                },
                &motion::Environment {
                    parameters: motion::Parameters {
                        acceleration: p(0),
                        speed: p(1),
                        braking: p(4),
                        attenuation: p(3),
                        turn_acceleration: p(5),
                        turn_speed: p(6),
                        turn_braking: p(7),
                        gravity: p(10),
                    },
                    global_seconds: time.seconds,
                    areas: time.areas,
                    control_factor: time.control_factor,
                    running_factor: time.running_factor,
                    skip_heading: time.skip_heading,
                    controlled_actor: false,
                },
            )?;
            header.position = next.position;
            header.angles[0] = next.heading;
            self.rates = next.rates;
            self.previous_rates = next.previous_rates;
            self.look_pitch = next.look_pitch;
        }
        let mut state = self.finalization.clone();
        state.fields.current = header.position;
        state.fields.previous = header.current;
        state.fields.angles = header.angles;
        state.fields.object_flags = header.object_flags;
        state.fields.response_flags = header.response_flags;
        state.fields.collision_flags = header.configuration[14];
        state.fields.model_vertical_speed = self.rates[0];
        let result = actor_finalization::finalize(
            state,
            actor_finalization::Time {
                global_seconds: time.seconds,
                context: header.dispatch_context,
                areas: time.areas,
            },
            services,
        );
        self.finalization = match result {
            Ok(state) => state,
            Err(failure) => {
                self.finalization = *failure.state;
                self.publish_header(header);
                return Err(failure.message);
            }
        };
        self.rates[0] = self.finalization.fields.model_vertical_speed;
        self.publish_header(header);
        self.finalization.fields.bounds = crate::collision_bounds::cached(
            Some(crate::collision_bounds::Models {
                primary: self.resources.radius.primary,
                auxiliary: None,
                slot: 254,
            }),
            self.affine()?.matrix[..12].try_into().unwrap(),
            Precision::Double53,
        )?;
        self.steps += 1;
        Ok(entry.invoke)
    }
    fn publish_header(&self, header: &mut Fields) {
        let f = self.finalization.fields;
        header.position = f.current;
        header.current = f.previous;
        header.angles = f.angles;
        header.object_flags = f.object_flags;
        header.response_flags = f.response_flags;
        header.configuration[14] = f.collision_flags;
    }
    /// Current native collision selector and ordered parts, independent of LOD.
    /// This prepares a shape; the caller still owns role/list admission.
    pub fn player_mesh(&self) -> Result<crate::player_world_collision::Object, String> {
        if self.failure.is_some() {
            return Err("pub collision phase failed".into());
        }
        let f = self.fields();
        let pose = SourcePose::from_matrix(self.affine()?.matrix)?;
        let triangles = crate::collision_geometry::world_triangles(
            &self.resources.mesh,
            pose,
            Precision::Double53,
        )?;
        let identities =
            self.resources
                .mesh
                .subobjects
                .iter()
                .enumerate()
                .flat_map(|(part, sub)| {
                    (0..sub.triangles.len()).map(move |ordinal| (part as u32, ordinal as u32 + 1))
                });
        Ok(crate::player_world_collision::Object {
            fields: crate::mecc_collision_candidates::Object {
                id: self.identity,
                object_flags: f.object_flags,
                collision_flags: f.collision_flags,
                has_model: true,
                parent: 0,
                bounds: crate::volume_queries::Bounds {
                    min: f.bounds[..3].try_into().unwrap(),
                    max: f.bounds[3..].try_into().unwrap(),
                },
            },
            response_flags: f.response_flags,
            triangles: triangles
                .into_iter()
                .zip(identities)
                .map(
                    |(vertices, (part, ordinal))| crate::player_mesh_contact::Triangle {
                        vertices,
                        part,
                        ordinal,
                    },
                )
                .collect(),
        })
    }
}
