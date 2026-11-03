//! Cold normal-Mecc physics/control fields after native initialization.
//! Inventory-adjusted mass and later state/underwater overrides are separate.
use giants_formats::{Error, global_data::GlobalData};
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub gravity_mass: f32,
    pub mass: f32,
    pub restitution: f32,
    pub retention: f32,
    pub speed_limit: f32,
    pub friction_limit: f32,
    pub damping_xy: f32,
    pub damping_z: f32,
    pub control_gain: f32,
    pub control_speed: f32,
    pub slope_power: f32,
}
impl Parameters {
    pub fn from_globals(g: &GlobalData) -> Result<Self, Error> {
        Ok(Self {
            gravity_mass: g.scalar(0x674)?,
            mass: g.scalar(0x678)?,
            restitution: g.scalar(0x680)?,
            retention: 1.,
            speed_limit: g.scalar(0x67c)?,
            friction_limit: g.scalar(0x684)?,
            damping_xy: g.scalar(0x5f0)?,
            damping_z: g.scalar(0x698)?,
            control_gain: g.scalar(0x688)?,
            control_speed: g.scalar(0x6d8)?,
            slope_power: g.scalar(0x68c)?,
        })
    }
    pub fn words(self) -> [u32; 11] {
        [
            self.gravity_mass,
            self.mass,
            self.restitution,
            self.retention,
            self.speed_limit,
            self.friction_limit,
            self.damping_xy,
            self.damping_z,
            self.control_gain,
            self.control_speed,
            self.slope_power,
        ]
        .map(f32::to_bits)
    }
}
