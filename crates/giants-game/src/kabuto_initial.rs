//! Ticket-owned composition of cold Kabuto actor fields, auxiliary links and
//! physics. Missing model/material/camera services are explicit pending work;
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
    ModelCallbacks,
    ProceduralMaterial,
    GeometryRegistration,
}
pub struct Group {
    pub owner: Id,
    pub first: Id,
    pub second: Id,
    pub effects: [Id; 8],
    pub final_effect: Id,
    pub declared_class: Arc<specification::Prepared>,
    pub geometry: Geometry,
    pub radius: f32,
    pub physics: Option<Physics>,
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
pub struct Factory {
    resources: Resources,
    auxiliary: Auxiliaries,
    globals: Arc<GlobalData>,
    bounds: Resolved,
}
impl Factory {
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
        })
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
        session.stage_kabuto_group(|s| self.build(s, frame, blending, generation))
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
            declared_class: self.resources.specification.clone(),
            geometry,
            radius: prepared.radius,
            physics: prepared.physics,
            pending: [
                Pending::ProjectionReset,
                Pending::ModelCallbacks,
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
        let factory = Factory::load(
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
    }
}
