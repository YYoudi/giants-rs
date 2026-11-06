//! Ticket-owned composition of cold Kabuto actor fields, auxiliary links and
//! physics. Missing animation-cache/material/camera services are explicit pending work;
//! this group is not admitted to live gameplay.
use crate::{
    animation_channels::Blending,
    instances::Plan,
    kabuto_auxiliary::{Factory as Auxiliaries, Kind},
    kabuto_geometry::Geometry,
    kabuto_physics::{Physics, Refresh},
    kabuto_player::ColdFrame,
    kabuto_resources::Resources,
    resident_states::{Id, Session},
    specification,
};
use giants_formats::{
    global_data::GlobalData, mission::Mission, world::WorldConfig, world_bounds::Resolved,
};
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pending {
    ProjectionReset,
    AnimationCache,
    ProceduralMaterial,
    GeometryRegistration,
}
pub struct Group {
    pub owner: Id,
    pub first: Id,
    pub second: Id,
    pub effects: [Id; 8],
    pub final_effect: Id,
    /// Active dynamic auxiliary identity; marked actors can remain owned after
    /// this reference is cleared. No original pointer is stored in the payload.
    pub extra: Option<Id>,
    pub(crate) geometry_transition_failure: Option<String>,
    pub(crate) primary_started: bool,
    pub declared_class: Arc<specification::Prepared>,
    pub geometry: Geometry,
    pub radius: f32,
    pub physics: Option<Physics>,
    /// Optional acquired cold definitions. Process/world reset scheduling is
    /// still part of Pending::AnimationCache, not acknowledged by this value.
    pub cold_animation_cache: Option<Arc<crate::kabuto_animation_cache::ColdCache>>,
    /// Optional CPU image. Material upload/routing remains pending.
    pub procedural_texture: Option<Arc<crate::kabuto_texture::Texture>>,
    pub geometry_registration: Option<crate::kabuto_geometry_registry::Handle>,
    pub(crate) cleaned_up: bool,
    pub pending: [Pending; 4],
}
impl std::fmt::Debug for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KabutoColdGroup")
            .field("owner", &self.owner)
            .field("first", &self.first)
            .field("second", &self.second)
            .field("effects", &self.effects)
            .field("final_effect", &self.final_effect)
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}
impl Group {
    pub fn geometry_transition_failure(&self) -> Option<&str> {
        self.geometry_transition_failure.as_deref()
    }
    /// Custom cold cleanup only; the common owner state transition is separate.
    pub fn cleaned_up(&self) -> bool {
        self.cleaned_up
    }
}
pub struct Factory {
    resources: Resources,
    auxiliary: Auxiliaries,
    globals: Arc<GlobalData>,
    bounds: Resolved,
    animation_cache: Option<crate::kabuto_animation_cache::Lifecycle>,
    procedural_texture: Option<Arc<crate::kabuto_texture::Texture>>,
}
impl Factory {
    /// Acquired size-pursuit subphase using this factory's actual class, global
    /// parameters and bounds. This does not choose a target or admit a player.
    pub fn advance_growth(
        &self,
        session: &mut Session,
        owner: Id,
        frame: crate::kabuto_growth::Frame<'_>,
    ) -> Result<crate::kabuto_growth::View, String> {
        session.advance_kabuto_growth(
            owner,
            &self.resources,
            &self.globals,
            self.bounds.clone(),
            frame,
        )
    }
    pub fn load(
        plan: &Plan,
        world: &WorldConfig,
        mission: &Mission,
        globals: Arc<GlobalData>,
    ) -> Result<Self, String> {
        let resources = Resources::load(plan, world, mission)?;
        if resources.specification.metadata.references.names.len() <= 10 {
            return Err("Kabuto parent reference10 unavailable".into());
        }
        Ok(Self {
            auxiliary: Auxiliaries::load(plan.resources())?,
            bounds: plan
                .resources()
                .collision_resource_bounds(1, Some(1))
                .map_err(|e| e.to_string())?,
            resources,
            globals,
            animation_cache: None,
            procedural_texture: None,
        })
    }
    /// Prepare cold data from an explicit external provider. Failure leaves
    /// this factory unchanged; no native process-global hook is manufactured.
    pub fn prepare_animation_definitions(
        &mut self,
        definitions: &crate::kabuto_animation_cache::Definitions,
    ) -> Result<(), String> {
        let mut cache = crate::kabuto_animation_cache::Lifecycle::new(definitions.clone());
        cache.initialize(self.resources.specification.clone())?;
        self.animation_cache = Some(cache);
        Ok(())
    }
    /// Explicit acquired reset phase; does not schedule world teardown.
    pub fn reset_animation_cache(&mut self) -> Result<(), String> {
        self.animation_cache
            .as_mut()
            .ok_or("Kabuto animation provider unavailable")?
            .reset();
        Ok(())
    }
    pub fn refresh_animation_cache(&mut self) -> Result<(), String> {
        self.animation_cache
            .as_mut()
            .ok_or("Kabuto animation provider unavailable")?
            .initialize(self.resources.specification.clone())?;
        Ok(())
    }
    /// Normal cold generation profile, with shared caller-owned RNG. Reusing
    /// the resource consumes no more draws. No GPU-resident bypass is implied.
    pub fn prepare_procedural_texture(
        &mut self,
        random: &mut crate::audio_parameters::Random,
    ) -> &Arc<crate::kabuto_texture::Texture> {
        self.procedural_texture
            .get_or_insert_with(|| Arc::new(crate::kabuto_texture::prepare(random)))
    }
    /// Atomic CPU group publication for the acquired normal cold profile. This
    /// does not perform marker relocation or satisfy the returned pending services.
    pub fn construct(
        &self,
        session: &mut Session,
        frame: ColdFrame,
        blending: Blending,
        generation: u32,
    ) -> Result<Id, String> {
        if self
            .animation_cache
            .as_ref()
            .is_some_and(|c| !c.initialized())
        {
            return Err("Kabuto animation reset awaits refresh".into());
        }
        session.stage_kabuto_group(|s| self.build(s, frame, blending, generation))
    }
    /// Atomic actor/resource publication for the normal cold CPU profile. The
    /// caller owns the mission registry and shared RNG; no GPU admission follows.
    pub fn construct_registered(
        &self,
        session: &mut Session,
        frame: ColdFrame,
        blending: Blending,
        generation: u32,
        registry: &mut crate::kabuto_geometry_registry::Registry,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<Id, String> {
        if self.procedural_texture.is_some()
            || self
                .animation_cache
                .as_ref()
                .is_some_and(|c| !c.initialized())
        {
            return Err(
                "Registered Kabuto requires registry-owned material and refreshed animation data"
                    .into(),
            );
        }
        let mut staged = registry.staged();
        let mut next_random = *random;
        let owner = session.stage_kabuto_group(|s| {
            let owner = self.build(s, frame, blending, generation)?;
            let handle = staged.register(owner, &mut next_random)?;
            s.bind_kabuto_geometry_registry(owner, handle, staged.texture().unwrap().clone())?;
            Ok(owner)
        })?;
        *registry = staged;
        *random = next_random;
        Ok(owner)
    }
    fn build(
        &self,
        s: &mut Session,
        frame: ColdFrame,
        blending: Blending,
        generation: u32,
    ) -> Result<Id, String> {
        let owner = self.resources.construct_ordinary(s)?;
        self.resources
            .initialize_player_fields(s, owner, frame, blending)?;
        let first = self.auxiliary.construct(s, Kind::First)?;
        s.bind_kabuto_attachment(owner, first, false)?;
        let second = self.auxiliary.construct(s, Kind::Second)?;
        s.bind_kabuto_attachment(owner, second, true)?;
        let mut effects = Vec::new();
        for _ in 0..8 {
            let id = self.auxiliary.construct(s, Kind::Effect)?;
            s.bind_kabuto_effect(owner, id)?;
            effects.push(id);
        }
        let final_effect = self.auxiliary.construct(s, Kind::FinalEffect)?;
        s.bind_kabuto_effect(owner, final_effect)?;
        let actor = s.get(owner).unwrap();
        let mut payload = actor.seed.storage.data().unwrap().to_vec();
        let put = |p: &mut [u8], at: usize, v: u32| p[at..at + 4].copy_from_slice(&v.to_le_bytes());
        // The acquired cold template supplies old164; later size preparation replaces it.
        let old164 = u32::from_le_bytes(payload[0x164..0x168].try_into().unwrap());
        for at in [
            4, 8, 0xc, 0x10, 0x14, 0x50, 0x54, 0x5c, 0x88, 0x8c, 0xc0, 0xc4, 0xcc, 0xd0, 0xe0,
            0x12c, 0x130, 0x138, 0x13c, 0x140,
        ] {
            put(&mut payload, at, 0);
        }
        for at in [0x78, 0x7c] {
            put(&mut payload, at, (-1f32).to_bits());
        }
        for at in [0x90, 0x98, 0x9c] {
            put(&mut payload, at, u32::MAX);
        }
        put(&mut payload, 0x84, 254);
        for at in [0xc8, 0xd4, 0x128, 0x134] {
            put(&mut payload, at, 1f32.to_bits());
        }
        put(&mut payload, 0xd8, 3);
        put(&mut payload, 0x160, old164);
        // Normal cold mode, not the unacquired special-size override profile.
        put(&mut payload, 0x158, 0);
        put(&mut payload, 0x15c, 0);
        // All native object/class/buffer pointers remain typed in Group instead.
        for at in [0x150, 0x154, 0x220, 0x314] {
            put(&mut payload, at, 0);
        }
        for i in 0..8 {
            put(&mut payload, 0x22c + i * 28, 0);
        }
        // The first size pass precedes geometry and the generation gate. The
        // physical refresh may repeat it, but an unchanged generation does not.
        let initial_size = crate::kabuto_size::prepare(
            actor.construction_fields.as_ref().unwrap(),
            &payload,
            0.,
            Some(crate::kabuto_size::Source {
                globals: &self.globals,
                class_scale: self.resources.class_fields().scale,
                bounds: self.bounds.clone(),
            }),
        )?;
        let geometry = crate::kabuto_geometry::prepare(32, 2, 4.)?;
        let mut payload = initial_size.payload;
        let mut flags = u32::from_le_bytes(payload[..4].try_into().unwrap());
        flags |= 1;
        put(&mut payload, 0, flags);
        let prepared = crate::kabuto_physics::prepare(
            &initial_size.fields,
            &payload,
            initial_size.radius,
            Refresh {
                generation,
                present: false,
                globals: &self.globals,
                size: Some(crate::kabuto_size::Source {
                    globals: &self.globals,
                    class_scale: self.resources.class_fields().scale,
                    bounds: self.bounds.clone(),
                }),
            },
        )?;
        let group = Group {
            owner,
            first,
            second,
            effects: effects.try_into().unwrap(),
            final_effect,
            extra: None,
            geometry_transition_failure: None,
            primary_started: false,
            declared_class: self.resources.specification.clone(),
            geometry,
            radius: prepared.radius,
            physics: prepared.physics,
            cold_animation_cache: self
                .animation_cache
                .as_ref()
                .and_then(|c| c.cached())
                .cloned(),
            procedural_texture: self.procedural_texture.clone(),
            geometry_registration: None,
            cleaned_up: false,
            pending: [
                Pending::ProjectionReset,
                Pending::AnimationCache,
                Pending::ProceduralMaterial,
                Pending::GeometryRegistration,
            ],
        };
        s.finish_kabuto_group(owner, prepared.fields, prepared.payload, group)?;
        Ok(owner)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires GIANTS_KABUTO_ROOT with user-supplied data; no window or sound"]
    fn source_kabuto_group_composes_fields_links_and_physics_atomically() {
        let root = std::path::PathBuf::from(std::env::var("GIANTS_KABUTO_ROOT").unwrap());
        let source = crate::world_source::resolve(&root, "Kstory1").unwrap();
        let menu = crate::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin").join(source.catalog)).unwrap(),
            u32::MAX,
            true,
        )
        .unwrap();
        let packet =
            crate::mission_loading::prepare(&root, menu.request("Kstory1", 0).unwrap()).unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let globals = Arc::new(
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
                .unwrap(),
        );
        let mut factory = Factory::load(
            &plan,
            packet.world.as_ref().unwrap(),
            packet.mission.as_ref().unwrap(),
            globals.clone(),
        )
        .unwrap();
        let frame = ColdFrame {
            position: [1., 2., 3.],
            current: [4., 5., 6.],
            previous: [7., 8., 9.],
            angles: [0., 40., 0.],
        };
        let mut session = Session::new(946);
        let sentinel = factory.resources.construct_ordinary(&mut session).unwrap();
        let counts = session.counts();
        let lists = (0..15).map(|i| session.list(i)).collect::<Vec<_>>();
        let mut invalid = GlobalData {
            words: globals.words.clone(),
            array_words: globals.array_words.clone(),
        };
        invalid.words[0x6b8 / 4] = 0;
        let bad = Factory::load(
            &plan,
            packet.world.as_ref().unwrap(),
            packet.mission.as_ref().unwrap(),
            Arc::new(invalid),
        )
        .unwrap();
        assert!(
            bad.construct(&mut session, frame, Blending::Enabled, 1)
                .is_err()
        );
        assert_eq!(session.counts(), counts);
        assert_eq!((0..15).map(|i| session.list(i)).collect::<Vec<_>>(), lists);
        assert!(session.get(sentinel).is_some());
        let owner = factory
            .construct(&mut session, frame, Blending::Enabled, 1)
            .unwrap();
        assert_eq!(session.counts().residents, 13);
        let group = session.kabuto_group(owner).unwrap();
        assert_eq!(group.owner, owner);
        assert!(group.physics.is_some());
        assert_eq!(group.geometry.vertices.len(), 65);
        assert_eq!(group.geometry.indices.len(), 288);
        assert_eq!(group.pending.len(), 4);
        assert!(Arc::ptr_eq(
            &group.declared_class,
            session.specification(owner).unwrap()
        ));
        for id in [group.first, group.second] {
            let actor = session.get(id).unwrap();
            let a = actor.attachment.unwrap();
            assert_eq!(a.parent, owner);
            assert_eq!(a.child_reference, 0);
            assert_eq!(a.parent_reference, 10);
        }
        for id in group.effects.into_iter().chain([group.final_effect]) {
            let a = session.get(id).unwrap();
            assert_eq!(a.seed.storage.state(), 0);
            assert!(a.seed.storage.data().is_none());
            assert!(session.kabuto_initial_effect_cache(owner, id).is_some());
            assert!(session.current_reference_matrices(id).is_err());
        }
        assert!(session.mecc_physical_cycle(owner).is_err());
        let before = session.counts();
        assert!(
            factory
                .construct(&mut session, frame, Blending::Enabled, 1)
                .is_err()
        );
        assert_eq!(session.counts(), before);
        session.clear();
        assert_eq!(session.counts().residents, 0);
        assert_eq!(session.state_resource_counts(), (0, 0));
        assert_eq!(session.actor_field_counts(), (0, 0, 0));
        assert!(session.kabuto_group(owner).is_none());
        let next = factory
            .construct(&mut session, frame, Blending::Disabled, 0)
            .unwrap();
        assert_ne!(next, owner);
        let group = session.kabuto_group(next).unwrap();
        assert!(group.physics.is_none());
        let expected = (f64::from(factory.resources.class_fields().scale)
            * f64::from(globals.scalar(0x10c4).unwrap())) as f32;
        assert_eq!(
            session
                .get(next)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .scale,
            expected
        );
        assert!(group.radius.is_finite());
        assert_eq!(session.counts().residents, 12);
        session.clear();
        assert_eq!(session.actor_field_counts(), (0, 0, 0));
        let executable = std::env::var_os("GIANTS_KABUTO_EXECUTABLE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| root.join("Giants.exe"));
        let definitions = crate::kabuto_animation_cache::Definitions::read(&executable).unwrap();
        factory.prepare_animation_definitions(&definitions).unwrap();
        let owner = factory
            .construct(&mut session, frame, Blending::Enabled, 1)
            .unwrap();
        let group = session.kabuto_group(owner).unwrap();
        let cache = group.cold_animation_cache.as_ref().unwrap();
        assert!(Arc::ptr_eq(cache.specification(), &group.declared_class));
        assert_eq!(cache.entries().len(), 62);
        assert_eq!(
            cache.entries().iter().filter(|e| e.node.is_some()).count(),
            61
        );
        // Cold data does not acknowledge process/world reset scheduling.
        assert!(group.pending.contains(&Pending::AnimationCache));
        let weak = Arc::downgrade(cache);
        factory.refresh_animation_cache().unwrap();
        assert!(Arc::ptr_eq(
            factory.animation_cache.as_ref().unwrap().cached().unwrap(),
            group.cold_animation_cache.as_ref().unwrap()
        ));
        session.clear();
        assert!(weak.upgrade().is_some()); // The resource factory still owns it.
        factory.reset_animation_cache().unwrap();
        let counts = session.counts();
        assert!(
            factory
                .construct(&mut session, frame, Blending::Enabled, 1)
                .is_err()
        );
        assert_eq!(session.counts(), counts);
        // Reset preserves the prior snapshot until a successful refresh.
        assert!(weak.upgrade().is_some());
        factory.refresh_animation_cache().unwrap();
        assert!(weak.upgrade().is_none());
        let mut random = crate::audio_parameters::Random(37);
        let mut expected = random;
        for _ in 0..crate::kabuto_texture::WIDTH {
            expected.next_15();
        }
        let texture = Arc::downgrade(factory.prepare_procedural_texture(&mut random));
        assert_eq!(random.0, expected.0);
        assert!(Arc::ptr_eq(
            &texture.upgrade().unwrap(),
            factory.prepare_procedural_texture(&mut random)
        ));
        assert_eq!(random.0, expected.0);
        let owner = factory
            .construct(&mut session, frame, Blending::Enabled, 1)
            .unwrap();
        let weak = Arc::downgrade(
            session
                .kabuto_group(owner)
                .unwrap()
                .cold_animation_cache
                .as_ref()
                .unwrap(),
        );
        let group = session.kabuto_group(owner).unwrap();
        assert!(Arc::ptr_eq(
            group.procedural_texture.as_ref().unwrap(),
            &texture.upgrade().unwrap()
        ));
        assert!(group.pending.contains(&Pending::ProceduralMaterial));
        session.clear();
        drop(factory);
        assert!(weak.upgrade().is_none());
        assert!(texture.upgrade().is_none());
        let factory = Factory::load(
            &plan,
            packet.world.as_ref().unwrap(),
            packet.mission.as_ref().unwrap(),
            globals,
        )
        .unwrap();
        let mut registry = crate::kabuto_geometry_registry::Registry::new(946);
        let mut wrong_ticket = crate::kabuto_geometry_registry::Registry::new(947);
        let mut random = crate::audio_parameters::Random(1);
        let counts = session.counts();
        assert!(
            factory
                .construct_registered(
                    &mut session,
                    frame,
                    Blending::Enabled,
                    1,
                    &mut wrong_ticket,
                    &mut random
                )
                .is_err()
        );
        assert_eq!(session.counts(), counts);
        assert_eq!(random.0, 1);
        assert!(wrong_ticket.is_empty());
        let owner = factory
            .construct_registered(
                &mut session,
                frame,
                Blending::Enabled,
                1,
                &mut registry,
                &mut random,
            )
            .unwrap();
        let group = session.kabuto_group(owner).unwrap();
        let handle = group.geometry_registration.as_ref().unwrap().clone();
        let image = Arc::downgrade(group.procedural_texture.as_ref().unwrap());
        let counts = session.counts();
        let lists = (0..15).map(|i| session.list(i)).collect::<Vec<_>>();
        let mut wrong_scope = crate::kabuto_geometry_registry::Registry::new(946);
        assert!(
            session
                .cleanup_registered_kabuto(owner, &mut wrong_scope)
                .is_err()
        );
        assert!(!session.kabuto_group(owner).unwrap().cleaned_up());
        let cleanup = session
            .cleanup_registered_kabuto(owner, &mut registry)
            .unwrap();
        assert_eq!(cleanup.marked.len(), 11);
        assert!(cleanup.material_retired);
        assert!(registry.is_empty());
        assert!(image.upgrade().is_none());
        assert_eq!(session.counts(), counts);
        assert_eq!((0..15).map(|i| session.list(i)).collect::<Vec<_>>(), lists);
        for id in cleanup.marked {
            assert_ne!(
                session
                    .get(id)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap()
                    .object_flags
                    & 0x8000,
                0
            );
        }
        assert_eq!(session.get(owner).unwrap().seed.storage.state(), 1);
        assert!(session.kabuto_group(owner).unwrap().cleaned_up());
        assert!(
            session
                .kabuto_group(owner)
                .unwrap()
                .geometry
                .vertices
                .is_empty()
        );
        assert!(
            session
                .cleanup_registered_kabuto(owner, &mut registry)
                .is_err()
        );
        let mut expected = crate::audio_parameters::Random(1);
        for _ in 0..128 {
            expected.next_15();
        }
        assert_eq!(random.0, expected.0);
        session.clear();
        let owner = factory
            .construct_registered(
                &mut session,
                frame,
                Blending::Enabled,
                1,
                &mut registry,
                &mut random,
            )
            .unwrap();
        assert!(registry.owner(&handle).is_err());
        for _ in 0..128 {
            expected.next_15();
        }
        assert_eq!(random.0, expected.0);
        session
            .cleanup_registered_kabuto(owner, &mut registry)
            .unwrap();
        session.clear();
    }
}
