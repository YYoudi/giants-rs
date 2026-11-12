//! Owned PC scroll and single-segment fade setup/callback fields.
//! Prototype PS2 search routes `fx_setupUVScrolling`, `fx_setupVerticleFade`
//! and `fx_move` remain unknown attributions. No prototype layout is imported.
//! This supplies current parameters, not a vertex-buffer or rendering receipt.
use crate::{actor_fields::Fields, effect_scene::RenderCommand, state_storage::Seed};

#[derive(Default)]
pub(crate) struct Parameters {
    uv_rate: Option<[f32; 2]>,
    fade: Option<crate::vertical_fade::Playback>,
}
impl Parameters {
    pub(crate) fn configure(
        &mut self,
        command: &RenderCommand,
        fields: &mut Fields,
        seed: &mut Seed,
    ) -> Result<(), String> {
        if seed.storage.state() != 78
            || seed.storage.profile() != crate::state_storage::Profile::Debug
        {
            return Err("render setup requires its acquired PC state78 profile".into());
        }
        let data = seed
            .storage
            .data_mut()
            .ok_or("render setup payload absent")?;
        if data.len() != 268 {
            return Err("render setup requires the acquired state78 payload".into());
        }
        match command {
            RenderCommand::UvScroll(rate) => {
                if rate.iter().any(|v| !v.is_finite()) {
                    return Err("UV scroll rate nonfinite".into());
                }
                // Reconfiguration replaces rates without resetting current UV.
                data[0x80..0x84].copy_from_slice(&rate[0].to_le_bytes());
                data[0x84..0x88].copy_from_slice(&rate[1].to_le_bytes());
                fields.control_flags |= 0x2000_0000;
                fields.response_flags |= 0x80000;
                self.uv_rate = Some(*rate);
            }
            RenderCommand::VerticalFade(v) => {
                if self.fade.is_some() {
                    return Err(
                        "multiple vertical-fade segments outside acquired owner profile".into(),
                    );
                }
                let fade =
                    crate::vertical_fade::Playback::new(crate::vertical_fade::Configuration {
                        start: [v[0], v[1]],
                        end: [v[2], v[3]],
                        duration: v[4],
                    })?;
                fields.control_flags |= 0x200000;
                fields.response_flags |= 0x40000;
                // Own the dynamic allocation as Rust data, never a fabricated
                // native pointer in the legacy payload's pointer word.
                self.fade = Some(fade);
            }
        }
        Ok(())
    }
    pub(crate) fn control_mask(&self) -> u32 {
        (if self.uv_rate.is_some() {
            0x2000_0000
        } else {
            0
        }) | if self.fade.is_some() { 0x200000 } else { 0 }
    }
    pub(crate) fn validate(&self, fields: &Fields, seed: &Seed) -> Result<(), String> {
        let data = seed.storage.data().ok_or("render owner payload absent")?;
        let word = |at| u32::from_le_bytes(data[at..at + 4].try_into().unwrap());
        if data.len() != 268
            || (fields.control_flags & 0x2000_0000 != 0) != self.uv_rate.is_some()
            || (fields.control_flags & 0x200000 != 0)
                != self.fade.as_ref().is_some_and(|f| f.active)
        {
            return Err("render owner flags or payload differ".into());
        }
        if let Some(rate) = self.uv_rate
            && ([word(0x80), word(0x84)] != rate.map(f32::to_bits)
                || [word(0x88), word(0x8c)]
                    .iter()
                    .any(|v| !f32::from_bits(*v).is_finite())
                || fields.response_flags & 0x80000 == 0)
        {
            return Err("UV scroll owner differs from current payload".into());
        }
        if self.fade.is_some() && fields.response_flags & 0x40000 == 0 {
            return Err("vertical fade render mark absent".into());
        }
        Ok(())
    }
    /// Run only on a visible, non-first, non-expired callback before ANM.
    pub(crate) fn advance(
        &mut self,
        fields: &mut Fields,
        seed: &mut Seed,
        seconds: f32,
    ) -> Result<(), String> {
        self.validate(fields, seed)?;
        if !seconds.is_finite() || seconds < 0. {
            return Err("render callback delta outside acquired profile".into());
        }
        if let Some(rate) = self.uv_rate {
            let data = seed.storage.data_mut().unwrap();
            for (at, rate) in [0x88, 0x8c].into_iter().zip(rate) {
                let current = f32::from_le_bytes(data[at..at + 4].try_into().unwrap());
                let next = (f64::from(seconds) * f64::from(rate) + f64::from(current)) as f32;
                if !next.is_finite() {
                    return Err("UV scroll overflow".into());
                }
                data[at..at + 4].copy_from_slice(&next.to_le_bytes());
            }
        }
        if let Some(fade) = &mut self.fade {
            fade.advance(seconds)?;
            if !fade.active {
                fields.control_flags &= !0x200000;
            }
        }
        Ok(())
    }
    pub(crate) fn uv(&self, seed: &Seed) -> Option<[f32; 2]> {
        self.uv_rate?;
        let data = seed.storage.data()?;
        Some([0x88, 0x8c].map(|at| f32::from_le_bytes(data[at..at + 4].try_into().unwrap())))
    }
    pub(crate) fn fade(&self) -> Option<&crate::vertical_fade::Playback> {
        self.fade.as_ref()
    }
}
