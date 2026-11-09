//! Acquired common attachment request used by Kabuto. Actor/resource lifetime,
//! current binding rules, state transitions, animation and action dispatch remain
//! required owned services; this does not admit an actor to a world schedule.
use crate::{
    actor_fields::Fields, kabuto_contact::Request, kabuto_geometry_mode::NotificationContext,
};
pub mod catalog;

#[derive(Clone, Copy, Debug)]
pub struct Metadata {
    pub state: u32,
    pub type_id: i32,
    pub physical_flags: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub scale_factor: f32,
    pub parent_reference: u32,
    pub notify: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Rule {
    pub child_reference: u32,
    pub animation: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub type_id: i32,
    /// Acquired parent_reference indices0..3. Other matched indices need acquisition.
    pub bindings: [Rule; 4],
}
/// Preserve first-match/terminator ordering. The caller owns current rule data;
/// this does not embed an executable table or equate its keys with class IDs.
pub fn resolve(
    entries: &[Entry],
    type_id: i32,
    parent_reference: u32,
) -> Result<Option<Rule>, String> {
    let entry = entries
        .iter()
        .find(|entry| entry.type_id < 0 || entry.type_id == type_id)
        .ok_or("attachment rule table has no match or terminator")?;
    if entry.type_id != type_id {
        return Ok(None);
    }
    entry
        .bindings
        .get(parent_reference as usize)
        .copied()
        .map(Some)
        .ok_or_else(|| "attachment matched parent_reference outside acquired indices".into())
}
#[derive(Clone, Copy, Debug)]
pub struct Notice {
    pub scale_factor: f32,
    pub child_owner_word: u32,
    /// The transport carries the low byte; the binding stores the complete word.
    pub parent_reference: u8,
}
pub trait Services<K> {
    fn fields(&self, actor: K) -> Result<&Fields, String>;
    fn fields_mut(&mut self, actor: K) -> Result<&mut Fields, String>;
    fn metadata(&self, actor: K) -> Result<Metadata, String>;
    fn owner_word(&self, actor: K) -> Result<u32, String>;
    fn prepare_state3(&mut self, actor: K) -> Result<(), String>;
    fn prepare_state99(&mut self, actor: K) -> Result<(), String>;
    fn notification_context(&self, parent: K) -> Result<NotificationContext<'_>, String>;
    fn notify(&mut self, parent: K, notice: Notice) -> Result<(), String>;
    /// Direct owned setters preserve the other reference/binding fields.
    fn set_parent(&mut self, child: K, parent: K) -> Result<(), String>;
    fn set_parent_reference(&mut self, child: K, parent_reference: u32) -> Result<(), String>;
    /// First matching current type in the acquired ordered rule table. Absence
    /// must be established by its terminator; no invented empty/default table.
    /// Unsupported matched parent_reference indices need an explicit owned refusal.
    fn rule(&self, type_id: i32, parent_reference: u32) -> Result<Option<Rule>, String>;
    fn set_child_reference(&mut self, child: K, reference: u32) -> Result<(), String>;
    /// Current full slot word, flags0x21 and three zero parameters. Reuse the
    /// actual primary selector; do not truncate a slot before its native gate.
    fn select_animation(&mut self, child: K, slot: u32) -> Result<(), String>;
    fn action(&mut self, request: Request<K>) -> Result<(), String>;
}

/// Execute once with current ownership. Services observe each preceding store;
/// late errors retain effects and require owner quarantine rather than replay.
/// No native exception-path parity or lifetime/animation completion is implied.
pub fn bind<K: Copy>(
    parent: K,
    child: K,
    input: Input,
    services: &mut impl Services<K>,
) -> Result<(), String> {
    match services.metadata(child)?.state {
        3 => services.prepare_state3(child)?,
        99 => services.prepare_state99(child)?,
        _ => {}
    }
    if input.notify
        && services
            .notification_context(parent)?
            .eligible()
            .map_err(str::to_owned)?
    {
        services.notify(
            parent,
            Notice {
                scale_factor: input.scale_factor,
                child_owner_word: services.owner_word(child)?,
                parent_reference: input.parent_reference as u8,
            },
        )?;
    }
    services.set_parent(child, parent)?;
    services.set_parent_reference(child, input.parent_reference)?;
    if let Some(rule) = services.rule(services.metadata(child)?.type_id, input.parent_reference)? {
        services.set_child_reference(child, rule.child_reference)?;
        if services.fields(child)?.channels[0][0] != rule.animation {
            services.select_animation(child, rule.animation)?;
        }
    } else {
        services.set_child_reference(child, u32::MAX)?;
    }
    let scale = services.fields(child)?.scale;
    let scaled = (f64::from(scale) * f64::from(input.scale_factor)) as f32;
    if !scale.is_finite() || !input.scale_factor.is_finite() || !scaled.is_finite() {
        return Err("attachment scale outside acquired finite multiplication".into());
    }
    services.fields_mut(child)?.scale = scaled;
    if input.parent_reference as i32 <= 1 && services.metadata(child)?.physical_flags & 1 != 0 {
        let context = services.notification_context(parent)?;
        if context.session_mode != 4 || context.eligible().map_err(str::to_owned)? {
            services.action(Request {
                action: 3,
                amount: 20.,
                owner: Some(parent),
                peer: child,
                trailing: [0; 2],
            })?;
        }
    }
    Ok(())
}
