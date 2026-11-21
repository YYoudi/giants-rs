use super::reference_frame_tests::Case;
use super::*;
use crate::{
    kabuto_geometry_mode as mode, kabuto_initial::Factory, kabuto_query_geometry as geometry,
};

struct World {
    owner: Id,
    target: Id,
    fail: bool,
    notifications: Vec<(mode::Notification, mode::State<Id>)>,
    sounds: Vec<crate::kabuto_geometry_transition::Sound>,
    controls: usize,
    cameras: usize,
}
impl geometry::Services for World {
    fn notify(&mut self, s: &Session, owner: Id, event: mode::Notification) {
        assert_eq!(owner, self.owner);
        self.notifications
            .push((event, s.kabuto_query_geometry_mode_state(owner).unwrap()));
    }
    fn sound(
        &mut self,
        s: &Session,
        event: crate::kabuto_geometry_transition::Sound,
    ) -> Result<(), String> {
        assert_eq!(event.owner, self.owner);
        assert_eq!(s.kabuto_query_geometry_mode_state(self.owner)?.mode, 3);
        assert!(
            s.kabuto_group(self.owner)
                .unwrap()
                .query_geometry
                .mode_completed
                .is_some()
        );
        self.sounds.push(event);
        Ok(())
    }
    fn controlled(&mut self, s: &Session) -> Result<Option<Id>, String> {
        self.controls += 1;
        assert!(
            !s.kabuto_group(self.owner)
                .unwrap()
                .query_geometry
                .header
                .unwrap()
                .enabled
        );
        Ok(Some(self.owner))
    }
    fn camera(
        &mut self,
        _: &Session,
        owner: Id,
    ) -> Result<crate::kabuto_geometry_state::Camera, String> {
        assert_eq!(owner, self.owner);
        self.cameras += 1;
        if self.fail {
            return Err("declared camera projection unavailable".into());
        }
        // Explicit supplied camera input, not ordinary App camera acceptance.
        Ok(crate::kabuto_geometry_state::Camera {
            position: [10., 20., 30.],
            direction: [0., 1., 0.],
        })
    }
    fn attachment(&mut self, _: &Session, _: Id) -> Result<[f32; 3], String> {
        panic!("idle/selected-camera branches must not acquire body attachment")
    }
    fn bounds(
        &mut self,
        _: &Session,
        target: Id,
    ) -> Result<crate::kabuto_reference_target::Bounds, String> {
        assert_eq!(target, self.target);
        panic!("idle/selected-camera branches must not acquire body target bounds")
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn probe(
    factory: &Factory,
    foreign: &Factory,
    s: &mut Session,
    owner: Id,
    target: Id,
    number: u64,
    case: Case,
    animation: &mut crate::actor_animation_phase::Services<'_>,
) {
    let active = case != Case::GeometryIdle;
    let fail = case == Case::GeometryFailure;
    let header = s
        .kabuto_group(owner)
        .unwrap()
        .query_geometry
        .header
        .unwrap();
    assert!(!header.enabled);
    assert_eq!(header.weight, 0.);
    assert_eq!(header.position, [0.; 3]);
    let a = s.residents.get_mut(&owner.serial).unwrap();
    let p = a.seed.storage.data_mut().unwrap();
    let flags = u32::from_le_bytes(p[..4].try_into().unwrap());
    let flags = if active {
        flags | 0x80001
    } else {
        flags & !0x80000
    };
    p[..4].copy_from_slice(&flags.to_le_bytes());
    p[0x224..0x228].copy_from_slice(&0.5f32.to_le_bytes());
    p[0x228] = u8::from(active);
    p[0x310..0x314].copy_from_slice(&7u32.to_le_bytes());
    // Declared late caller packet for this independent CPU context.
    a.kabuto_group.as_mut().unwrap().motion.commands[4..8]
        .copy_from_slice(&(if active { 4u32 } else { 0 }).to_le_bytes());
    let (_, extra) = super::transition_tests::factories();
    let mut world = World {
        owner,
        target,
        fail,
        notifications: vec![],
        sounds: vec![],
        controls: 0,
        cameras: 0,
    };
    let network = mode::NotificationContext {
        session_mode: 4,
        peer_ids: &[19],
        local_peer: 19,
        owner_word: 0,
    };
    // Intentionally differs from the consumed query clock .25: mode uses the
    // actual supplied raw global frame interval .125, not object-specific time.
    let frame = || geometry::Frame {
        global_seconds: 0.125,
        areas: &[],
        network: &network,
        materials: None,
    };
    assert!(factory.captured_query_geometry(s, owner, number).is_err());
    assert!(
        foreign
            .advance_query_geometry(s, owner, number, &extra, frame(), animation, &mut world)
            .is_err()
    );
    assert!(
        factory
            .advance_query_geometry(s, owner, number + 1, &extra, frame(), animation, &mut world)
            .is_err()
    );
    assert_eq!(
        world.controls + world.cameras + world.notifications.len() + world.sounds.len(),
        0
    );
    let wrong_network = mode::NotificationContext {
        session_mode: 4,
        peer_ids: &[19],
        local_peer: 19,
        owner_word: 1,
    };
    assert!(
        factory
            .advance_query_geometry(
                s,
                owner,
                number,
                &extra,
                geometry::Frame {
                    network: &wrong_network,
                    ..frame()
                },
                animation,
                &mut world
            )
            .unwrap_err()
            .contains("actual actor word")
    );
    assert!(
        s.kabuto_group(owner)
            .unwrap()
            .query_geometry
            .entered
            .is_none()
    );
    assert_eq!(
        world.controls + world.cameras + world.notifications.len() + world.sounds.len(),
        0
    );
    let before = s.counts().residents;
    let result =
        factory.advance_query_geometry(s, owner, number, &extra, frame(), animation, &mut world);
    assert_eq!(world.controls, 1);
    let g = s.kabuto_group(owner).unwrap();
    assert_eq!(g.query_geometry.mode_completed, Some(number));
    assert!(!g.query_geometry.running);
    assert_eq!(g.pending.len(), 4);
    assert_eq!(g.motion_history, Some(number));
    if active {
        assert_eq!(world.cameras, 1);
        assert_eq!(world.notifications.len(), 2);
        let (event, first) = world.notifications[0];
        assert!(matches!(event, mode::Notification::Target(_)));
        assert_eq!(
            (first.mode, first.target, first.target_status),
            (1, None, 7)
        );
        assert_eq!(first.duration, 0.625);
        assert_eq!(first.scalar, 0.3125);
        let (event, second) = world.notifications[1];
        assert_eq!(event, mode::Notification::Mode(3));
        assert_eq!(
            (second.mode, second.target, second.target_status),
            (3, Some(target), 0)
        );
        assert!(!world.sounds.is_empty());
        assert!(g.extra.is_some());
        assert_eq!(s.counts().residents, before + 1);
    } else {
        assert_eq!(world.cameras, 0);
        assert!(world.notifications.is_empty());
    }
    if fail {
        assert!(result.unwrap_err().contains("projection unavailable"));
        assert!(!g.query_geometry.header.unwrap().enabled);
        assert_eq!(g.query_geometry.header.unwrap().weight, 0.3125);
        assert!(g.geometry_transition_failure.is_some());
        assert!(factory.captured_query_geometry(s, owner, number).is_err());
    } else {
        let view = result.unwrap();
        assert_eq!((view.owner, view.number), (owner, number));
        assert_eq!(
            view.next,
            if active {
                geometry::Next::PrimaryTailRequired
            } else {
                geometry::Next::ReactionAdmission
            }
        );
        assert_eq!(view.mode, if active { 3 } else { 0 });
        assert_eq!(view.header.enabled, active);
        if active {
            assert_eq!(view.header.weight, 0.3125);
            assert_eq!(view.header.position, [10., 40., 30.]);
            assert_eq!(view.header.flags, 0x60);
            assert_eq!(view.header.state_word, 2);
        }
        assert!(factory.captured_query_geometry(s, owner, number).is_ok());
        assert!(
            factory
                .captured_query_geometry(s, owner, number + 1)
                .is_err()
        );
    }
    let calls = world.controls + world.cameras + world.notifications.len() + world.sounds.len();
    assert!(
        factory
            .advance_query_geometry(s, owner, number, &extra, frame(), animation, &mut world)
            .is_err()
    );
    assert_eq!(
        world.controls + world.cameras + world.notifications.len() + world.sounds.len(),
        calls
    );
}
