//! Ordinary offline mode1 producer through commands and orientation, before physical movement.
//! Threat response and feeding transitions require their own acquired providers.
use crate::{audio_parameters::Random, object_time::SlowArea};

#[derive(Clone, Copy)]
pub struct Tick<'a> {
    pub global_delta: f32,
    /// Shared native control sensitivity and heading pause gate.
    pub control_factor: f32,
    pub skip_heading: bool,
    pub slow_areas: &'a [SlowArea],
    /// Actual shared world noise/clock values, not a supplied AI decision.
    pub now: f32,
    pub noise_time: f32,
    pub noise_position: [f32; 3],
}

/// A published primary prefix; physics/ANM must still consume this owner.
#[derive(Clone, Copy, Debug)]
pub struct PendingPhysics {
    pub commands: [u8; 8],
    pub retargeted: bool,
}

pub(crate) fn word(body: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(body[at..at + 4].try_into().unwrap())
}
pub(crate) fn scalar(body: &[u8], at: usize) -> f32 {
    f32::from_bits(word(body, at))
}
pub(crate) fn put(body: &mut [u8], at: usize, value: u32) {
    body[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
pub(crate) fn squared(a: [f32; 3], b: [f32; 3]) -> f64 {
    let x = f64::from(b[0]) - f64::from(a[0]);
    let y = f64::from(b[1]) - f64::from(a[1]);
    x * x + y * y
}
pub(crate) fn interval(random: &mut Random, low: f32, high: f32) -> f32 {
    let range = (f64::from(high) - f64::from(low)) as f32;
    (f64::from(random.next_15()) * f64::from(range) / 32768. + f64::from(low)) as f32
}
pub(crate) fn target(random: &mut Random, position: [f32; 3], radius: f32) -> ([f32; 2], [f32; 2]) {
    let angle = interval(random, 0., 360.);
    let (sin, cos) = crate::object_transform::sincos(angle);
    // Radius draws remain wide through their following multiplication.
    let mut sample = || f64::from(random.next_15()) * f64::from(radius) / 32768. + 0.;
    let offsets = [(sample() * cos) as f32, (sample() * sin) as f32];
    (
        offsets,
        std::array::from_fn(|i| (f64::from(position[i]) + f64::from(offsets[i])) as f32),
    )
}
pub(crate) fn commands(position: [f32; 3], heading: f32, target: [f32; 3]) -> [u8; 8] {
    let bearing = crate::actor_volume::heading(position, target);
    let error = crate::ripper_steering::difference(bearing, heading);
    let turn = (error * f64::from(0.033333335_f32)) as f32;
    let speed = (f64::from(squared(position, target) as f32).sqrt() * f64::from(0.025_f32))
        .min(f64::from(0.15_f32));
    let mut commands = [0; 8];
    commands[0] = crate::ripper_steering::command(turn, 0.1);
    commands[2] = (speed * 32.) as i32 as u8;
    commands
}
