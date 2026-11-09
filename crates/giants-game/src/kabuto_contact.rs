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

#[cfg(test)]
mod tests {
    use super::*;

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
