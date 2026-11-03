//! Debug audio software-spatial kernel under supplied transform/owner inputs.
//! This is not a listener binding, device mixer, occlusion or audible proof.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub position: [f32; 3],
    pub pan: f32,
    pub attenuation: f32,
}
impl State {
    pub fn words(self) -> [u32; 5] {
        [
            self.position[0],
            self.position[1],
            self.position[2],
            self.pan,
            self.attenuation,
        ]
        .map(f32::to_bits)
    }
}
pub struct Input {
    pub flags: u32,
    pub hardware: bool,
    pub fixed_position: [f32; 3],
    pub owner_position: [f32; 3],
    pub previous: State,
    pub distances: [f32; 2],
    pub matrix: [f32; 16],
}
pub fn attenuation(distance: f32, minimum: f32, maximum: f32) -> Result<f32, &'static str> {
    if ![distance, minimum, maximum].iter().all(|v| v.is_finite())
        || distance < 0.
        || minimum <= 0.
        || maximum < minimum
    {
        return Err("sound distances outside finite positive profile");
    }
    Ok(if distance <= minimum {
        1.
    } else if distance >= maximum {
        0.
    } else {
        (f64::from(minimum) / f64::from(distance)) as f32
    })
}
pub fn update(input: Input) -> Result<State, &'static str> {
    let mut state = input.previous;
    if !input
        .matrix
        .iter()
        .chain(input.fixed_position.iter())
        .chain(input.owner_position.iter())
        .chain(state.position.iter())
        .chain([state.pan, state.attenuation].iter())
        .all(|v| v.is_finite())
    {
        return Err("nonfinite sound spatial input");
    }
    if input.flags & 0xe == 0 {
        return Ok(state);
    }
    state.position = if input.flags & 0x10000 != 0 {
        input.fixed_position
    } else if input.flags & 0x20000 != 0 {
        input.owner_position
    } else {
        [0.; 3]
    };
    if input.hardware {
        return Ok(state);
    }
    let m = input.matrix;
    let p = state.position.map(f64::from);
    let transformed: [f32; 3] = std::array::from_fn(|i| {
        (f64::from(m[i]) * p[0]
            + f64::from(m[4 + i]) * p[1]
            + f64::from(m[8 + i]) * p[2]
            + f64::from(m[12 + i])) as f32
    });
    let v = transformed.map(f64::from);
    let squared = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]) as f32;
    if !squared.is_finite() {
        return Err("sound spatial distance overflow");
    }
    let distance = if squared > 0. {
        f64::from(squared).sqrt() as f32
    } else {
        1.
    };
    if input.flags & 4 != 0 {
        let inverse = (1. / f64::from(distance)) as f32;
        let x = (f64::from(transformed[0]) * f64::from(inverse)) as f32;
        let z = (f64::from(transformed[2]) * f64::from(inverse)) as f32;
        state.pan = (f64::from(x) * f64::from(m[9]) - f64::from(z) * f64::from(m[8])) as f32;
        if !state.pan.is_finite() {
            return Err("sound spatial pan overflow");
        }
    }
    if input.flags & 2 != 0 {
        state.attenuation = attenuation(distance, input.distances[0], input.distances[1])?;
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distance_boundary_and_unsupported_ranges_are_explicit() {
        assert_eq!(attenuation(10., 10., 10.), Ok(1.));
        assert_eq!(
            attenuation(f32::from_bits(10f32.to_bits() + 1), 10., 10.),
            Ok(0.)
        );
        for values in [
            [-1., 1., 2.],
            [1., 0., 2.],
            [1., 2., 1.],
            [f32::INFINITY, 1., 2.],
        ] {
            assert!(attenuation(values[0], values[1], values[2]).is_err());
        }
    }
    #[test]
    fn hardware_path_preserves_software_outputs_after_position_resolution() {
        let state = update(Input {
            flags: 0x1000e,
            hardware: true,
            fixed_position: [1., 2., 3.],
            owner_position: [4.; 3],
            previous: State {
                position: [9.; 3],
                pan: -0.25,
                attenuation: 0.75,
            },
            distances: [10., 150.],
            matrix: [0.; 16],
        })
        .unwrap();
        assert_eq!(
            state.words(),
            [
                1f32.to_bits(),
                2f32.to_bits(),
                3f32.to_bits(),
                (-0.25f32).to_bits(),
                0.75f32.to_bits()
            ]
        );
    }
}
