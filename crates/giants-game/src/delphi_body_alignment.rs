//! Acquired body pitch/roll producer in the measured finite Double53 profile.
//! Mode selection, immersion detection and world scheduling remain separate.

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub mode: i32,
    pub controls: u32,
    pub previous: [f32; 3],
    pub position: [f32; 3],
    pub pitch_roll: [f32; 2],
    /// Actual object-local time at the position before physical integration.
    pub seconds: f64,
}

pub fn advance(i: Input) -> Result<[f32; 2], &'static str> {
    let step = (i.seconds * 100.) as f32;
    if !i
        .previous
        .into_iter()
        .chain(i.position)
        .chain(i.pitch_roll)
        .all(f32::is_finite)
        || !i.seconds.is_finite()
        || i.seconds < 0.
        || !step.is_finite()
    {
        return Err("Delphi body alignment outside acquired finite time/pose profile");
    }
    let mut target = [0.; 2];
    if i.controls & 0x8000 != 0 && matches!(i.mode, 49 | 54 | 55 | 76) {
        let [x, y, z] =
            std::array::from_fn(|j| f64::from(i.position[j]) - f64::from(i.previous[j]));
        let horizontal = (y * y + x * x).sqrt();
        let angle = if horizontal == 0. && z == 0. {
            0.
        } else {
            // The acquired conversion factor is a promoted float, not 180/pi.
            (z.atan2(horizontal) * f64::from(57.295_776_f32)) as f32
        };
        match i.mode {
            49 | 76 => target[0] = angle,
            54 => target[1] = angle,
            55 => target[1] = -angle,
            _ => unreachable!(),
        }
    }
    Ok(std::array::from_fn(|j| {
        crate::actor_angle_approach::approach(i.pitch_roll[j], target[j], step)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn targets_depend_on_mode_and_control_while_zero_time_still_normalizes() {
        let input = Input {
            mode: 49,
            controls: 0x8000,
            previous: [0.; 3],
            position: [1., 0., 1.],
            pitch_roll: [0.; 2],
            seconds: 0.1,
        };
        assert_eq!(advance(input).unwrap(), [10., 0.]);
        assert_eq!(advance(Input { mode: 55, ..input }).unwrap(), [0., 350.]);
        assert_eq!(
            advance(Input {
                controls: 0,
                ..input
            })
            .unwrap(),
            [0.; 2]
        );
        assert_eq!(
            advance(Input {
                pitch_roll: [720., -120.],
                seconds: 0.,
                ..input
            })
            .unwrap(),
            [0., 240.]
        );
        assert_eq!(
            advance(Input {
                pitch_roll: [-0., -0.],
                seconds: 0.,
                controls: 0,
                ..input
            })
            .unwrap()
            .map(f32::to_bits),
            [(-0_f32).to_bits(); 2]
        );
        assert!(
            advance(Input {
                seconds: f64::NAN,
                ..input
            })
            .is_err()
        );
    }
}
