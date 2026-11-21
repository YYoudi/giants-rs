//! Final-effect publication immediately after the acquired target filter.
//! Prototype PS2 `kabuto_move` is an unknown parent navigation route, not an
//! attributed signature. Mode/transition and reaction admission follow this.
use crate::actor_affine::Cache;

#[derive(Clone, Copy, Debug)]
pub struct State {
    /// Acquired effect header scalar; its renderer interpretation is separate.
    pub scalar: f32,
    pub flags: u32,
    pub position: [f32; 3],
    pub affine: Cache,
}

/// The transform producer is required only for a present raw query identity.
/// It must provide the common target transform at factor two; it is not a
/// guessed target position, identity matrix or an accepted-target fallback.
/// Completed writes remain on error; native exception parity is not claimed.
pub fn advance<K: Copy>(
    state: &mut State,
    raw_target: Option<K>,
    seconds: f32,
    mut transform: impl FnMut(K, f32) -> Result<Cache, String>,
) -> Result<(), String> {
    if !state.scalar.is_finite() || !seconds.is_finite() || seconds < 0. {
        return Err("Kabuto query feedback scalar/time outside finite profile".into());
    }
    if let Some(target) = raw_target {
        let affine = transform(target, 2.)?;
        if !affine.matrix.into_iter().all(f32::is_finite) {
            return Err("Kabuto query feedback transform outside finite profile".into());
        }
        state.affine = affine;
        state.position = [affine.matrix[3], affine.matrix[7], affine.matrix[11]];
        state.scalar = (f64::from(state.scalar) - f64::from(seconds) * 3.) as f32;
        // The source reloads the narrowed store before this comparison.
        if state.scalar < 0. {
            state.scalar = 0.;
        }
        state.flags &= !0x4000_0000;
    } else {
        state.scalar = (f64::from(state.scalar) + f64::from(seconds) * 3.) as f32;
        if state.scalar > 1. {
            state.scalar = 1.;
            state.flags |= 0x4000_0000;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub owner: crate::resident_states::Id,
    pub number: u64,
    pub effect: crate::resident_states::Id,
    pub state: State,
}

pub trait Services {
    fn target_transform(
        &mut self,
        session: &crate::resident_states::Session,
        target: crate::resident_states::Id,
        factor: f32,
    ) -> Result<Cache, String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_query_flag_uses_strict_comparison_of_the_narrowed_store() {
        for (value, seconds, scalar, flag) in [
            (1., -0., 1., 0),
            (0.95, 1. / 60., 1., 0),
            (1., 0.001, 1., 0x4000_0000),
        ] {
            let mut state = State {
                scalar: value,
                flags: 7,
                position: [f32::NAN; 3],
                affine: Cache {
                    flags: 123,
                    matrix: [f32::NAN; 16],
                },
            };
            advance::<u32>(&mut state, None, seconds, |_, _| {
                panic!("empty query must not require a transform")
            })
            .unwrap();
            assert_eq!(state.scalar, scalar);
            assert_eq!(state.flags, 7 | flag);
            assert!(state.position.into_iter().all(f32::is_nan));
            assert_eq!(state.affine.flags, 123);
        }
    }
}
