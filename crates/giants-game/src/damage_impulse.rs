//! Shared hit-direction velocity addition. The caller owns source identity,
//! current pose and any auxiliary-model center used by a radial hit.
#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub current: [f32; 3],
    pub previous: [f32; 3],
}

pub struct Hit {
    pub kind: u32,
    pub horizontal: f32,
    pub vertical: f32,
    pub recipient: Motion,
    pub source: Option<Motion>,
    pub impact: Option<[f32; 3]>,
    /// Actual selected auxiliary-model center transformed by its current pose,
    /// or the recipient origin when the source selects no auxiliary model.
    pub radial_center: [f32; 3],
}

/// Radial kind2 uses the explicit impact first, then the source origin. Other
/// kinds use source motion, or recipient motion when no source exists. This
/// function neither invents a shooter nor changes an actor's position.
pub fn apply(velocity: [f32; 3], hit: Hit) -> Result<[f32; 3], &'static str> {
    if !velocity
        .into_iter()
        .chain([hit.horizontal, hit.vertical])
        .all(f32::is_finite)
    {
        return Err("damage impulse requires finite velocity and amounts");
    }
    let endpoints = if hit.kind == 2 {
        hit.impact
            .or_else(|| hit.source.map(|s| s.current))
            .map(|origin| (hit.radial_center, origin))
    } else {
        let motion = hit.source.unwrap_or(hit.recipient);
        Some((motion.current, motion.previous))
    };
    let Some((end, start)) = endpoints else {
        return Ok(velocity);
    };
    if !end.into_iter().chain(start).all(f32::is_finite) {
        return Err("damage impulse requires finite consumed direction inputs");
    }
    let direction: [f32; 3] =
        std::array::from_fn(|i| (f64::from(end[i]) - f64::from(start[i])) as f32);
    let d = direction.map(f64::from);
    let square = (d[0] * d[0] + d[1] * d[1]) + d[2] * d[2];
    if square <= f64::from(0.001_f32) {
        return Ok(velocity);
    }
    let length = square.sqrt();
    let unit = d.map(|v| (v / length) as f32);
    let horizontal = hit.horizontal.min(100.);
    let vertical = hit.vertical.min(200.);
    let result = std::array::from_fn(|i| {
        (f64::from(velocity[i])
            + f64::from(unit[i]) * f64::from(if i == 2 { vertical } else { horizontal }))
            as f32
    });
    if !result.into_iter().all(f32::is_finite) {
        return Err("damage impulse output overflow");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hit() -> Hit {
        Hit {
            kind: 0,
            horizontal: 7.,
            vertical: 10.,
            recipient: Motion {
                current: [1.; 3],
                previous: [0.; 3],
            },
            source: None,
            impact: None,
            radial_center: [0.; 3],
        }
    }
    #[test]
    fn source_motion_has_precedence_and_zero_motion_preserves_velocity() {
        let mut h = hit();
        h.source = Some(Motion {
            current: [1., 0., 0.],
            previous: [0.; 3],
        });
        assert_eq!(apply([2.; 3], h).unwrap(), [9., 2., 2.]);
        let mut h = hit();
        h.recipient.previous = h.recipient.current;
        assert_eq!(apply([2.; 3], h).unwrap(), [2.; 3]);
    }
    #[test]
    fn radial_impact_precedes_source_and_magnitudes_only_have_upper_caps() {
        let mut h = hit();
        h.kind = 2;
        h.horizontal = 300.;
        h.vertical = -4.;
        h.source = Some(Motion {
            current: [0., 10., 0.],
            previous: [0.; 3],
        });
        h.impact = Some([-1., 0., 0.]);
        assert_eq!(apply([0.; 3], h).unwrap(), [100., 0., 0.]);
        let mut h = hit();
        h.kind = 2;
        assert_eq!(apply([2.; 3], h).unwrap(), [2.; 3]);
    }
}
