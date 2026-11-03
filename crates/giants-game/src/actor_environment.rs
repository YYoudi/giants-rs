//! Exact-reference ordinary environment composition over prepared owned geometry.
//! Alternative actor states and ripples remain outside this composition.
use crate::{
    actor_finalization, actor_terrain,
    arithmetic::Precision,
    collider_copy::Fields,
    ground_contact, mecc_mesh_contact,
    mecc_mesh_dispatch::{self, MeshObject},
    physical_ground::{self, Ground},
    platform_queries::PlatformSample,
};

pub struct Collider {
    pub mesh: MeshObject,
    pub class: u32,
    pub state: u32,
    pub volume: Option<crate::actor_volume::Volume>,
}
#[derive(Clone, Copy, Debug)]
pub struct Physics {
    pub gravity_mass: f32,
    pub restitution: f32,
    pub retention: f32,
    pub force_z: f32,
    pub flags: u32,
}
pub struct Input {
    pub actor: u32,
    /// Actor dispatch state, distinct from class and context identity.
    pub state: u32,
    pub fields: Fields,
    pub slope_parameter: f32,
    pub physics: Option<Physics>,
    pub skip_object_responses: bool,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Statistics {
    pub passes: u32,
    pub mesh_responses: u32,
    pub fallback_xy: bool,
    pub ground_queries: u32,
    pub terrain_responses: u32,
    pub junctions: u32,
    pub platform_queries: u32,
    pub time_queries: u32,
    pub segment_responses: u32,
    pub volume_responses: u32,
}
pub struct Output {
    pub fields: Fields,
    pub physics: Option<Physics>,
    pub statistics: Statistics,
}
pub fn resolve(
    input: Input,
    objects: &[Collider],
    mut ground: impl FnMut([f32; 2]) -> Result<Ground, &'static str>,
    mut platform: impl FnMut([f32; 3]) -> Result<PlatformSample, &'static str>,
    mut seconds: impl FnMut([f32; 3]) -> f64,
) -> Result<Output, &'static str> {
    if input.actor == 0 || input.fields.physical != input.physics.is_some() {
        return Err("actor environment identity/physics mismatch");
    }
    let mut out = Output {
        fields: input.fields,
        physics: input.physics,
        statistics: Statistics::default(),
    };
    let forced = out.fields.object_flags & 0x10000000;
    if forced == 0 && out.fields.current == out.fields.previous {
        return Ok(out);
    }
    out.fields.object_flags &= !0x10000000;
    out.fields.hit = 0;
    out.fields.support = 0;
    let initial = ground([out.fields.previous[0], out.fields.previous[1]])?;
    out.statistics.ground_queries = 1;
    out.fields.floor = initial.height;
    out.fields.plane = initial.plane;
    let has_specification = out.fields.model != 0;
    if has_specification && out.fields.collision_flags & 0x200 != 0 {
        return Ok(out);
    }
    if has_specification && [1, 15].contains(&input.state) {
        return Err("alternative actor environment dispatch required");
    }
    let selected = objects
        .iter()
        .filter(|o| {
            !(out.fields.class == 3 && o.class == 3
                || out.fields.class == 572 && o.state == 2
                || out.fields.class == 746 && o.state == 1)
        })
        .map(|o| &o.mesh)
        .collect::<Vec<_>>();
    out.fields.object_flags |= forced;
    let mut volume_responses = 0;
    let mesh = mecc_mesh_dispatch::resolve_with_response(
        input.actor,
        has_specification && !input.skip_object_responses,
        mesh_state(out.fields),
        mecc_mesh_dispatch::Ground {
            height: initial.height,
            plane: initial.plane,
        },
        &selected,
        Precision::Double53,
        |state, object| {
            if object.fields.collision_flags & 0x1008 != 0 {
                return mecc_mesh_contact::resolve(
                    state,
                    object.fields.id,
                    &object.triangles,
                    Precision::Double53,
                )
                .map(Into::into);
            }
            let collider = objects
                .iter()
                .find(|o| o.mesh.fields.id == object.fields.id)
                .ok_or("volume identity absent")?;
            let volume = collider
                .volume
                .ok_or("owned radial or box fields required")?;
            if matches!(volume, crate::actor_volume::Volume::Boxed(_))
                != (object.fields.collision_flags & 0x80000 != 0)
            {
                return Err("actor volume family mismatch");
            }
            volume_responses += 1;
            let response = crate::actor_volume::resolve(state, object.fields.id, volume)?;
            let stop_pass = matches!(volume, crate::actor_volume::Volume::Radial(_))
                && state.collision_flags & 0x8000 != 0
                && response.changed;
            Ok(mecc_mesh_dispatch::DispatchResponse {
                response,
                preserve_retry: true,
                stop_pass,
            })
        },
    )?;
    write_mesh(&mut out.fields, mesh.state);
    out.statistics.passes = mesh.passes;
    out.statistics.mesh_responses = mesh.responses - volume_responses;
    out.statistics.volume_responses = volume_responses;
    out.statistics.fallback_xy = mesh.fallback_xy;
    if let Some(mut physics) = out.physics {
        let f = &mut out.fields;
        let result = physical_ground::resolve(
            physical_ground::Input {
                position: f.current,
                previous: f.previous,
                velocity: f.velocity,
                scale: f.scale,
                ground_offset: f.ground_offset,
                center_offset: f.center_offset,
                gravity_mass: physics.gravity_mass,
                restitution: physics.restitution,
                retention: physics.retention,
                force_z: physics.force_z,
                flags: physics.flags,
                floor: f.floor,
                plane: f.plane,
            },
            Precision::Double53,
            &mut ground,
        )?;
        f.current = result.position;
        f.previous = result.previous;
        f.velocity = result.velocity;
        f.floor = result.floor;
        f.plane = result.plane;
        physics.flags = result.flags;
        physics.force_z = result.force_z;
        out.physics = Some(physics);
        out.statistics.ground_queries += result.queries;
        out.statistics.terrain_responses = result.terrain_responses;
        out.statistics.junctions = result.junctions;
    } else {
        let result = actor_terrain::resolve(
            out.fields,
            input.slope_parameter,
            initial,
            &mut ground,
            &mut seconds,
        )?;
        out.fields = result.fields;
        out.statistics.ground_queries += result.queries;
        out.statistics.time_queries = result.time_queries;
        out.statistics.segment_responses = result.segment_responses;
    }
    let surface = platform(out.fields.current)?;
    out.statistics.platform_queries = 1;
    let f = &mut out.fields;
    let contact = ground_contact::resolve(
        ground_contact::Input {
            z: f.current[2],
            vertical_velocity: f.model_vertical_speed,
            flags: f.object_flags,
            lock_z: f.physical,
            ground: f.floor,
            plane: f.plane,
            vertical_offset: f.ground_offset,
            platform: surface.object.map(|_| surface.height),
        },
        Precision::Double53,
    )?;
    f.current[2] = contact.z;
    f.model_vertical_speed = contact.vertical_velocity;
    f.object_flags = contact.flags;
    f.floor = contact.ground;
    f.plane = contact.plane;
    Ok(out)
}
fn mesh_state(f: Fields) -> mecc_mesh_contact::State {
    mecc_mesh_contact::State {
        current: f.current,
        previous: f.previous,
        radius: f.radius,
        scale: f.scale,
        center_offset: f.center_offset,
        height: f.height,
        ground_offset: f.ground_offset,
        floor: f.floor,
        plane: f.plane,
        model_vertical_speed: f.model_vertical_speed,
        world_vertical_speed: f.velocity[2],
        object_flags: f.object_flags,
        response_flags: f.response_flags,
        collision_flags: f.collision_flags,
        physical: f.physical,
        hit: f.hit,
    }
}
fn write_mesh(f: &mut Fields, s: mecc_mesh_contact::State) {
    f.current = s.current;
    f.previous = s.previous;
    f.floor = s.floor;
    f.plane = s.plane;
    f.model_vertical_speed = s.model_vertical_speed;
    f.velocity[2] = s.world_vertical_speed;
    f.object_flags = s.object_flags;
    f.response_flags = s.response_flags;
    f.hit = s.hit;
}

