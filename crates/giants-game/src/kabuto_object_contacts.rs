//! Additional object-contact traversal with the acquired fixed Kabuto callback.
//! Prepared membership, references, poses and action execution are explicit.
use crate::{
    actor_affine::Cache,
    arithmetic::Precision,
    contact_pose::Cached,
    kabuto_contact::{self, FollowupOwner, FollowupPeer, Request},
    kabuto_geometry_mode::NotificationContext,
    uncached_pose::Pose,
    volume_queries::Bounds,
};

pub struct Owner<K> {
    pub identity: K,
    pub position: [f32; 3],
    pub scale: f32,
    pub state: u32,
    pub factor: f32,
    pub owner_word: u32,
    pub affine: Option<Cache>,
    /// Local translations of the acquired reference slots, in source order.
    pub references: Option<[[f32; 3]; 2]>,
}
pub struct Peer<K> {
    pub contact: FollowupPeer<K>,
    pub object_flags: u32,
    pub collision_flags: u32,
    pub class_present: bool,
    pub parent: Option<K>,
    pub upper_z: f32,
    pub radius: f32,
    pub pose: Pose,
    pub affine: Option<Cache>,
    pub bounds: Option<Bounds>,
}
pub struct Frame<'a> {
    pub disabled: bool,
    /// A missing table word is refused only at its actual load boundary.
    pub base_amount: Result<f32, &'static str>,
    pub session_mode: u32,
    pub peer_ids: &'a [u32],
    pub local_peer: u32,
}
pub struct Scene<'a> {
    pub owner: u32,
    pub disabled: bool,
    pub peers: &'a mut [Peer<u32>],
    pub context: &'a NotificationContext<'a>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Probe {
    pub position: [f32; 3],
    pub height: f32,
    pub radius: f32,
}
#[derive(Clone, Copy, Debug)]
pub enum Event<K> {
    Probes {
        peer: K,
        mask: u32,
        probes: [Probe; 2],
        count: usize,
    },
    Contact {
        peer: K,
        mask: u32,
        flags: u32,
        reference: Option<K>,
    },
    Reference {
        peer: K,
        reference: Option<K>,
    },
    BoxQuery {
        peer: K,
        point: [f32; 3],
        extent: f32,
        bounds: Bounds,
    },
    Distance {
        peer: K,
        distance: f32,
    },
}
fn finite(values: impl IntoIterator<Item = f32>) -> Result<(), &'static str> {
    if values.into_iter().all(f32::is_finite) {
        Ok(())
    } else {
        Err("additional contact outside finite profile")
    }
}
fn reference_world(cache: Cache, p: [f32; 3]) -> Result<[f32; 3], &'static str> {
    finite(p)?;
    let m = cache.matrix.map(f64::from);
    let p = p.map(f64::from);
    let mut out = p;
    if cache.flags & 0xb != 0 {
        // The batch transform's third row uses a different accumulation order
        // from its first two rows and from the ordinary cached point helper.
        for a in 0..2 {
            out[a] = (m[a * 4 + 1] * p[1] + m[a * 4] * p[0]) + m[a * 4 + 2] * p[2];
        }
        out[2] = (m[10] * p[2] + m[9] * p[1]) + m[8] * p[0];
    }
    if cache.flags & 0x10 != 0 {
        for a in 0..3 {
            out[a] += m[a * 4 + 3];
        }
    }
    let out = out.map(|v| v as f32);
    finite(out)?;
    Ok(out)
}
fn local<K>(peer: &Peer<K>, p: [f32; 3]) -> Result<[f32; 3], &'static str> {
    if peer.object_flags & 0x20 != 0 {
        let cache = peer.affine.ok_or("additional contact peer affine absent")?;
        return Cached::new(0x20, cache.flags, cache.matrix)
            .map_err(|_| "additional contact inverse cache outside profile")?
            .to_local(p);
    }
    let delta: [f32; 3] =
        std::array::from_fn(|i| (f64::from(p[i]) - f64::from(peer.pose.position[i])) as f32);
    let m = crate::uncached_pose::matrix(
        Pose {
            position: [0.; 3],
            angles: peer.pose.angles,
            scale: (1. / f64::from(peer.pose.scale)) as f32,
        },
        Precision::Double53,
    )?;
    let out = std::array::from_fn(|a| {
        ((f64::from(m[8 + a]) * f64::from(delta[2]) + f64::from(m[4 + a]) * f64::from(delta[1]))
            + f64::from(m[a]) * f64::from(delta[0])) as f32
    });
    finite(out)?;
    Ok(out)
}
/// The fixed original callback always returns zero: after an overlap it executes
/// its real request/reference effects, then the traversal advances to the next
/// peer. The later response blocks are not reachable through that callback.
/// The returned native collision-result bit is therefore false, not a primary-
/// completion acknowledgment. Errors retain earlier owner/peer service effects.
pub fn traverse<K: Copy + Eq>(
    owner: &mut Owner<K>,
    peers: &mut [Peer<K>],
    frame: Frame<'_>,
    mut execute: impl FnMut(Request<K>, &mut Owner<K>, &mut [Peer<K>]) -> Result<(), &'static str>,
    mut observe: impl FnMut(Event<K>),
) -> Result<bool, &'static str> {
    if frame.disabled {
        return Ok(false);
    }
    finite(owner.position.into_iter().chain([owner.scale]))?;
    let upper = (f64::from(owner.scale) * 35. + f64::from(owner.position[2])) as f32;
    let lower = (f64::from(owner.scale) * 20. + f64::from(owner.position[2])) as f32;
    let owner_key = owner.identity;
    let keys: Vec<_> = peers.iter().map(|p| p.contact.identity).collect();
    for i in 0..peers.len() {
        if peers[i].contact.identity != keys[i] {
            return Err("additional contact prepared membership identity changed");
        }
        let peer = &peers[i];
        if peer.object_flags & 0x4000_c000 != 0
            || peer.collision_flags & 0x800 != 0
            || peer.contact.flags & 1 != 0
            || peer.contact.identity == owner.identity
            || !peer.class_present
            || peer.parent == Some(owner.identity)
        {
            continue;
        }
        finite([peer.upper_z])?;
        let mut probes = [Probe::default(); 2];
        let (mask, count) = if peer.upper_z > upper {
            probes[0] = Probe {
                position: owner.position,
                height: (f64::from(owner.scale) * 75.) as f32,
                radius: (f64::from(owner.scale) * 25.) as f32,
            };
            (4, 1)
        } else if peer.upper_z > lower || {
            finite([peer.radius])?;
            f64::from(peer.radius) * f64::from(owner.scale) > 20.
        } {
            probes[0] = Probe {
                position: owner.position,
                height: (f64::from(owner.scale) * 75.) as f32,
                radius: (f64::from(owner.scale) * 20.) as f32,
            };
            (2, 1)
        } else {
            let refs = owner
                .references
                .ok_or("additional contact references absent")?;
            let cache = owner
                .affine
                .ok_or("additional contact owner affine absent")?;
            for n in (0..2).rev() {
                probes[n] = Probe {
                    position: reference_world(cache, refs[n])?,
                    height: (f64::from(owner.scale) * 20.) as f32,
                    radius: (f64::from(owner.scale) * 10.) as f32,
                };
            }
            (1, 2)
        };
        for p in &probes[..count] {
            finite(p.position.into_iter().chain([p.height, p.radius]))?;
        }
        observe(Event::Probes {
            peer: peer.contact.identity,
            mask,
            probes,
            count,
        });
        let mut overlaps = false;
        for probe in &probes[..count] {
            if peer.collision_flags & 0x80000 != 0 {
                let point = local(peer, probe.position)?;
                let extent = (f64::from(probe.radius) / f64::from(peer.pose.scale)) as f32;
                let bounds = peer.bounds.ok_or("additional contact box bounds absent")?;
                observe(Event::BoxQuery {
                    peer: peer.contact.identity,
                    point,
                    extent,
                    bounds,
                });
                overlaps = crate::volume_queries::expanded_point(
                    point,
                    extent,
                    bounds,
                    Precision::Double53,
                )
                .ok_or("additional contact box query outside profile")?;
            } else {
                if probe.position[2] >= peer.upper_z {
                    continue;
                }
                finite([peer.pose.position[2]])?;
                if f64::from(probe.position[2]) + f64::from(probe.height)
                    <= f64::from(peer.pose.position[2])
                {
                    continue;
                }
                finite(peer.pose.position.into_iter().chain([peer.radius]))?;
                let dx = f64::from(peer.pose.position[0]) - f64::from(probe.position[0]);
                let dy = f64::from(peer.pose.position[1]) - f64::from(probe.position[1]);
                let distance = (dx * dx + dy * dy).sqrt() as f32;
                observe(Event::Distance {
                    peer: peer.contact.identity,
                    distance,
                });
                overlaps =
                    f64::from(peer.radius) + f64::from(probe.radius) - f64::from(distance) >= 0.;
            }
            if overlaps {
                break;
            }
        }
        if !overlaps {
            continue;
        }
        let mut contact = peers[i].contact;
        observe(Event::Contact {
            peer: contact.identity,
            mask,
            flags: contact.flags,
            reference: contact.reference,
        });
        let context = NotificationContext {
            session_mode: frame.session_mode,
            peer_ids: frame.peer_ids,
            local_peer: frame.local_peer,
            owner_word: owner.owner_word,
        };
        let amount = if contact.flags & 0x3c0_0000 != 0 && mask & 1 != 0 {
            frame.base_amount?
        } else {
            0.
        };
        let result = kabuto_contact::followup(
            Some(FollowupOwner {
                identity: owner.identity,
                state: owner.state,
                factor: owner.factor,
            }),
            Some(&mut contact),
            mask,
            amount,
            &context,
            |request, p| {
                peers[i].contact = *p;
                let result = execute(request, owner, peers);
                *p = peers[i].contact;
                result?;
                if owner.identity != owner_key || p.identity != keys[i] {
                    return Err("additional contact action replaced an identity");
                }
                Ok(())
            },
        );
        peers[i].contact = contact;
        let result = result?;
        if result.bind_peer_reference {
            observe(Event::Reference {
                peer: contact.identity,
                reference: contact.reference,
            });
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn owner() -> Owner<u32> {
        Owner {
            identity: 1,
            position: [0.; 3],
            scale: 1.,
            state: 1,
            factor: 1.,
            owner_word: 0,
            affine: Some(Cache {
                flags: 0,
                matrix: [0.; 16],
            }),
            references: Some([[0.; 3]; 2]),
        }
    }
    fn peer(key: u32) -> Peer<u32> {
        Peer {
            contact: FollowupPeer {
                identity: key,
                flags: 0,
                reference: None,
            },
            object_flags: 0,
            collision_flags: 0,
            class_present: true,
            parent: None,
            upper_z: 100.,
            radius: 0.,
            pose: Pose {
                position: [25., 0., 1.],
                angles: [f32::NAN; 3],
                scale: 1.,
            },
            affine: None,
            bounds: None,
        }
    }
    fn frame() -> Frame<'static> {
        Frame {
            disabled: false,
            base_amount: Err("unused table"),
            session_mode: 0,
            peer_ids: &[],
            local_peer: 0,
        }
    }
    #[test]
    fn strict_vertical_and_inclusive_radial_edges_preserve_lazy_inputs() {
        let mut o = owner();
        o.factor = f32::NAN;
        o.affine = None;
        o.references = None;
        let mut peers = [peer(2)];
        assert!(
            !traverse(
                &mut o,
                &mut peers,
                frame(),
                |_, _, _| Err("no request"),
                |_| {}
            )
            .unwrap()
        );
        assert_eq!(peers[0].contact.reference, Some(1));
        peers[0].contact.reference = None;
        peers[0].pose.position[2] = 75.;
        traverse(
            &mut o,
            &mut peers,
            frame(),
            |_, _, _| Err("no request"),
            |_| {},
        )
        .unwrap();
        assert_eq!(peers[0].contact.reference, None);
        let mut o = owner();
        peers[0].upper_z = 0.;
        peers[0].pose.position = [0., 0., -1.];
        traverse(
            &mut o,
            &mut peers,
            frame(),
            |_, _, _| Err("no request"),
            |_| {},
        )
        .unwrap();
        assert_eq!(peers[0].contact.reference, None);
        o.position = [f32::NAN; 3];
        let mut disabled = frame();
        disabled.disabled = true;
        assert!(
            !traverse(
                &mut o,
                &mut peers,
                disabled,
                |_, _, _| Err("disabled"),
                |_| panic!("disabled observer")
            )
            .unwrap()
        );
    }
    #[test]
    fn action_mutations_are_reread_before_next_peer_and_reference_store() {
        let mut o = owner();
        let mut peers = [peer(2), peer(3)];
        for p in &mut peers {
            p.contact.flags = 0x3c0_0000;
            p.upper_z = 10.;
            p.radius = 1.;
            p.pose.position = [0., 0., -1.];
        }
        let mut f = frame();
        f.base_amount = Ok(4.);
        let mut requests = 0;
        traverse(
            &mut o,
            &mut peers,
            f,
            |r, o, p| {
                requests += 1;
                assert_eq!((r.peer, r.action, r.amount), (2, 11, 4.));
                p[0].contact.flags = 1;
                p[1].contact.flags = 1;
                o.factor = 2.;
                Ok(())
            },
            |_| {},
        )
        .unwrap();
        assert_eq!(requests, 1);
        assert_eq!(o.factor, 2.);
        assert_eq!(peers[0].contact.reference, Some(1));
        assert_eq!(peers[1].contact.reference, None);
    }
    #[test]
    fn provider_identity_replacement_refuses_before_a_reference_store() {
        let mut o = owner();
        let mut peers = [peer(2)];
        peers[0].upper_z = 10.;
        peers[0].radius = 1.;
        peers[0].pose.position = [0., 0., -1.];
        peers[0].contact.flags = 0x3c0_0000;
        let mut f = frame();
        f.base_amount = Ok(1.);
        let result = traverse(
            &mut o,
            &mut peers,
            f,
            |_, _, p| {
                p[0].contact.identity = 99;
                Ok(())
            },
            |_| {},
        );
        assert!(result.is_err());
        assert_eq!(peers[0].contact.identity, 99);
        assert_eq!(peers[0].contact.reference, None);
    }
}
