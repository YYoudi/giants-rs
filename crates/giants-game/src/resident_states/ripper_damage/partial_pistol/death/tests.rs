//! Installed callback composition, not ordinary collision, playback or GPU proof.
//! Pose, prior damage and attached dirt are explicit constructor fixtures.
use super::*;
use crate::{
    attached_effect_context::{Creation, Pool},
    ripper_health_drop,
};

struct Attach<'a>(&'a mut Pool<Id>);
impl ripper_health_drop::Effects for Attach<'_> {
    fn attach(
        &mut self,
        session: &mut Session,
        item: Id,
        name: &'static str,
        _: &mut Random,
    ) -> Result<Creation, String> {
        let flags = session
            .get(item)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .object_flags;
        self.0.prepare(name, item, flags, u32::MAX)
    }
}
struct Backend<'a> {
    pool: Pool<Id>,
    factory: &'a ripper_health_drop::Factory,
    drop: Option<Id>,
    death: Option<Effect>,
    calls: Vec<&'static str>,
    fail: Option<&'static str>,
    selection: crate::effect_initialization::Selection,
}
impl DeathServices for Backend<'_> {
    fn check(&self, _: &Session, actor: &RipperDamageState) -> Result<(), String> {
        if actor.parent.is_some()
            || actor.owner.is_some()
            || actor.group.is_some()
            || actor.target.is_some()
        {
            return Err("fixture does not supply hierarchy/group/target services".into());
        }
        let effect = actor
            .effect
            .ok_or("fixture requires actual attached context")?;
        if !matches!(self.pool.get(effect).ok_or("context absent")?.binding,
            crate::attached_effect_context::Binding::Attached { recipient, .. } if recipient == actor.id)
        {
            return Err("foreign context".into());
        }
        Ok(())
    }
}
impl State23Services for Backend<'_> {
    fn execute(
        &mut self,
        session: &mut Session,
        actor: &RipperDamageState,
        request: RipperDamageService,
        random: &mut Random,
    ) -> Result<(), String> {
        use state23_damage::Service;
        assert_eq!(session.state23_combat(actor.id), Some(actor));
        let name = match request {
            Service::StopAttached { .. } => "stop",
            Service::Drop { .. } => "drop",
            Service::Effect { .. } => "death",
            Service::PlayPosition { .. } => "position_sound",
            Service::RemoveEffect { .. } => "remove",
            _ => return Err("service outside installed callback fixture".into()),
        };
        self.calls.push(name);
        if self.fail == Some(name) {
            return Err(format!("supplied {name} failure"));
        }
        match request {
            // Deliberate sound queue boundaries, not production playback.
            Service::StopAttached { .. } | Service::PlayPosition { .. } => Ok(()),
            Service::Drop { actor: owner, .. } => {
                self.drop = Some(self.factory.spawn(
                    session,
                    owner,
                    random,
                    &mut Attach(&mut self.pool),
                )?);
                Ok(())
            }
            Service::Effect { name, position } => {
                let mut selection = self.selection;
                selection.active_objects = session.counts().residents as u32;
                let (creation, _) = self
                    .pool
                    .prepare_positional(name, position, selection, random)?;
                let Creation::Prepared(effect) = creation else {
                    return Err("installed death effect absent".into());
                };
                self.death = Some(effect);
                Ok(())
            }
            Service::RemoveEffect { effect } => self.pool.release_prepared(effect).map(|_| ()),
            _ => unreachable!(),
        }
    }
}

