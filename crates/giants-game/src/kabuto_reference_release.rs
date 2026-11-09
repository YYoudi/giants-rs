//! Acquired pre-query reference release. Current actor/payload ownership,
//! clock, state18 reset, physical dispatch and transport remain required.
use crate::{actor_fields::Fields, kabuto_geometry_mode::NotificationContext};

#[derive(Clone, Copy, Debug)]
pub struct State<K> {
    pub reference: Option<K>,
    pub remaining: f32,
    pub reference_scale: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct Metadata {
    pub physical_flags: u32,
    pub state: u32,
    pub parent_reference: i32,
    pub owner_word: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Notice<K> {
    pub captured_target: K,
    pub captured_scale: f32,
    pub target_owner_word: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    OtherOwner,
    QueryReference,
}
pub trait Services<K> {
    fn fields(&self, actor: K) -> Result<&Fields, String>;
    fn fields_mut(&mut self, actor: K) -> Result<&mut Fields, String>;
    /// Current complete words, without additional visibility/readiness filters.
    fn metadata(&self, actor: K) -> Result<Metadata, String>;
    fn seconds(&mut self, published: &mut State<K>, source: K) -> Result<f64, String>;
    fn notification_context(&self, source: K) -> Result<NotificationContext<'_>, String>;
    fn notify_release(
        &mut self,
        published: &mut State<K>,
        source: K,
        notice: Notice<K>,
    ) -> Result<(), String>;
    /// Clear only the parent relationship; retain reference/behavior words.
    fn clear_parent(&mut self, actor: K) -> Result<(), String>;
    fn set_state89_parameter(&mut self, actor: K, value: f32) -> Result<(), String>;
    /// Execute the actual acquired route/body with current ownership/lifetime.
    fn reset_state18(&mut self, published: &mut State<K>, actor: K) -> Result<(), String>;
    fn state3_scale(&self, actor: K) -> Result<f32, String>;
    /// Dispatch through the actual current physical owner. The shared impulse
    /// service's state guard and post-store notification still apply.
    fn impulse(
        &mut self,
        published: &mut State<K>,
        source: K,
        target: K,
        delta: [f32; 3],
    ) -> Result<(), String>;
}

/// The caller supplies the actual local-owner identity. This consumes no primary
/// or history token. Services see published state and may change it; errors retain
/// prior effects and require caller quarantine rather than wholesale replay.
pub fn advance<K: Copy + Eq>(
    state: &mut State<K>,
    source: K,
    local_owner: Option<K>,
    services: &mut impl Services<K>,
) -> Result<Next, String> {
    if local_owner != Some(source) {
        return Ok(Next::OtherOwner);
    }
    let active = match state.reference {
        Some(target) => services.metadata(target)?.physical_flags & 1 != 0,
        None => false,
    };
    if !active {
        state.remaining = 10.;
        return Ok(Next::QueryReference);
    }
    let seconds = services.seconds(state, source)?;
    if !seconds.is_finite() || seconds < 0. || !state.remaining.is_finite() {
        return Err("Kabuto release clock outside acquired finite profile".into());
    }
    // Unlike the later retention caller, this subtraction uses the wide clock
    // result directly. Its sign test precedes the second, stored-timer test.
    let remaining = f64::from(state.remaining) - seconds;
    state.remaining = remaining as f32;
    if remaining < 0. {
        state.remaining = 0.;
    }
    if state.remaining > 0. {
        return Ok(Next::QueryReference);
    }
    let scale = state.reference_scale;
    let target = state
        .reference
        .ok_or("Kabuto release reference removed before capture")?;
    if services
        .notification_context(source)?
        .eligible()
        .map_err(str::to_owned)?
    {
        let owner_word = services.metadata(target)?.owner_word;
        services.notify_release(
            state,
            source,
            Notice {
                captured_target: target,
                captured_scale: scale,
                target_owner_word: owner_word,
            },
        )?;
    }
    if scale != 0. {
        let current = services.fields(target)?.scale;
        let divided = (f64::from(current) / f64::from(scale)) as f32;
        if !scale.is_finite() || !current.is_finite() || !divided.is_finite() {
            return Err("Kabuto release scale outside acquired finite division".into());
        }
        services.fields_mut(target)?.scale = divided;
    }
    let target_state = services.metadata(target)?.state;
    services.clear_parent(target)?;
    match target_state {
        89 if services.metadata(target)?.parent_reference <= 1 => {
            services.set_state89_parameter(target, 1.)?;
        }
        18 => services.reset_state18(state, target)?,
        3 => {
            let scale = services.state3_scale(target)?;
            services.fields_mut(target)?.scale = scale;
        }
        _ => {}
    }
    let flags = &mut services.fields_mut(target)?.object_flags;
    *flags = (*flags & !0x80000) | 0x10000000;
    // The captured actor above and the current reference below need not agree
    // after a service. Preserve the original reread, including source aliasing.
    let current = state
        .reference
        .ok_or("Kabuto release reference absent before impulse")?;
    let target_position = services.fields(current)?.position;
    let source_position = services.fields(source)?.position;
    let direction: [f32; 3] = std::array::from_fn(|i| {
        (f64::from(target_position[i]) - f64::from(source_position[i])) as f32
    });
    if !target_position
        .into_iter()
        .chain(source_position)
        .chain(direction)
        .all(f32::is_finite)
    {
        return Err("Kabuto release direction outside acquired finite geometry".into());
    }
    let delta = crate::sun_glare::normalize(direction).map(|v| (f64::from(v) * 50.) as f32);
    services.impulse(state, source, current, delta)?;
    state.reference = None;
    Ok(Next::QueryReference)
}
