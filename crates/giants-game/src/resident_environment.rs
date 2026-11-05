//! Owned late collision initialization for fresh constructed Ripper residents.
//! This is not behavior scheduling or a complete world/mission readiness signal.
use crate::{
    actor_affine::Cache,
    actor_fields,
    arithmetic::Precision,
    collider_copy::Fields,
    collider_init::{self, GeometryResources, Instance, SourceTransform},
    scene_resources::SceneResources,
    specification,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct Resources {
    radius: giants_formats::world_bounds::Resolved,
    animations: BTreeMap<u32, Option<[f32; 6]>>,
    missing: BTreeSet<u32>,
}
#[derive(Clone, Copy, Debug)]
pub struct ResourceBounds {
    pub primary: [f32; 6],
    pub radius_auxiliary: Option<[f32; 6]>,
    pub current_auxiliary: Option<[f32; 6]>,
}
impl Resources {
    pub fn load(source: &SceneResources, spec: &specification::Prepared) -> Result<Self, String> {
        let binding = source
            .classes
            .models
            .first_selection(spec.class)
            .ok_or("environment class binding absent")?
            .as_ref()
            .map_err(Clone::clone)?;
        if binding.class_catalog != spec.class_catalog
            || binding.spec_catalog != spec.specification_catalog
            || binding.spec_id != spec.specification
        {
            return Err("environment specification belongs to another resource context".into());
        }
        let radius = source
            .collision_resource_bounds(spec.class, Some(1))
            .map_err(|e| e.to_string())?;
        if radius.animation.is_some() && !spec.animations.contains_key(&1) {
            return Err("environment radius animation resource unavailable".into());
        }
        let mut animations = BTreeMap::new();
        let mut missing = BTreeSet::new();
        for declared in &spec.metadata.selection.animations {
            let slot = u32::from(declared.slot);
            if !spec.animations.contains_key(&declared.slot) {
                missing.insert(slot);
                continue;
            }
            animations.insert(
                slot,
                Some(source.world_bounds.animation_bounds(
                    spec.specification,
                    declared.slot,
                    radius.primary,
                )),
            );
        }
        Ok(Self {
            radius,
            animations,
            missing,
        })
    }
    fn auxiliary(&self, slot: u32) -> Result<Option<[f32; 6]>, &'static str> {
        if slot > 255 {
            return Err("environment animation slot outside acquired profile");
        }
        if slot == 254 {
            return Ok(None);
        }
        if self.missing.contains(&slot) {
            return Err("environment current animation resource unavailable");
        }
        Ok(self.animations.get(&slot).copied().flatten())
    }
    /// Source primary and independently selected radius/active wrappers, for
    /// diagnostics. These are acquired bounds, not inferred deformed extents.
    pub fn bounds(&self, slot: u32) -> Result<ResourceBounds, &'static str> {
        Ok(ResourceBounds {
            primary: self.radius.primary,
            radius_auxiliary: self.radius.animation,
            current_auxiliary: self.auxiliary(slot)?,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Prepared {
    initial_fields: Fields,
    fields: Fields,
    state: u32,
    usage: [u32; 2],
    identity: u32,
    resources: Resources,
    stamp: Vec<u32>,
    cache: Option<Cache>,
}
fn stamp(f: &actor_fields::Fields, slot: u32) -> Vec<u32> {
    let mut w = vec![
        f.class as u32,
        f.scale.to_bits(),
        f.object_flags & !0x20,
        f.response_flags & 0x80000000,
        slot,
    ];
    w.extend(
        f.position
            .into_iter()
            .chain(f.current)
            .chain(f.angles)
            .map(f32::to_bits),
    );
    w.extend([14, 15, 16, 17, 20, 22, 23, 25, 26, 27].map(|i| f.configuration[i]));
    w
}
/// Project fresh owned fields only; radius, ground and bounds are later phases.
pub(crate) fn fresh_instance(
    identity: u32,
    header: &actor_fields::Fields,
    channels: crate::animation_channels::Channels,
) -> Result<Instance, &'static str> {
    let c = header.configuration;
    let mut fields = Fields::zeroed();
    fields.class = header.class as u32;
    // Presence/ownership of this bound specification, not its source ID or address.
    fields.model = identity;
    fields.current = header.position;
    fields.previous = header.current;
    fields.scale = header.scale;
    fields.angles = header.angles;
    fields.object_flags = header.object_flags;
    fields.response_flags = header.response_flags;
    fields.collision_flags = c[14];
    fields.ground_offset = f32::from_bits(c[17]);
    fields.center_offset = f32::from_bits(c[22]);
    fields.height = f32::from_bits(c[23]);
    fields.packed_parameters = c[25];
    fields.radius_limit = f32::from_bits(c[26]);
    fields.radius_override = f32::from_bits(c[27]);
    fields.animation_slot = channels[0][0];
    // Do not silently turn an unprepared mesh candidate into an empty mesh.
    if fields.collision_flags & 0x1008 != 0 {
        return Err("resident environment mesh producer required");
    }
    Ok(Instance {
        fields,
        state: c[20],
        usage: [c[15], c[16]],
        footprint: 0,
    })
}
impl Prepared {
    /// Fresh constructor zero velocity/physics profile is acquired for these
    /// two classes only. Existing callbacks/parents are rejected by Session.
    pub(crate) fn initialize(
        identity: u32,
        mut header: actor_fields::Fields,
        channels: crate::animation_channels::Channels,
        resources: Resources,
        cache: Option<Cache>,
        ground: impl FnMut([f32; 3]) -> Result<(f32, [f32; 4]), &'static str>,
    ) -> Result<(Self, actor_fields::Fields), String> {
        if identity == 0
            || !matches!(header.class, 73 | 230)
            || header.configuration[20] != 23
            || header.response_flags & 0x80000000 == 0
        {
            return Err("environment initialization requires a fresh acquired Ripper".into());
        }
        let instance = fresh_instance(identity, &header, channels)?;
        let fields = instance.fields;
        let result = collider_init::initialize_geometry(
            instance,
            GeometryResources {
                radius: resources.radius.clone(),
                current_slot: fields.animation_slot,
                current_auxiliary: resources.auxiliary(fields.animation_slot)?,
            },
            cache.map_or(SourceTransform::Uncached, |c| {
                SourceTransform::Cached(c.matrix[..12].try_into().unwrap())
            }),
            Precision::Double53,
            ground,
        )?;
        if !result.initialized {
            return Err("fresh environment initialization was not executed".into());
        }
        let instance = result.instance;
        header.object_flags = instance.fields.object_flags;
        header.response_flags = instance.fields.response_flags;
        header.configuration[14] = instance.fields.collision_flags;
        header.configuration[15] = instance.usage[0];
        header.configuration[16] = instance.usage[1];
        let source_stamp = stamp(&header, fields.animation_slot);
        Ok((
            Self {
                initial_fields: instance.fields,
                fields: instance.fields,
                state: instance.state,
                usage: instance.usage,
                identity,
                resources,
                stamp: source_stamp,
                cache,
            },
            header,
        ))
    }
    pub(crate) fn validate(
        &self,
        header: &actor_fields::Fields,
        slot: u32,
        cache: Cache,
    ) -> Result<(), String> {
        if self.stamp != stamp(header, slot)
            || self.cache.is_some_and(|saved| {
                saved.flags != cache.flags
                    || saved.matrix.map(f32::to_bits) != cache.matrix.map(f32::to_bits)
            })
        {
            return Err("resident environment geometry is stale; explicit refresh required".into());
        }
        Ok(())
    }
    /// Retained late-initialization fields, not a live velocity/response history.
    pub fn initialized_fields(&self) -> Fields {
        self.initial_fields
    }
    /// Current geometry projection only. Floor/physics are retained initialization
    /// values, not a substitute for the actor's environment/finalization callback.
    pub fn geometry_fields(&self) -> Fields {
        self.fields
    }
    /// Refresh only the acquired bounds writer and pose. Never replay late
    /// initialization, terrain queries, usage writes or footprint release.
    pub(crate) fn refresh_geometry(
        &mut self,
        header: &actor_fields::Fields,
        slot: u32,
        cache: Cache,
    ) -> Result<(), String> {
        let f = self.initial_fields;
        let c = header.configuration;
        if header.class as u32 != f.class
            || header.scale.to_bits() != f.scale.to_bits()
            || c[20] != self.state
            || [c[15], c[16]] != self.usage
            || c[14] & 0x81008 != f.collision_flags & 0x81008
            || [c[17], c[22], c[23], c[25], c[26], c[27]]
                != [
                    f.ground_offset.to_bits(),
                    f.center_offset.to_bits(),
                    f.height.to_bits(),
                    f.packed_parameters,
                    f.radius_limit.to_bits(),
                    f.radius_override.to_bits(),
                ]
            || header.response_flags & 0x80000000 != 0
            || !header
                .position
                .into_iter()
                .chain(header.current)
                .chain(header.angles)
                .all(f32::is_finite)
        {
            return Err("geometry refresh requires unchanged acquired radius/dimensions and initialized ownership".into());
        }
        let models = crate::collision_bounds::Models {
            primary: self.resources.radius.primary,
            auxiliary: self.resources.auxiliary(slot)?,
            slot,
        };
        let bounds = crate::collision_bounds::cached(
            Some(models),
            cache.matrix[..12].try_into().unwrap(),
            Precision::Double53,
        )?;
        // Compute and validate everything before replacing the owned snapshot.
        let mut fields = self.fields;
        fields.current = header.position;
        fields.previous = header.current;
        fields.angles = header.angles;
        fields.object_flags = header.object_flags;
        fields.response_flags = header.response_flags;
        fields.collision_flags = c[14];
        fields.animation_slot = slot;
        fields.bounds = bounds;
        self.fields = fields;
        self.stamp = stamp(header, slot);
        self.cache = Some(cache);
        Ok(())
    }
    pub fn state(&self) -> u32 {
        self.state
    }
    pub fn usage(&self) -> [u32; 2] {
        self.usage
    }
    pub fn resource_bounds(&self) -> Result<ResourceBounds, &'static str> {
        self.resources.bounds(self.fields.animation_slot)
    }
    pub(crate) fn collider(
        &self,
        active: u32,
        cache: Cache,
    ) -> Result<crate::actor_environment::Collider, String> {
        use crate::actor_volume::{Boxed, Radial, Volume};
        let f = self.fields;
        let volume = if f.collision_flags & 0x80000 != 0 {
            Volume::Boxed(Boxed {
                pose: crate::contact_pose::Cached::new(active, cache.flags, cache.matrix)?,
                scale: f.scale,
                bounds: crate::volume_queries::Bounds {
                    min: self.resources.radius.primary[..3].try_into().unwrap(),
                    max: self.resources.radius.primary[3..].try_into().unwrap(),
                },
            })
        } else {
            Volume::Radial(Radial {
                position: f.current,
                radius: f.radius,
                scale: f.scale,
                height: f.height,
            })
        };
        Ok(crate::actor_environment::Collider {
            class: f.class,
            state: self.state,
            volume: Some(volume),
            mesh: crate::mecc_mesh_dispatch::MeshObject {
                fields: crate::mecc_collision_candidates::Object {
                    id: self.identity,
                    object_flags: active,
                    collision_flags: f.collision_flags,
                    has_model: true,
                    parent: 0,
                    bounds: crate::volume_queries::Bounds {
                        min: f.bounds[..3].try_into().unwrap(),
                        max: f.bounds[3..].try_into().unwrap(),
                    },
                },
                triangles: vec![],
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (
        actor_fields::Fields,
        crate::animation_channels::Channels,
        Resources,
        Cache,
    ) {
        let mut c = [0; 30];
        c[14] = 2;
        c[20] = 23;
        c[22] = 6.8_f32.to_bits();
        c[23] = 8_f32.to_bits();
        let channels = crate::actor_context::initial_channels(Some(2), false);
        let header = actor_fields::Fields {
            class: 73,
            dispatch_context: 0,
            configuration: c,
            scale: 1.,
            object_flags: 0x20,
            response_flags: 0x80000000,
            control_flags: 0,
            action: 0,
            shadow_reciprocal: 0.,
            shadow: None,
            gunlist: std::sync::Arc::from([]),
            channels,
            position: [0.; 3],
            current: [0.; 3],
            previous: [0.; 3],
            angles: [0.; 3],
        };
        let resources = Resources {
            radius: giants_formats::world_bounds::Resolved {
                primary: [-1., -1., -1., 1., 1., 1.],
                animation: Some([-10., -1., -1., 10., 1., 1.]),
            },
            animations: BTreeMap::from([(2, Some([100., 100., 100., 101., 101., 101.]))]),
            missing: BTreeSet::new(),
        };
        let cache = Cache {
            flags: 0x11,
            matrix: std::array::from_fn(|i| f32::from(i / 4 == i % 4)),
        };
        (header, channels, resources, cache)
    }
    #[test]
    fn radius_slot_one_and_current_bounds_are_independent() {
        let (header, channels, resources, cache) = fixture();
        let (out, header) =
            Prepared::initialize(1, header, channels, resources, Some(cache), |_| {
                Ok((0., [0., 0., 1., 0.]))
            })
            .unwrap();
        assert!(out.fields.radius > 8. && out.fields.radius < 9.);
        assert_eq!(out.fields.bounds, [100., 100., 100., 101., 101., 101.]);
        assert_eq!(out.fields.model, 1);
        assert!(!out.fields.physical);
        assert_eq!(out.fields.velocity, [0.; 3]);
        assert_eq!(header.response_flags & 0x80000000, 0);
        assert_eq!(header.configuration[15], 0x3c03fc0);
        out.validate(&header, 2, cache).unwrap();
    }
    #[test]
    fn source_pose_slot_and_cache_changes_are_not_accepted_as_current() {
        let (header, channels, resources, cache) = fixture();
        let (out, mut header) =
            Prepared::initialize(1, header, channels, resources, Some(cache), |_| {
                Ok((0., [0., 0., 1., 0.]))
            })
            .unwrap();
        header.configuration[12] = 10_f32.to_bits(); // Health does not select collision geometry.
        out.validate(&header, 2, cache).unwrap();
        assert!(out.validate(&header, 1, cache).is_err());
        header.position[0] = 1.;
        assert!(out.validate(&header, 2, cache).is_err());
        header.position[0] = 0.;
        let mut changed = cache;
        changed.matrix[3] = 1.;
        assert!(out.validate(&header, 2, changed).is_err());
    }
    #[test]
    fn missing_current_resource_or_cache_does_not_reach_ground() {
        let (header, channels, mut resources, cache) = fixture();
        resources.missing.insert(2);
        assert!(
            Prepared::initialize(
                1,
                header.clone(),
                channels,
                resources.clone(),
                Some(cache),
                |_| panic!("missing resource")
            )
            .is_err()
        );
        resources.missing.clear();
        let mut missing_cache = header.clone();
        missing_cache.object_flags = 0;
        assert!(
            Prepared::initialize(
                1,
                missing_cache,
                channels,
                resources.clone(),
                Some(cache),
                |_| panic!("missing affine")
            )
            .is_err()
        );
        let before = header.words();
        assert!(
            Prepared::initialize(
                1,
                header.clone(),
                channels,
                resources,
                Some(cache),
                |_| Err("terrain unavailable")
            )
            .is_err()
        );
        assert_eq!(header.words(), before);
    }
    #[test]
    fn boxed_volume_keeps_primary_model_bounds_and_mesh_is_refused() {
        let (mut header, channels, resources, cache) = fixture();
        header.configuration[14] = 0x80000;
        let (out, _) = Prepared::initialize(1, header, channels, resources, Some(cache), |_| {
            Ok((0., [0., 0., 1., 0.]))
        })
        .unwrap();
        let collider = out.collider(0x20, cache).unwrap();
        let Some(crate::actor_volume::Volume::Boxed(boxed)) = collider.volume else {
            panic!("box expected")
        };
        assert_eq!(boxed.bounds.min, [-1.; 3]);
        assert_eq!(collider.mesh.fields.bounds.min, [100.; 3]);
        let (mut header, channels, resources, cache) = fixture();
        header.configuration[14] = 8;
        assert!(
            Prepared::initialize(1, header, channels, resources, Some(cache), |_| panic!(
                "mesh producer required"
            ))
            .is_err()
        );
    }
    #[test]
    fn uncached_initialization_then_current_affine_keeps_the_proven_bounds() {
        let (mut header, channels, resources, cache) = fixture();
        header.object_flags = 0;
        let (out, mut header) = Prepared::initialize(1, header, channels, resources, None, |_| {
            Ok((0., [0., 0., 1., 0.]))
        })
        .unwrap();
        assert_eq!(out.fields.object_flags & 0x20, 0);
        header.object_flags |= 0x20;
        out.validate(&header, 2, cache).unwrap();
        let collider = out.collider(header.object_flags, cache).unwrap();
        assert_eq!(collider.mesh.fields.object_flags & 0x20, 0x20);
        assert_eq!(collider.mesh.fields.bounds.min, [100.; 3]);
    }
    #[test]
    fn moving_geometry_keeps_initialization_and_rejects_changed_dimensions_atomically() {
        let (header, channels, resources, cache) = fixture();
        let (mut out, mut header) =
            Prepared::initialize(1, header, channels, resources, Some(cache), |_| {
                Ok((7., [0., 0., 1., -7.]))
            })
            .unwrap();
        let initial = out.initialized_fields().words();
        header.position = [10., 20., 30.];
        header.current = [9., 19., 29.];
        let mut moved = cache;
        moved.matrix[3] = 10.;
        moved.matrix[7] = 20.;
        moved.matrix[11] = 30.;
        assert!(out.validate(&header, 2, moved).is_err());
        out.refresh_geometry(&header, 254, moved).unwrap();
        out.validate(&header, 254, moved).unwrap();
        assert_eq!(out.initialized_fields().words(), initial);
        assert_eq!(out.geometry_fields().bounds, [9., 19., 29., 11., 21., 31.]);
        assert_eq!(
            out.geometry_fields().radius.to_bits(),
            out.initialized_fields().radius.to_bits()
        );
        let collider = out.collider(header.object_flags, moved).unwrap();
        let Some(crate::actor_volume::Volume::Radial(radial)) = collider.volume else {
            panic!("radial expected")
        };
        assert_eq!(radial.position, header.position);
        let before = out.geometry_fields().words();
        for bad in [255, 256] {
            // A declared missing wrapper is an error; absent table slots retain
            // the acquired primary fallback. Out-of-range slots are refused.
            if bad == 255 {
                out.resources.missing.insert(bad);
            }
            assert!(out.refresh_geometry(&header, bad, moved).is_err());
            assert_eq!(out.geometry_fields().words(), before);
        }
        header.configuration[22] = 2_f32.to_bits();
        assert!(out.refresh_geometry(&header, 254, moved).is_err());
        assert_eq!(out.geometry_fields().words(), before);
    }
}
