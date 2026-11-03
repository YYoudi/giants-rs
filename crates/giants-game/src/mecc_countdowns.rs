//! Paired primary-prefix countdowns. The world value is shared between Meccs;
//! neither value is clamped. Clock and initial world state are supplied.
#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub world: f32,
    pub actor: f32,
}
pub fn advance(world: f32, actor: f32, global_seconds: f32) -> Result<Output, &'static str> {
    if ![world, actor, global_seconds]
        .into_iter()
        .all(f32::is_finite)
        || global_seconds < 0.
    {
        return Err("Mecc countdowns require finite values and nonnegative global delta");
    }
    let subtract = |value| (f64::from(value) - f64::from(global_seconds)) as f32;
    let result = Output {
        world: subtract(world),
        actor: subtract(actor),
    };
    if ![result.world, result.actor].into_iter().all(f32::is_finite) {
        return Err("Mecc countdown overflow");
    }
    Ok(result)
}
