//! Source state98 building-hit decisions and the two distinct alert writes.
//! Prototype PS2 `barracks_shot`, `barracks_alert_rippers__FP6OBJECT`,
//! `rpman_alert_team__FP6OBJECTf` and `distancesq2d__FP3P3DT0` are unknown
//! attribution navigation routes. These acquired
//! PC contracts do not construct actors, admit world lists or execute NPC AI.
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug)]
pub struct State<K> {
    pub position: [f32; 3],
    pub target: Option<K>,
    pub phase: u32,
    pub selector: i32,
    pub damage: f32,
    pub limit: f32,
    pub mark_flags: u32,
    pub remaining: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Explosion {
    Mini,
    Barracks,
    Ripper,
}
impl Explosion {
    pub fn name(self) -> &'static str {
        match self {
            Self::Mini => "fx_minibarracks",
            Self::Barracks => "fx_barracks",
            Self::Ripper => "fx_ripperbarracks",
        }
    }
}
/// Actual stock and health-animation globals, not inferred defaults or an
/// actor census. All three thresholds remain owned by their source consumer.
#[derive(Clone, Copy, Debug)]
pub struct InitialParameters {
    pub mini: i32,
    pub barracks: i32,
    pub health: crate::building_damage::Thresholds,
}
impl InitialParameters {
    pub fn load(data: &giants_formats::global_data::GlobalData) -> Result<Self, String> {
        let word = |offset: usize| {
            data.words
                .get(offset / 4)
                .copied()
                .map(|v| v as i32)
                .ok_or_else(|| "barracks initial stock global absent".to_owned())
        };
        Ok(Self {
            mini: word(0xca0)?,
            barracks: word(0x5c8)?,
            health: crate::building_damage::Thresholds::load(data)?,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HealthFrame {
    pub fraction: f32,
    pub terminal: bool,
}
/// Acquired PC health-animation suffix reached by `barracks_logic` (unknown
/// attribution). Keep intermediate division/subtraction/products wide until
/// the selected fraction is stored. This deliberately differs from the house
/// callback's rounding. Neither a selected frame nor terminal health retires an
/// actor or acknowledges its primary/world services.
pub fn health_frame(
    damage: f32,
    limit: f32,
    thresholds: crate::building_damage::Thresholds,
) -> Result<HealthFrame, String> {
    if ![damage, limit]
        .into_iter()
        .chain(thresholds.0)
        .all(f32::is_finite)
        || damage < 0.
        || limit < 0.
    {
        return Err("barracks health frame outside finite nonnegative profile".into());
    }
    let remaining = 1. - f64::from(damage) / f64::from(limit);
    if remaining <= 0. {
        return Ok(HealthFrame {
            fraction: 0.,
            terminal: true,
        });
    }
    let adjusted = remaining - 0.01_f64;
    for index in [2, 1, 0] {
        let boundary = f64::from(thresholds.0[index]) * f64::from(0.01_f32);
        if adjusted <= boundary {
            return Ok(HealthFrame {
                fraction: boundary as f32,
                terminal: false,
            });
        }
    }
    // Zero/zero is unordered and selects no damaged fraction in the original.
    Ok(HealthFrame {
        fraction: 1.,
        terminal: false,
    })
}
/// Required object-owned sound request. Acceptance is distinct from the
/// subsequent audio device start; do not replace it with positional audio.
pub trait InitialSound<K> {
    fn object_sound(&mut self, state: &State<K>, cue: u32) -> Result<(), String>;
}
/// Acquired initial branch of `barracks_logic` (unknown attribution), including
/// its actual zero-damage health-animation suffix. This updates numeric channels,
/// not evaluated poses, a world tick, spawning or subsequent primary behavior.
/// On error retain this completed prefix and latch failure at the owning caller.
pub fn initialize<K>(
    state: &mut State<K>,
    class: i32,
    stock: InitialParameters,
    suppressed: bool,
    channels: &mut crate::animation_channels::Channels,
    sound: &mut impl InitialSound<K>,
) -> Result<bool, String> {
    if suppressed {
        return Ok(false);
    }
    if state.phase != 0
        || !matches!(class, 686 | 1060 | 1226)
        || !matches!(state.selector, 0..=5)
        || !state.limit.is_finite()
        || state.limit < 0.
        || !stock.health.0.into_iter().all(f32::is_finite)
    {
        return Err("barracks initialization outside acquired fresh class/selector profile".into());
    }
    state.phase = 1;
    state.damage = 0.;
    crate::animation_channels::select_primary(
        channels,
        crate::animation_channels::Request {
            slot: 1,
            flags: 0x84,
            speed_source: 0,
            reference_speed: 0.,
            blend_seconds: 0.,
        },
        crate::animation_channels::Blending::Disabled,
    )?;
    channels[0][1] = 1_f32.to_bits();
    let (cue, remaining) = if matches!(state.selector, 0 | 2 | 4) {
        state.selector = match class {
            1060 => 0,
            686 => 2,
            _ => 4,
        };
        match state.selector {
            0 => (0x6bf, stock.mini),
            2 => (0x6bf, stock.barracks),
            _ => (0x4db, 12),
        }
    } else {
        (if state.selector == 5 { 0x4db } else { 0x6bf }, 0)
    };
    sound.object_sound(state, cue)?;
    state.remaining = remaining;
    let health = health_frame(state.damage, state.limit, stock.health)?;
    channels[0][1] = health.fraction.to_bits();
    channels[0][8] |= 0x80;
    if health.terminal {
        state.remaining = 0;
    }
    Ok(true)
}
/// Each service must use its real owner. The caller must preflight every
/// required consumer before admitting ordinary damage; an error retains the
/// already completed prefix and must not be replayed as a fresh hit.
pub trait Services<K> {
    fn alert_team(&mut self, state: &State<K>, radius: f32) -> Result<(), String>;
    fn alert_rippers(&mut self, state: &State<K>, radius: f32) -> Result<(), String>;
    fn object_sound(&mut self, cue: u32) -> Result<(), String>;
    fn explosion(&mut self, kind: Explosion, point: [f32; 3]) -> Result<(), String>;
    fn positional_sound(&mut self, cue: u32, point: [f32; 3]) -> Result<(), String>;
    fn mission_trigger(&mut self, state: &State<K>) -> Result<(), String>;
    fn retire(&mut self) -> Result<(), String>;
}
/// `hit_owner` is the hit actor's source owner, with the real current player
/// supplied separately for the native fallback. This is not a house-stage hit:
/// damage is accumulated without clamping or health-animation thresholds.
pub fn hit<K: Copy>(
    state: &mut State<K>,
    amount: f32,
    hit_owner: Option<K>,
    player: K,
    radius_base: f32,
    services: &mut impl Services<K>,
) -> Result<bool, String> {
    if !state
        .position
        .into_iter()
        .chain([state.damage, state.limit, amount, radius_base])
        .all(f32::is_finite)
    {
        return Err("barracks hit outside finite acquired profile".into());
    }
    if state.damage >= state.limit {
        return Ok(false);
    }
    let damage = (f64::from(state.damage) + f64::from(amount)) as f32;
    if !damage.is_finite() {
        return Err("barracks accumulated damage overflow".into());
    }
    let radius = (f64::from(radius_base) + 20.) as f32;
    state.target = Some(hit_owner.unwrap_or(player));
    state.phase = 2;
    services.alert_team(state, radius)?;
    services.alert_rippers(state, radius)?;
    state.damage = damage;
    if damage < state.limit {
        services.object_sound(if matches!(state.selector, 0..=3) {
            0x60f
        } else {
            0x67b
        })?;
    } else {
        let kind = match state.selector {
            0 | 1 => Explosion::Mini,
            2 | 3 => Explosion::Barracks,
            _ => Explosion::Ripper,
        };
        services.explosion(kind, state.position)?;
        services.positional_sound(
            if matches!(state.selector, 0..=3) {
                0x610
            } else {
                0x67c
            },
            state.position,
        )?;
        if !matches!(state.selector, 4 | 5) && state.mark_flags & 15 == 2 {
            while state.remaining > 0 {
                services.mission_trigger(state)?;
                state.remaining -= 1;
            }
        }
        services.retire()?;
    }
    Ok(true)
}

#[derive(Clone, Copy, Debug)]
pub enum Data<K> {
    Other,
    Ripper {
        target: Option<K>,
    },
    Reaperman {
        /// Observed zero-word eligibility gate, without assigning an AI phase.
        opaque_0: u32,
        target: Option<K>,
        /// Retained words from the acquired PC mode102 gate. No broader
        /// personality, readiness or pointer-layout interpretation is assigned.
        flags_1b8: u32,
        opaque_1f0: u32,
        opaque_228: u32,
    },
}
#[derive(Clone, Copy, Debug)]
pub struct Candidate<K> {
    pub class: i32,
    pub state: u32,
    pub mode: u32,
    pub control_flags: u32,
    pub position: [f32; 3],
    pub data: Data<K>,
}
fn validate<K>(
    position: [f32; 3],
    radius: f32,
    candidates: &[Candidate<K>],
) -> Result<(), &'static str> {
    if !position.into_iter().chain([radius]).all(f32::is_finite)
        || candidates.iter().any(|c| {
            !c.position.into_iter().all(f32::is_finite)
                || c.class == 215 && !matches!(c.data, Data::Reaperman { .. })
                || c.state == 23 && !matches!(c.data, Data::Ripper { .. })
        })
    {
        return Err("building alert requires finite positions and actual class/state payloads");
    }
    Ok(())
}
fn distance(a: [f32; 3], b: [f32; 3]) -> f64 {
    let x = f64::from(b[0]) - f64::from(a[0]);
    let y = f64::from(b[1]) - f64::from(a[1]);
    x * x + y * y
}
/// Building-origin branch of the broader team helper: only class215, inclusive
/// horizontal radius, mode102 gates, and an unconditional target replacement.
/// The input order and completeness belong to the real world-list provider.
pub fn alert_team_from_building<K: Copy>(
    position: [f32; 3],
    radius: f32,
    target: K,
    candidates: &mut [Candidate<K>],
) -> Result<(), &'static str> {
    validate(position, radius, candidates)?;
    let squared = f64::from(radius) * f64::from(radius);
    for candidate in candidates {
        if candidate.class != 215 {
            continue;
        }
        let Data::Reaperman {
            opaque_0,
            target: current,
            flags_1b8,
            opaque_1f0,
            opaque_228,
        } = &mut candidate.data
        else {
            unreachable!()
        };
        if *opaque_0 != 0
            || candidate.mode == 102
                && (*opaque_1f0 != 0
                    || *flags_1b8 & 1 == 0
                    || candidate.control_flags & 0x1000000 != 0
                    || *opaque_228 != 0)
        {
            continue;
        }
        if distance(position, candidate.position) <= squared {
            candidate.control_flags |= 0x21;
            *current = Some(target);
        }
    }
    Ok(())
}
/// Actual state23 predicate, strict horizontal radius, and empty-target-only
/// assignment. The team helper above must complete first for the shot path.
pub fn alert_rippers<K: Copy>(
    position: [f32; 3],
    radius: f32,
    target: K,
    candidates: &mut [Candidate<K>],
) -> Result<(), &'static str> {
    validate(position, radius, candidates)?;
    let squared = f64::from(radius) * f64::from(radius);
    for candidate in candidates {
        if candidate.state != 23 || distance(position, candidate.position) >= squared {
            continue;
        }
        let Data::Ripper { target: current } = &mut candidate.data else {
            unreachable!()
        };
        if current.is_none() {
            *current = Some(target);
        }
    }
    Ok(())
}
