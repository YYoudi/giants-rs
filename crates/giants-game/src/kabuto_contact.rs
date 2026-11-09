//! Acquired state1 contact policy. Peer support writes are returned explicitly;
//! applying them to a scene/Session and full primary scheduling remain separate.
#[derive(Clone, Copy, Debug)]
pub struct Peer {
    pub class: i32,
    pub upper_z: f32,
    pub radius: f32,
    pub collision_mask: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    /// A handled event suppresses the ordinary object-plane response.
    pub handled: bool,
    /// The caller must apply this peer-side effect even when `handled` is true.
    pub bind_peer_support: bool,
}
/// Acquired finite PC profile. Null and class-669 peers bypass numeric reads.
/// These mask bits have no assigned gameplay meaning or prototype PS2 layout.
pub fn decide(position_z: f32, scale: f32, peer: Option<Peer>) -> Result<Decision, &'static str> {
    let Some(peer) = peer.filter(|p| p.class != 669) else {
        return Ok(Decision {
            handled: true,
            bind_peer_support: false,
        });
    };
    if ![position_z, scale, peer.upper_z, peer.radius]
        .into_iter()
        .all(f32::is_finite)
    {
        return Err("Kabuto contact policy outside finite peer profile");
    }
    let mut mask = if f64::from(peer.upper_z) > f64::from(scale) * 35. + f64::from(position_z) {
        7u8
    } else if f64::from(peer.upper_z) > f64::from(scale) * 20. + f64::from(position_z)
        || f64::from(peer.radius) * f64::from(scale) > 20.
    {
        3
    } else {
        1
    };
    let flags = peer.collision_mask;
    if flags & 1 != 0 {
        return Ok(Decision {
            handled: true,
            bind_peer_support: false,
        });
    }
    if flags & 0x3c00 != 0 {
        mask &= !1;
    }
    if flags & 0x3c0 != 0 {
        mask &= !2;
    }
    if flags & 0x3c != 0 {
        mask &= !4;
    }
    Ok(Decision {
        handled: mask == 0,
        bind_peer_support: flags & 2 == 0,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct FollowupOwner<K> {
    pub identity: K,
    pub state: u32,
    /// Opaque acquired state scalar, consumed only for a state-1 request.
    pub factor: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct FollowupPeer<K> {
    pub identity: K,
    pub flags: u32,
    pub reference: Option<K>,
}
#[derive(Clone, Copy, Debug)]
pub struct Request<K> {
    pub action: u32,
    pub amount: f32,
    pub owner: Option<K>,
    pub peer: K,
    pub trailing: [u32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Followup {
    /// Owned publication result, not the native callback's handled return.
    pub bind_peer_reference: bool,
}
/// Additional-contact callback, distinct from `decide`. Its native return is
/// always zero. The service may mutate peer flags before the reference store.
/// The request's eventual gameplay effects and parent traversal remain upstream.
pub fn followup<K: Copy>(
    owner: Option<FollowupOwner<K>>,
    peer: Option<&mut FollowupPeer<K>>,
    contact_flags: u32,
    base_amount: f32,
    context: &crate::kabuto_geometry_mode::NotificationContext<'_>,
    mut execute: impl FnMut(Request<K>, &mut FollowupPeer<K>) -> Result<(), &'static str>,
) -> Result<Followup, &'static str> {
    let Some(peer) = peer.filter(|p| p.flags & 1 == 0) else {
        return Ok(Followup {
            bind_peer_reference: false,
        });
    };
    if peer.flags & 0x3c0_0000 != 0 && contact_flags & 1 != 0 {
        if owner.is_none() && context.session_mode == 4 {
            return Err("contact authority requires its actual owner in multiplayer profile");
        }
        let eligible = context.session_mode != 4 || context.eligible()?;
        if eligible {
            let amount = if let Some(owner) = owner.filter(|o| o.state == 1) {
                (f64::from(base_amount) * f64::from(owner.factor)) as f32
            } else {
                base_amount
            };
            if !amount.is_finite() {
                return Err("additional contact request outside finite amount profile");
            }
            execute(
                Request {
                    action: 11,
                    amount,
                    owner: owner.map(|o| o.identity),
                    peer: peer.identity,
                    trailing: [0; 2],
                },
                peer,
            )?;
        }
    }
    // Reread only this bit after the service. A newly set bit1 does not repeat
    // the entry bypass and cannot suppress this reference write on its own.
    let bind_peer_reference = peer.flags & 2 == 0;
    if bind_peer_reference {
        peer.reference = owner.map(|o| o.identity);
    }
    Ok(Followup {
        bind_peer_reference,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> crate::kabuto_geometry_mode::NotificationContext<'static> {
        crate::kabuto_geometry_mode::NotificationContext {
            session_mode: 0,
            peer_ids: &[],
            local_peer: 0,
            owner_word: 0,
        }
    }
    #[test]
    fn additional_request_precedes_reference_and_rereads_only_final_exclusion() {
        let owner = Some(FollowupOwner {
            identity: 9,
            state: 1,
            factor: 0.5,
        });
        for (changed_flags, writes) in [(1, true), (2, false)] {
            let mut peer = FollowupPeer {
                identity: 77,
                flags: 0x40_0000,
                reference: Some(3),
            };
            let result = followup(owner, Some(&mut peer), 1, 6., &context(), |r, p| {
                assert_eq!(
                    (r.action, r.amount, r.owner, r.peer, r.trailing),
                    (11, 3., Some(9), 77, [0, 0])
                );
                assert_eq!(p.reference, Some(3));
                p.flags = changed_flags;
                p.reference = Some(4);
                Ok(())
            })
            .unwrap();
            assert_eq!(result.bind_peer_reference, writes);
            assert_eq!(peer.reference, Some(if writes { 9 } else { 4 }));
        }
    }
    #[test]
    fn additional_bypasses_leave_unused_scalars_and_services_unconsumed() {
        let owner = Some(FollowupOwner {
            identity: 9,
            state: 2,
            factor: f32::NAN,
        });
        let mut peer = FollowupPeer {
            identity: 77,
            flags: 1,
            reference: Some(3),
        };
        assert!(
            !followup(owner, Some(&mut peer), 1, f32::NAN, &context(), |_, _| Err(
                "bypassed"
            ))
            .unwrap()
            .bind_peer_reference
        );
        assert_eq!(peer.reference, Some(3));
        peer.flags = 0;
        assert!(
            followup(owner, Some(&mut peer), 0, f32::NAN, &context(), |_, _| Err(
                "no request"
            ))
            .unwrap()
            .bind_peer_reference
        );
        assert_eq!(peer.reference, Some(9));
        peer.flags = 0x40_0000;
        followup(owner, Some(&mut peer), 1, 6., &context(), |r, _| {
            assert_eq!(r.amount, 6.); // Non-state1 never reads the NaN factor.
            Ok(())
        })
        .unwrap();
    }

    fn peer(upper_z: f32, radius: f32, collision_mask: u32) -> Peer {
        Peer {
            class: 2,
            upper_z,
            radius,
            collision_mask,
        }
    }

    #[test]
    fn bypasses_do_not_consume_invalid_numeric_fields() {
        let expected = Decision {
            handled: true,
            bind_peer_support: false,
        };
        assert_eq!(decide(f32::NAN, f32::NAN, None).unwrap(), expected);
        let mut p = peer(f32::NAN, f32::NAN, 0);
        p.class = 669;
        assert_eq!(decide(f32::NAN, f32::NAN, Some(p)).unwrap(), expected);
        p.class = 2;
        assert!(decide(0., 1., Some(p)).is_err());
    }

    #[test]
    fn strict_height_and_scaled_radius_thresholds_select_separate_masks() {
        // Mask bit 1 is excluded; a selected bit 2 or 4 still leaves an event unhandled.
        assert!(
            decide(10., 2., Some(peer(50., 10., 0x3c00)))
                .unwrap()
                .handled
        );
        assert!(
            !decide(10., 2., Some(peer(50.000004, 10., 0x3c00)))
                .unwrap()
                .handled
        );
        assert!(
            !decide(10., 2., Some(peer(50., 10.000001, 0x3c00)))
                .unwrap()
                .handled
        );
        // At the 35 threshold mask 3 applies; strictly above it mask 7 applies.
        assert!(
            decide(10., 2., Some(peer(80., 0., 0x3fc0)))
                .unwrap()
                .handled
        );
        assert!(
            !decide(10., 2., Some(peer(80.00001, 0., 0x3fc0)))
                .unwrap()
                .handled
        );
    }

    #[test]
    fn peer_support_write_is_independent_of_final_handled_result() {
        let handled = decide(0., 1., Some(peer(0., 0., 0x3ffc))).unwrap();
        assert_eq!(
            handled,
            Decision {
                handled: true,
                bind_peer_support: true
            }
        );
        let unhandled = decide(0., 1., Some(peer(0., 0., 0))).unwrap();
        assert_eq!(
            unhandled,
            Decision {
                handled: false,
                bind_peer_support: true
            }
        );
        assert_eq!(
            decide(0., 1., Some(peer(0., 0., 1))).unwrap(),
            Decision {
                handled: true,
                bind_peer_support: false
            }
        );
        assert_eq!(
            decide(0., 1., Some(peer(0., 0., 2))).unwrap(),
            Decision {
                handled: false,
                bind_peer_support: false
            }
        );
    }
}
