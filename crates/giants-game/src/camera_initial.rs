//! External PC loader-image camera words, before CRT/game startup runs.
//! These seed a cold camera lifecycle only; they do not establish the state at
//! mission entry, execute startup writers or reset a warm/re-entered lifecycle.
use crate::{camera_mode, camera_owner, camera_projection, executable_metadata};

#[derive(Clone, Copy, Debug)]
pub struct Cold {
    pub mode: u32,
    pub view: u32,
    pub transition: f32,
    pub reference: i32,
    pub reference_status: u32,
    pub override_enabled: u32,
    pub scaled_input: bool,
    pub extended_modes: bool,
    pub near: f32,
    pub far: f32,
    pub depth_reference: [f32; 3],
    pub depth_axis: [f32; 3],
}
impl Cold {
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        Self::decode(&executable_metadata::read(path)?)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        use executable_metadata::{Image, Layout};
        let image = Image::decode(bytes)?;
        if image.layout != Layout::DebugReference {
            return Err("Cold camera metadata layout unacquired".into());
        }
        let word = |address| image.initial_word(address - image.data.base);
        if word(0x55a1a0)? != 0 || word(0x635624)? != 0 {
            return Err(
                "Cold camera native owner requires an unacquired identity translation".into(),
            );
        }
        let f = |at| word(at).map(f32::from_bits);
        let out = Self {
            mode: word(0x55a18c)?,
            view: word(0x55a188)?,
            transition: f(0x55a190)?,
            reference: word(0x55a184)? as i32,
            reference_status: word(0x55a1bc)?,
            override_enabled: word(0x635620)?,
            scaled_input: word(0x63539f)? & 0xff != 0,
            extended_modes: word(0x635980)? != 0,
            near: f(0x5823c8)?,
            far: f(0x5823cc)?,
            depth_reference: [f(0x5e01a4)?, f(0x5e01a8)?, f(0x5e01ac)?],
            depth_axis: [f(0x5e01c8)?, f(0x5e01cc)?, f(0x5e01d0)?],
        };
        if out.view > 2
            || ![out.transition, out.near, out.far]
                .into_iter()
                .chain(out.depth_reference)
                .chain(out.depth_axis)
                .all(f32::is_finite)
            || out.near <= 0.
            || out.far <= out.near
        {
            return Err("Cold camera words outside acquired finite profile".into());
        }
        Ok(out)
    }
    pub fn modes(&self) -> Result<camera_mode::State, String> {
        camera_mode::State::new(self.mode, self.view, self.transition)
    }
    pub fn owners<K: Copy>(&self) -> camera_owner::State<K> {
        camera_owner::State::new(None, self.reference_status)
    }
    pub fn owner_input<K>(&self, controlled: Option<K>) -> camera_owner::Input<K> {
        camera_owner::Input {
            controlled,
            reference: self.reference,
            override_enabled: self.override_enabled,
            override_owner: None,
        }
    }
    /// Initial projection context only. Later calls must use the current depth
    /// caches; actual viewport dimensions are supplied by the rendering owner.
    pub fn projection(&self, viewport: [f32; 2]) -> camera_projection::Parameters {
        camera_projection::Parameters {
            aspect: None,
            viewport,
            near: self.near,
            far: self.far,
            limit_far: true,
            depth_reference: self.depth_reference,
            depth_axis: self.depth_axis,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cold_words_respect_raw_data_and_pe_zero_fill_without_translating_pointers() {
        use executable_metadata::{Image, Layout, synthetic_image};
        let mut bytes = synthetic_image(Layout::DebugReference);
        // Extend only the synthetic data mapping into its valid uninitialized
        // tail; no original image or original coefficient table is embedded.
        bytes[464..468].copy_from_slice(&1_055_540u32.to_le_bytes());
        let put = |bytes: &mut [u8], address: u32, value: u32| {
            let image = Image::decode(bytes).unwrap();
            let data = image.data.data(address - image.data.base, 4).unwrap();
            let at = data.as_ptr() as usize - bytes.as_ptr() as usize;
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        };
        put(&mut bytes, 0x55a184, (-3i32) as u32);
        put(&mut bytes, 0x55a1bc, 9);
        put(&mut bytes, 0x5823c8, 0.75f32.to_bits());
        put(&mut bytes, 0x5823cc, 10_000f32.to_bits());
        let cold = Cold::decode(&bytes).unwrap();
        assert_eq!(cold.reference, -3);
        assert_eq!(cold.owners::<u32>().reference_status(), 9);
        assert_eq!(cold.owners::<u32>().owner(), None);
        assert_eq!(cold.depth_reference, [0.; 3]);
        assert!(!cold.scaled_input);
        let image = Image::decode(&bytes).unwrap();
        assert!(image.initial_word(0x25b000).is_err());
        put(&mut bytes, 0x55a1a0, 1);
        assert!(Cold::decode(&bytes).is_err());
        assert!(Cold::decode(&synthetic_image(Layout::Retail)).is_err());
    }
}
