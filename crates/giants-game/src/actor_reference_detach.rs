//! Shared acquired target-detachment stores, without caller selection,
//! transport, impulse, reference publication or world scheduling.
use crate::actor_fields::Fields;

pub trait Services<K, P> {
    fn fields(&self, actor: K) -> Result<&Fields, String>;
    fn fields_mut(&mut self, actor: K) -> Result<&mut Fields, String>;
    fn state(&self, actor: K) -> Result<u32, String>;
    fn parent_reference(&self, actor: K) -> Result<i32, String>;
    /// Remove only the parent relationship, retaining the other header words.
    fn clear_parent(&mut self, actor: K) -> Result<(), String>;
    fn set_state89_parameter(&mut self, actor: K, value: f32) -> Result<(), String>;
    fn reset_state18(&mut self, published: &mut P, actor: K) -> Result<(), String>;
    fn state3_scale(&self, actor: K) -> Result<f32, String>;
}

/// Preserve the captured target/scale across caller notifications. A late
/// failure retains preceding stores and callbacks; the caller owns quarantine.
pub fn advance<K: Copy, P>(
    target: K,
    scale: f32,
    published: &mut P,
    services: &mut impl Services<K, P>,
) -> Result<(), String> {
    if scale != 0. {
        let current = services.fields(target)?.scale;
        let divided = (f64::from(current) / f64::from(scale)) as f32;
        if !scale.is_finite() || !current.is_finite() || !divided.is_finite() {
            return Err("reference release scale outside acquired finite division".into());
        }
        services.fields_mut(target)?.scale = divided;
    }
    let state = services.state(target)?;
    services.clear_parent(target)?;
    match state {
        89 if services.parent_reference(target)? <= 1 => {
            services.set_state89_parameter(target, 1.)?;
        }
        18 => services.reset_state18(published, target)?,
        3 => {
            let scale = services.state3_scale(target)?;
            services.fields_mut(target)?.scale = scale;
        }
        _ => {}
    }
    let flags = &mut services.fields_mut(target)?.object_flags;
    *flags = (*flags & !0x80000) | 0x10000000;
    Ok(())
}
