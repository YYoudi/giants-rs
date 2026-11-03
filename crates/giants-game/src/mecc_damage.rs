//! Versioned normal-Mecc health/shield arithmetic. Target eligibility, team rules,
//! gyro redirection, voice/knockback and the death callback remain caller work.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub limit: f32,
    pub damage: f32,
    pub shield: f32,
    pub flags: u32,
}
#[derive(Clone, Copy)]
pub struct Hit {
    pub kind: i32,
    pub amount: f32,
    pub globally_disabled: bool,
    pub cutscene: bool,
    /// Result of the separate attacker/team eligibility path.
    pub allowed: bool,
    pub shield_percent: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Result {
    pub state: State,
    pub died: bool,
    pub shield_disabled: bool,
    pub effective_amount: f32,
}
impl State {
    /// Cold object defaults; later class/state/mission overrides are separate.
    pub fn cold() -> Self {
        Self {
            limit: 100.,
            damage: 0.,
            shield: 100.,
            flags: 0,
        }
    }
    pub fn alive(self) -> bool {
        self.damage < self.limit
    }
    pub fn remaining(self) -> f32 {
        (self.limit - self.damage).max(0.)
    }
    pub fn apply(self, hit: Hit) -> std::result::Result<Result, &'static str> {
        self.apply_for(crate::state_storage::Profile::Gog, hit)
    }
    /// Exact-version arithmetic only. The caller still owns the real source,
    /// context and services; a death decision is not execution of death.
    pub fn apply_for(
        self,
        profile: crate::state_storage::Profile,
        hit: Hit,
    ) -> std::result::Result<Result, &'static str> {
        if ![
            self.limit,
            self.damage,
            self.shield,
            hit.amount,
            hit.shield_percent,
        ]
        .into_iter()
        .all(f32::is_finite)
            || self.limit <= 0.
            || !(0. ..=100.).contains(&hit.shield_percent)
            || self.shield < 0.
        {
            return Err("Mecc damage outside finite health/shield profile");
        }
        let mut result = Result {
            state: self,
            died: false,
            shield_disabled: false,
            effective_amount: hit.amount,
        };
        let was_dead = !self.alive();
        if hit.globally_disabled || (hit.amount < 0. && was_dead) {
            return Ok(result);
        }
        let mut amount = hit.amount;
        if (amount < 0. || hit.allowed) && !hit.cutscene {
            if amount >= 0. && result.state.flags & 0x800000 != 0 {
                // Absorbed damage stays wide through comparison/subtraction;
                // only the remainder and reserve assignments are float stores.
                let hundredth = f64::from(0.01_f32);
                let absorbed = f64::from(hit.shield_percent) * f64::from(amount) * hundredth;
                amount =
                    ((1. - f64::from(hit.shield_percent) * hundredth) * f64::from(amount)) as f32;
                if absorbed >= f64::from(result.state.shield) {
                    let reserve = result.state.shield;
                    result.state.shield = 0.;
                    if profile == crate::state_storage::Profile::Debug || hit.kind != 19 {
                        amount = ((absorbed - f64::from(reserve)) + f64::from(amount)) as f32;
                    }
                } else {
                    result.state.shield = (f64::from(result.state.shield) - absorbed) as f32;
                }
                if result.state.shield <= 0. {
                    result.state.shield = 0.;
                    result.shield_disabled = true;
                    result.state.flags &= !0x800000;
                }
            }
            result.state.damage = (f64::from(amount) + f64::from(result.state.damage)) as f32;
        }
        result.effective_amount = amount;
        match (profile, hit.kind) {
            (crate::state_storage::Profile::Gog, 20..=22)
            | (crate::state_storage::Profile::Debug, 18..=20) => result.state.flags |= 0x200000,
            (crate::state_storage::Profile::Gog, 13 | 14 | 2 | 18 | 26)
            | (crate::state_storage::Profile::Debug, 13 | 14 | 3 | 16) => {}
            _ => result.state.flags |= 0x10,
        }
        if result.state.damage >= result.state.limit {
            result.died = !was_dead;
            result.state.damage = result.state.limit;
        } else if result.state.damage < 0. {
            result.state.damage = 0.;
        }
        Ok(result)
    }
}
