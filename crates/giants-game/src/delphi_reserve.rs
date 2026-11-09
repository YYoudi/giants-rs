//! Acquired reserve/delay callback. Activation producers, movement, immersion
//! effects and live scheduling are separate; source flags are not PS2 layouts.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub reserve: f32,
    pub delay: f32,
    pub movement_seconds: f32,
    pub controls: u32,
    pub internal: u32,
    pub gravity_mass: f32,
    pub mass: f32,
    /// Presence only. The required service owns the actual owned resource identity.
    pub context_present: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub gravity_mass: f32,
    pub mass: f32,
    pub capacity: f32,
    pub recharge_rate: f32,
    pub alternate_numerator: f32,
    pub alternate_denominator: f32,
}
impl Parameters {
    pub fn from_globals(g: &giants_formats::global_data::GlobalData) -> Result<Self, String> {
        let f = |at| g.scalar(at).map_err(|e| e.to_string());
        Ok(Self {
            gravity_mass: f(0x704)?,
            mass: f(0x708)?,
            capacity: f(0x868)?,
            recharge_rate: f(0x86c)?,
            alternate_numerator: f(0xfe0)?,
            alternate_denominator: f(0x748)?,
        })
    }
}
pub struct Input {
    pub physics_flags: u32,
    pub hit: bool,
    pub damage: f32,
    pub limit: f32,
    pub linked_actor_present: bool,
    /// Actual level/global flag, with its producer still supplied by the caller.
    pub recharge_inhibited: bool,
    pub position: [f32; 3],
}
/// Services are bound to the same owner. They must execute or refuse: a request
/// record alone cannot acknowledge retirement, splash or movement execution.
pub trait Services {
    fn retire(&mut self, present: &mut bool) -> Result<(), String>;
    fn splash(&mut self, position: [f32; 3]) -> Result<(), String>;
    fn movement(&mut self) -> Result<(), String>;
}
/// Explicit acquired empty-context profile. Other effects remain unprepared.
pub struct EmptyContext;
impl Services for EmptyContext {
    fn retire(&mut self, present: &mut bool) -> Result<(), String> {
        if *present {
            Err("Delphi reserve context retirement unprepared".into())
        } else {
            Ok(())
        }
    }
    fn splash(&mut self, _: [f32; 3]) -> Result<(), String> {
        Err("Delphi reserve splash producer unprepared".into())
    }
    fn movement(&mut self) -> Result<(), String> {
        Err("Delphi reserve movement producer unprepared".into())
    }
}
pub trait World: Services {
    /// Query actual object-local time again at each source call site, including
    /// after completed required services. Do not cache a pre-movement delta here.
    fn seconds(&mut self) -> Result<f64, String>;
}
pub struct Context<'a> {
    pub post: crate::delphi_post_physical::Context<'a>,
    pub recharge_inhibited: bool,
    pub services: &'a mut dyn Services,
}
fn seconds(world: &mut impl World) -> Result<f64, String> {
    let value = world.seconds()?;
    if value.is_finite() && value >= 0. {
        Ok(value)
    } else {
        Err("Delphi reserve requires finite nonnegative object time".into())
    }
}
/// Retain completed prefix and service effects on refusal. The surrounding
/// phase owns quarantine; calling this again would replay those effects.
pub fn advance(
    state: &mut State,
    i: Input,
    p: Parameters,
    world: &mut impl World,
) -> Result<(), String> {
    if !state.reserve.is_finite() || !state.delay.is_finite() {
        return Err("Delphi reserve requires finite owned state".into());
    }
    let delay = f64::from(state.delay) - seconds(world)?;
    state.delay = delay as f32;
    if delay > 0. {
        state.reserve = 0.;
    }
    if state.controls & 0x100 == 0 {
        if i.recharge_inhibited {
            state.reserve = 0.;
            return Ok(());
        }
        if delay > 0. {
            return Ok(());
        }
        if !p.capacity.is_finite()
            || !p.recharge_rate.is_finite()
            || p.capacity < 0.
            || p.recharge_rate < 0.
        {
            return Err("Delphi reserve recharge parameters outside finite profile".into());
        }
        let reserve = seconds(world)? * f64::from(p.recharge_rate) + f64::from(state.reserve);
        state.reserve = if reserve > f64::from(p.capacity) {
            p.capacity
        } else {
            reserve as f32
        };
        return Ok(());
    }
    if !i.damage.is_finite() || !i.limit.is_finite() {
        return Err("Delphi reserve requires actual finite damage and limit".into());
    }
    if i.physics_flags & 0x44 != 0 || i.hit || state.reserve <= 0. || i.limit <= i.damage {
        world.retire(&mut state.context_present)?;
        if !p.gravity_mass.is_finite() || !p.mass.is_finite() {
            return Err("Delphi reserve physical parameters outside finite profile".into());
        }
        state.gravity_mass = p.gravity_mass;
        state.mass = p.mass;
        state.controls &= !0x100;
        if i.physics_flags & 0x40 == 0 {
            if state.reserve > 0. {
                state.internal |= 0x10;
            }
        } else {
            world.splash([i.position[0], i.position[1], 0.])?;
        }
    } else if i.linked_actor_present && state.movement_seconds > 0. {
        world.movement()?;
        state.movement_seconds = (f64::from(state.movement_seconds) - seconds(world)?) as f32;
    }
    let amount = if i.linked_actor_present {
        if !p.alternate_numerator.is_finite()
            || !p.alternate_denominator.is_finite()
            || p.alternate_numerator < 0.
            || p.alternate_denominator <= 0.
        {
            return Err("Delphi reserve alternate ratio outside finite profile".into());
        }
        (f64::from(p.alternate_numerator) / f64::from(p.alternate_denominator)) * seconds(world)?
    } else {
        seconds(world)?
    };
    let reserve = (f64::from(state.reserve) - amount) as f32;
    state.reserve = if reserve < 0. { 0. } else { reserve };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Provider {
        times: std::collections::VecDeque<f64>,
        events: Vec<&'static str>,
        fail: bool,
    }
    impl Services for Provider {
        fn retire(&mut self, p: &mut bool) -> Result<(), String> {
            self.events.push("retire");
            *p = false;
            Ok(())
        }
        fn splash(&mut self, p: [f32; 3]) -> Result<(), String> {
            assert_eq!(p[2], 0.);
            self.events.push("splash");
            if self.fail {
                Err("splash failure".into())
            } else {
                Ok(())
            }
        }
        fn movement(&mut self) -> Result<(), String> {
            self.events.push("movement");
            Ok(())
        }
    }
    impl World for Provider {
        fn seconds(&mut self) -> Result<f64, String> {
            self.events.push("time");
            self.times.pop_front().ok_or("time exhausted".into())
        }
    }
    fn state() -> State {
        State {
            reserve: 2.,
            delay: 0.,
            movement_seconds: 1.,
            controls: 0x100,
            internal: 0,
            gravity_mass: 7.,
            mass: 11.,
            context_present: true,
        }
    }
    fn input() -> Input {
        Input {
            physics_flags: 0,
            hit: false,
            damage: 0.,
            limit: 10.,
            linked_actor_present: true,
            recharge_inhibited: false,
            position: [3., 4., 9.],
        }
    }
    fn parameters() -> Parameters {
        Parameters {
            gravity_mass: 20.,
            mass: 3.,
            capacity: 5.,
            recharge_rate: 1.,
            alternate_numerator: 2.,
            alternate_denominator: 1.,
        }
    }
    #[test]
    fn movement_time_and_drain_are_queried_after_the_service() {
        let mut state = state();
        let mut world = Provider {
            times: [0.1, 0.2, 0.3].into(),
            events: vec![],
            fail: false,
        };
        advance(&mut state, input(), parameters(), &mut world).unwrap();
        assert_eq!(world.events, ["time", "movement", "time", "time"]);
        assert_eq!(state.movement_seconds, 0.8);
        assert_eq!(state.reserve, 1.4);
    }
    #[test]
    fn splash_refusal_retains_retirement_and_physical_restore_before_drain() {
        let mut state = state();
        let mut world = Provider {
            times: [0.1].into(),
            events: vec![],
            fail: true,
        };
        let mut input = input();
        input.physics_flags = 0x40;
        assert!(advance(&mut state, input, parameters(), &mut world).is_err());
        assert_eq!(world.events, ["time", "retire", "splash"]);
        assert!(!state.context_present);
        assert_eq!(
            (
                state.gravity_mass,
                state.mass,
                state.controls,
                state.reserve
            ),
            (20., 3., 0, 2.)
        );
        assert_eq!(state.delay, -0.1);
    }
}