#[test]
#[ignore = "Requires GIANTS_NPC_ROOT and private GIANTS_RIPPER_DEATH_NULL_CONTROLS; original mode8/null-VM selection and RNG, supplied count/quality/camera/free storage/TLS; no active body/render proof"]
fn native_ripper_death_root_preparation_preserves_rng_and_context_kind() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources =
        crate::scene_resources::SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    let text =
        std::fs::read_to_string(std::env::var_os("GIANTS_RIPPER_DEATH_NULL_CONTROLS").unwrap())
            .unwrap();
    let mut count = 0;
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let name = parts.next().unwrap();
        assert!(name.to_ascii_lowercase().starts_with("fx_ripper_death_"));
        let w = parts
            .map(|p| u32::from_str_radix(p, 16).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(w.len(), 12);
        let mut pool = Pool::<Id>::from_active_resources(9846, &resources);
        let mut random = Random(w[9]);
        let position = [w[6], w[7], w[8]].map(f32::from_bits);
        let (Creation::Prepared(id), intervals) = pool
            .prepare_positional(
                name,
                position,
                crate::effect_initialization::Selection {
                    active_objects: w[0],
                    quality: f32::from_bits(w[1]),
                    lite_distance: f32::from_bits(w[2]),
                    camera: [w[3], w[4], w[5]].map(f32::from_bits),
                    recipient_position: position,
                },
                &mut random,
            )
            .unwrap()
        else {
            panic!("native root absent");
        };
        let context = pool.get(id).unwrap();
        assert_eq!(context.entry_offset, w[10]);
        assert_eq!(random.0, w[11]);
        assert!(intervals.is_empty());
        assert_eq!(
            context.position().unwrap().map(f32::to_bits),
            position.map(f32::to_bits)
        );
        assert!(context.recipient().is_none() && context.tag().is_none());
        assert_eq!(
            (context.flags(), context.cursor(), context.emitted_objects()),
            (0, 0, 0)
        );
        pool.release_prepared(id).unwrap();
        assert!(pool.active().is_empty());
        count += 1;
    }
    assert_eq!(count, 8);
}
fn terrain() -> giants_formats::gti::Terrain {
    giants_formats::gti::Terrain {
        source_flags: 3,
        width: 4,
        height: 2,
        origin: [0.; 2],
        spacing: 10.,
        height_bounds: [0.; 2],
        fallback_texture: String::new(),
        heights: vec![0.; 8],
        colors: vec![0; 8],
        tags: vec![6; 8],
        consumed_bytes: 0,
        opaque_tail: vec![],
    }
}
fn emitted_hit(owner: Id, catalog: &giants_formats::weapons::Catalog) -> mecc_pistol::OwnedHit {
    let terrain = terrain();
    let (a, b, c, d) = (
        [8., 0., -10.],
        [8., 10., -10.],
        [8., 0., 10.],
        [8., 10., 10.],
    );
    let object = crate::projectile_world::Object {
        id: 2,
        parent: 0,
        active: 0,
        personality: 0x20000,
        bounds: crate::volume_queries::Bounds {
            min: [7.9, 0., -10.],
            max: [8.1, 10., 10.],
        },
        mesh: crate::object_segment::Mesh::new(2, vec![[a, b, c], [c, b, a], [c, b, d], [d, b, c]])
            .unwrap(),
    };
    let mut pistol = mecc_pistol::Pistol::from_initial(
        crate::player_equipment::Initial::non_base(true),
        catalog,
    )
    .unwrap();
    pistol.bind_source(owner).unwrap();
    pistol.source_camera([1., 5., 5.]).unwrap();
    pistol
        .step(0.05, [1., 5., 5.], [29., 5., 5.], true, &terrain, &[object])
        .unwrap()
        .world_hits
        .pop()
        .unwrap()
        .into_source()
        .unwrap()
}
#[test]
#[ignore = "Requires GIANTS_NPC_ROOT; actual partial-pistol emission, health birth and death/context preparations; supplied damage/pose/audio and service failures, no ordinary frame/GPU/OS proof"]
fn source_lethal_partial_pistol_keeps_context_domain_and_failed_prefix() {
    let root = std::path::PathBuf::from(std::env::var_os("GIANTS_NPC_ROOT").unwrap());
    let resources =
        crate::scene_resources::SceneResources::load(&root, "w_Story2.bin", "English").unwrap();
    let globals =
        giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
            .unwrap();
    let factory = ripper_health_drop::Factory::load(&resources, &globals).unwrap();
    let catalog = giants_formats::weapons::parse_with_profile(
        &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
        giants_formats::weapons::Profile::Debug,
    )
    .unwrap();
    for class in [73, 230] {
        for fail in [None, Some("death"), Some("remove")] {
            let mut session = Session::new(9845);
            let target =
                ripper_health_drop::tests::source_at(&resources, &mut session, class, [4., 5., 5.]);
            let shooter =
                ripper_health_drop::tests::source_at(&resources, &mut session, 2, [1., 5., 5.]);
            let terrain = terrain();
            let ground = crate::terrain_queries::StaticTerrainQuery::new(
                &terrain,
                1. / terrain.spacing,
                crate::arithmetic::Precision::Double53,
            )
            .unwrap();
            session.prepare_actor_affine(target).unwrap();
            session
                .prepare_actor_environment(target, &resources, &ground)
                .unwrap();
            session.prepare_actor_finalization(target).unwrap();
            session.prepare_actor_motion(target).unwrap();
            session.begin_actor_frame_history(target).unwrap();
            session.begin_actor_motion_history(target, [0; 8]).unwrap();
            session.prepare_ripper_primary(target).unwrap();
            let mut backend = Backend {
                pool: Pool::from_active_resources(session.ticket(), &resources),
                factory: &factory,
                drop: None,
                death: None,
                calls: vec![],
                fail,
                selection: crate::effect_initialization::Selection {
                    active_objects: session.counts().residents as u32,
                    quality: globals.scalar(0x13e0).unwrap(),
                    lite_distance: globals.scalar(0x1570).unwrap(),
                    camera: [1., 5., 5.], // Supplied fixture camera, not renderer state.
                    recipient_position: [4., 5., 5.],
                },
            };
            let (Creation::Prepared(effect), _) = backend
                .pool
                .prepare_extended(
                    "fx_ripper_dirt",
                    target,
                    0,
                    u32::MAX,
                    backend.selection,
                    &mut Random(17),
                )
                .unwrap()
            else {
                panic!("source dirt context absent")
            };
            let primary = session
                .residents
                .get_mut(&target.serial)
                .unwrap()
                .ripper_primary
                .as_mut()
                .unwrap();
            let mut state = primary.state().clone();
            state.effect = Some(effect);
            primary.refresh_owned_frame(state).unwrap();
            let hit = emitted_hit(shooter, &catalog);
            let mut initial = session.state23_snapshot(target).unwrap();
            initial.damage = initial.limit - hit.damage();
            backend.check(&session, &initial).unwrap();
            let before = session.counts().residents;
            let result = session.partial_pistol_death(
                initial,
                hit,
                state23_damage::Context {
                    player_is_state1: false,
                    drop_percent: globals.words[0x1080 / 4] as i32,
                    death_timer: globals.scalar(0x10f4).unwrap(),
                },
                &mut Random(17),
                &mut backend,
            );
            assert_eq!(
                session.counts().residents,
                before + 1,
                "only the real health child is allocated, never a synthetic projectile"
            );
            let item = session.ripper_health_drop(backend.drop.unwrap()).unwrap();
            assert!(item.initialized && !item.failed && item.context.is_some());
            let combat = session.state23_combat(target).unwrap();
            assert!(combat.damage >= combat.limit);
            if fail.is_none() {
                assert!(
                    matches!(result, Ok(Contact::Died)),
                    "{result:?}; calls={:?}",
                    backend.calls
                );
            }
            assert_eq!(
                session
                    .ripper_primary(target)
                    .unwrap()
                    .state()
                    .effect
                    .is_none(),
                fail.is_none()
            );
            assert_eq!(backend.pool.get(effect).is_none(), fail.is_none());
            assert_eq!(combat.effect.is_none(), fail.is_none());
            if fail.is_none() {
                assert!(matches!(result, Ok(Contact::Died)));
                assert_eq!(combat.mode, 94);
                assert_eq!(
                    &backend.calls[5..],
                    [
                        "drop",
                        "death",
                        "position_sound",
                        "position_sound",
                        "remove"
                    ]
                );
                assert!(backend.death.is_some());
                assert!(session.state23_damage_failure(target).is_none());
            } else {
                assert!(result.is_err());
                assert!(session.state23_damage_failure(target).is_some());
                assert!(session.check_ripper_damage_available(target).is_err());
            }
        }
    }
}
