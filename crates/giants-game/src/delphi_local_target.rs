//! Local primary target publication and movement-readiness caller. Providers
//! own actual geometry, camera, item and linked-target work. This does not admit
//! a controller or replace first-turn/previous-phase caches with current poses.
use crate::delphi_target_readiness::{self as readiness, Linked};
#[cfg(test)]
mod tests;

#[derive(Clone, Debug)]
pub struct State<K, P> {
    pub action_available: u32,
    pub item_available: u32,
    pub movement_ready: u32,
    pub point_available: u32,
    pub highlighted: Option<K>,
    pub item_target: Option<K>,
    pub terrain_selected: u32,
    pub cooldown: f32,
    pub controls: u32,
    pub proxy: Option<P>,
    pub candidate: Option<K>,
    pub point: [f32; 3],
    pub terrain_point: [f32; 3],
    pub aim_point: [f32; 3],
    pub pending: Option<K>,
}

#[derive(Clone, Copy)]
pub struct Input {
    pub local_owner: bool,
    pub response_flags: u32,
    pub suppressed: bool,
    pub animation: i32,
    pub selected: i32,
    /// Actual selected slot's kind, including slot zero; not a default weapon.
    pub selected_kind: i32,
    pub selected_item: i32,
    pub item_kind: i32,
    pub pending_seconds: f32,
    pub position: [f32; 3],
    pub scale: f32,
    pub reference_scale: f32,
    pub height_limit: f32,
    pub reserve: f32,
    pub readiness: readiness::Parameters,
}

pub struct Terrain {
    pub hit: bool,
    /// Sampled/refined distance, including a miss's acquired overshoot.
    pub distance: f32,
}

pub trait World<K, P> {
    fn seconds(&mut self) -> Result<f64, String>;
    fn hide_proxy(&mut self, proxy: P) -> Result<(), String>;
    fn projectile_range(&mut self, definition: i32) -> Result<f32, String>;
    /// Called twice independently; a caller must not merge these source reads.
    fn viewport_route(&mut self) -> Result<bool, String>;
    fn terrain(
        &mut self,
        viewport: bool,
        range: f32,
        point: &mut [f32; 3],
    ) -> Result<Terrain, String>;
    fn query(
        &mut self,
        viewport: bool,
        distance: f32,
        point: &mut [f32; 3],
    ) -> Result<Option<K>, String>;
    fn eligible(&mut self, candidate: K) -> Result<bool, String>;
    fn personality(&mut self, candidate: K) -> Result<u32, String>;
    fn camera_fallback(&mut self) -> Result<[f32; 3], String>;
    fn pending(&mut self) -> Result<Option<K>, String>;
    fn item_eligible(&mut self, selected: i32) -> Result<bool, String>;
    /// Item queries can replace their own target slot, including before a
    /// later refusal. This is distinct from candidate/highlight/pending links.
    fn item_available(
        &mut self,
        selected: i32,
        terrain_point: [f32; 3],
        item_target: &mut Option<K>,
    ) -> Result<u32, String>;
    /// Run the acquired linked helper; retain its state even on failure.
    fn linked(&mut self, state: &mut Linked<K, P>) -> Result<(), String>;
    fn physics_blocks_action(&mut self) -> Result<bool, String>;
    fn busy(&mut self) -> Result<bool, String>;
    fn class(&mut self, candidate: K) -> Result<i32, String>;
    fn flags(&mut self, candidate: K) -> Result<u32, String>;
    /// Actual melee or viewport-distance branch and its strict range comparison.
    fn highlight_in_range(
        &mut self,
        candidate: K,
        selected: i32,
        kind: i32,
    ) -> Result<bool, String>;
}

fn fallback<K, P>(state: &mut State<K, P>, input: Input) -> Result<(), String> {
    if !input
        .position
        .into_iter()
        .chain(state.point)
        .chain([input.reserve, input.readiness.fallback_rate])
        .all(f32::is_finite)
        || !input.readiness.exponent.is_finite()
        || input.readiness.fallback_rate <= 0.
    {
        return Err("Delphi inline readiness outside acquired finite profile".into());
    }
    let delta: [f64; 3] =
        std::array::from_fn(|i| f64::from(state.point[i]) - f64::from(input.position[i]));
    // Unlike the linked helper, the inline caller stores a diagnostic f32
    // without popping/reloading the wide squared distance before the power.
    let squared = (delta[0] * delta[0] + delta[1] * delta[1]) + delta[2] * delta[2];
    let cost = squared.powf(input.readiness.exponent) * f64::from(input.readiness.fallback_rate);
    if !cost.is_finite() {
        return Err("Delphi inline readiness cost outside acquired finite profile".into());
    }
    if cost <= f64::from(input.reserve) {
        state.movement_ready = 1;
    }
    Ok(())
}
fn linked<K: Copy, P: Copy>(
    state: &mut State<K, P>,
    world: &mut impl World<K, P>,
) -> Result<(), String> {
    let mut value = Linked {
        prefix: readiness::State {
            fallback: state.point,
            readiness: state.movement_ready,
        },
        target: state.candidate.ok_or("Delphi linked candidate absent")?,
        proxy: state.proxy,
    };
    let result = world.linked(&mut value);
    state.point = value.prefix.fallback;
    state.movement_ready = value.prefix.readiness;
    state.candidate = Some(value.target);
    state.proxy = value.proxy;
    result
}

