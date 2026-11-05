//! Immutable character animation identifiers from user-supplied initialized PE data.
//! The file is read as data; native code is never invoked or distributed.
//! Layout recognition is not authentication or gameplay/platform parity.
use crate::{
    delphi_locomotion::Clips,
    executable_metadata::{Image, Layout, word},
};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Definitions {
    entries: [[u8; 2]; 94],
}
impl Definitions {
    pub fn read(executable: &Path) -> Result<Self, String> {
        Self::decode(&crate::executable_metadata::read(executable)?)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let image = Image::decode(bytes)?;
        let table = match image.layout {
            Layout::DebugReference => 0x167bd8,
            Layout::Retail => 0x160c50,
            Layout::Original => return Err("Delphi animation metadata layout not acquired".into()),
        };
        let data = image.data.data(table, 94 * 8)?;
        let mut entries = [[0; 2]; 94];
        for (i, row) in entries.iter_mut().enumerate() {
            for (column, slot) in row.iter_mut().enumerate() {
                let value = word(data, i * 8 + column * 4)?;
                if value > 254 {
                    return Err("Delphi animation identifier outside its source table".into());
                }
                *slot = value as u8;
            }
        }
        Ok(Self { entries })
    }
    /// Both equipment columns are retained; 254 is the external inactive slot.
    /// Reading a row does not admit its corresponding state-machine action.
    pub fn entries(&self) -> &[[u8; 2]; 94] {
        &self.entries
    }
    pub fn sword_locomotion(&self) -> Result<Clips, String> {
        let slot = |mode: usize| -> Result<u8, String> {
            let value = self.entries[mode][0];
            if value == 254 {
                Err("required Delphi locomotion definition is inactive".into())
            } else {
                Ok(value)
            }
        };
        Ok(Clips {
            idle: [slot(0)?, slot(1)?, slot(2)?],
            side: [slot(3)?, slot(4)?],
            forward: [slot(5)?, slot(35)?],
            turn: slot(8)?,
            jump: [slot(21)?, slot(25)?],
            fall: [slot(28)?, slot(29)?],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(layout: Layout) -> Vec<u8> {
        let mut bytes = crate::executable_metadata::synthetic_image(layout);
        let (table, rva) = match layout {
            Layout::DebugReference => (0x167bd8, 1_413_120),
            Layout::Retail => (0x160c50, 1_392_640),
            Layout::Original => unreachable!(),
        };
        let at = 4096 + table - rva;
        for i in 0..188 {
            bytes[at + i * 4..at + i * 4 + 4]
                .copy_from_slice(&((i % 200 + 7) as u32).to_le_bytes());
        }
        bytes
    }
    #[test]
    fn relocated_external_data_keeps_both_columns_and_selected_definitions() {
        for layout in [Layout::DebugReference, Layout::Retail] {
            let bytes = fixture(layout);
            let definitions = Definitions::decode(&bytes).unwrap();
            assert_eq!(definitions.entries()[0], [7, 8]);
            assert_eq!(definitions.entries()[35], [77, 78]);
            let clips = definitions.sword_locomotion().unwrap();
            assert_eq!(clips.forward, [17, 77]);
            assert_eq!(clips.jump, [49, 57]);
        }
    }
    #[test]
    fn inactive_required_rows_and_out_of_range_identifiers_do_not_get_fallbacks() {
        let mut bytes = fixture(Layout::DebugReference);
        let at = 4096 + 0x167bd8 - 1_413_120;
        bytes[at..at + 4].copy_from_slice(&254u32.to_le_bytes());
        assert!(
            Definitions::decode(&bytes)
                .unwrap()
                .sword_locomotion()
                .is_err()
        );
        bytes[at..at + 4].copy_from_slice(&255u32.to_le_bytes());
        assert!(Definitions::decode(&bytes).is_err());
        assert!(
            Definitions::decode(&crate::executable_metadata::synthetic_image(
                Layout::Original
            ))
            .is_err()
        );
    }
}
