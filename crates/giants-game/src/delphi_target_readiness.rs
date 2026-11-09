//! Acquired linked-target readiness prefix and eligibility policy. These CPU
//! services require actual target/proxy providers. They do not produce a pending
//! target, establish its lifetime or establish primary/world scheduling.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub fallback: [f32; 3],
    /// Preserve the incoming word when the cost exceeds the available reserve.
    pub readiness: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    /// The original readiness path uses the fallback rate even for a linked actor.
    pub fallback_rate: f32,
    /// Immutable external scalar; independently acquired from movement duration.
    pub exponent: f64,
}
impl Parameters {
    pub fn from_globals(
        globals: &giants_formats::global_data::GlobalData,
        exponent: f64,
    ) -> Result<Self, String> {
        Ok(Self {
            fallback_rate: globals.scalar(0xfe0).map_err(|e| e.to_string())?,
            exponent,
        })
    }
}
/// Copy the actual linked position before comparing cost. A successful result
/// means only that the original comparison passed: eligibility, optional proxy
/// work and caller activation are separate. On refusal retain prefix effects;
/// the caller must quarantine replay.
pub fn prepare(
    state: &mut State,
    owner_position: [f32; 3],
    linked_position: [f32; 3],
    reserve: f32,
    parameters: Parameters,
) -> Result<bool, String> {
    state.fallback = linked_position;
    if !owner_position
        .into_iter()
        .chain(linked_position)
        .chain([reserve, parameters.fallback_rate])
        .all(f32::is_finite)
        || !parameters.exponent.is_finite()
        || parameters.fallback_rate <= 0.
    {
        return Err("Delphi target readiness outside acquired finite profile".into());
    }
    let delta: [f64; 3] =
        std::array::from_fn(|i| f64::from(owner_position[i]) - f64::from(linked_position[i]));
    // The original stores squared distance as f32, but does not narrow the
    // subsequent power/rate product to movement's f32 duration before comparison.
    let squared = ((delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2]) as f32;
    let cost = f64::from(squared).powf(parameters.exponent) * f64::from(parameters.fallback_rate);
    if !squared.is_finite() || !cost.is_finite() {
        return Err("Delphi target readiness cost outside acquired finite profile".into());
    }
    let affordable = cost <= f64::from(reserve);
    if affordable {
        state.readiness = 1;
    }
    Ok(affordable)
}

#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    pub state: u32,
    pub class: u32,
    pub flags: u32,
    pub personality: u32,
    pub limit: f32,
    pub accumulated: f32,
    pub exclusion: u8,
    /// Compare with the actual current player's context, not the owner's context.
    pub shares_current_player_context: bool,
    pub position: [f32; 3],
}
/// This is the readiness helper's eligibility policy, distinct from the simpler
/// geometry-scan predicate. The supplied affine column remains unnormalized.
pub fn eligible(
    candidate: Candidate,
    owner_position: [f32; 3],
    owner_second_column: [f32; 3],
) -> Result<bool, String> {
    if candidate.flags & 0x4000c000 != 0 {
        return Ok(false);
    }
    if matches!(candidate.state, 0x4a | 0x20 | 0x53 | 0x52) {
        if candidate.personality & 0x20000 == 0 {
            return Ok(false);
        }
    } else {
        if ![candidate.limit, candidate.accumulated]
            .into_iter()
            .all(f32::is_finite)
        {
            return Err("Delphi target eligibility outside acquired finite profile".into());
        }
        if candidate.limit <= candidate.accumulated || candidate.exclusion != 0 {
            return Ok(false);
        }
    }
    if candidate.shares_current_player_context
        || (candidate.class != 0xb && candidate.personality & 0x20000 == 0)
    {
        return Ok(false);
    }
    crate::delphi_target_predicate::in_front(
        candidate.position,
        owner_position,
        owner_second_column,
    )
}

