//! Acquired shared velocity addition used by Kabuto's following actor-reference
//! consumer. Actor ownership, physical dispatch and network transport are separate.
use crate::kabuto_geometry_mode::NotificationContext;

pub struct Target {
    pub state: u32,
    pub velocity: [f32; 3],
    pub owner_word: u32,
}
pub struct DefaultImpulse {
    pub scale: f32,
    /// Acquired source physical scalar; its broader producer remains separate.
    pub factor: f32,
    pub basis: [f32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Notification {
    pub delta: [f32; 3],
    pub target_owner_word: u32,
}
/// The notification guard belongs to the source actor, while the packet names
/// the target owner. The complete velocity store precedes that guard/transport.
/// Errors retain preceding stores; this is owned refusal, not exception parity.
pub fn apply(
    target: Option<&mut Target>,
    explicit: Option<[f32; 3]>,
    fallback: Option<DefaultImpulse>,
    source_context: &NotificationContext<'_>,
    mut notify: impl FnMut(Notification, &Target) -> Result<(), String>,
) -> Result<Option<[f32; 3]>, String> {
    let Some(target) = target else {
        return Ok(None);
    };
    if !matches!(target.state, 2 | 3 | 0x12 | 0x17 | 0x15 | 0x33 | 0x35) {
        return Ok(None);
    }
    let delta = if let Some(v) = explicit {
        v
    } else {
        let s = fallback.ok_or("velocity impulse default source absent")?;
        if ![s.scale, s.factor, s.basis[0], s.basis[1]]
            .into_iter()
            .all(f32::is_finite)
            || s.scale == 0.
        {
            return Err(
                "velocity impulse default source outside finite nonzero-scale profile".into(),
            );
        }
        let amount = (3. / f64::from(s.scale)) * f64::from(s.factor);
        [
            (-(amount * f64::from(s.basis[0]))) as f32,
            (-(amount * f64::from(s.basis[1]))) as f32,
            (amount * 0.25) as f32,
        ]
    };
    if !delta.into_iter().chain(target.velocity).all(f32::is_finite) {
        return Err("velocity impulse outside finite input profile".into());
    }
    for (velocity, delta) in target.velocity.iter_mut().zip(delta) {
        let value = (f64::from(*velocity) + f64::from(delta)) as f32;
        if !value.is_finite() {
            return Err("velocity impulse output overflow".into());
        }
        *velocity = value;
    }
    if source_context.eligible().map_err(str::to_owned)? {
        notify(
            Notification {
                delta,
                target_owner_word: target.owner_word,
            },
            target,
        )?;
    }
    Ok(Some(delta))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_target_does_not_require_default_or_transport() {
        let mut t = Target {
            state: 1,
            velocity: [f32::NAN; 3],
            owner_word: 0,
        };
        let c = NotificationContext {
            session_mode: 4,
            peer_ids: &[],
            local_peer: 0,
            owner_word: 255,
        };
        assert_eq!(
            apply(Some(&mut t), None, None, &c, |_, _| panic!(
                "unexpected transport"
            ))
            .unwrap(),
            None
        );
        assert!(t.velocity.into_iter().all(f32::is_nan));
    }
    #[test]
    fn failed_transport_keeps_velocity_and_uses_source_guard_with_target_identity() {
        let mut t = Target {
            state: 2,
            velocity: [4., 5., 6.],
            owner_word: 7,
        };
        let c = NotificationContext {
            session_mode: 4,
            peer_ids: &[11],
            local_peer: 11,
            owner_word: 0,
        };
        let mut calls = 0;
        assert!(
            apply(Some(&mut t), Some([1., -2., 3.]), None, &c, |p, current| {
                assert_eq!(current.velocity, [5., 3., 9.]);
                assert_eq!(p.target_owner_word, 7);
                assert_eq!(p.delta, [1., -2., 3.]);
                calls += 1;
                Err("transport unavailable".into())
            })
            .is_err()
        );
        assert_eq!(calls, 1);
        assert_eq!(t.velocity, [5., 3., 9.]);
    }
}
