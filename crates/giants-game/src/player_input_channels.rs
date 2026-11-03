//! Gameplay channel assembly after physical bindings have been resolved.
//! Device polling, binding lookup and global UI commands remain separate.

#[derive(Clone, Copy, Debug)]
pub enum MouseGate {
    Enabled,
    /// The native mouse tail resets all three residuals.
    Blocked,
    /// The outer mouse-tail guard returns without changing residuals.
    Skipped,
}
#[derive(Clone, Copy, Debug)]
pub struct Mouse {
    pub gate: MouseGate,
    pub present: [bool; 3],
    pub values: [f32; 3],
    pub divisor: f32,
    /// 1=heading, 2=side, 3=forward, 4=look; other selectors write no channel.
    pub routes: [u8; 3],
    pub inverted: [bool; 3],
}
/// Finite normalized setting profile of the native sensitivity setter. Its
/// two slopes are intentional here; smoothing their boundary would differ.
pub fn sensitivity_divisor(setting: f32) -> Result<f32, &'static str> {
    if !setting.is_finite() {
        return Err("mouse sensitivity outside finite profile");
    }
    let scaled = (1. - f64::from(setting.clamp(0., 1.))) * 20.;
    let minimum = f64::from(0.1f32);
    let value = scaled.max(minimum);
    Ok((value * if value > 10. { 0.5 } else { minimum }) as f32)
}
impl Mouse {
    /// Raw signed device counters supplied at the native snapshot boundary.
    /// OS polling and physical routing selection are not performed here.
    pub fn from_raw(
        gate: MouseGate,
        counters: [i32; 3],
        setting: f32,
        routes: [u8; 3],
        inverted: [bool; 3],
    ) -> Result<Self, &'static str> {
        Ok(Self {
            gate,
            present: counters.map(|v| v != 0),
            values: counters.map(|v| v as f32),
            divisor: sensitivity_divisor(setting)?,
            routes,
            inverted,
        })
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    pub residuals: [f32; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    /// Heading, look, forward and side before the keyboard/mouse pass.
    pub axes: [i8; 4],
    pub buttons: u32,
    /// Resolved native movement actions36..39: forward, backward, right, left.
    pub movement: [bool; 4],
    pub mouse: Mouse,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Commands {
    pub axes: [i8; 4],
    pub buttons: u32,
}
impl Commands {
    pub fn bytes(self) -> [u8; 8] {
        let mut out = [0; 8];
        out[..4].copy_from_slice(&self.axes.map(|v| v as u8));
        out[4..].copy_from_slice(&self.buttons.to_le_bytes());
        out
    }
}
fn apply_movement(axes: &mut [i8; 4], held: [bool; 4]) {
    for (active, channel, value) in [
        (held[0], 2, 32),
        (held[1], 2, -32),
        (held[2], 3, 32),
        (held[3], 3, -32),
    ] {
        if active {
            axes[channel] = value;
        }
    }
}
/// Source-ordered keyboard writes, including opposite and diagonal directions.
pub fn movement_commands(held: [bool; 4]) -> [i8; 2] {
    let mut axes = [0; 4];
    apply_movement(&mut axes, held);
    [axes[2], axes[3]]
}
impl State {
    /// Finite, positive-divisor and signed32 conversion profile. Unsupported
    /// inputs are refused atomically; that recovery policy is not native error
    /// behavior. Residuals are retained before inversion and byte saturation.
    pub fn assemble(&mut self, frame: Frame) -> Result<Commands, &'static str> {
        let mut axes = frame.axes;
        apply_movement(&mut axes, frame.movement);
        let mut next = *self;
        match frame.mouse.gate {
            MouseGate::Skipped => {}
            MouseGate::Blocked => next.residuals = [0.; 3],
            MouseGate::Enabled => {
                for axis in 0..3 {
                    if !frame.mouse.present[axis] {
                        next.residuals[axis] = 0.;
                        continue;
                    }
                    if !frame.mouse.divisor.is_finite() || frame.mouse.divisor <= 0. {
                        return Err("input channel divisor outside positive finite profile");
                    }
                    let value = f64::from(frame.mouse.values[axis])
                        / f64::from(frame.mouse.divisor)
                        + f64::from(next.residuals[axis]);
                    let integer = value.trunc();
                    if !value.is_finite()
                        || integer < f64::from(i32::MIN)
                        || integer > f64::from(i32::MAX)
                    {
                        return Err("input channel value outside finite signed32 profile");
                    }
                    next.residuals[axis] = (value - integer) as f32;
                    let mut integer = integer as i32;
                    if frame.mouse.inverted[axis] {
                        integer = integer.wrapping_neg();
                    }
                    let value = integer.clamp(-128, 127) as i8;
                    let channel = match frame.mouse.routes[axis] {
                        1 => 0,
                        2 => 3,
                        3 => 2,
                        4 => 1,
                        _ => continue,
                    };
                    axes[channel] = value;
                }
            }
        }
        *self = next;
        Ok(Commands {
            axes,
            buttons: frame.buttons,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refused_snapshot_preserves_state_and_skipped_tail_accepts_no_provider() {
        let mut state = State {
            residuals: [0.25, -0.5, 0.75],
        };
        let before = state;
        let mut frame = Frame {
            axes: [1, 2, 3, 4],
            buttons: 9,
            movement: [false; 4],
            mouse: Mouse {
                gate: MouseGate::Enabled,
                present: [true; 3],
                values: [0.5, f32::NAN, 0.],
                divisor: 1.,
                routes: [1, 4, 0],
                inverted: [false; 3],
            },
        };
        assert!(state.assemble(frame).is_err());
        assert_eq!(state, before);
        frame.mouse.gate = MouseGate::Skipped;
        frame.mouse.divisor = f32::NAN;
        assert_eq!(state.assemble(frame).unwrap().axes, frame.axes);
        assert_eq!(state, before);
        frame.mouse.gate = MouseGate::Blocked;
        assert_eq!(state.assemble(frame).unwrap().axes, frame.axes);
        assert_eq!(state.residuals, [0.; 3]);
    }
}