/// Typed CPU composition state. Native pointer slots remain caller-owned typed
/// identities. Publishing these fields to a live actor needs its actual caller.
#[derive(Clone, Copy, Debug)]
pub struct Linked<K, P> {
    pub prefix: State,
    pub target: K,
    pub proxy: Option<P>,
}
/// Required services, in original call order. Construction must return a real
/// acquired proxy; callers must retain provider effects and quarantine failures.
/// No default success/no-op implementation is provided.
pub trait Services<K, P> {
    fn position(&mut self, target: K) -> Result<[f32; 3], String>;
    fn candidate(&mut self, target: K) -> Result<Candidate, String>;
    /// Actual acquired state1 payload relationship, not an arbitrary parent.
    fn replacement(&mut self, target: K) -> Result<K, String>;
    fn construct_proxy(&mut self) -> Result<P, String>;
    fn proxy_flags(&mut self, proxy: P) -> Result<u32, String>;
    fn write_proxy_flags(&mut self, proxy: P, flags: u32) -> Result<(), String>;
    fn write_proxy_matrix(
        &mut self,
        proxy: P,
        matrix: crate::procedural_aim::Matrix,
    ) -> Result<(), String>;
    fn shape(&mut self, target: K) -> Result<crate::delphi_target_proxy::Shape, String>;
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub owner_position: [f32; 3],
    pub owner_second_column: [f32; 3],
    pub reserve: f32,
    pub parameters: Parameters,
    pub half: f32,
}
/// Complete acquired helper over required providers. The original cost is NOT
/// recomputed after retargeting. Readiness precedes eligibility; replacement,
/// construction, flag writes, identity and final matrix retain that order.
/// Prefix and external effects remain after a late refusal; replay is unsafe.
pub fn complete<K: Copy, P: Copy>(
    state: &mut Linked<K, P>,
    input: Input,
    services: &mut impl Services<K, P>,
) -> Result<(), String> {
    let position = services.position(state.target)?;
    if !prepare(
        &mut state.prefix,
        input.owner_position,
        position,
        input.reserve,
        input.parameters,
    )? {
        return Ok(());
    }
    let candidate = services.candidate(state.target)?;
    let personality = candidate.personality;
    let target_state = candidate.state;
    if !eligible(candidate, input.owner_position, input.owner_second_column)?
        || personality & 0x400000 == 0
    {
        return Ok(());
    }
    if target_state == 1 {
        state.target = services.replacement(state.target)?;
        state.prefix.fallback = services.position(state.target)?;
    }
    let proxy = match state.proxy {
        Some(proxy) => proxy,
        None => {
            let proxy = services.construct_proxy()?;
            state.proxy = Some(proxy);
            proxy
        }
    };
    let flags = services.proxy_flags(proxy)? & !0x40000000;
    services.write_proxy_flags(proxy, flags)?;
    // Read current flags again: do not overwrite effects of an actual provider.
    let flags = services.proxy_flags(proxy)? | 0x10000;
    services.write_proxy_flags(proxy, flags)?;
    services.write_proxy_matrix(proxy, crate::procedural_aim::Matrix::identity())?;
    let shape = services.shape(state.target)?;
    let matrix = crate::delphi_target_proxy::matrix(shape, 2., input.half)?;
    services.write_proxy_matrix(proxy, matrix)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_shape_refusal_retains_ready_retarget_birth_flags_and_identity() {
        struct World {
            flags: u32,
            matrix: crate::procedural_aim::Matrix,
        }
        impl Services<u32, u32> for World {
            fn position(&mut self, key: u32) -> Result<[f32; 3], String> {
                Ok(if key == 1 { [0., -1., 0.] } else { [1000.; 3] })
            }
            fn candidate(&mut self, _: u32) -> Result<Candidate, String> {
                Ok(Candidate {
                    state: 1,
                    class: 11,
                    flags: 0,
                    personality: 0x400000,
                    limit: 100.,
                    accumulated: 0.,
                    exclusion: 0,
                    shares_current_player_context: false,
                    position: [0., -1., 0.],
                })
            }
            fn replacement(&mut self, _: u32) -> Result<u32, String> {
                Ok(2)
            }
            fn construct_proxy(&mut self) -> Result<u32, String> {
                self.flags = 0x40000010;
                Ok(7)
            }
            fn proxy_flags(&mut self, _: u32) -> Result<u32, String> {
                Ok(self.flags)
            }
            fn write_proxy_flags(&mut self, _: u32, flags: u32) -> Result<(), String> {
                self.flags = flags;
                Ok(())
            }
            fn write_proxy_matrix(
                &mut self,
                _: u32,
                matrix: crate::procedural_aim::Matrix,
            ) -> Result<(), String> {
                self.matrix = matrix;
                Ok(())
            }
            fn shape(&mut self, _: u32) -> Result<crate::delphi_target_proxy::Shape, String> {
                Err("required current shape unavailable".into())
            }
        }
        let mut world = World {
            flags: 0,
            matrix: crate::procedural_aim::Matrix {
                flags: 99,
                values: [7.; 16],
            },
        };
        let mut state = Linked {
            prefix: State {
                fallback: [0.; 3],
                readiness: 0,
            },
            target: 1,
            proxy: None,
        };
        let result = complete(
            &mut state,
            Input {
                owner_position: [0.; 3],
                owner_second_column: [0., 1., 0.],
                reserve: 1.,
                parameters: Parameters {
                    fallback_rate: 1.,
                    exponent: 0.25,
                },
                half: 0.5,
            },
            &mut world,
        );
        assert!(result.is_err());
        assert_eq!(state.prefix.readiness, 1);
        assert_eq!(state.target, 2);
        assert_eq!(state.prefix.fallback, [1000.; 3]);
        assert_eq!(state.proxy, Some(7));
        assert_eq!(world.flags, 0x10010);
        assert_eq!(world.matrix.flags, 0);
        assert_eq!(
            world.matrix.values,
            crate::procedural_aim::Matrix::identity().values
        );
    }

    #[test]
    fn readiness_keeps_both_precision_boundaries_and_incoming_word() {
        let mut state = State {
            fallback: [7.; 3],
            readiness: 3,
        };
        let p = Parameters {
            fallback_rate: 1.,
            exponent: 0.25,
        };
        assert!(prepare(&mut state, [0.; 3], [1., 2f32.powi(-13), 0.], 1., p).unwrap());
        assert_eq!(state.readiness, 1);
        state.readiness = 3;
        assert!(!prepare(&mut state, [0.; 3], [2., 0., 0.], 2f32.sqrt(), p).unwrap());
        assert_eq!(state.readiness, 3);
        assert_eq!(state.fallback, [2., 0., 0.]);
        assert!(prepare(&mut state, [0.; 3], [3.; 3], f32::NAN, p).is_err());
        assert_eq!(state.fallback, [3.; 3]);
        assert_eq!(state.readiness, 3);
    }

    #[test]
    fn eligibility_keeps_special_state_policy_and_actual_player_context_distinct() {
        let mut candidate = Candidate {
            state: 23,
            class: 11,
            flags: 0,
            personality: 0,
            limit: 10.,
            accumulated: 0.,
            exclusion: 0,
            shares_current_player_context: false,
            position: [0., -1., 0.],
        };
        let owner = [0.; 3];
        let column = [0., 2., 0.];
        let check = |c| eligible(c, owner, column).unwrap();
        assert!(check(candidate));
        candidate.shares_current_player_context = true;
        assert!(!check(candidate));
        candidate.shares_current_player_context = false;
        candidate.state = 0x4a;
        assert!(!check(candidate));
        candidate.personality = 0x20000;
        candidate.limit = 0.;
        candidate.accumulated = 100.;
        candidate.exclusion = 1;
        assert!(check(candidate));
        candidate.flags = 0x4000;
        assert!(!check(candidate));
    }
}
