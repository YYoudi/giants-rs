//! Common reaction caller before the resource-driven action prefix.
//! Prototype PS2 `kabuto_move` remains an unknown parent navigation route.
//! Unacquired branches are explicit requirements, never successful fallbacks.
use crate::kabuto_action_select::Scratch;

#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub earlier_behavior: u32,
    pub previous_clip: u32,
    pub previous_selection: i32,
    pub wrapped: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Required {
    HeldReaction,
    GlobalRequest,
    ButtonSelection,
    Behavior(u32),
    ControlAction,
    Clip(u32),
    DefinitionEvents,
    FlaggedReaction,
    TargetReaction,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    ResourceCharge,
    CheckSelection,
    PrimaryTailRequired,
    Required(Required),
}
pub trait World<K: Copy> {
    fn flags(&self) -> Result<u32, String>;
    fn set_flags(&mut self, flags: u32) -> Result<(), String>;
    fn buttons(&self) -> Result<u32, String>;
    fn interaction(&self) -> Result<Option<K>, String>;
    fn class(&self, target: K) -> Result<u32, String>;
    fn held_present(&self) -> Result<bool, String>;
    fn global_request(&mut self) -> Result<bool, String>;
    fn tracked(&self) -> Result<(i32, u32), String>;
    fn clear_tracked(&mut self) -> Result<(), String>;
    fn checkpoint(&mut self, scratch: Scratch) -> Result<(), String>;
    fn stop_sound(&mut self, id: u32) -> Result<(), String>;
    fn notifications(&mut self) -> Result<bool, String>;
    fn notify_stop_sound(&mut self, packet: &mut u32) -> Result<(), String>;
    fn header_flags(&self) -> Result<u32, String>;
    fn physics_flags(&self) -> Result<u32, String>;
    fn seconds(&mut self) -> Result<f64, String>;
    fn cooldown(&self) -> Result<f32, String>;
    fn set_cooldown(&mut self, value: f32) -> Result<(), String>;
    fn configured_rate(&self) -> Result<f32, String>;
    fn set_rate(&mut self, value: f32) -> Result<(), String>;
    fn definition_flags(&self, key: u32) -> Result<u32, String>;
    fn related_class(&self) -> Result<Option<u32>, String>;
}

