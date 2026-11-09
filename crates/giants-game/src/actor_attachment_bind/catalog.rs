//! Read-only binding rules from user-supplied initialized PE data. Layout
//! recognition is not authenticity, code execution or runtime actor admission.
use super::{Entry, Rule};
use crate::executable_metadata::{Image, Layout, word};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Catalog {
    entries: Box<[Entry]>,
}
impl Catalog {
    pub fn read(executable: &Path) -> Result<Self, String> {
        Self::decode(&crate::executable_metadata::read(executable)?)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let image = Image::decode(bytes)?;
        if image.layout != Layout::DebugReference {
            return Err("attachment rule metadata layout not acquired".into());
        }
        let mut entries = Vec::new();
        for index in 0..256u32 {
            let data = image.data.data(0x160310 + index * 56, 56)?;
            let type_id = word(data, 0)? as i32;
            let mut bindings = [Rule {
                child_reference: 0,
                animation: 0,
            }; 4];
            for (index, binding) in bindings.iter_mut().enumerate() {
                binding.child_reference = word(data, 12 + index * 4)?;
                binding.animation = word(data, 32 + index * 4)?;
            }
            entries.push(Entry { type_id, bindings });
            if type_id < 0 {
                return Ok(Self {
                    entries: entries.into_boxed_slice(),
                });
            }
        }
        Err("attachment rule metadata terminator absent within limit".into())
    }
    /// Source order, including the acquired negative terminator. Type keys are
    /// not equated with an owned class/specification identity by this reader.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub fn rule(&self, type_id: i32, parent_reference: u32) -> Result<Option<Rule>, String> {
        super::resolve(&self.entries, type_id, parent_reference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut bytes = crate::executable_metadata::synthetic_image(Layout::DebugReference);
        let at = 4096 + 0x160310 - 1_413_120;
        for (index, key) in [7i32, 7, -1].into_iter().enumerate() {
            let row = at + index * 56;
            bytes[row..row + 4].copy_from_slice(&(key as u32).to_le_bytes());
            for slot in 0..4 {
                bytes[row + 12 + slot * 4..row + 16 + slot * 4]
                    .copy_from_slice(&(100u32 + index as u32 + slot as u32).to_le_bytes());
                bytes[row + 32 + slot * 4..row + 36 + slot * 4]
                    .copy_from_slice(&(0x1000000u32 + index as u32 + slot as u32).to_le_bytes());
            }
        }
        bytes
    }
    #[test]
    fn relocated_data_retains_full_words_and_first_match_before_terminator() {
        let catalog = Catalog::decode(&fixture()).unwrap();
        assert_eq!(catalog.entries().len(), 3);
        let rule = catalog.rule(7, 3).unwrap().unwrap();
        assert_eq!(rule.child_reference, 103);
        assert_eq!(rule.animation, 0x1000003);
        assert!(catalog.rule(8, u32::MAX).unwrap().is_none());
        assert!(catalog.rule(7, 4).is_err());
    }
    #[test]
    fn unsupported_layout_and_unterminated_data_do_not_invent_rules() {
        for layout in [Layout::Retail, Layout::Original] {
            assert!(Catalog::decode(&crate::executable_metadata::synthetic_image(layout)).is_err());
        }
        let bytes = crate::executable_metadata::synthetic_image(Layout::DebugReference);
        assert!(Catalog::decode(&bytes).is_err());
        assert!(Catalog::decode(&fixture()[..5000]).is_err());
    }
}
