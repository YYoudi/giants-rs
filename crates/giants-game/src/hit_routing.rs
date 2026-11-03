//! D hit-wrapper prefix. Callback execution, finalization and network delivery
//! remain separate services; a computed route does not acknowledge them.
#[derive(Clone, Copy, Debug)]
pub struct Actor<I> {
    pub id: I,
    pub parent: Option<I>,
    pub personality: u32,
    /// Header1CC, distinct from the dynamic response/status word DC.
    pub reception: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Context {
    /// Original global guard words; their wider gameplay meaning is not inferred.
    pub blocked: [u32; 2],
    pub mode: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route<I> {
    Suppressed,
    FinalizeOnly { target: I },
    Dispatch { target: I, kind: u32 },
}
/// State77's source-personality range gate before the damage wrapper. The
/// anchor is the current payload vector, not an inferred muzzle/camera point.
pub fn range_rejected(
    flags: u32,
    owner_personality: u32,
    radius: f32,
    anchor: [f32; 3],
    point: [f32; 3],
) -> Result<bool, &'static str> {
    if flags & 0x20000 == 0 || owner_personality & 1 == 0 {
        return Ok(false);
    }
    if ![radius]
        .into_iter()
        .chain(anchor)
        .chain(point)
        .all(f32::is_finite)
    {
        return Err("non-finite projectile range input");
    }
    let d: [f64; 3] = std::array::from_fn(|i| f64::from(point[i]) - f64::from(anchor[i]));
    let distance = (d[0] * d[0] + d[1] * d[1]) + d[2] * d[2];
    Ok(f64::from(radius) * f64::from(radius) <= distance)
}
pub fn resolve<I: Copy + Eq>(
    mut actor: Actor<I>,
    mut kind: u32,
    context: Context,
    mut lookup: impl FnMut(I) -> Result<Actor<I>, String>,
) -> Result<Route<I>, String> {
    if context.blocked.iter().any(|v| *v != 0)
        || (actor.reception & 0x4000000 != 0 && !matches!(kind, 4 | 6))
    {
        return Ok(Route::Suppressed);
    }
    if kind != 2 && actor.parent.is_some() && actor.personality & 0x4000000 != 0 {
        if kind == 15 {
            return Ok(Route::Suppressed);
        }
        let mut seen = vec![actor.id];
        // The native walk retains the initial child's flag, even across an
        // unflagged parent. It does not retest the intermediate personalities.
        while let Some(parent) = actor.parent {
            if seen.contains(&parent) || seen.len() >= 256 {
                return Err("hit parent cycle or traversal bound".into());
            }
            seen.push(parent);
            actor = lookup(parent)?;
            if actor.id != parent {
                return Err("hit parent lookup identity differs".into());
            }
            kind = 12;
        }
    }
    if actor.personality & 0x4000000 != 0 {
        return Ok(Route::FinalizeOnly { target: actor.id });
    }
    if context.mode == 4 {
        return Err("hit route requires network delivery service in mode4".into());
    }
    Ok(Route::Dispatch {
        target: actor.id,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const CONTEXT: Context = Context {
        blocked: [0, 0],
        mode: 0,
    };
    #[test]
    fn initial_flag_walks_through_an_unflagged_parent_to_root() {
        let child = Actor {
            id: 1,
            parent: Some(2),
            personality: 0x4000000,
            reception: 0,
        };
        assert_eq!(
            resolve(child, 0, CONTEXT, |id| Ok(Actor {
                id,
                parent: (id == 2).then_some(3),
                personality: 0,
                reception: u32::MAX,
            })),
            Ok(Route::Dispatch {
                target: 3,
                kind: 12
            })
        );
        assert_eq!(
            resolve(child, 15, CONTEXT, |_| panic!("early suppression")),
            Ok(Route::Suppressed)
        );
    }
    #[test]
    fn direct_kind_and_missing_services_remain_explicit() {
        let child = Actor {
            id: 1,
            parent: Some(2),
            personality: 0x4000000,
            reception: 0,
        };
        assert_eq!(
            resolve(child, 2, CONTEXT, |_| panic!("kind2 does not walk")),
            Ok(Route::FinalizeOnly { target: 1 })
        );
        let normal = Actor {
            personality: 0,
            parent: None,
            ..child
        };
        assert!(
            resolve(
                normal,
                0,
                Context { mode: 4, ..CONTEXT },
                |_| unreachable!()
            )
            .is_err()
        );
        assert!(resolve(child, 0, CONTEXT, |_| Ok(child)).is_err());
    }
}
