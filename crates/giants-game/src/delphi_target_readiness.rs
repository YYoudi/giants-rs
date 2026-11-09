//! Acquired linked-target readiness prefix and eligibility policy. These CPU
//! services do not create the optional target proxy, retarget Kabuto, publish an
//! owned target or establish primary/world scheduling.
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

#[cfg(test)]
mod tests {
    use super::*;

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
