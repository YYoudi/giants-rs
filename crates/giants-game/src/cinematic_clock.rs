//! Measured director clock correction; the audio backend supplies its own time.
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub seconds: f32,
    pub raw_dt: f32,
    pub finished: bool,
    pub sync: Option<AudioTime>,
}
#[derive(Clone, Copy, Debug)]
pub struct AudioTime {
    pub origin: f32,
    pub seconds: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct Step {
    pub seconds: f32,
    pub dt: f32,
    pub drift: Option<f32>,
}

pub fn handoff(origin: f32, director_seconds: f32) -> Result<f32, &'static str> {
    let result = (f64::from(origin) + f64::from(director_seconds)) as f32;
    if !origin.is_finite()
        || !director_seconds.is_finite()
        || director_seconds < 0.
        || !result.is_finite()
    {
        return Err("invalid director audio handoff");
    }
    Ok(result)
}

pub fn advance(input: Input) -> Result<Step, &'static str> {
    if !input.seconds.is_finite()
        || !input.raw_dt.is_finite()
        || !(0. ..=1.).contains(&input.raw_dt)
        || input
            .sync
            .is_some_and(|s| !s.origin.is_finite() || !s.seconds.is_finite())
    {
        return Err("invalid director clock input");
    }
    let seconds = if input.finished { 0. } else { input.seconds };
    let mut dt = if input.finished { 0. } else { input.raw_dt };
    let drift = input.sync.map(|sync| {
        let error =
            (f64::from(sync.origin) + f64::from(seconds) + f64::from(dt) - sync.seconds) as f32;
        if sync.seconds > 0. {
            if error > dt {
                dt = (f64::from(dt) / (f64::from(error) - f64::from(dt) + 1.)) as f32;
            } else if error < -dt {
                let corrected = (-f64::from(error) - f64::from(dt) + 1.) * f64::from(dt);
                dt = if corrected > 1. { 1. } else { corrected as f32 };
            }
        }
        error
    });
    let seconds = (f64::from(seconds) + f64::from(dt)) as f32;
    if !seconds.is_finite() || !dt.is_finite() || drift.is_some_and(|v| !v.is_finite()) {
        return Err("director clock overflow");
    }
    Ok(Step { seconds, dt, drift })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handoff_keeps_accumulated_time_and_rejects_invalid_state() {
        assert_eq!(handoff(-0.25, 1.5).unwrap(), 1.25);
        assert!(handoff(0., -1.).is_err());
        assert!(handoff(f32::NAN, 1.).is_err());
        assert!(handoff(f32::MAX, f32::MAX).is_err());
    }
    #[test]
    fn correction_deadband_reset_and_clamp() {
        let base = Input {
            seconds: 1.,
            raw_dt: 0.25,
            finished: false,
            sync: None,
        };
        assert_eq!(advance(base).unwrap().seconds, 1.25);
        let ahead = advance(Input {
            sync: Some(AudioTime {
                origin: 0.,
                seconds: 0.5,
            }),
            ..base
        })
        .unwrap();
        assert_eq!(ahead.dt, 1. / 6.);
        let behind = advance(Input {
            sync: Some(AudioTime {
                origin: 0.,
                seconds: 2.,
            }),
            ..base
        })
        .unwrap();
        assert_eq!(behind.dt, 0.375);
        let deadband = advance(Input {
            sync: Some(AudioTime {
                origin: 0.,
                seconds: 1.25,
            }),
            ..base
        })
        .unwrap();
        assert_eq!(deadband.dt, 0.25);
        assert_eq!(
            advance(Input {
                sync: Some(AudioTime {
                    origin: 0.,
                    seconds: 100.
                }),
                ..base
            })
            .unwrap()
            .dt,
            1.
        );
        assert_eq!(
            advance(Input {
                finished: true,
                ..base
            })
            .unwrap()
            .seconds,
            0.
        );
        assert!(
            advance(Input {
                raw_dt: f32::NAN,
                ..base
            })
            .is_err()
        );
    }
}
