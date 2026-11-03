//! D nonlethal state23 source77 propagation over supplied current actors.
//! This service neither applies damage nor establishes live list completeness.
#[derive(Clone, Debug)]
pub struct Target<I> {
    pub id: I,
    pub state: u32,
    pub position: [f32; 3],
    pub retaliation: Option<I>,
}
/// Caller has already established the nonlethal/action0/source77 branch.
/// Existing retaliation survives, including a different owner. Squared XY
/// distance must be strictly below200; Z is not consumed. Use current simulation
/// positions, not render poses or parent positions.
/// Validate all consumed positions before publishing any mutation.
pub(crate) fn validate<I: Copy + Eq>(
    center: [f32; 3],
    actors: &[Target<I>],
) -> Result<(), &'static str> {
    // One mutable view per identity; repeated native list nodes must resolve to
    // that same view rather than independent copies of an actor's credit.
    for (i, actor) in actors.iter().enumerate() {
        if actors[..i].iter().any(|old| old.id == actor.id) {
            return Err("duplicate retaliation actor identity");
        }
    }
    if !center[..2].iter().copied().all(f32::is_finite)
        || actors
            .iter()
            .filter(|actor| actor.state == 23)
            .flat_map(|actor| actor.position[..2].iter().copied())
            .any(|v| !v.is_finite())
    {
        return Err("non-finite retaliation position");
    }
    Ok(())
}
pub fn propagate<I: Copy + Eq>(
    center: [f32; 3],
    owner: I,
    actors: &mut [Target<I>],
) -> Result<usize, &'static str> {
    validate(center, actors)?;
    let mut changed = 0;
    for actor in actors.iter_mut().filter(|actor| actor.state == 23) {
        let d: [f64; 2] =
            std::array::from_fn(|i| f64::from(actor.position[i]) - f64::from(center[i]));
        let distance_squared = d[0] * d[0] + d[1] * d[1];
        if distance_squared < 200. && actor.retaliation.is_none() {
            actor.retaliation = Some(owner);
            changed += 1;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn actor(x: f32) -> Target<u32> {
        Target {
            id: 1,
            state: 23,
            position: [x, 0., 0.],
            retaliation: None,
        }
    }
    #[test]
    fn strict_radius_and_existing_credit() {
        let mut actors = [actor(14.), actor(10.), actor(0.)];
        actors[1].id = 2;
        actors[2].id = 3;
        actors[1].position[1] = 10.;
        actors[2].retaliation = Some(8);
        assert_eq!(propagate([0.; 3], 7, &mut actors), Ok(1));
        assert_eq!(actors[0].retaliation, Some(7));
        assert_eq!(actors[1].retaliation, None);
        assert_eq!(actors[2].retaliation, Some(8));
        let mut vertical = [actor(0.)];
        vertical[0].position[2] = f32::NAN;
        assert_eq!(propagate([0., 0., f32::NAN], 7, &mut vertical), Ok(1));
    }
    #[test]
    fn consumed_invalid_view_is_atomic() {
        let mut actors = [actor(0.), actor(f32::NAN)];
        actors[1].id = 2;
        assert!(propagate([0.; 3], 7, &mut actors).is_err());
        assert!(actors.iter().all(|actor| actor.retaliation.is_none()));
        actors[1].state = 0;
        assert_eq!(propagate([0.; 3], 7, &mut actors), Ok(1));
        actors[0].retaliation = None;
        actors[1].id = actors[0].id;
        assert!(propagate([0.; 3], 7, &mut actors).is_err());
        assert!(actors[0].retaliation.is_none());
    }
}
