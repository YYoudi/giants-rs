//! Ordered catalog staging. Pending file data and a resident sample are distinct.
//! Resource reads and resident sample objects belong to the caller; this table
//! never creates a device, resolves archive precedence or acknowledges playback.
use giants_formats::sound_catalog::Definition;
use std::collections::BTreeMap;

pub struct Payload<P> {
    pub filename: String,
    pub data: P,
}

pub struct Slot<P, S> {
    pub descriptor: [u32; 6],
    pub payload: Option<Payload<P>>,
    /// Caller-owned sample object; its presence alone does not prove playback.
    pub resident: Option<S>,
}

pub struct Bank<P, S> {
    slots: BTreeMap<u32, Slot<P, S>>,
}

impl<P, S> Default for Bank<P, S> {
    fn default() -> Self {
        Self {
            slots: BTreeMap::new(),
        }
    }
}

impl<P, S> Bank<P, S> {
    /// Consume staged source state without inventing upload/release outcomes.
    pub fn into_slots(self) -> impl Iterator<Item = (u32, Slot<P, S>)> {
        self.slots.into_iter()
    }
    pub fn slot(&self, id: u32) -> Option<&Slot<P, S>> {
        self.slots.get(&id)
    }

    /// Used by the owner to record real upload/release results. Catalog staging
    /// does not synthesize a sample object or release source data on its own.
    pub fn slot_mut(&mut self, id: u32) -> Option<&mut Slot<P, S>> {
        self.slots.get_mut(&id)
    }

    pub fn stage(
        &mut self,
        definitions: &[Definition],
        alternate: bool,
        mut load: impl FnMut(&str) -> Result<P, &'static str>,
    ) -> Result<(), &'static str> {
        for definition in definitions {
            if definition.id >= 2000 {
                return Err("sound binding outside checked table capacity");
            }
            let slot = self.slots.entry(definition.id).or_insert_with(|| Slot {
                descriptor: [0; 6],
                payload: None,
                resident: None,
            });
            slot.descriptor = definition.descriptor_words(alternate);
            if slot.payload.is_none() {
                slot.payload = Some(Payload {
                    filename: definition.filename.clone(),
                    data: load(&definition.filename)?,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Bank;
    use giants_formats::sound_catalog::Definition;

    fn definition(name: &str, word: u32) -> Definition {
        Definition {
            id: 973,
            words: [973, word, 2, 3, 4, 0, 0, 7, 8],
            filename: name.into(),
        }
    }

    #[test]
    fn source_data_and_resident_sample_have_independent_lifetimes() {
        let mut bank = Bank::<String, &str>::default();
        let first = definition("first.wav", 32);
        bank.stage(&[first], false, |name| Ok(name.into())).unwrap();
        bank.slot_mut(973).unwrap().resident = Some("resident-first");
        bank.stage(&[definition("second.wav", 33)], true, |_| {
            panic!("Resident source payload must prevent a new read")
        })
        .unwrap();
        let slot = bank.slot_mut(973).unwrap();
        assert_eq!(slot.payload.as_ref().unwrap().filename, "first.wav");
        assert_eq!(slot.descriptor, [33, 2, 3, 4, 7, 8]);
        // This explicit release represents an owner/provider event, not staging.
        slot.payload.take();
        bank.stage(
            &[definition("third.wav", 34)],
            false,
            |name| Ok(name.into()),
        )
        .unwrap();
        let slot = bank.slot(973).unwrap();
        assert_eq!(slot.payload.as_ref().unwrap().filename, "third.wav");
        assert_eq!(slot.payload.as_ref().unwrap().data, "third.wav");
        assert_eq!(slot.resident, Some("resident-first"));
        assert_eq!(slot.descriptor, [34, 2, 3, 4, 0, 0]);
    }

    #[test]
    fn failed_read_keeps_sample_and_does_not_invent_pending_data() {
        let mut bank = Bank::<(), u32>::default();
        bank.stage(&[definition("first.wav", 1)], false, |_| Ok(()))
            .unwrap();
        let slot = bank.slot_mut(973).unwrap();
        slot.payload.take();
        slot.resident = Some(123);
        assert!(
            bank.stage(&[definition("missing.wav", 9)], false, |_| Err("missing"))
                .is_err()
        );
        let slot = bank.slot(973).unwrap();
        assert!(slot.payload.is_none());
        assert_eq!(slot.resident, Some(123));
        assert_eq!(slot.descriptor[0], 9);
    }
}
