//! Acquired Delphi candidate predicate. Geometry traversal and selection are
//! shared services; this policy does not compare teams or normalize the column.
pub struct Candidate {
    pub flags: u32,
    pub personality: u32,
    pub position: [f32; 3],
    /// Actual identity comparison with the current selected player.
    pub is_current_player: bool,
}
pub fn accepts(
    candidate: Candidate,
    owner_position: [f32; 3],
    owner_second_column: [f32; 3],
) -> Result<bool, String> {
    if candidate.personality & 0x800 != 0
        || candidate.flags & 0x4000c000 != 0
        || candidate.is_current_player
    {
        return Ok(false);
    }
    in_front(candidate.position, owner_position, owner_second_column)
}
pub(crate) fn in_front(
    position: [f32; 3],
    owner_position: [f32; 3],
    owner_second_column: [f32; 3],
) -> Result<bool, String> {
    if !position
        .into_iter()
        .chain(owner_position)
        .chain(owner_second_column)
        .all(f32::is_finite)
    {
        return Err("Delphi target predicate outside finite source profile".into());
    }
    let delta: [f32; 3] =
        std::array::from_fn(|at| (f64::from(position[at]) - f64::from(owner_position[at])) as f32);
    if !delta.into_iter().all(f32::is_finite) {
        return Err("Delphi target predicate displacement overflow".into());
    }
    let column = owner_second_column.map(|v| -f64::from(v));
    let dot = (column[0] * f64::from(delta[0]) + column[1] * f64::from(delta[1]))
        + column[2] * f64::from(delta[2]);
    Ok(dot >= 0.)
}
