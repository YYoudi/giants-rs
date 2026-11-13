//! Acquired debug-PC house damage and health-animation decisions.
//!
//! Prototype PS2 search routes: `house_shot`, `house_process`,
//! `house_handle_destruction`. Exact function attributions remain unknown.
//! The private records pin both binaries and preserve the original PC aliases.
//! This kernel emits required services; it does not allocate an effect, play a
//! sound, destroy an actor, update a collider or acknowledge runtime admission.
#[derive(Clone, Copy, Debug)]
pub struct Thresholds(pub [f32; 4]);
impl Thresholds {
    pub fn load(data: &giants_formats::global_data::GlobalData) -> Result<Self, String> {
        Ok(Self([
            data.scalar(0xb74).map_err(|e| e.to_string())?,
            data.scalar(0xb78).map_err(|e| e.to_string())?,
            data.scalar(0xb7c).map_err(|e| e.to_string())?,
            data.scalar(0xb80).map_err(|e| e.to_string())?,
        ]))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoneEffect {
    Small,
    Large,
}

/// Ordered external calls/stores. A consumer must execute these with the real
/// owner and resources; recording a request alone cannot satisfy the service.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    Stone { kind: StoneEffect, point: [f32; 3] },
    Sound { selector: u32, point: [f32; 3] },
    MissionDeath,
    MarkTerminal,
    Retire,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct House {
    pub damage: f32,
    pub limit: f32,
}

pub struct Hit {
    pub amount: f32,
    pub impact: Option<[f32; 3]>,
    pub origin: [f32; 3],
    pub response_flags: u32,
}

pub struct Outcome {
    pub state: House,
    pub accepted: bool,
    pub requests: Vec<Request>,
}

impl House {
    fn validate(self, thresholds: Thresholds) -> Result<(), &'static str> {
        if ![self.damage, self.limit].into_iter().all(f32::is_finite)
            || self.damage < 0.
            || self.limit < 0.
            || self.damage > self.limit
            || !thresholds.0.into_iter().all(f32::is_finite)
        {
            return Err("house damage outside finite fresh/accumulated profile");
        }
        Ok(())
    }

    /// Related search key `house_shot`; only the acquired PC decision body.
    /// Zero-limit fresh houses reject hits. No arbitrary hit points are supplied.
    pub fn hit(self, hit: Hit, thresholds: Thresholds) -> Result<Outcome, &'static str> {
        self.validate(thresholds)?;
        if !hit.amount.is_finite()
            || !hit.origin.into_iter().all(f32::is_finite)
            || hit
                .impact
                .is_some_and(|p| !p.into_iter().all(f32::is_finite))
        {
            return Err("house hit requires finite actual damage and positions");
        }
        let mut result = Outcome {
            state: self,
            accepted: false,
            requests: vec![],
        };
        if hit.amount <= 0. || self.damage >= self.limit {
            return Ok(result);
        }
        result.accepted = true;
        let accumulated = (f64::from(self.damage) + f64::from(hit.amount)) as f32;
        result.state.damage = if accumulated > self.limit {
            self.limit
        } else {
            accumulated
        };
        // The native reciprocal stays wide. Rounding it to binary32 here
        // spuriously re-crosses a previously reached destruction threshold.
        let reciprocal = 1. / f64::from(self.limit);
        let before = ((1. - f64::from(self.damage) * reciprocal) * 100.) as f32;
        let after = ((1. - f64::from(result.state.damage) * reciprocal) * 100.
            - f64::from(0.01_f32)) as f32;
        let crosses = |threshold| before > threshold && after <= threshold;
        let terminal = crosses(thresholds.0[3]);
        if let Some(point) = hit.impact
            && thresholds.0[..3].iter().copied().any(crosses)
        {
            result.requests.push(Request::Stone {
                kind: StoneEffect::Small,
                point,
            });
            if !terminal {
                result.requests.push(Request::Sound {
                    selector: 0x614,
                    point,
                });
            }
        }
        if terminal {
            if let Some(point) = hit.impact {
                result.requests.push(Request::Stone {
                    kind: StoneEffect::Large,
                    point,
                });
            }
            if hit.response_flags & 0xf == 2 {
                result.requests.push(Request::MissionDeath);
            }
            result.requests.push(Request::MarkTerminal);
            result.requests.push(Request::Sound {
                selector: 0x615,
                point: hit.origin,
            });
            result.requests.push(Request::Retire);
        }
        Ok(result)
    }

