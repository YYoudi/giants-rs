//! Mecc jet force and resource step. Presentation and inventory are separate.
use crate::arithmetic::Precision;
use giants_formats::{Error, global_data::GlobalData};
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    pub thrust: f32,
    pub boost_multiplier: f32,
    pub boost_drain: f32,
    pub move_speed: f32,
    pub up_bias: f32,
    pub fuel_drain: f32,
    pub fuel_recharge: f32,
    pub boost_recharge: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub normal: Profile,
    pub gyro: Profile,
    pub minimum_start_fuel: f32,
    pub world_height_limit: f32,
    pub recharge_equipment_multiplier: f32,
}
impl Parameters {
    pub fn from_globals(g: &GlobalData) -> Result<Self, Error> {
        Ok(Self {
            normal: Profile {
                thrust: g.scalar(0x690)?,
                boost_multiplier: g.scalar(0x6ac)?,
                boost_drain: g.scalar(0x6a8)?,
                move_speed: g.scalar(0x6d8)?,
                up_bias: g.scalar(0x5f4)?,
                fuel_drain: g.scalar(0x6a4)?,
                fuel_recharge: g.scalar(0x6a0)?,
                boost_recharge: 0.,
            },
            gyro: Profile {
                thrust: g.scalar(0xaac)?,
                boost_multiplier: g.scalar(0xab0)?,
                boost_drain: g.scalar(0xab4)?,
                move_speed: g.scalar(0xab8)?,
                up_bias: g.scalar(0xabc)?,
                fuel_drain: g.scalar(0xac0)?,
                fuel_recharge: g.scalar(0xac4)?,
                boost_recharge: g.scalar(0xac8)?,
            },
            minimum_start_fuel: g.scalar(0x69c)?,
            world_height_limit: g.scalar(0x157c)?,
            recharge_equipment_multiplier: g.scalar(0x7f8)?,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub fuel: f32,
    pub boost: f32,
    pub boost_cooldown: f32,
    pub ignition_delay: f32,
    pub boost_active: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            fuel: 100.,
            boost: 1.,
            boost_cooldown: 0.,
            ignition_delay: 0.,
            boost_active: false,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub state: State,
    pub matrix: [f32; 16],
    pub force: [f32; 3],
    pub scale: f32,
    pub height: f32,
    pub dt: f32,
    pub controls: [i8; 2],
    pub object_flags: u32,
    pub motion_flags: u32,
    pub input_flags: u32,
    pub physics_flags: u32,
    pub recharge_blocked: bool,
    pub recharge_equipment: bool,
    pub disable_boost: bool,
    pub paused: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub state: State,
    pub force: [f32; 3],
    pub motion_flags: u32,
    pub input_flags: u32,
    pub throttle: i8,
    pub boost_started: bool,
    pub shield_depleted: bool,
}
impl Output {
    pub fn words(self) -> [u32; 13] {
        [
            self.force[0].to_bits(),
            self.force[1].to_bits(),
            self.force[2].to_bits(),
            self.state.fuel.to_bits(),
            self.state.boost.to_bits(),
            self.state.boost_cooldown.to_bits(),
            self.state.ignition_delay.to_bits(),
            self.state.boost_active as u32,
            self.motion_flags,
            self.input_flags,
            self.throttle as u8 as u32,
            self.boost_started as u32,
            self.shield_depleted as u32,
        ]
    }
}
/// The two identified executables apply different vertical thrust ceilings.
#[derive(Clone, Copy, Debug)]
pub enum HeightCeiling {
    GogHalf,
    DebugFull,
}

pub fn step(i: Input, p: Parameters, precision: Precision) -> Result<Output, &'static str> {
    step_with_ceiling(i, p, precision, HeightCeiling::GogHalf)
}

// The measured original diagonal multiplier is 0.7071, not 1/sqrt(2).
#[allow(clippy::approx_constant)]
pub fn step_with_ceiling(
    i: Input,
    p: Parameters,
    precision: Precision,
    ceiling: HeightCeiling,
) -> Result<Output, &'static str> {
    let profile = if i.object_flags & 0x20000 != 0 {
        p.gyro
    } else {
        p.normal
    };
    let gyro = i.object_flags & 0x20000 != 0;
    if !i
        .matrix
        .into_iter()
        .chain(i.force)
        .chain([
            i.scale,
            i.height,
            i.dt,
            i.state.fuel,
            i.state.boost,
            i.state.boost_cooldown,
            i.state.ignition_delay,
            profile.thrust,
            profile.boost_multiplier,
            profile.boost_drain,
            profile.move_speed,
            profile.up_bias,
            profile.fuel_drain,
            profile.fuel_recharge,
            profile.boost_recharge,
            p.minimum_start_fuel,
            p.world_height_limit,
            p.recharge_equipment_multiplier,
        ])
        .all(f32::is_finite)
        || i.scale <= 0.
        || i.dt < 0.
    {
        return Err("jet outside finite positive-scale nonnegative-time profile");
    }
    let r = |v| precision.round(v);
    let s = |v| f64::from(v as f32);
    let d = f64::from;
    let mut o = Output {
        state: i.state,
        force: i.force,
        motion_flags: i.motion_flags,
        input_flags: i.input_flags,
        throttle: 0,
        boost_started: false,
        shield_depleted: false,
    };
    if i.paused {
        o.input_flags &= !0x400;
    }
    o.throttle = if o.input_flags & 0x400 != 0 { 32 } else { 0 };
    let delay = r(d(o.state.ignition_delay) - d(i.dt));
    o.state.ignition_delay = delay as f32;
    if delay > 0. {
        o.input_flags &= !0x400;
    }
    let active = i.object_flags & 0x2000000 == 0
        && o.throttle != 0
        && (o.state.fuel > p.minimum_start_fuel
            || (o.motion_flags & 0x1000 != 0 && o.state.fuel > 0.));
    if active {
        let mut thrust = s(r(
            r(f64::from(o.throttle) * 0.03125) * r(d(profile.thrust) / d(i.scale))
        ));
        if o.input_flags & 0x800 != 0 && o.state.boost >= 1. {
            o.boost_started = true;
            o.state.boost_cooldown = 3.;
            o.state.boost_active = true;
        }
        if o.state.boost_active && o.state.boost > 0. {
            thrust = s(r(thrust * d(profile.boost_multiplier)));
            o.state.boost = r(d(o.state.boost) - r(d(i.dt) * d(profile.boost_drain))) as f32;
            if o.state.boost < 0. {
                o.state.boost = 0.;
            }
        } else {
            o.state.boost_active = false;
        }
        let mut forward = r(r(f64::from(i.controls[0]) * 0.03125) * d(profile.move_speed));
        let mut side = r(r(f64::from(i.controls[1]) * 0.03125) * d(profile.move_speed));
        forward = s(forward);
        if forward != 0. && side != 0. {
            side = r(side * d(0.7071f32));
            forward = r(forward * d(0.7071f32));
        }
        let m = i.matrix.map(d);
        let mut direction = [
            r(r(side * m[0]) - r(forward * m[1])) as f32,
            r(r(side * m[4]) - r(forward * m[5])) as f32,
            r(r(side * m[8]) + s(-r(forward * m[9]))) as f32,
        ];
        direction[2] = r(r(d(profile.move_speed) * d(profile.up_bias)) + d(direction[2])) as f32;
        let v = direction.map(d);
        let sum = r(r(r(v[0] * v[0]) + r(v[2] * v[2])) + r(v[1] * v[1]));
        if sum < d(0.0001f32) {
            direction = [0.; 3];
        } else {
            let inv = r(1. / r(sum.sqrt()));
            direction = direction.map(|v| r(d(v) * inv) as f32);
        }
        let height_limit = match ceiling {
            HeightCeiling::GogHalf => r(d(p.world_height_limit) * 0.5),
            HeightCeiling::DebugFull => d(p.world_height_limit),
        };
        if height_limit < d(i.height) {
            direction[2] = 0.;
        }
        o.force = std::array::from_fn(|a| r(r(d(direction[a]) * thrust) + d(o.force[a])) as f32);
        o.motion_flags |= 0x1000;
        let drain = r(r(r(f64::from(o.throttle) * 0.03125) * d(i.dt)) * d(profile.fuel_drain));
        o.state.fuel = r(d(o.state.fuel) - drain) as f32;
        if o.state.fuel <= 0. {
            o.shield_depleted = true;
            o.state.fuel = 0.;
        }
    } else {
        o.state.boost_active = false;
        o.motion_flags &= !0x1000;
        if i.object_flags & 0x2800000 == 0 && !i.recharge_blocked {
            let mut recharge = s(r(d(i.dt) * d(profile.fuel_recharge)));
            if i.recharge_equipment {
                recharge = r(recharge * d(p.recharge_equipment_multiplier));
            }
            let fuel = r(recharge + d(o.state.fuel));
            o.state.fuel = fuel as f32;
            if fuel > 100. {
                o.state.fuel = 100.;
            }
        }
    }
    if !o.state.boost_active {
        o.state.boost_cooldown = r(d(o.state.boost_cooldown) - d(i.dt)) as f32;
        let mut recharge = if gyro {
            s(r(d(i.dt) * d(profile.boost_recharge)))
        } else if i.physics_flags & 0x80 != 0
            && i.physics_flags & 0x40 == 0
            && o.state.boost_cooldown <= 0.
        {
            1.
        } else {
            0.
        };
        if i.recharge_equipment {
            recharge = r(recharge * d(p.recharge_equipment_multiplier));
        }
        let boost = r(recharge + d(o.state.boost));
        o.state.boost = boost as f32;
        if boost > 1. {
            o.state.boost = 1.;
        }
        if i.disable_boost {
            o.state.boost = 0.;
        }
    }
    if o.state.ignition_delay > 0. {
        o.state.boost = 0.;
        o.state.fuel = 0.;
    }
    if !o
        .force
        .into_iter()
        .chain([
            o.state.fuel,
            o.state.boost,
            o.state.boost_cooldown,
            o.state.ignition_delay,
        ])
        .all(f32::is_finite)
    {
        return Err("jet output overflow");
    }
    Ok(o)
}