/// Acquired local block before authority-driven selection/input. Header inputs
/// remain fixed during these supplied providers. Geometry points can be written
/// before a provider fails; all preceding state is retained, never rolled back.
/// The owning frame must quarantine replay after an error.
pub fn advance<K: Copy, P: Copy>(
    state: &mut State<K, P>,
    input: Input,
    world: &mut impl World<K, P>,
) -> Result<(), String> {
    if !input.local_owner {
        return Ok(());
    }
    state.action_available = 0;
    state.item_available = 0;
    state.movement_ready = 0;
    state.point_available = 0;
    state.highlighted = None;
    state.item_target = None;
    state.terrain_selected = 0;
    if !state.cooldown.is_finite() {
        return Err("Delphi local cooldown outside finite profile".into());
    }
    if state.cooldown <= 0. {
        state.controls &= !8;
    } else {
        let seconds = world.seconds()?;
        if !seconds.is_finite() {
            return Err("Delphi local clock outside finite profile".into());
        }
        state.cooldown = (f64::from(state.cooldown) - seconds) as f32;
    }
    if let Some(proxy) = state.proxy {
        world.hide_proxy(proxy)?;
    }
    if input.response_flags & 0x04000000 != 0 || input.suppressed || input.animation == 0x4d {
        return Ok(());
    }
    let range = if input.selected < 1 {
        9000.
    } else {
        let id = match input.selected_kind {
            0x2c => 0x2b,
            0x2d => 0x16,
            0x2e => 0x15,
            0x2f | 0x30 => 0x17,
            _ => return Err("Delphi local target has an unacquired default range branch".into()),
        };
        world.projectile_range(id)?
    };
    let viewport = world.viewport_route()?;
    let terrain = world.terrain(viewport, range, &mut state.terrain_point)?;
    if terrain.hit {
        state.point = state.terrain_point;
        state.point_available = 1;
        if input.selected > 0 {
            state.terrain_selected = 1;
        }
    }
    let viewport = world.viewport_route()?;
    state.candidate = world.query(viewport, terrain.distance, &mut state.point)?;
    let mut accepted = false;
    if let Some(candidate) = state.candidate
        && world.eligible(candidate)?
    {
        if world.personality(candidate)? & 0x400000 != 0 {
            state.highlighted = Some(candidate);
        }
        accepted = true;
        state.point_available = 1;
        state.terrain_point = state.point;
    }
    state.aim_point = if state.candidate.is_some() {
        state.point
    } else if state.point_available != 0 {
        state.terrain_point
    } else {
        world.camera_fallback()?
    };
    if input.selected_kind == 0x2f {
        if input.pending_seconds > 0. {
            state.pending = world.pending()?;
        }
    } else {
        state.pending = None;
    }
    let free = state.controls & 0xd1bf0000 == 0 && input.reference_scale == input.scale;
    if state.point_available == 0 {
        if state.cooldown <= 0.
            && input.item_kind == 0x17
            && world.item_eligible(input.selected_item)?
        {
            state.item_available = world.item_available(
                input.selected_item,
                state.terrain_point,
                &mut state.item_target,
            )?;
        }
        if state.candidate.is_some() && free {
            state.candidate = None;
            fallback(state, input)?;
        }
        if input.selected_kind == 0x2f {
            state.action_available = 1;
        }
        return Ok(());
    }
    if free {
        if state.highlighted.is_none() {
            state.candidate = None;
        }
        if state.candidate.is_none() && !accepted {
            state.candidate = world.pending()?;
            let eligible = match state.candidate {
                Some(candidate) => world.eligible(candidate)?,
                None => false,
            };
            if eligible {
                linked(state, world)?;
            } else {
                state.candidate = None;
                if -input.height_limit < state.point[2] {
                    fallback(state, input)?;
                }
            }
        } else if state.highlighted.is_none() {
            if -input.height_limit < state.point[2] {
                fallback(state, input)?;
            }
        } else {
            linked(state, world)?;
        }
    }
    if state.cooldown <= 0. && world.item_eligible(input.selected_item)? {
        state.item_available = world.item_available(
            input.selected_item,
            state.terrain_point,
            &mut state.item_target,
        )?;
    }
    if world.physics_blocks_action()? || world.busy()? {
        return Ok(());
    }
    let Some(highlighted) = state.highlighted else {
        return Ok(());
    };
    if world.class(highlighted)? == 0xb {
        let personality = world.personality(highlighted)?;
        let flags = world.flags(highlighted)?;
        if personality & 0x20000 == 0 || flags & 0x4000c000 != 0 {
            return Ok(());
        }
    }
    if world.highlight_in_range(highlighted, input.selected, input.selected_kind)? {
        state.action_available = 1;
    }
    Ok(())
}
