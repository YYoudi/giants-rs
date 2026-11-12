//! Acquired debug-PC state101 damage decisions for destructible scenery.
//! Prototype PS2 `deathfx_shot` / `deathfx_logic` are search routes with unknown
//! exact PC attribution. This emits required services; it does not construct a
//! replacement, allocate particles, retire a world actor or validate fidelity.
use crate::building_damage::{Hit, Thresholds};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectSlot {
    Death,
    Hit,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Request {
    SpecialDeath,
    Effect { slot: EffectSlot, point: [f32; 3] },
    Replace { class: i32 },
    Retire,
}
#[derive(Clone, Copy, Debug)]
pub struct Projectile {
    pub class: i32,
    pub state: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Configuration {
    pub class: i32,
    pub limit: f32,
    pub replacement: i32,
    /// Presence of the positive source payload reference, separately bound to
    /// its captured image/entry. A zero raw pointer is not a missing lookup.
    pub death_effect: bool,
    pub hit_effect: bool,
    pub restricted: bool,
}
pub struct Outcome {
    pub damage: f32,
    pub accepted: bool,
    pub requests: Vec<Request>,
}
impl Configuration {
    pub fn hit(
        self,
        damage: f32,
        hit: Hit,
        projectile: Option<Projectile>,
        thresholds: Thresholds,
    ) -> Result<Outcome, &'static str> {
        if ![damage, self.limit, hit.amount]
            .into_iter()
            .chain(hit.origin)
            .chain(hit.impact.into_iter().flatten())
            .chain(thresholds.0)
            .all(f32::is_finite)
            || self.limit < 0.
        {
            return Err("death-FX hit outside finite nonnegative-limit profile");
        }
        let mut out = Outcome {
            damage,
            accepted: false,
            requests: vec![],
        };
        if self.restricted && !projectile.is_some_and(|p| p.class == 0x336 || p.state == 8) {
            return Ok(out);
        }
        out.accepted = true;
        out.damage = (f64::from(damage) + f64::from(hit.amount)) as f32;
        if !out.damage.is_finite() {
            return Err("death-FX damage overflow");
        }
        if out.damage > self.limit {
            out.damage = self.limit;
        }
        if out.damage >= self.limit {
            if self.class == 0x2a5 {
                out.requests.push(Request::SpecialDeath);
            }
            if self.death_effect {
                out.requests.push(Request::Effect {
                    slot: EffectSlot::Death,
                    point: hit.origin,
                });
            }
            if self.replacement > 0 {
                out.requests.push(Request::Replace {
                    class: self.replacement,
                });
            }
            out.requests.push(Request::Retire);
        } else if let Some(point) = hit.impact
            && self.hit_effect
        {
            let reciprocal = 1. / f64::from(self.limit);
            let before = ((1. - f64::from(damage) * reciprocal) * 100.) as f32;
            let after =
                ((1. - f64::from(out.damage) * reciprocal) * 100. - f64::from(0.01_f32)) as f32;
            if thresholds.0[..3].iter().any(|t| before > *t && after <= *t) {
                out.requests.push(Request::Effect {
                    slot: EffectSlot::Hit,
                    point,
                });
            }
        }
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Configuration {
        Configuration {
            class: 515,
            limit: 100.,
            replacement: 23,
            death_effect: true,
            hit_effect: true,
            restricted: false,
        }
    }
    fn hit(amount: f32) -> Hit {
        Hit {
            amount,
            impact: Some([7., 8., 9.]),
            origin: [1., 2., 3.],
            response_flags: 0,
        }
    }
    const LEVELS: Thresholds = Thresholds([75., 50., 15., 0.]);
    #[test]
    fn restricted_weapon_never_consumes_damage_or_services() {
        let c = Configuration {
            restricted: true,
            ..config()
        };
        for p in [None, Some(Projectile { class: 1, state: 0 })] {
            let o = c.hit(10., hit(100.), p, LEVELS).unwrap();
            assert!(!o.accepted);
            assert_eq!(o.damage, 10.);
            assert!(o.requests.is_empty());
        }
        for p in [
            Projectile {
                class: 0x336,
                state: 0,
            },
            Projectile { class: 1, state: 8 },
        ] {
            assert!(c.hit(10., hit(100.), Some(p), LEVELS).unwrap().accepted);
        }
    }
    #[test]
    fn lethal_services_replace_the_hit_effect_and_keep_source_order() {
        let c = Configuration {
            class: 0x2a5,
            ..config()
        };
        let o = c.hit(0., hit(150.), None, LEVELS).unwrap();
        assert_eq!(o.damage, 100.);
        assert_eq!(
            o.requests,
            [
                Request::SpecialDeath,
                Request::Effect {
                    slot: EffectSlot::Death,
                    point: [1., 2., 3.]
                },
                Request::Replace { class: 23 },
                Request::Retire
            ]
        );
        let o = config().hit(0., hit(60.), None, LEVELS).unwrap();
        assert_eq!(
            o.requests,
            [Request::Effect {
                slot: EffectSlot::Hit,
                point: [7., 8., 9.]
            }]
        );
    }
    #[test]
    fn zero_limit_and_negative_damage_are_not_house_semantics() {
        let c = Configuration {
            limit: 0.,
            replacement: 0,
            death_effect: false,
            ..config()
        };
        assert_eq!(
            c.hit(0., hit(0.), None, LEVELS).unwrap().requests,
            [Request::Retire]
        );
        let o = config().hit(10., hit(-20.), None, LEVELS).unwrap();
        assert_eq!(o.damage, -10.);
        assert!(o.requests.is_empty());
    }
}