    /// Related search key `house_handle_destruction`: the channel receives a
    /// fraction and its existing source bit80. Duration conversion and actual
    /// skinning/collider rebuild remain with their respective owners.
    pub fn animation_fraction(self, thresholds: Thresholds) -> Result<f32, &'static str> {
        self.validate(thresholds)?;
        let remaining = (1. - f64::from(self.damage) / f64::from(self.limit)) as f32;
        if remaining <= 0. {
            return Ok(0.);
        }
        let adjusted = (f64::from(remaining) - 0.01_f64) as f32;
        for index in [2, 1, 0] {
            let boundary = (f64::from(thresholds.0[index]) * f64::from(0.01_f32)) as f32;
            if adjusted <= boundary {
                return Ok(boundary);
            }
        }
        // The acquired zero-limit fresh profile has an unordered intermediate;
        // none of the comparisons selects a damaged channel fraction.
        Ok(1.)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hit(amount: f32, impact: bool) -> Hit {
        Hit {
            amount,
            impact: impact.then_some([7., 8., 9.]),
            origin: [1., 2., 3.],
            response_flags: 2,
        }
    }
    const LEVELS: Thresholds = Thresholds([75., 50., 15., 0.]);
    #[test]
    fn several_crossed_intermediate_levels_request_one_real_impact_effect() {
        let result = House {
            damage: 0.,
            limit: 100.,
        }
        .hit(hit(60., true), LEVELS)
        .unwrap();
        assert_eq!(result.state.damage, 60.);
        assert_eq!(
            result.requests,
            [
                Request::Stone {
                    kind: StoneEffect::Small,
                    point: [7., 8., 9.]
                },
                Request::Sound {
                    selector: 0x614,
                    point: [7., 8., 9.]
                },
            ]
        );
        assert_eq!(result.state.animation_fraction(LEVELS).unwrap(), 0.5);
    }
    #[test]
    fn lethal_crossing_preserves_impact_origin_mission_and_retirement_order() {
        let result = House {
            damage: 0.,
            limit: 100.,
        }
        .hit(hit(200., true), LEVELS)
        .unwrap();
        assert_eq!(result.state.damage, 100.);
        assert_eq!(
            result.requests,
            [
                Request::Stone {
                    kind: StoneEffect::Small,
                    point: [7., 8., 9.]
                },
                Request::Stone {
                    kind: StoneEffect::Large,
                    point: [7., 8., 9.]
                },
                Request::MissionDeath,
                Request::MarkTerminal,
                Request::Sound {
                    selector: 0x615,
                    point: [1., 2., 3.]
                },
                Request::Retire,
            ]
        );
        assert_eq!(result.state.animation_fraction(LEVELS).unwrap(), 0.);
        assert!(
            result
                .state
                .hit(hit(10., true), LEVELS)
                .unwrap()
                .requests
                .is_empty()
        );
    }
    #[test]
    fn absent_impact_does_not_fabricate_an_effect_or_suppress_actual_retirement() {
        let result = House {
            damage: 0.,
            limit: 100.,
        }
        .hit(hit(100., false), LEVELS)
        .unwrap();
        assert_eq!(
            result.requests,
            [
                Request::MissionDeath,
                Request::MarkTerminal,
                Request::Sound {
                    selector: 0x615,
                    point: [1., 2., 3.]
                },
                Request::Retire
            ]
        );
    }
    #[test]
    fn zero_limit_is_intact_and_nonpositive_or_postmortem_hits_are_inactive() {
        let house = House {
            damage: 0.,
            limit: 0.,
        };
        assert_eq!(house.animation_fraction(LEVELS).unwrap(), 1.);
        assert!(!house.hit(hit(100., true), LEVELS).unwrap().accepted);
        let house = House {
            damage: 0.,
            limit: 100.,
        };
        assert!(!house.hit(hit(-1., true), LEVELS).unwrap().accepted);
        assert_eq!(house.animation_fraction(LEVELS).unwrap(), 1.);
    }
    #[test]
    fn an_already_reached_threshold_does_not_explode_again() {
        let house = House {
            damage: 85.,
            limit: 100.,
        };
        assert!(
            house
                .hit(hit(1., true), LEVELS)
                .unwrap()
                .requests
                .is_empty()
        );
        assert_eq!(
            house.hit(hit(25., true), LEVELS).unwrap().requests,
            [
                Request::Stone {
                    kind: StoneEffect::Large,
                    point: [7., 8., 9.]
                },
                Request::MissionDeath,
                Request::MarkTerminal,
                Request::Sound {
                    selector: 0x615,
                    point: [1., 2., 3.]
                },
                Request::Retire,
            ]
        );
    }
}
