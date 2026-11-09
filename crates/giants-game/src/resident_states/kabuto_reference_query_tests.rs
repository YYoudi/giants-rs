use super::*;
pub(super) fn probe(
    factory: &crate::kabuto_initial::Factory,
    foreign: &crate::kabuto_initial::Factory,
    s: &mut Session,
    owner: Id,
    number: u64,
) {
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
    let ordinary = Resources::load(
        &plan,
        packet.world.as_ref().unwrap(),
        packet.mission.as_ref().unwrap(),
    )
    .unwrap();
    let target = ordinary.construct_ordinary(s).unwrap();
    let p = s.get(owner).unwrap().seed.storage.data().unwrap();
    let ray_parameters = crate::kabuto_reference_ray::Parameters::from_owned_payload(p).unwrap();
    let mask = (0..3).fold(0, |mask, i| {
        mask | u32::from_le_bytes(p[0x334 + i * 4..0x338 + i * 4].try_into().unwrap())
    });
    assert_ne!(mask, 0);
    // Declared visibility/category context on real constructed actors; the
    // analytic boxes below are not claimed as a live world-bounds producer.
    let saved: Vec<_> = s
        .list(0)
        .into_iter()
        .filter(|id| *id != owner)
        .map(|id| {
            let f = s
                .residents
                .get_mut(&id.serial)
                .unwrap()
                .construction_fields
                .as_mut()
                .unwrap();
            let saved = (id, f.object_flags, f.configuration[16]);
            if id == target {
                f.object_flags &= !0x4000c000;
                f.configuration[16] = mask;
            } else {
                f.object_flags |= 0x4000;
            }
            saved
        })
        .collect();
    let position = s
        .get(target)
        .unwrap()
        .construction_fields
        .as_ref()
        .unwrap()
        .position;
    let origin = [position[0] - 10., position[1], position[2]];
    let ray = || crate::kabuto_reference_query::Ray {
        origin,
        direction: [1., 0., 0.],
        forward_limit: 20.,
        score_limit: 1.,
    };
    let before = super::motion_tests::snapshot(s, owner);
    let bounds = |s: &Session, id: Id| {
        assert_eq!(id, target);
        let p = s
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        Ok((p.map(|v| v - 0.25), p.map(|v| v + 0.25)))
    };
    assert!(
        foreign
            .query_reference(s, owner, number, ray(), |_, _| panic!(
                "foreign query entered producer"
            ))
            .is_err()
    );
    assert!(
        factory
            .query_reference(s, owner, number + 1, ray(), |_, _| panic!(
                "wrong frame entered producer"
            ))
            .is_err()
    );
    for _ in 0..2 {
        let found = factory
            .query_reference(s, owner, number, ray(), bounds)
            .unwrap();
        assert_eq!(found.target, Some(target));
        assert_eq!(found.score, 0.);
    }
    // Compose acquired argument production with real owned size parameters.
    // This declared viewport is an analytic input, not the live camera producer.
    struct View(crate::kabuto_reference_ray::Viewport);
    impl crate::kabuto_reference_ray::Geometry for View {
        fn actor(&mut self) -> Result<crate::kabuto_reference_ray::ActorGeometry, String> {
            panic!("viewport route requested actor geometry")
        }
        fn viewport(&mut self) -> Result<crate::kabuto_reference_ray::Viewport, String> {
            Ok(crate::kabuto_reference_ray::Viewport {
                origin: self.0.origin,
                direction: self.0.direction,
            })
        }
    }
    let [offset, range] = ray_parameters.pairs[1];
    let distance = (f64::from(range) - f64::from(offset)) * 0.5;
    assert!(distance > 0.);
    let produced = crate::kabuto_reference_ray::prepare(
        crate::kabuto_reference_ray::Context {
            words: [0; 3],
            active: 0,
            current_state: None,
        },
        false,
        ray_parameters,
        &mut View(crate::kabuto_reference_ray::Viewport {
            origin: [
                (f64::from(position[0]) - distance - f64::from(offset)) as f32,
                position[1],
                position[2],
            ],
            direction: [1., 0., 0.],
        }),
    )
    .unwrap();
    assert_eq!(
        factory
            .query_reference(s, owner, number, produced, bounds)
            .unwrap()
            .target,
        Some(target)
    );
    assert!(
        factory
            .query_reference(s, owner, number, ray(), |_, _| Err(
                "missing current bounds".into()
            ))
            .is_err()
    );
    s.residents.get_mut(&target.serial).unwrap().attachment =
        Some(crate::resident_states::Attachment {
            parent: owner,
            child_reference: 0,
            parent_reference: 0,
            behavior: 0,
        });
    assert_eq!(
        factory
            .query_reference(s, owner, number, ray(), |_, _| panic!(
                "rejected parent consumed bounds"
            ))
            .unwrap()
            .target,
        None
    );
    s.clear_actor_parent(target).unwrap();
    assert_eq!(
        s.get(target).unwrap().attachment_words(),
        Some(crate::resident_states::AttachmentWords {
            child_reference: 0,
            parent_reference: 0,
            behavior: 0,
        })
    );
    assert_eq!(before, super::motion_tests::snapshot(s, owner));
    let g = s.kabuto_group(owner).unwrap();
    assert_eq!(g.resource_phase_completed, Some(number));
    assert_eq!(g.motion_history, Some(number));
    assert_eq!(g.pending.len(), 4);
    for (id, flags, mask) in saved {
        let f = s
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap();
        f.object_flags = flags;
        f.configuration[16] = mask;
    }
}
