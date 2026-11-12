//! Ordered proximity selection before ordinary state18 pickup.
//! Prototype search key: `smartie_check_follow__FP6OBJECTf`; the relationship
//! to PC debug-1.1 address `0x004fe080` is probable, not a confirmed ABI identity.
//! This is the selector3 / Mecc branch, not Smartie AI or a pickup callback.
//! The caller supplies actual timestep, ordered players and live actor fields.

pub struct State {
    pub cooldown: f32,
    /// A real pending object reference resets the scan timer before subtraction.
    pub reset_reference: Option<u64>,
}

pub struct Child {
    pub selector: u32,
    pub flags: u32,
    pub position: [f32; 3],
    /// Team of the actual existing parent, when present.
    pub parent_team: Option<u32>,
}

pub struct Candidate<I> {
    pub id: I,
    pub state: u16,
    pub control_flags: u32,
    pub response_flags: u32,
    pub object_flags: u32,
    pub damage: f32,
    pub damage_limit: f32,
    pub team: u32,
    pub position: [f32; 3],
    /// Required only when the native Mecc special-action gate requests it.
    pub inventory_index: Option<u32>,
}

/// Select the first eligible player strictly inside the supplied XYZ radius.
/// Writes before a missing service remain committed, as in the native caller.
/// The result still requires the real pickup, animation, sound and notice owners.
pub fn smartie_check_follow<I: Copy>(
    state: &mut State,
    child: &Child,
    object_seconds: f32,
    radius: f32,
    players: &[Candidate<I>],
) -> Result<Option<I>, &'static str> {
    if child.selector == 7 {
        return Ok(None);
    }
    if child.selector != 3 {
        return Err("Smartie follow requires acquired selector3 branch");
    }
    if ![state.cooldown, object_seconds, radius]
        .into_iter()
        .chain(child.position)
        .all(f32::is_finite)
        || object_seconds < 0.
        || radius < 0.
    {
        return Err("Smartie follow outside finite clock/position/radius profile");
    }
    if state.reset_reference.take().is_some() {
        state.cooldown = 0.;
    }
    // PC stores f32 but tests the unrounded x87 subtraction register.
    let remaining = f64::from(state.cooldown) - f64::from(object_seconds);
    state.cooldown = remaining as f32;
    if remaining > 0. {
        return Ok(None);
    }
    state.cooldown = if child.flags & 8 == 0 { 2. } else { 0.8 };
    for player in players {
        if player.control_flags & 0x8000 != 0 || !matches!(player.state, 2 | 3) {
            continue;
        }
        if ![player.damage, player.damage_limit]
            .into_iter()
            .all(f32::is_finite)
        {
            return Err("Smartie follow requires finite player damage fields");
        }
        if player.damage >= player.damage_limit || player.response_flags & 0x800000 != 0 {
            continue;
        }
        if player.state == 3 {
            return Err("Smartie follow Delphi service is not acquired here");
        }
        if player.response_flags & 0x400000 != 0 {
            match player.inventory_index {
                Some(10) => continue,
                Some(_) => {}
                None => return Err("Smartie follow current inventory service absent"),
            }
        }
        if player.object_flags & 0x2000000 != 0
            || (child.flags & 8 != 0 && child.parent_team == Some(player.team))
        {
            continue;
        }
        if !player.position.into_iter().all(f32::is_finite) {
            return Err("Smartie follow requires finite candidate XYZ position");
        }
        let [dx, dy, dz] = std::array::from_fn::<_, 3, _>(|i| {
            f64::from(child.position[i]) - f64::from(player.position[i])
        });
        if (dx * dx + dy * dy) + dz * dz < f64::from(radius) * f64::from(radius) {
            return Ok(Some(player.id));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn child() -> Child {
        Child {
            selector: 3,
            flags: 0,
            position: [0.; 3],
            parent_team: None,
        }
    }
    fn player(id: u32, position: [f32; 3]) -> Candidate<u32> {
        Candidate {
            id,
            state: 2,
            control_flags: 0,
            response_flags: 0,
            object_flags: 0,
            damage: 0.,
            damage_limit: 1.,
            team: 1,
            position,
            inventory_index: None,
        }
    }
    #[test]
    fn uses_ordered_strict_xyz_distance_and_preserves_cooldown_between_scans() {
        let mut state = State {
            cooldown: 0.,
            reset_reference: None,
        };
        let players = [
            player(1, [0., 0., 7.]),
            player(2, [0., 0., 6.]),
            player(3, [0.; 3]),
        ];
        assert_eq!(
            smartie_check_follow(&mut state, &child(), 0., 7., &players),
            Ok(Some(2))
        );
        assert_eq!(state.cooldown, 2.);
        assert_eq!(
            smartie_check_follow(&mut state, &child(), 1., 7., &players),
            Ok(None)
        );
        assert_eq!(state.cooldown, 1.);
        assert_eq!(
            smartie_check_follow(&mut state, &child(), 1., 7., &players),
            Ok(Some(2))
        );
    }
    #[test]
    fn reset_and_short_circuit_gates_preserve_actual_service_requirements() {
        let mut state = State {
            cooldown: 20.,
            reset_reference: Some(12),
        };
        let mut child = child();
        child.selector = 7;
        assert_eq!(
            smartie_check_follow(&mut state, &child, 0., 7., &[] as &[Candidate<u32>]),
            Ok(None)
        );
        assert_eq!(state.reset_reference, Some(12));
        assert_eq!(state.cooldown, 20.);
        child.selector = 3;
        child.flags = 8;
        child.parent_team = Some(1);
        let mut players = [player(1, [0.; 3]), player(2, [0.; 3])];
        players[1].team = 2;
        players[1].response_flags = 0x400000;
        assert!(smartie_check_follow(&mut state, &child, 0., 7., &players).is_err());
        assert_eq!(state.reset_reference, None);
        assert_eq!(state.cooldown, 0.8);
        state.cooldown = 0.;
        players[1].inventory_index = Some(10);
        assert_eq!(
            smartie_check_follow(&mut state, &child, 0., 7., &players),
            Ok(None)
        );
        state.cooldown = 0.;
        players[1].inventory_index = Some(9);
        assert_eq!(
            smartie_check_follow(&mut state, &child, 0., 7., &players),
            Ok(Some(2))
        );
    }
}