/// Real environment provider for the finalizer. Geometry, time, platform and
/// footprint ownership are explicit; this does not run the mission scheduler.
pub struct FinalizationServices<'a, G, P, T, R> {
    pub actor: u32,
    pub state: u32,
    pub slope_parameter: f32,
    pub physics: Option<Physics>,
    pub skip_object_responses: bool,
    pub objects: &'a [Collider],
    pub ground: G,
    pub platform: P,
    pub seconds: T,
    pub release: R,
    pub statistics: Option<Statistics>,
}
impl<G, P, T, R> actor_finalization::Services for FinalizationServices<'_, G, P, T, R>
where
    G: FnMut([f32; 2]) -> Result<Ground, &'static str>,
    P: FnMut([f32; 3]) -> Result<PlatformSample, &'static str>,
    T: FnMut([f32; 3]) -> f64,
    R: FnMut(Option<u32>) -> Result<(), String>,
{
    fn release_footprint(&mut self, identity: Option<u32>) -> Result<(), String> {
        (self.release)(identity)
    }
    fn environment(&mut self, state: &mut actor_finalization::State) -> Result<(), String> {
        self.statistics = None;
        let result = resolve(
            Input {
                actor: self.actor,
                state: self.state,
                fields: state.fields,
                slope_parameter: self.slope_parameter,
                physics: self.physics,
                skip_object_responses: self.skip_object_responses,
            },
            self.objects,
            &mut self.ground,
            &mut self.platform,
            &mut self.seconds,
        )?;
        state.fields = result.fields;
        self.physics = result.physics;
        self.statistics = Some(result.statistics);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ground() -> Ground {
        Ground {
            height: 0.,
            plane: [0., 0., 1., 0.],
            key: [0, 1],
        }
    }
    fn no_platform(_: [f32; 3]) -> Result<PlatformSample, &'static str> {
        Ok(PlatformSample {
            height: -61.1,
            object: None,
        })
    }
    fn input() -> Input {
        let mut f = Fields::zeroed();
        f.class = 73;
        f.model = 1;
        f.scale = 1.;
        f.radius = 1.;
        f.center_offset = 5.;
        f.height = 10.;
        f.current = [-1., 0., 1.];
        f.previous = [5., 0., 1.];
        Input {
            actor: 7,
            state: 23,
            fields: f,
            slope_parameter: 0.,
            physics: None,
            skip_object_responses: false,
        }
    }
    #[test]
    fn unsupported_dispatch_and_physics_are_refused() {
        for state in [1, 15] {
            let mut i = input();
            i.state = state;
            assert_eq!(
                resolve(i, &[], |_| Ok(ground()), no_platform, |_| 0.).err(),
                Some("alternative actor environment dispatch required")
            );
        }
        let mut i = input();
        i.fields.physical = true;
        assert_eq!(
            resolve(
                i,
                &[],
                |_| panic!("must preflight ownership"),
                no_platform,
                |_| 0.
            )
            .err(),
            Some("actor environment identity/physics mismatch")
        );
    }
    #[test]
    fn personality_exit_is_after_initial_query_and_resets() {
        let mut i = input();
        i.state = 1;
        i.fields.collision_flags = 0x200;
        i.fields.hit = 123;
        i.fields.support = 234;
        let out = resolve(
            i,
            &[],
            |_| Ok(ground()),
            |_| panic!("native early exit"),
            |_| 0.,
        )
        .unwrap();
        assert_eq!(out.statistics.ground_queries, 1);
        assert_eq!((out.fields.hit, out.fields.support), (0, 0));
        assert_eq!(out.statistics.platform_queries, 0);
    }
    #[test]
    fn skipped_objects_still_allow_platform_takeover() {
        let mut i = input();
        i.skip_object_responses = true;
        i.fields.current = [1., 0., 2.05];
        i.fields.previous = [0., 0., 3.];
        let out = resolve(
            i,
            &[],
            |_| Ok(ground()),
            |_| {
                Ok(PlatformSample {
                    height: 2.,
                    object: Some(9),
                })
            },
            |_| 0.,
        )
        .unwrap();
        assert_eq!(out.statistics.passes, 0);
        assert_eq!(out.statistics.platform_queries, 1);
        assert_eq!(out.fields.current[2], 2.);
        assert_eq!(out.fields.floor, 2.);
        assert_eq!(out.fields.support, 0);
        assert_ne!(out.fields.object_flags & 0x80000, 0);
    }
    #[test]
    fn native_class_exclusions_precede_unsupported_object_responses() {
        let object = Collider {
            class: 3,
            state: 2,
            volume: None,
            mesh: MeshObject {
                fields: crate::mecc_collision_candidates::Object {
                    id: 9,
                    object_flags: 0,
                    collision_flags: 0,
                    has_model: true,
                    parent: 0,
                    bounds: crate::volume_queries::Bounds {
                        min: [-100.; 3],
                        max: [100.; 3],
                    },
                },
                triangles: vec![],
            },
        };
        for class in [3, 572] {
            let mut i = input();
            i.fields.class = class;
            let out = resolve(
                i,
                std::slice::from_ref(&object),
                |_| Ok(ground()),
                no_platform,
                |_| 0.,
            )
            .unwrap();
            assert_eq!(out.statistics.mesh_responses, 0);
        }
        assert_eq!(
            resolve(input(), &[object], |_| Ok(ground()), no_platform, |_| 0.).err(),
            Some("owned radial or box fields required")
        );
    }
    #[test]
    fn finalization_consumes_real_ground_then_produces_current_affine() {
        let mut fields = input().fields;
        fields.current = [2., 0., -1.];
        fields.previous = [0., 0., 1.];
        let state = actor_finalization::State {
            fields,
            previous_physical_velocity: [0.; 3],
            friction: 1.,
            parent_present: false,
            footprint: Some(17),
            affine: None,
            physical_argument_word: None,
        };
        let mut released = Vec::new();
        let mut services = FinalizationServices {
            actor: 7,
            state: 23,
            slope_parameter: 0.,
            physics: None,
            skip_object_responses: false,
            objects: &[],
            ground: |_| Ok(ground()),
            platform: no_platform,
            seconds: |_| 0.1,
            release: |id| {
                released.push(id);
                Ok(())
            },
            statistics: None,
        };
        let out = actor_finalization::finalize(
            state,
            actor_finalization::Time {
                global_seconds: 0.1,
                context: 1,
                areas: &[],
            },
            &mut services,
        )
        .unwrap();
        assert_eq!(out.fields.current, [2., 0., 0.]);
        assert_eq!(out.footprint, None);
        assert_eq!(services.statistics.unwrap().ground_queries, 3);
        let matrix = out.affine.unwrap().matrix;
        assert_eq!([matrix[3], matrix[7], matrix[11]], out.fields.current);
        assert_eq!(released, vec![Some(17)]);
    }
    #[test]
    fn failed_environment_preserves_consumed_finalizer_phase() {
        let fields = input().fields;
        let state = actor_finalization::State {
            fields,
            previous_physical_velocity: [0.; 3],
            friction: 1.,
            parent_present: false,
            footprint: Some(17),
            affine: None,
            physical_argument_word: None,
        };
        let mut releases = 0;
        let mut services = FinalizationServices {
            actor: 7,
            state: 23,
            slope_parameter: 0.,
            physics: None,
            skip_object_responses: false,
            objects: &[],
            ground: |_| Err("owned terrain unavailable"),
            platform: no_platform,
            seconds: |_| 0.1,
            release: |_| {
                releases += 1;
                Ok(())
            },
            statistics: Some(Statistics::default()),
        };
        let error = actor_finalization::finalize(
            state,
            actor_finalization::Time {
                global_seconds: 0.1,
                context: 1,
                areas: &[],
            },
            &mut services,
        )
        .unwrap_err();
        assert!(error.phase_started);
        assert_eq!(error.state.footprint, None);
        assert!(error.state.affine.is_none());
        assert!(services.statistics.is_none());
        assert_eq!(releases, 1);
    }
    fn volume_objects() -> Vec<Collider> {
        let fields = |id, flags| crate::mecc_collision_candidates::Object {
            id,
            object_flags: 0x20,
            collision_flags: flags,
            has_model: true,
            parent: 0,
            bounds: crate::volume_queries::Bounds {
                min: [-100.; 3],
                max: [100.; 3],
            },
        };
        let identity = std::array::from_fn(|i| f32::from(i / 4 == i % 4));
        vec![
            Collider {
                class: 1000,
                state: 23,
                mesh: MeshObject {
                    fields: fields(9, 0),
                    triangles: vec![],
                },
                volume: Some(crate::actor_volume::Volume::Radial(
                    crate::actor_volume::Radial {
                        position: [0.; 3],
                        radius: 2.,
                        scale: 1.,
                        height: 10.,
                    },
                )),
            },
            Collider {
                class: 1000,
                state: 23,
                mesh: MeshObject {
                    fields: fields(10, 0x80000),
                    triangles: vec![],
                },
                volume: Some(crate::actor_volume::Volume::Boxed(
                    crate::actor_volume::Boxed {
                        pose: crate::contact_pose::Cached::new(0x20, 1, identity).unwrap(),
                        scale: 1.,
                        bounds: crate::volume_queries::Bounds {
                            min: [-1.; 3],
                            max: [1.; 3],
                        },
                    },
                )),
            },
        ]
    }
    #[test]
    fn special_radial_rewind_ends_the_candidate_list() {
        let mut i = input();
        i.fields.collision_flags = 0x8000;
        let previous = i.fields.previous;
        let out = resolve(i, &volume_objects(), |_| Ok(ground()), no_platform, |_| 0.).unwrap();
        assert_eq!(out.statistics.volume_responses, 1);
        assert!(out.statistics.fallback_xy);
        assert_eq!(out.fields.current, previous);
    }
    #[test]
    fn volume_miss_retains_retry_from_previous_candidate() {
        let out = resolve(
            input(),
            &volume_objects(),
            |_| Ok(ground()),
            no_platform,
            |_| 0.,
        )
        .unwrap();
        assert_eq!(out.statistics.passes, 2);
        assert_eq!(out.statistics.volume_responses, 4);
        assert_eq!(out.statistics.mesh_responses, 0);
    }
}
