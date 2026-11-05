//! Mixed context visual preview, with services retained as unresolved needs.
//! Static contact is explicit; broader native services remain unresolved.
use crate::{
    effect_scene::{Construction, Cursor, Origin, Plan},
    procedural_aim::Matrix,
};
use std::{collections::BTreeMap, sync::Arc};

pub struct References {
    classes: BTreeMap<i32, crate::effect_animation::Playback>,
    surface: Option<(Arc<giants_formats::gti::Terrain>, f32)>,
    contacts: BTreeMap<i32, crate::effect_contact::Seed>,
}
impl References {
    /// Opt in separately from source surface placement. Admission is bounded by
    /// the established class profile, not a guessed plane or per-scene rule.
    pub fn with_static_contacts(
        mut self,
        resources: &crate::scene_resources::SceneResources,
        phases: &[Vec<Construction>],
    ) -> Result<Self, String> {
        if self.surface.is_none() {
            return Err("mixed static contact surface absent".into());
        }
        for object in phases
            .iter()
            .flatten()
            .filter(|o| o.contact_requested && o.physics_enabled && o.lifetime.is_some())
        {
            if let std::collections::btree_map::Entry::Vacant(entry) =
                self.contacts.entry(object.class)
            {
                let program = resources
                    .classes
                    .models
                    .first_program(object.class)
                    .ok_or("mixed contact class absent")?
                    .program
                    .as_ref()
                    .map_err(Clone::clone)?;
                entry.insert(
                    crate::effect_contact::Seed::from_class(program).map_err(str::to_owned)?,
                );
            }
        }
        Ok(self)
    }
    /// Explicit static-terrain preview. Dynamic ripple is not simulated.
    pub fn with_static_surface(
        mut self,
        terrain: Arc<giants_formats::gti::Terrain>,
        inverse_spacing: f32,
    ) -> Result<Self, String> {
        crate::terrain_queries::StaticTerrainQuery::new(
            &terrain,
            inverse_spacing,
            crate::arithmetic::Precision::Double53,
        )
        .map_err(str::to_owned)?;
        self.surface = Some((terrain, inverse_spacing));
        Ok(self)
    }
    /// Admit all source phases before the renderer allocates entities. An
    /// unsupported later callback must not become a successful preparation.
    pub fn validate(&self, objects: &[Construction]) -> Result<(), String> {
        let matrices = self.matrices(objects, Matrix::identity(), [0.; 3])?;
        for (object, matrix) in objects.iter().cloned().zip(matrices) {
            self.body(object, matrix)?;
        }
        Ok(())
    }
    pub fn load(
        resources: &crate::scene_resources::SceneResources,
        image: &giants_formats::effect_image::Image,
        phases: &[Vec<Construction>],
    ) -> Result<Self, String> {
        let mut needed = std::collections::BTreeSet::new();
        for phase in phases {
            for object in phase {
                if object.services.iter().any(|s| {
                    matches!(
                        s.kind,
                        crate::effect_scene::ServiceKind::Source28
                            | crate::effect_scene::ServiceKind::Source57
                    )
                }) {
                    needed.insert(object.class);
                }
                match object.origin {
                    Origin::Attached { .. } => {
                        needed.insert(object.class);
                    }
                    Origin::Immediate { parent, .. } => {
                        needed.insert(object.class);
                        needed.insert(phase.get(parent).ok_or("mixed parent absent")?.class);
                    }
                    Origin::Detached => {}
                }
            }
        }
        let mut classes = BTreeMap::new();
        for class in needed {
            let actor = crate::actor_context::Prepared::class_initial(Arc::new(
                crate::actor_context::Class::load(resources, class, image)?,
            ));
            classes.insert(
                class,
                crate::effect_animation::Playback::new(actor)?
                    .ok_or("mixed reference animation absent")?,
            );
        }
        Ok(Self {
            classes,
            surface: None,
            contacts: BTreeMap::new(),
        })
    }
    fn body(&self, construction: Construction, matrix: Matrix) -> Result<Body, String> {
        let pose = self
            .classes
            .get(&construction.class)
            .map(crate::effect_animation::Playback::fresh_instance)
            .transpose()?;
        let profile = self.contacts.get(&construction.class).copied();
        let mut body = Body::new(construction, matrix, pose).map_err(str::to_owned)?;
        if let Some(playback) = &mut body.finite
            && playback.spawn.contact_requested
            && playback.spawn.physics_enabled
            && let Some(profile) = profile
        {
            playback
                .bind_static_contact(profile)
                .map_err(str::to_owned)?;
        }
        Ok(body)
    }
    pub fn matrices(
        &self,
        objects: &[Construction],
        owner: Matrix,
        position: [f32; 3],
    ) -> Result<Vec<Matrix>, String> {
        let mut out = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            let size = object.size.map_or(1., |v| v[0]);
            let mut matrix = match object.origin {
                Origin::Attached { reference, .. } => {
                    if reference != 0 {
                        return Err("mixed external reference operand pending".into());
                    }
                    let own = self
                        .classes
                        .get(&object.class)
                        .ok_or("mixed root references absent")?
                        .reference(0)?;
                    crate::effect_attachment::normal_matrix(owner, size, 1., own)
                        .map_err(str::to_owned)?
                }
                Origin::Immediate {
                    parent,
                    reference,
                    own_reference,
                } => {
                    if parent >= index {
                        return Err("mixed parent construction order invalid".into());
                    }
                    let parent_ref = self
                        .classes
                        .get(&objects[parent].class)
                        .ok_or("mixed parent references absent")?
                        .reference(reference as usize)?;
                    let own = self
                        .classes
                        .get(&object.class)
                        .ok_or("mixed child references absent")?
                        .reference(own_reference as usize)?;
                    crate::effect_attachment::initial_child_matrix(
                        out[parent],
                        parent_ref,
                        own,
                        size,
                        objects[parent].size.map_or(1., |v| v[0]),
                    )
                    .map_err(str::to_owned)?
                }
                Origin::Detached => {
                    let z = (f64::from(position[2]) + f64::from(object.offset_z)) as f32;
                    let position = [
                        position[0],
                        position[1],
                        (f64::from(z) + f64::from(object.height)) as f32,
                    ];
                    if object.size.is_none() {
                        // An unscaled point support still has an identity basis.
                        // Immediate children transform model references through it.
                        let mut values = Matrix::identity().values;
                        for (i, value) in [3, 7, 11].into_iter().zip(position) {
                            values[i] = value;
                        }
                        Matrix { flags: 16, values }
                    } else {
                        crate::effect_orientation::matrix(position, [0.; 3], size)
                            .map_err(str::to_owned)?
                    }
                }
            };
            if let Some(offset) = object.surface_offset {
                if object.size.is_none() {
                    return Err("surface object size profile absent".into());
                }
                let (terrain, inverse) = self
                    .surface
                    .as_ref()
                    .ok_or("surface query profile absent")?;
                let query = crate::terrain_queries::StaticTerrainQuery::new(
                    terrain,
                    *inverse,
                    crate::arithmetic::Precision::Double53,
                )
                .map_err(str::to_owned)?;
                let point = [matrix.values[3], matrix.values[7], matrix.values[11]];
                let sample = query.sample([point[0], point[1]]).map_err(str::to_owned)?;
                let placement =
                    crate::effect_surface::placement(point, sample.plane, sample.height, offset)
                        .map_err(str::to_owned)?;
                matrix = crate::effect_surface::alignment_matrix(
                    matrix,
                    sample.plane[..3]
                        .try_into()
                        .map_err(|_| "invalid surface normal")?,
                    placement,
                )
                .map_err(str::to_owned)?;
            }
            out.push(matrix);
        }
        Ok(out)
    }
}
pub fn templates(plan: &Plan) -> Result<Vec<Vec<Construction>>, &'static str> {
    let mut seed = 1;
    let (first, mut cursor) = plan.start(&mut seed)?;
    let mut out = vec![first];
    while let Some(wait) = cursor.remaining {
        out.push(
            cursor
                .advance(plan, wait, &mut seed)?
                .ok_or("mixed preparation failed to reach phase")?,
        );
    }
    Ok(out)
}
/// Curve34 guide references checked through the real animation/world helpers
/// for a timed detached object without translation physics or surface alignment.
/// Other transform owners remain explicit needs, independently of scalar size.
pub fn curve_reference_pending(construction: &Construction) -> bool {
    construction.size_opcode == Some(0x34)
        && (construction.lifetime.is_none()
            || !matches!(construction.origin, Origin::Detached)
            || construction.physics_enabled
            || construction.speed != 0.
            || construction.surface_offset.is_some())
}
pub struct Body {
    pub construction: Construction,
    finite: Option<crate::effect_burst::Playback>,
    birth: Matrix,
    pose: Option<crate::effect_animation::Playback>,
    /// Scalar playback is independent of the admitted spatial guide profile.
    pub curve_pending: bool,
}
impl Body {
    pub fn playback(&self) -> Option<&crate::effect_burst::Playback> {
        self.finite.as_ref()
    }
    fn new(
        construction: Construction,
        birth: Matrix,
        pose: Option<crate::effect_animation::Playback>,
    ) -> Result<Self, &'static str> {
        if construction.lifetime.is_none()
            && (construction.physics_enabled || construction.spin != [0.; 3])
        {
            return Err("duration-free dynamic effect callback pending");
        }
        let curve_pending = curve_reference_pending(&construction);
        let finite = if let Some([lifetime, fade_in, fade_out_start]) = construction.lifetime {
            let size = construction.size.unwrap_or([1.; 2]);
            let spawn = crate::effect_burst::Spawn {
                class: construction.class,
                parent: None,
                reference: 0,
                own_reference: 0,
                visual: crate::effect_visual::Configuration {
                    lifetime,
                    fade_in,
                    fade_out_start,
                    start_size: size[0],
                    end_size: size[1],
                },
                height: construction.height,
                hidden: construction.hidden,
                spin: construction.spin,
                direction_requested: construction.direction_requested,
                speed: construction.speed,
                gravity_mass: construction.gravity,
                physics_enabled: construction.physics_enabled,
                damping: construction.damping,
                bounce: construction.bounce,
                contact_requested: construction.contact_requested,
                parameter_b0: 1f32.to_bits(),
                attribute: None,
                sounds: Vec::new(),
                texture_animation: None,
                size_configured: construction.size.is_some(),
                child_services: Vec::new(),
            };
            // Static contact can be bound by References after construction.
            // Broader services remain needs, never implicitly fulfilled.
            Some(crate::effect_burst::Playback::with_size_curve(
                spawn,
                birth,
                if construction.size_opcode == Some(0x34) {
                    crate::effect_visual::SizeCurve::EaseIn
                } else {
                    crate::effect_visual::SizeCurve::Linear
                },
            )?)
        } else {
            None
        };
        Ok(Self {
            construction,
            finite,
            birth,
            pose,
            curve_pending,
        })
    }
    pub fn matrix(&self) -> Result<Matrix, &'static str> {
        self.finite.as_ref().map_or(Ok(self.birth), |p| p.matrix())
    }
    /// A guide's loop belongs to the constructed object, including hidden
    /// objects and objects whose materials the renderer cannot yet admit.
    pub fn local_reference(&self, index: usize) -> Result<Matrix, String> {
        self.pose
            .as_ref()
            .ok_or("mixed guide reference pose absent")?
            .reference(index)
    }
    /// Cached affine profile only. No computed Euler fallback, paired-context
    /// attachment, unsupported curve ownership or missing-reference substitution
    /// is invented.
    pub fn reference_point(&self, index: usize) -> Result<[f32; 3], String> {
        if self.curve_pending {
            return Err("mixed guide reference curve34 scale pending".into());
        }
        let local = self.local_reference(index)?.values;
        crate::render_sort::world_point(
            [local[3], local[7], local[11]],
            [0.; 3],
            crate::render_sort::Transform::Cached(self.matrix().map_err(str::to_owned)?.values),
            crate::arithmetic::Precision::Double53,
        )
    }
    pub fn pose_seconds(&self) -> Option<f32> {
        self.pose.as_ref().map(|p| p.clock.seconds)
    }
    pub fn visible(&self) -> bool {
        !self.construction.hidden
            && !self.curve_pending
            && self.finite.as_ref().is_none_or(|p| !p.expired)
    }
    pub fn expired(&self) -> bool {
        self.finite.as_ref().is_some_and(|p| p.expired)
    }
    pub fn size(&self) -> f32 {
        self.finite.as_ref().map_or_else(
            || self.construction.size.map_or(1., |s| s[0]),
            |p| p.visual.size,
        )
    }
    pub fn transparency(&self) -> f32 {
        self.finite.as_ref().map_or(0., |p| p.visual.transparency)
    }
    pub fn angles(&self) -> Option<[f32; 3]> {
        self.finite.as_ref().and_then(|p| p.angles)
    }
}
/// A configured child requested at lifetime expiry, before child activation.
/// Detail selection, context allocation and renderer/service completion remain
/// the caller's responsibility. No elapsed-time substitute for other deletion
/// paths is introduced here.
#[derive(Clone, Debug)]
pub struct RetirementRequest {
    pub phase: usize,
    pub object: usize,
    pub binding: crate::effect_scene::RetirementBinding,
    pub position: [f32; 3],
}
pub struct Runtime {
    pub plan: Arc<Plan>,
    cursor: Cursor,
    pub bodies: Vec<Vec<Body>>,
    retirements: Vec<RetirementRequest>,
}
impl Runtime {
    pub fn activate(
        queued: crate::effect_scene::Queued,
        references: &References,
        owner: Matrix,
        position: [f32; 3],
        seed: &mut u32,
    ) -> Result<Self, String> {
        let (plan, objects, cursor) = queued.activate(seed).map_err(str::to_owned)?;
        let matrices = references.matrices(&objects, owner, position)?;
        let bodies = objects
            .into_iter()
            .zip(matrices)
            .map(|(o, m)| references.body(o, m))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            plan,
            cursor,
            bodies: vec![bodies],
            retirements: Vec::new(),
        })
    }
    pub fn start(
        plan: Arc<Plan>,
        references: &References,
        owner: Matrix,
        position: [f32; 3],
        seed: &mut u32,
    ) -> Result<Self, String> {
        let (objects, cursor) = plan.start(seed).map_err(str::to_owned)?;
        let matrices = references.matrices(&objects, owner, position)?;
        let bodies = objects
            .into_iter()
            .zip(matrices)
            .map(|(o, m)| references.body(o, m))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            plan,
            cursor,
            bodies: vec![bodies],
            retirements: Vec::new(),
        })
    }
    /// Drain each observed expiry request once, without activating descendants
    /// or consuming their random ranges during this parent callback.
    pub fn take_retirements(&mut self) -> Vec<RetirementRequest> {
        std::mem::take(&mut self.retirements)
    }
    /// Observe pending requests while a child activation service is unbound.
    pub fn retirements(&self) -> &[RetirementRequest] {
        &self.retirements
    }
    pub fn advance(
        &mut self,
        dt: f32,
        references: &References,
        owner: Matrix,
        position: [f32; 3],
        seed: &mut u32,
    ) -> Result<Option<usize>, String> {
        let mut phase = None;
        if let Some(objects) = self
            .cursor
            .advance(&self.plan, dt, seed)
            .map_err(str::to_owned)?
        {
            let matrices = references.matrices(&objects, owner, position)?;
            let bodies = objects
                .into_iter()
                .zip(matrices)
                .map(|(o, m)| references.body(o, m))
                .collect::<Result<Vec<_>, _>>()?;
            phase = Some(self.bodies.len());
            self.bodies.push(bodies);
        }
        self.advance_bodies_with_references(dt, references)?;
        Ok(phase)
    }
    pub fn advance_bodies_with_references(
        &mut self,
        dt: f32,
        references: &References,
    ) -> Result<(), String> {
        let query = if self
            .bodies
            .iter()
            .flatten()
            .any(|body| body.finite.as_ref().is_some_and(|p| p.contact.is_some()))
        {
            let (terrain, inverse) = references
                .surface
                .as_ref()
                .ok_or("mixed bound contact surface absent")?;
            Some((
                crate::terrain_queries::StaticTerrainQuery::new(
                    terrain,
                    *inverse,
                    crate::arithmetic::Precision::Double53,
                )
                .map_err(str::to_owned)?,
                terrain.width,
            ))
        } else {
            None
        };
        self.advance_body_callbacks(dt, query.as_ref())
    }
    /// Separate initial body callbacks from the context's first wait. This
    /// preserves the existing preview convention without consuming that wait.
    pub fn advance_bodies(&mut self, dt: f32) -> Result<(), String> {
        self.advance_body_callbacks(dt, None)
    }
    fn advance_body_callbacks(
        &mut self,
        dt: f32,
        query: Option<&(crate::terrain_queries::StaticTerrainQuery<'_>, usize)>,
    ) -> Result<(), String> {
        for (phase, bodies) in self.bodies.iter_mut().enumerate() {
            for (object, body) in bodies.iter_mut().enumerate() {
                let was_expired = body.expired();
                if let Some(p) = &mut body.finite {
                    if p.contact.is_some() {
                        let (query, width) = query.ok_or("mixed bound contact query absent")?;
                        p.advance_static(dt, |point| {
                            crate::physical_ground::Ground::from_sample(
                                query.sample(point)?,
                                *width,
                            )
                        })
                        .map_err(str::to_owned)?;
                    } else {
                        p.advance_free(dt).map_err(str::to_owned)?;
                    }
                }
                if !was_expired
                    && body.expired()
                    && let Some(binding) = &body.construction.object_services.retirement
                    && let Some(playback) = body.playback()
                {
                    self.retirements.push(RetirementRequest {
                        phase,
                        object,
                        binding: binding.clone(),
                        position: playback.motion.position,
                    });
                }
                if !body.expired()
                    && let Some(pose) = &mut body.pose
                {
                    pose.advance(dt).map_err(str::to_owned)?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn unscaled_point_support_preserves_its_basis_for_child_references() {
        let refs = super::References {
            classes: std::collections::BTreeMap::new(),
            surface: None,
            contacts: std::collections::BTreeMap::new(),
        };
        let (mut objects, _) = plan(1.).start(&mut 17).unwrap();
        objects[0].size = None;
        let point = [7., -11., 13.];
        let support = refs
            .matrices(&objects[..1], super::Matrix::identity(), point)
            .unwrap()[0];
        assert_eq!(support.flags, 16);
        for i in [0, 5, 10, 15] {
            assert_eq!(support.values[i], 1.);
        }
        let mut reference = super::Matrix::identity();
        reference.flags = 16;
        for (i, value) in [3, 7, 11].into_iter().zip([2., -3., 5.]) {
            reference.values[i] = value;
        }
        let child = crate::effect_attachment::initial_child_matrix(
            support,
            reference,
            super::Matrix::identity(),
            2.,
            1.,
        )
        .unwrap();
        assert_eq!(
            [child.values[3], child.values[7], child.values[11]],
            [9., -14., 18.]
        );
        assert_eq!(
            [child.values[0], child.values[5], child.values[10]],
            [2.; 3]
        );
    }
    #[test]
    fn retirement_requests_are_once_only_and_do_not_expire_duration_free_objects() {
        let refs = super::References {
            classes: std::collections::BTreeMap::new(),
            surface: None,
            contacts: std::collections::BTreeMap::new(),
        };
        let plan = std::sync::Arc::new(plan(0.02));
        let mut seed = 17;
        let mut runtime = super::Runtime::start(
            plan,
            &refs,
            super::Matrix::identity(),
            [7., -11., 13.],
            &mut seed,
        )
        .unwrap();
        let binding = crate::effect_scene::RetirementBinding {
            effect: "child".into(),
            program_offset: 123,
        };
        runtime.bodies[0][0].construction.object_services.retirement = Some(binding.clone());
        runtime.advance_bodies(0.).unwrap();
        runtime.advance_bodies(0.01).unwrap();
        assert!(runtime.take_retirements().is_empty());
        runtime.advance_bodies(0.2).unwrap();
        let requests = runtime.take_retirements();
        assert_eq!(requests.len(), 1);
        assert_eq!((requests[0].phase, requests[0].object), (0, 0));
        assert_eq!(requests[0].position, [7., -11., 13.]);
        assert_eq!(requests[0].binding.program_offset, 123);
        assert_eq!(requests[0].binding.effect, "child");
        assert!(runtime.take_retirements().is_empty());
        runtime
            .advance(0.1, &refs, super::Matrix::identity(), [1.; 3], &mut seed)
            .unwrap();
        runtime.bodies[1][0].construction.object_services.retirement = Some(binding);
        runtime.advance_bodies(1000.).unwrap();
        assert!(!runtime.bodies[1][0].expired());
        assert!(runtime.take_retirements().is_empty());
        assert_eq!(seed, 17);
    }
    #[test]
    fn mixed_bound_contacts_keep_first_frame_expiry_and_instance_ownership() {
        let plan = std::sync::Arc::new(plan(1.));
        let (mut objects, cursor) = plan.start(&mut 1).unwrap();
        objects[0].contact_requested = true;
        objects[0].physics_enabled = true;
        let program = giants_formats::class_program::parse(&[0x1e, 78, 0x36, 6, 0xff]).unwrap();
        let profile = crate::effect_contact::Seed::from_class(&program).unwrap();
        let mut refs = super::References {
            classes: std::collections::BTreeMap::new(),
            surface: None,
            contacts: [(objects[0].class, profile)].into(),
        };
        let body = refs
            .body(objects[0].clone(), super::Matrix::identity())
            .unwrap();
        let mut runtime = super::Runtime {
            plan,
            cursor,
            bodies: vec![vec![body]],
            retirements: Vec::new(),
        };
        assert!(runtime.advance_bodies(0.01).is_err());
        assert!(runtime.advance_bodies_with_references(0.01, &refs).is_err());
        refs = refs
            .with_static_surface(
                std::sync::Arc::new(giants_formats::gti::Terrain {
                    source_flags: 3,
                    width: 2,
                    height: 2,
                    origin: [0.; 2],
                    spacing: 1.,
                    height_bounds: [0.; 2],
                    fallback_texture: String::new(),
                    heights: vec![0.; 4],
                    colors: vec![],
                    tags: vec![5; 4],
                    consumed_bytes: 0,
                    opaque_tail: vec![],
                }),
                1.,
            )
            .unwrap();
        runtime.advance_bodies_with_references(0.01, &refs).unwrap();
        assert_eq!(
            runtime.bodies[0][0]
                .playback()
                .unwrap()
                .contact
                .as_ref()
                .unwrap()
                .steps,
            0
        );
        runtime.advance_bodies_with_references(100., &refs).unwrap();
        assert!(runtime.bodies[0][0].expired());
        let fresh = refs
            .body(objects.remove(0), super::Matrix::identity())
            .unwrap();
        assert!(!fresh.expired());
        assert_eq!(fresh.playback().unwrap().contact.as_ref().unwrap().steps, 0);
    }
    #[test]
    fn accelerated_body_admits_only_the_checked_detached_spatial_profile() {
        let (mut objects, _) = plan(1.).start(&mut 1).unwrap();
        let mut construction = objects.remove(0);
        construction.size_opcode = Some(0x34);
        construction.size = Some([4., 10.]);
        construction.lifetime = Some([2., 0., 1.]);
        let birth = crate::effect_orientation::matrix([0.; 3], [0.; 3], 4.).unwrap();
        let mut body = super::Body::new(construction.clone(), birth, None).unwrap();
        for _ in 0..2 {
            body.finite.as_mut().unwrap().advance_free(0.1).unwrap();
        }
        assert_eq!(body.size().to_bits(), 1082161889);
        assert!(!body.curve_pending);
        assert!(body.visible());
        assert!(body.reference_point(0).unwrap_err().contains("pose absent"));
        for origin in [
            super::Origin::Attached {
                opaque: 0,
                reference: 0,
            },
            super::Origin::Immediate {
                parent: 0,
                reference: 0,
                own_reference: 0,
            },
        ] {
            let mut unsupported = construction.clone();
            unsupported.origin = origin;
            let body = super::Body::new(unsupported, birth, None).unwrap();
            assert!(body.curve_pending);
            assert!(!body.visible());
            assert!(body.reference_point(0).unwrap_err().contains("curve34"));
        }
        let mut unsupported = construction.clone();
        unsupported.lifetime = None;
        assert!(super::curve_reference_pending(&unsupported));
        unsupported = construction.clone();
        unsupported.physics_enabled = true;
        assert!(super::curve_reference_pending(&unsupported));
        unsupported = construction.clone();
        unsupported.speed = 1.;
        assert!(super::curve_reference_pending(&unsupported));
        unsupported = construction;
        unsupported.surface_offset = Some(0.);
        assert!(super::curve_reference_pending(&unsupported));
    }
    #[test]
    fn surface_birth_requires_an_explicit_query_profile() {
        let plan = plan(1.);
        let (mut objects, _) = plan.start(&mut 1).unwrap();
        objects[0].surface_offset = Some(0.125);
        let refs = super::References {
            classes: std::collections::BTreeMap::new(),
            surface: None,
            contacts: std::collections::BTreeMap::new(),
        };
        assert!(
            refs.matrices(&objects, super::Matrix::identity(), [0.25, 0.25, 17.])
                .unwrap_err()
                .contains("surface query profile absent")
        );
        let terrain = giants_formats::gti::Terrain {
            source_flags: 3,
            width: 2,
            height: 2,
            origin: [0.; 2],
            spacing: 1.,
            height_bounds: [4.; 2],
            fallback_texture: String::new(),
            heights: vec![4.; 4],
            colors: vec![],
            tags: vec![5; 4],
            consumed_bytes: 0,
            opaque_tail: vec![],
        };
        let refs = refs
            .with_static_surface(std::sync::Arc::new(terrain), 1.)
            .unwrap();
        let matrices = refs
            .matrices(&objects, super::Matrix::identity(), [0.25, 0.25, 17.])
            .unwrap();
        assert_eq!(
            [
                matrices[0].values[3],
                matrices[0].values[7],
                matrices[0].values[11]
            ],
            [0.25, 0.25, 4.125]
        );
    }
    fn plan(wait: f32) -> super::Plan {
        let mut b = vec![3];
        b.extend(1u32.to_le_bytes());
        b.extend([6, 0x1b]);
        b.extend(1f32.to_le_bytes());
        b.push(4);
        for v in [0.1f32, 0., 0.] {
            b.extend(v.to_le_bytes());
        }
        b.push(0x13);
        for value in [0.25f32, 0.5] {
            b.extend(value.to_le_bytes());
        }
        b.push(0x19);
        b.extend(0.75f32.to_le_bytes());
        b.extend([7, 0xb]);
        b.extend(wait.to_le_bytes());
        b.push(3);
        b.extend(2u32.to_le_bytes());
        b.extend([6, 0x1b]);
        b.extend(2f32.to_le_bytes());
        b.extend([7, 2]);
        let mut payload = b"p\0".to_vec();
        payload.extend(6u32.to_le_bytes());
        payload.extend(b);
        let mut data = 1u32.to_le_bytes().to_vec();
        data.extend((payload.len() as u32).to_le_bytes());
        data.extend(1u32.to_le_bytes());
        data.extend(payload);
        let image = giants_formats::effect_image::parse(&data).unwrap();
        super::Plan::project(&image, "p").unwrap()
    }
    #[test]
    fn preparation_reaches_all_phases_without_a_fake_short_clock_limit() {
        let p = plan(10000.);
        let objects = super::templates(&p).unwrap();
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[1][0].class, 2);
    }
    #[test]
    fn unconfigured_lifetime_survives_and_later_birth_reads_current_support() {
        let plan = std::sync::Arc::new(plan(1.));
        let refs = super::References {
            classes: std::collections::BTreeMap::new(),
            surface: None,
            contacts: std::collections::BTreeMap::new(),
        };
        let owner = super::Matrix::identity();
        let mut seed = 1;
        let mut runtime =
            super::Runtime::start(plan, &refs, owner, [1., 2., 3.], &mut seed).unwrap();
        let finite = runtime.bodies[0][0].finite.as_ref().unwrap();
        assert_eq!(finite.spawn.damping, [0.25, 0.5]);
        assert_eq!(finite.spawn.bounce, 0.75);
        runtime.advance_bodies(0.01).unwrap();
        runtime
            .advance(1., &refs, owner, [50., 60., 70.], &mut seed)
            .unwrap();
        assert!(runtime.bodies[0][0].expired());
        let body = &runtime.bodies[1][0];
        assert!(body.construction.lifetime.is_none());
        let m = body.matrix().unwrap();
        assert_eq!([m.values[3], m.values[7], m.values[11]], [50., 60., 70.]);
        runtime
            .advance(50000., &refs, owner, [0.; 3], &mut seed)
            .unwrap();
        assert!(runtime.bodies[1][0].visible());
        assert!(!runtime.bodies[1][0].expired());
    }
}
