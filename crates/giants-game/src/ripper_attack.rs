//! Ordinary class73 modes37/38/39 and class230 mode65 after conditional perception.
//! Real firing and owner audio services are required. No mission readiness.
//! Prototype PS2 search key: `ripper_move`; PC debug-1.1 0x004d61f0.
//! The previous PC alias remains in docs/SYMBOL_NAVIGATION.md.
//! The exact prototype-to-PC attribution remains unknown.
use crate::{
    audio_parameters::Random, enemy_projectile::SoundOutcome, ripper_perception::Candidate,
    ripper_pursuit, ripper_steering, state23_animation::FireOutcome,
};
#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    pub repeat_percent: f32,
    pub charge_percent: f32,
    pub announce_percent: f32,
    pub vertical_spread_percent: f32,
    pub orbit: crate::ripper_turn_completion::Parameters,
}
/// Class230 mode65 reads these values from its actual global-data profile.
#[derive(Clone, Copy, Debug)]
pub struct VariantParameters {
    pub close_range: f32,
    pub reconsider_range: f32,
    pub charge_percent: f32,
    pub cooldown: f32,
    pub vertical_spread_percent: f32,
}
#[cfg(test)]
mod tests {
    use super::*;
    fn variant_parameters() -> VariantParameters {
        VariantParameters {
            close_range: 5.,
            reconsider_range: 20.,
            charge_percent: 100.,
            cooldown: 3.,
            vertical_spread_percent: 10.,
        }
    }
    struct VariantServices {
        fires: usize,
        fail: bool,
    }
    impl Services<u32> for VariantServices {
        fn owner_sound(
            &mut self,
            _: &State<u32>,
            _: u32,
            _: &mut Random,
        ) -> Result<SoundOutcome, String> {
            Err("unexpected variant sound".into())
        }
        fn fire(
            &mut self,
            s: &State<u32>,
            point: [f32; 3],
            spread: f32,
            _: &mut Random,
        ) -> Result<FireOutcome, String> {
            assert_eq!((s.actor.class, s.actor.mode), (230, 65));
            assert_eq!(point, [1., 2., 13.]);
            assert_eq!(spread, 10.);
            self.fires += 1;
            if self.fail {
                Err("supplied launch failure".into())
            } else {
                Ok(FireOutcome::Spawned)
            }
        }
    }
    #[test]
    fn variant_fire_crossing_uses_real_target_shape_and_retains_a_failed_attempt() {
        for fail in [false, true] {
            let mut s = state();
            s.actor.class = 230;
            s.actor.mode = 65;
            let mut m = Machine::new(s);
            let mut random = Random(17);
            let mut services = VariantServices { fires: 0, fail };
            let primary = [50, 0x3edd_ddde, 0x3edd_dddd, 0, 0, 0, 0, 0, 0];
            let result = m.advance_variant(
                Some(target()),
                primary,
                variant_parameters(),
                &mut random,
                &mut services,
            );
            assert_eq!(result.is_err(), fail);
            assert_eq!(m.failure().is_some(), fail);
            assert_eq!(m.state().actor.mode, 65);
            assert_ne!(m.state().actor.steering.commands[0], 0);
            assert_eq!(random.0, 17);
            assert!(
                m.advance_variant(
                    Some(target()),
                    primary,
                    variant_parameters(),
                    &mut random,
                    &mut services
                )
                .is_err()
            );
            assert_eq!(services.fires, 1);
        }
    }
    #[test]
    fn variant_completed_attack_keeps_short_circuit_rng_and_actual_cooldown() {
        for (distance, charge, expected, draws) in [
            (4., 0., 67, false),
            (5., 100., 67, true),
            (19.99, 0., 66, true),
            (20., 100., 66, false),
        ] {
            let mut s = state();
            s.actor.class = 230;
            s.actor.mode = 65;
            let mut target = target();
            target.actor.position = [distance, 0., 500.];
            let mut m = Machine::new(s);
            let mut random = Random(17);
            let mut services = VariantServices {
                fires: 0,
                fail: true,
            };
            let p = VariantParameters {
                charge_percent: charge,
                ..variant_parameters()
            };
            m.advance_variant(
                Some(target),
                [50, 0, 0, 0, 0, 0, 0, 0, 2],
                p,
                &mut random,
                &mut services,
            )
            .unwrap();
            assert_eq!(m.state().actor.mode, expected);
            assert_eq!(
                m.state().timer,
                if expected == 66 { p.cooldown } else { 0. }
            );
            assert_eq!(random.0 != 17, draws);
            assert_eq!(services.fires, 0);
        }
    }
    fn state() -> State<u32> {
        State {
            actor: ripper_pursuit::State {
                class: 73,
                mode: 39,
                control: 0,
                retained_timer: 4.,
                retaliation: Some(2),
                orbit: ripper_steering::Orbit {
                    angle: 0.,
                    radius: 10.,
                },
                steering: ripper_steering::State {
                    position: [0.; 3],
                    heading: 0.,
                    flags: 0,
                    personality: 0x40,
                    commands: [0; 8],
                    target_heading: 0.,
                    target_position: [0.; 3],
                },
            },
            timer: 0.,
        }
    }
    fn target() -> Target<u32> {
        Target {
            actor: Candidate {
                id: 2,
                class: 2,
                state: 2,
                active: 0,
                personality: 1,
                control: 0,
                position: [1., 2., 3.],
                damage: 0.,
                limit: 1.,
                parent_present: false,
            },
            scale: 2.,
            center_offset: 5.,
        }
    }
    fn parameters() -> Parameters {
        Parameters {
            repeat_percent: 0.,
            charge_percent: 100.,
            announce_percent: 100.,
            vertical_spread_percent: 75.,
            orbit: crate::ripper_turn_completion::Parameters {
                radius: [10., 20.],
                seconds: 3.,
            },
        }
    }
    struct Refuse {
        sound: u32,
        fire: u32,
    }
    impl Services<u32> for Refuse {
        fn owner_sound(
            &mut self,
            state: &State<u32>,
            cue: u32,
            random: &mut Random,
        ) -> Result<SoundOutcome, String> {
            assert_eq!((state.actor.mode, cue, state.timer), (39, 1256, 3.));
            self.sound += 1;
            random.next_15();
            Err("real sound unavailable".into())
        }
        fn fire(
            &mut self,
            state: &State<u32>,
            target: [f32; 3],
            spread: f32,
            random: &mut Random,
        ) -> Result<FireOutcome, String> {
            assert_eq!((state.actor.mode, target, spread), (39, [1., 2., 13.], 75.));
            self.fire += 1;
            random.next_15();
            Err("real projectile service unavailable".into())
        }
    }
    #[test]
    fn failed_fire_retains_aim_and_service_rng_without_replaying() {
        let mut machine = Machine::new(state());
        let mut random = Random(7);
        let mut backend = Refuse { sound: 0, fire: 0 };
        let channel = [50, 0x3edd_ddde, 0x3edd_dddd, 0, 0, 0, 0, 0, 0];
        assert!(
            machine
                .advance(None, channel, parameters(), &mut random, &mut backend)
                .is_err()
        );
        assert!(machine.failure().is_none());
        assert_eq!(random.0, 7);
        assert!(
            machine
                .advance(
                    Some(target()),
                    channel,
                    parameters(),
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        let seed = random.0;
        assert_ne!(seed, 7);
        assert_ne!(machine.state().actor.steering.commands[0], 0);
        assert_eq!(machine.state().actor.mode, 39);
        assert!(
            machine
                .advance(
                    Some(target()),
                    channel,
                    parameters(),
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        assert_eq!((backend.fire, random.0), (1, seed));
    }
    #[test]
    fn failed_post_animation_sound_keeps_orbit_draws_and_old_mode() {
        let mut machine = Machine::new(state());
        let mut random = Random(7);
        let mut backend = Refuse { sound: 0, fire: 0 };
        let channel = [50, 0, 0, 0, 0, 0, 0, 0, 2];
        assert!(
            machine
                .advance(
                    Some(target()),
                    channel,
                    parameters(),
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        let seed = random.0;
        let orbit = machine.state().actor.orbit;
        assert_eq!(machine.state().actor.mode, 39);
        assert_eq!(machine.state().timer, 3.);
        assert_ne!(orbit.radius, 10.);
        assert!(
            machine
                .advance(
                    Some(target()),
                    channel,
                    parameters(),
                    &mut random,
                    &mut backend
                )
                .is_err()
        );
        assert_eq!((backend.sound, backend.fire, random.0), (1, 0, seed));
        assert_eq!(
            machine.state().actor.orbit.radius.to_bits(),
            orbit.radius.to_bits()
        );
    }
}
impl Parameters {
    pub fn from_globals(
        data: &giants_formats::global_data::GlobalData,
    ) -> Result<Self, giants_formats::Error> {
        Ok(Self {
            repeat_percent: data.scalar(0xe8)?,
            charge_percent: data.scalar(0xec)?,
            announce_percent: data.scalar(0x3e4)?,
            vertical_spread_percent: data.scalar(0xf0)?,
            orbit: crate::ripper_turn_completion::Parameters::from_globals(data)?,
        })
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Target<K> {
    pub actor: Candidate<K>,
    pub scale: f32,
    pub center_offset: f32,
}
#[derive(Clone, Debug)]
pub struct State<K> {
    pub actor: ripper_pursuit::State<K>,
    pub timer: f32,
}
pub trait Services<K> {
    fn owner_sound(
        &mut self,
        state: &State<K>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String>;
    fn fire(
        &mut self,
        state: &State<K>,
        target: [f32; 3],
        vertical_spread_percent: f32,
        random: &mut Random,
    ) -> Result<FireOutcome, String>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    ContinuePrimaryTail,
    FireRequested(FireOutcome),
}
#[derive(Clone, Debug)]
pub struct Machine<K> {
    state: State<K>,
    failed: Option<String>,
    consumed: bool,
}
impl<K: Copy + Ord> Machine<K> {
    /// Ordinary class230 mode65. This is the primary's launch request, not its
    /// secondary animation callback or an acknowledgement of a projectile hit.
    pub fn advance_variant(
        &mut self,
        target: Option<Target<K>>,
        primary: [u32; 9],
        parameters: VariantParameters,
        random: &mut Random,
        services: &mut impl Services<K>,
    ) -> Result<Outcome, String> {
        if self.failed.is_some() || self.consumed {
            return Err("variant attack continuation already consumed or failed".into());
        }
        if self.state.actor.class != 230 || self.state.actor.mode != 65 {
            return Err("variant attack requires ordinary class230/mode65".into());
        }
        if self.state.actor.retaliation != target.as_ref().map(|t| t.actor.id) {
            return Err("variant attack target identity absent or inconsistent".into());
        }
        let mut state = self.state.clone();
        let (lost, control) = ripper_pursuit::target_control(
            target.as_ref().map(|t| &t.actor),
            state.actor.retained_timer,
            state.actor.control,
        )?;
        state.actor.control = control;
        if lost {
            if let Err(error) = services.owner_sound(&state, 1248, random) {
                return self.fail(state, error);
            }
            state.actor.mode = 71;
            state.actor.retaliation = None;
        } else {
            let target = target.unwrap();
            state.actor.steering =
                ripper_steering::aim_at_point(state.actor.steering, target.actor.position)?;
            if primary[8] & 2 == 0 {
                let now = f32::from_bits(primary[1]);
                let previous = f32::from_bits(primary[2]);
                if !now.is_finite() || !previous.is_finite() {
                    return Err("variant attack clock outside finite profile".into());
                }
                let at = f32::from_bits(0x3edd_ddde);
                if previous < at && now >= at {
                    let position = [
                        target.actor.position[0],
                        target.actor.position[1],
                        (f64::from(target.actor.position[2])
                            + f64::from(target.center_offset) * f64::from(target.scale))
                            as f32,
                    ];
                    if !position
                        .into_iter()
                        .chain([
                            target.scale,
                            target.center_offset,
                            parameters.vertical_spread_percent,
                        ])
                        .all(f32::is_finite)
                    {
                        return Err("variant attack fire target outside finite profile".into());
                    }
                    match services.fire(
                        &state,
                        position,
                        parameters.vertical_spread_percent,
                        random,
                    ) {
                        Ok(outcome) => {
                            self.state = state;
                            self.consumed = true;
                            return Ok(Outcome::FireRequested(outcome));
                        }
                        Err(error) => return self.fail(state, error),
                    }
                }
            } else {
                if ![
                    parameters.close_range,
                    parameters.reconsider_range,
                    parameters.charge_percent,
                    parameters.cooldown,
                ]
                .into_iter()
                .all(f32::is_finite)
                {
                    return Err("variant attack decision parameters outside finite profile".into());
                }
                let dx = f64::from(target.actor.position[0])
                    - f64::from(state.actor.steering.position[0]);
                let dy = f64::from(target.actor.position[1])
                    - f64::from(state.actor.steering.position[1]);
                let distance = (dx * dx + dy * dy) as f32;
                if !distance.is_finite() {
                    return Err("variant attack distance overflow".into());
                }
                if f64::from(distance) < f64::from(parameters.close_range).powi(2) {
                    state.actor.mode = 67;
                } else if f64::from(distance) >= f64::from(parameters.reconsider_range).powi(2)
                    || f64::from(parameters.charge_percent)
                        <= f64::from((random.next_15() * 100) >> 15)
                {
                    state.timer = parameters.cooldown;
                    state.actor.mode = 66;
                } else {
                    state.actor.mode = 67;
                }
            }
        }
        self.state = state;
        self.consumed = true;
        Ok(Outcome::ContinuePrimaryTail)
    }
    pub fn new(state: State<K>) -> Self {
        Self {
            state,
            failed: None,
            consumed: false,
        }
    }
    pub fn state(&self) -> &State<K> {
        &self.state
    }
    pub fn failure(&self) -> Option<&str> {
        self.failed.as_deref()
    }
    fn fail(&mut self, state: State<K>, error: String) -> Result<Outcome, String> {
        self.state = state;
        self.failed = Some(error.clone());
        Err(error)
    }
    pub fn advance(
        &mut self,
        target: Option<Target<K>>,
        primary: [u32; 9],
        parameters: Parameters,
        random: &mut Random,
        services: &mut impl Services<K>,
    ) -> Result<Outcome, String> {
        if self.failed.is_some() || self.consumed {
            return Err("attack continuation already consumed or failed".into());
        }
        if self.state.actor.class != 73 || !matches!(self.state.actor.mode, 37..=39) {
            return Err("attack requires ordinary class73/modes37/38/39".into());
        }
        if self.state.actor.retaliation != target.as_ref().map(|t| t.actor.id) {
            return Err("attack target identity absent or inconsistent".into());
        }
        let mut state = self.state.clone();
        if state.actor.mode == 37 {
            state.actor.mode = 38;
        }
        let (lost, control) = ripper_pursuit::target_control(
            target.as_ref().map(|t| &t.actor),
            state.actor.retained_timer,
            state.actor.control,
        )?;
        state.actor.control = control;
        if lost {
            if let Err(error) = services.owner_sound(&state, 1269, random) {
                return self.fail(state, error);
            }
            state.actor.mode = 43;
            state.actor.retaliation = None;
        } else {
            let target = target.unwrap();
            state.actor.steering =
                ripper_steering::aim_at_point(state.actor.steering, target.actor.position)?;
            if state.actor.mode == 38 {
                if primary[8] & 2 != 0 {
                    state.actor.mode = 39;
                }
            } else if primary[8] & 2 == 0 {
                let now = f32::from_bits(primary[1]);
                let previous = f32::from_bits(primary[2]);
                if !now.is_finite() || !previous.is_finite() {
                    return Err("attack clock outside finite profile".into());
                }
                let at = f32::from_bits(0x3edd_ddde);
                if previous < at && now >= at {
                    let position = [
                        target.actor.position[0],
                        target.actor.position[1],
                        (f64::from(target.actor.position[2])
                            + f64::from(target.center_offset) * f64::from(target.scale))
                            as f32,
                    ];
                    if !position
                        .into_iter()
                        .chain([
                            target.scale,
                            target.center_offset,
                            parameters.vertical_spread_percent,
                        ])
                        .all(f32::is_finite)
                    {
                        return Err("attack fire target outside finite profile".into());
                    }
                    match services.fire(
                        &state,
                        position,
                        parameters.vertical_spread_percent,
                        random,
                    ) {
                        Ok(outcome) => {
                            self.state = state;
                            self.consumed = true;
                            return Ok(Outcome::FireRequested(outcome));
                        }
                        Err(error) => return self.fail(state, error),
                    }
                }
            } else {
                if ![
                    parameters.repeat_percent,
                    parameters.charge_percent,
                    parameters.announce_percent,
                    parameters.orbit.seconds,
                    parameters.orbit.radius[0],
                    parameters.orbit.radius[1],
                ]
                .into_iter()
                .all(f32::is_finite)
                {
                    return Err("attack decision parameters outside finite profile".into());
                }
                let roll = f64::from((random.next_15() * 100) >> 15);
                if roll < f64::from(parameters.repeat_percent) {
                    state.actor.mode = 37;
                } else if roll
                    >= f64::from(parameters.repeat_percent) + f64::from(parameters.charge_percent)
                {
                    state.actor.mode = 41;
                } else {
                    let bearing = crate::actor_volume::heading_wide(
                        target.actor.position,
                        state.actor.steering.position,
                    );
                    let low = (bearing - 80.) as f32;
                    let high = (bearing + 80.) as f32;
                    let interval = |random: &mut Random, low: f32, high: f32| {
                        (f64::from(random.next_15())
                            * (f64::from(high) - f64::from(low))
                            * (1. / 32768.)
                            + f64::from(low)) as f32
                    };
                    state.actor.orbit.angle =
                        ripper_steering::normalize(interval(random, low, high));
                    state.actor.orbit.radius = interval(
                        random,
                        parameters.orbit.radius[0],
                        parameters.orbit.radius[1],
                    );
                    state.timer = parameters.orbit.seconds;
                    let announce = (random.next_15() * 100) >> 15;
                    if f64::from(parameters.announce_percent) > f64::from(announce) {
                        if let Err(error) = services.owner_sound(&state, 1256, random) {
                            return self.fail(state, error);
                        }
                        state.actor.mode = 45;
                    } else {
                        state.actor.mode = 35;
                    }
                }
            }
        }
        self.state = state;
        self.consumed = true;
        Ok(Outcome::ContinuePrimaryTail)
    }
}
