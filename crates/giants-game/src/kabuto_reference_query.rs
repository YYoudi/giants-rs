//! Acquired reference-candidate filter and ordered bounds query. The caller owns
//! current bounds, list membership, identities and parent metadata.
pub struct Source<K> {
    pub identity: K,
    pub parent: Option<K>,
    pub masks: [u32; 3],
}
pub struct Candidate<K> {
    pub identity: K,
    pub state: u32,
    pub flags: u32,
    /// Separate from the actor personality word.
    pub category_mask: u32,
    pub parent: Option<(K, u32)>,
    pub lower: [f32; 3],
    pub upper: [f32; 3],
}
pub struct Ray {
    pub origin: [f32; 3],
    pub direction: [f32; 3],
    pub forward_limit: f32,
    pub score_limit: f32,
}
#[derive(Debug)]
pub struct Selection<K> {
    pub target: Option<K>,
    pub score: f32,
}
pub fn eligible<K: Copy + Eq>(
    source: &Source<K>,
    candidate: Option<&Candidate<K>>,
    kind: u8,
) -> bool {
    let Some(c) = candidate else {
        return false;
    };
    if c.identity == source.identity
        || c.flags & 0x4000c000 != 0
        || c.parent.is_some_and(|(_, state)| matches!(state, 1 | 76))
        || source.parent == Some(c.identity)
    {
        return false;
    }
    (kind & 1 != 0
        && source.masks[0] & c.category_mask != 0
        && (c.parent.is_none() || c.state == 51))
        || (kind & 2 != 0 && source.masks[1] & c.category_mask != 0)
        || (kind & 4 != 0 && source.masks[2] & c.category_mask != 0)
}
/// Keep declared list order on equal scores. Direction is not normalized, and
/// the acquired score expression is not clamped to a nonnegative distance.
pub fn select<K: Copy + Eq>(
    source: &Source<K>,
    candidates: &[Option<Candidate<K>>],
    ray: Ray,
) -> Result<Selection<K>, String> {
    if !ray
        .origin
        .into_iter()
        .chain(ray.direction)
        .chain([ray.forward_limit, ray.score_limit])
        .all(f32::is_finite)
    {
        return Err("Kabuto reference ray outside finite profile".into());
    }
    let mut out = Selection {
        target: None,
        score: ray.score_limit,
    };
    for candidate in candidates {
        if !eligible(source, candidate.as_ref(), 7) {
            continue;
        }
        let c = candidate.as_ref().unwrap();
        if !c.lower.into_iter().chain(c.upper).all(f32::is_finite) {
            return Err("Kabuto reference bounds outside finite profile".into());
        }
        let wide: [f64; 3] = std::array::from_fn(|i| {
            (f64::from(c.upper[i]) + f64::from(c.lower[i])) * 0.5 - f64::from(ray.origin[i])
        });
        let delta = wide.map(|v| v as f32);
        // Native Z remains wide after its store for the first dot term.
        let dot = (wide[2] * f64::from(ray.direction[2])
            + f64::from(delta[1]) * f64::from(ray.direction[1]))
            + f64::from(delta[0]) * f64::from(ray.direction[0]);
        if dot.is_nan() || dot < 0. {
            continue;
        }
        let dot = dot as f32;
        if dot.is_nan() || dot >= ray.forward_limit {
            continue;
        }
        let [x, y, z] = delta.map(f64::from);
        let score = (((z * z + y * y) + x * x) - f64::from(dot) * f64::from(dot)) as f32;
        if score < out.score {
            out.target = Some(c.identity);
            out.score = score;
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Source<u32> {
        Source {
            identity: 1,
            parent: None,
            masks: [1, 2, 4],
        }
    }
    #[test]
    fn rejected_bounds_do_not_poison_the_ordered_query() {
        let candidates = [
            Some(Candidate {
                identity: 2,
                state: 2,
                flags: 0x4000,
                category_mask: 1,
                parent: None,
                lower: [f32::NAN; 3],
                upper: [f32::NAN; 3],
            }),
            Some(Candidate {
                identity: 3,
                state: 2,
                flags: 0,
                category_mask: 1,
                parent: None,
                lower: [1., 0., 0.],
                upper: [1., 0., 0.],
            }),
        ];
        let found = select(
            &source(),
            &candidates,
            Ray {
                origin: [0.; 3],
                direction: [1., 0., 0.],
                forward_limit: 2.,
                score_limit: 1.,
            },
        )
        .unwrap();
        assert_eq!(found.target, Some(3));
    }
    #[test]
    fn wide_negative_dot_does_not_become_an_accepted_stored_zero() {
        let candidates = [Some(Candidate {
            identity: 2,
            state: 2,
            flags: 0,
            category_mask: 1,
            parent: None,
            lower: [0.15, 0., 0.1],
            upper: [0.15, 0., 0.2],
        })];
        let found = select(
            &source(),
            &candidates,
            Ray {
                origin: [0.; 3],
                direction: [-1e-38, 0., 1e-38],
                forward_limit: 1.,
                score_limit: 1.,
            },
        )
        .unwrap();
        assert_eq!(found.target, None);
    }
}