/// Prefix stores and required-provider effects survive refusal. A directed
/// selection/tail exit is not completion of that following phase or a primary.
pub fn advance<K: Copy>(
    scratch: &mut Scratch,
    input: Input,
    w: &mut impl World<K>,
) -> Result<Next, String> {
    let old = w.flags()?;
    let f = (old & !0xc000) | ((old & 0x4000) << 1);
    let f = (f & !0x3000) | ((f & 0x1000) << 1);
    let f = (f & !0x18) | ((f & 8) << 1);
    let f = (f & !6) | ((f & 2) << 1);
    let buttons = w.buttons()?;
    // This first store precedes even the target-class conditional.
    w.set_flags(f)?;
    let alternate = if f & 0x800 != 0 {
        true
    } else {
        w.interaction()?
            .map(|id| w.class(id).map(|c| c == 745))
            .transpose()?
            .unwrap_or(false)
    };
    let f = if alternate && buttons & 2 != 0 {
        f | if buttons & 4 != 0 { 0x4000 } else { 0 } | if buttons & 8 != 0 { 0x1000 } else { 0 }
    } else {
        f | if buttons & 4 != 0 { 8 } else { 0 } | if buttons & 8 != 0 { 2 } else { 0 }
    };
    w.set_flags(f)?;
    if w.held_present()? {
        return Ok(Next::Required(Required::HeldReaction));
    }
    if w.global_request()? {
        return Ok(Next::Required(Required::GlobalRequest));
    }
    if w.buttons()? & 0x40 != 0 {
        return Ok(Next::Required(Required::ButtonSelection));
    }
    let (slot, behavior) = w.tracked()?;
    if slot >= 0 {
        scratch.slot = slot as u32;
        scratch.behavior = behavior;
        w.clear_tracked()?;
        w.checkpoint(*scratch)?;
        return Ok(Next::CheckSelection);
    }
    if matches!(input.earlier_behavior, 12 | 14) {
        return Ok(Next::Required(Required::Behavior(input.earlier_behavior)));
    }
    w.checkpoint(*scratch)?;
    w.stop_sound(0x486)?;
    if w.notifications()? {
        scratch.retained_time = 0x486;
        w.checkpoint(*scratch)?;
        let result = w.notify_stop_sound(&mut scratch.retained_time);
        w.checkpoint(*scratch)?;
        result?;
    }
    if input.earlier_behavior == 13 && !input.wrapped {
        w.set_flags(w.flags()? | 0x300)?;
        return Ok(Next::PrimaryTailRequired);
    }
    if w.buttons()? & 0x1000 != 0 {
        return Ok(Next::Required(Required::ControlAction));
    }
    w.set_flags(w.flags()? & !0x20000)?;
    if matches!(input.previous_clip, 166 | 167 | 170) {
        return Ok(Next::Required(Required::Clip(input.previous_clip)));
    }
    if w.header_flags()? & 0x80000 != 0 || w.physics_flags()? & 0x80 != 0 {
        let seconds = w.seconds()?;
        let cooldown = w.cooldown()?;
        if !seconds.is_finite() || seconds < 0. || !cooldown.is_finite() {
            return Err("Kabuto reaction clock/cooldown outside finite profile".into());
        }
        let wide = f64::from(cooldown) - seconds;
        w.set_cooldown(wide as f32)?;
        if wide < 0. {
            w.set_cooldown(0.)?;
        }
        let rate = w.configured_rate()?;
        if !rate.is_finite() {
            return Err("Kabuto reaction configured rate outside finite profile".into());
        }
        w.set_rate(rate)?;
        w.set_flags(w.flags()? & !0x10000)?;
    }
    if matches!(input.previous_clip, 168 | 169 | 171) {
        return Ok(Next::Required(Required::Clip(input.previous_clip)));
    }
    if input.previous_selection >= 0
        && w.definition_flags(input.previous_selection as u32)? & 0x700 != 0
    {
        return Ok(Next::Required(Required::DefinitionEvents));
    }
    if w.related_class()?.is_some_and(|c| c != 745) {
        w.set_flags(w.flags()? & !0x800)?;
    }
    let flags = w.flags()?;
    if flags & 0x5000 != 0 {
        return Ok(Next::Required(Required::FlaggedReaction));
    }
    if flags & 0x800 != 0 {
        return Ok(Next::Required(Required::TargetReaction));
    }
    Ok(Next::ResourceCharge)
}

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub owner: crate::resident_states::Id,
    pub number: u64,
    pub input: Input,
    pub caller: Scratch,
    pub next: Next,
}
#[derive(Default)]
pub(crate) struct Phase {
    pub entered: Option<u64>,
    pub running: bool,
    pub completed: Option<u64>,
    pub caller: Option<Scratch>,
    pub view: Option<View>,
}
pub trait Services {
    fn global_request(&mut self, session: &crate::resident_states::Session)
    -> Result<bool, String>;
    fn stop_sound(
        &mut self,
        session: &mut crate::resident_states::Session,
        owner: crate::resident_states::Id,
        id: u32,
    ) -> Result<(), String>;
    fn network(
        &mut self,
        session: &crate::resident_states::Session,
        owner: crate::resident_states::Id,
    ) -> Result<crate::kabuto_resource_phase::Network, String>;
    fn notify_stop_sound(
        &mut self,
        session: &mut crate::resident_states::Session,
        owner: crate::resident_states::Id,
        packet: &mut u32,
    ) -> Result<(), String>;
    fn clock(
        &mut self,
        session: &crate::resident_states::Session,
        owner: crate::resident_states::Id,
    ) -> Result<crate::kabuto_resource_phase::Clock, String>;
}
