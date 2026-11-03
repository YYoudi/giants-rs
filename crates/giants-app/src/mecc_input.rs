//! Gameplay consumes the same per-profile physical bindings shown by the menu.
use bevy::prelude::*;
use giants_game::{control_layout, control_options::ControlOptions};
#[derive(Clone, Copy)]
pub enum Action {
    Fire,
    Thrust,
    Boost,
    View,
    Forward,
    Backward,
    Left,
    Right,
}
impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::Fire => "InpFire",
            Self::Thrust => "InpThrust",
            Self::Boost => "InpNitro",
            Self::View => "InpToggle1stPerson",
            Self::Forward => "InpRunFwd",
            Self::Backward => "InpRunBkwd",
            Self::Left => "InpSideLeft",
            Self::Right => "InpSideRight",
        }
    }
}
fn binding(options: Option<&ControlOptions>, action: Action) -> &str {
    let (row, (_, initial, _)) = control_layout::MECC
        .iter()
        .enumerate()
        .find(|(_, v)| v.0 == action.name())
        .expect("supported Mecc menu action");
    options
        .and_then(|o| o.0[0].get(&(row as u8)))
        .map_or(*initial, String::as_str)
}
pub fn pressed(
    options: Option<&ControlOptions>,
    action: Action,
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    edge: bool,
) -> bool {
    let physical = binding(options, action);
    match physical {
        "MouseLeft" => {
            if edge {
                mouse.just_pressed(MouseButton::Left)
            } else {
                mouse.pressed(MouseButton::Left)
            }
        }
        "MouseRight" => {
            if edge {
                mouse.just_pressed(MouseButton::Right)
            } else {
                mouse.pressed(MouseButton::Right)
            }
        }
        "MouseMiddle" => {
            if edge {
                mouse.just_pressed(MouseButton::Middle)
            } else {
                mouse.pressed(MouseButton::Middle)
            }
        }
        "" => false,
        _ => {
            if edge {
                keys.get_just_pressed()
                    .any(|k| format!("{k:?}") == physical)
            } else {
                keys.get_pressed().any(|k| format!("{k:?}") == physical)
            }
        }
    }
}
pub fn mouse_gain(options: Option<&ControlOptions>) -> f32 {
    0.0002 + options.map_or(0.5, |o| o.1) * 0.0036
}
/// Native movement-action order, sampled from the configured physical bindings.
pub fn movement_axes(
    options: Option<&ControlOptions>,
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
) -> Vec2 {
    let held = [
        Action::Forward,
        Action::Backward,
        Action::Right,
        Action::Left,
    ]
    .map(|action| pressed(options, action, keys, mouse, false));
    let [forward, side] = giants_game::player_input_channels::movement_commands(held);
    Vec2::new(f32::from(side) / 32., f32::from(forward) / 32.)
}
/// Host policy for the experimental owned driver. The existing UI preference
/// means normal mouse-down looks down; native UI default/routing provenance
/// remains separate from the acquired command arithmetic.
pub fn owned_commands(
    options: Option<&ControlOptions>,
    state: &mut giants_game::player_input_channels::State,
    delta: Vec2,
    enabled: bool,
    axes: Vec2,
    thrust: bool,
    boost: bool,
) -> Result<giants_game::player_input_channels::Commands, String> {
    let mut counters = [0; 3];
    if enabled {
        for (out, v) in counters[..2].iter_mut().zip([delta.x, delta.y]) {
            if !v.is_finite()
                || v.fract() != 0.
                || f64::from(v) < f64::from(i32::MIN)
                || f64::from(v) > f64::from(i32::MAX)
            {
                return Err("owned mouse sampling requires finite signed integer counters".into());
            }
            *out = v as i32;
        }
    }
    let mouse = giants_game::player_input_channels::Mouse::from_raw(
        if enabled {
            giants_game::player_input_channels::MouseGate::Enabled
        } else {
            giants_game::player_input_channels::MouseGate::Blocked
        },
        counters,
        options.map_or(0.5, |o| o.1),
        [1, 4, 0],
        [false, !options.is_some_and(|o| o.2), false],
    )?;
    state
        .assemble(giants_game::player_input_channels::Frame {
            axes: [0, 0, (axes.y * 32.) as i8, (axes.x * 32.) as i8],
            buttons: if thrust { 0x400 } else { 0 } | if boost { 0x800 } else { 0 },
            movement: [false; 4],
            mouse,
        })
        .map_err(str::to_owned)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opposite_and_shared_profile_bindings_follow_native_write_order() {
        let mut options = ControlOptions::default();
        let mut keys = ButtonInput::default();
        let mouse = ButtonInput::default();
        for key in [KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyA, KeyCode::KeyD] {
            keys.press(key);
        }
        assert_eq!(movement_axes(None, &keys, &mouse), Vec2::new(-1., -1.));
        for name in ["InpRunFwd", "InpRunBkwd", "InpSideRight", "InpSideLeft"] {
            let row = control_layout::MECC
                .iter()
                .position(|v| v.0 == name)
                .unwrap();
            options.bind(0, row, "ArrowUp");
        }
        assert_eq!(movement_axes(Some(&options), &keys, &mouse), Vec2::ZERO);
        keys.press(KeyCode::ArrowUp);
        assert_eq!(
            movement_axes(Some(&options), &keys, &mouse),
            Vec2::new(-1., -1.)
        );
    }
    #[test]
    fn profile_override_unbind_and_mouse_bindings_drive_gameplay() {
        let mut o = ControlOptions::default();
        let mut keys = ButtonInput::default();
        let mut mouse = ButtonInput::default();
        o.bind(0, 20, "ArrowUp");
        keys.press(KeyCode::KeyW);
        assert!(!pressed(Some(&o), Action::Forward, &keys, &mouse, false));
        keys.press(KeyCode::ArrowUp);
        assert!(pressed(Some(&o), Action::Forward, &keys, &mouse, false));
        o.bind(0, 20, "");
        assert!(!pressed(Some(&o), Action::Forward, &keys, &mouse, false));
        mouse.press(MouseButton::Right);
        assert!(pressed(None, Action::Thrust, &keys, &mouse, false));
        assert!(!pressed(None, Action::Boost, &keys, &mouse, false));
    }
    #[test]
    fn summed_native_controls_keep_the_diagonal_speed_advantage() {
        use giants_game::{
            arithmetic::Precision,
            mecc_control_force, mecc_control_target,
            mecc_motion::{self, Motion},
        };
        let simulate = |controls| {
            let desired = mecc_control_target::direct(
                giants_game::object_transform::euler([0.; 3], [0.; 3], 0.6).unwrap(),
                0.6,
                controls,
                [35.; 2],
                Precision::Double53,
            )
            .unwrap();
            let mut m = Motion {
                position: [0.; 3],
                velocity: [0.; 3],
                previous_velocity: [0.; 3],
                force: [0.; 3],
                mass: 20.,
                speed_limit: 800.,
                force_time_limit: 0.,
                stored_dt: 0.,
            };
            for _ in 0..720 {
                m = mecc_motion::integrate(m, 1. / 120., false, false, Precision::Double53)
                    .unwrap();
                let out = mecc_control_force::apply(
                    mecc_control_force::Input {
                        desired_velocity: desired,
                        velocity: m.velocity,
                        force: m.force,
                        gain: 200.,
                        slope_power: 2.,
                        mass: m.mass,
                        force_time_limit: m.force_time_limit,
                    },
                    Precision::Double53,
                )
                .unwrap();
                m.force = out.force;
                m.force_time_limit = out.force_time_limit;
            }
            m.velocity[0].hypot(m.velocity[1])
        };
        let straight = simulate([32, 0]);
        let diagonal = simulate([32, 32]);
        assert!((straight - 35.).abs() < 0.001);
        assert!((diagonal / straight - std::f32::consts::SQRT_2).abs() < 0.00001);
        println!(
            "mecc_diagonal_component straight={straight} diagonal={diagonal} ratio={}",
            diagonal / straight
        );
    }
}
