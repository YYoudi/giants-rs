//! Periodic, unbounded, position-based emission only. This is the measured
//! state78 update kernel, not effect allocation, command dispatch or rendering.
#[derive(Clone, Copy, Debug)]
pub struct Periodic {
    pub interval: f32,
    pub remaining: f32,
    pub phase: f32,
    pub previous_position: [f32; 3],
}
#[derive(Clone, Debug)]
pub struct Step {
    pub state: Periodic,
    pub positions: Vec<[f32; 3]>,
}
impl Periodic {
    /// Accept a supplied state rather than inventing the surrounding creation
    /// clock. Initial phase and cached position affect even the first emission.
    pub fn advance(self, dt: f32, position: [f32; 3]) -> Result<Step, &'static str> {
        if !dt.is_finite()
            || dt <= 0.
            || !self.interval.is_finite()
            || self.interval <= 0.
            || !self.remaining.is_finite()
            || !self.phase.is_finite()
            || self.previous_position.iter().any(|v| !v.is_finite())
            || position.iter().any(|v| !v.is_finite())
        {
            return Err("periodic emission outside finite positive-time profile");
        }
        let mut state = self;
        state.remaining = (f64::from(self.remaining) - f64::from(dt)) as f32;
        let mut positions = Vec::new();
        let mut pass = 1_f64;
        while state.remaining < 0. {
            if positions.len() >= 4096 {
                return Err("periodic emission catch-up budget");
            }
            let phase = pass * f64::from(self.interval) - f64::from(self.phase);
            let weight = phase / f64::from(dt);
            // Do not clamp: the native kernel can extrapolate, including from
            // the supplied cold cache. Each position is stored as binary32.
            let emitted = std::array::from_fn(|i| {
                let previous = f64::from(self.previous_position[i]);
                let delta = f64::from(position[i]) - previous;
                (delta * weight + previous) as f32
            });
            if emitted.iter().any(|v| !v.is_finite()) {
                return Err("periodic emission position overflow");
            }
            positions.push(emitted);
            let next = (f64::from(self.interval) + f64::from(state.remaining)) as f32;
            if next <= state.remaining {
                return Err("periodic emission interval cannot advance clock");
            }
            state.remaining = next;
            pass += 1.;
        }
        state.previous_position = position;
        state.phase = (f64::from(self.interval) - f64::from(state.remaining)) as f32;
        if !state.phase.is_finite() || !state.remaining.is_finite() {
            return Err("periodic emission clock overflow");
        }
        Ok(Step { state, positions })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catch_up_strict_boundary_and_cold_cache_are_preserved() {
        let initial = Periodic {
            interval: 0.5,
            remaining: 0.,
            phase: 0.,
            previous_position: [0.; 3],
        };
        let cold = initial.advance(0.01, [1., 2., 3.]).unwrap();
        assert_eq!(cold.positions, [[50., 100., 150.]]);
        let boundary = Periodic {
            remaining: 0.5,
            ..initial
        }
        .advance(0.5, [1.; 3])
        .unwrap();
        assert!(boundary.positions.is_empty());
        let caught = initial.advance(1.25, [5., 6., 7.]).unwrap();
        assert_eq!(caught.positions.len(), 3);
        assert_eq!(caught.state.remaining, 0.25);
        assert!(initial.advance(0., [0.; 3]).is_err());
        assert!(initial.advance(f32::NAN, [0.; 3]).is_err());
        assert!(
            Periodic {
                interval: 1e-10,
                ..initial
            }
            .advance(1., [0.; 3])
            .is_err()
        );
    }
}
