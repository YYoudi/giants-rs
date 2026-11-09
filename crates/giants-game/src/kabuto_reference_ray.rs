//! Acquired argument production for the ordered reference query. The caller
//! still owns mode/current-actor context, reference lifetime and current geometry.
use crate::kabuto_reference_query::Ray;

#[derive(Clone, Copy, Debug)]
pub struct Context {
    /// Actual three mode words; no broader camera/network names are assigned.
    pub words: [u32; 3],
    pub active: u8,
    pub current_state: Option<u32>,
}
impl Context {
    fn first(self) -> bool {
        self.words == [0; 3]
    }
    fn second(self) -> bool {
        self.words[0] == 0
            && ((self.words[1] == 0 && self.words[2] == 1)
                || (self.active != 0 && self.current_state.is_some_and(|s| matches!(s, 1..=3))))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    /// Each pair has its acquired caller-specific use: the first two hold
    /// viewport offset/range; the final pair holds actor range/score.
    pub pairs: [[f32; 2]; 3],
}
impl Parameters {
    pub fn from_owned_payload(payload: &[u8]) -> Result<Self, String> {
        if payload.len() != 844 {
            return Err("Kabuto reference ray payload outside acquired profile".into());
        }
        Ok(Self {
            pairs: std::array::from_fn(|i| {
                std::array::from_fn(|j| {
                    let at = 0x1b8 + i * 8 + j * 4;
                    f32::from_le_bytes(payload[at..at + 4].try_into().unwrap())
                })
            }),
        })
    }
}
pub struct ActorGeometry {
    pub lower: [f32; 3],
    pub upper: [f32; 3],
    pub basis_y: [f32; 3],
}
pub struct Viewport {
    pub origin: [f32; 3],
    /// Current viewport basis index two, without normalization.
    pub direction: [f32; 3],
}
pub trait Geometry {
    fn actor(&mut self) -> Result<ActorGeometry, String>;
    fn viewport(&mut self) -> Result<Viewport, String>;
}
pub fn prepare(
    context: Context,
    reference_present: bool,
    parameters: Parameters,
    geometry: &mut impl Geometry,
) -> Result<Ray, String> {
    let pair = if context.first() {
        1
    } else if context.second() {
        if reference_present { 1 } else { 0 }
    } else {
        2
    };
    let [first, range] = parameters.pairs[pair];
    if ![first, range].into_iter().all(f32::is_finite) {
        return Err("Kabuto reference ray parameters outside finite profile".into());
    }
    let ray = if pair == 2 {
        let g = geometry.actor()?;
        if !g
            .lower
            .into_iter()
            .chain(g.upper)
            .chain(g.basis_y)
            .all(f32::is_finite)
        {
            return Err("Kabuto reference actor geometry outside finite profile".into());
        }
        Ray {
            origin: std::array::from_fn(|i| {
                ((f64::from(g.lower[i]) + f64::from(g.upper[i])) * 0.5) as f32
            }),
            direction: g.basis_y.map(|v| -v),
            forward_limit: first,
            score_limit: range,
        }
    } else {
        let v = geometry.viewport()?;
        if !v.origin.into_iter().chain(v.direction).all(f32::is_finite) {
            return Err("Kabuto reference viewport outside finite profile".into());
        }
        Ray {
            origin: std::array::from_fn(|i| {
                (f64::from(v.direction[i]) * f64::from(first) + f64::from(v.origin[i])) as f32
            }),
            direction: v.direction,
            forward_limit: (f64::from(range) - f64::from(first)) as f32,
            score_limit: 10_000.,
        }
    };
    if !ray
        .origin
        .into_iter()
        .chain([ray.forward_limit])
        .all(f32::is_finite)
    {
        return Err("Kabuto reference ray output outside finite profile".into());
    }
    Ok(ray)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Missing;
    impl Geometry for Missing {
        fn actor(&mut self) -> Result<ActorGeometry, String> {
            Err("actor geometry unavailable".into())
        }
        fn viewport(&mut self) -> Result<Viewport, String> {
            panic!("inactive viewport must not be requested")
        }
    }
    #[test]
    fn actor_path_refuses_missing_geometry_without_entering_the_viewport() {
        let e = prepare(
            Context {
                words: [1, 0, 0],
                active: 1,
                current_state: Some(1),
            },
            true,
            Parameters {
                pairs: [[f32::NAN; 2], [f32::NAN; 2], [1., 2.]],
            },
            &mut Missing,
        )
        .err()
        .unwrap();
        assert_eq!(e, "actor geometry unavailable");
    }
}
