use super::{Attachment, Id, Resident, Session};

/// Acquired attachment words whose lifetime exceeds the parent relationship.
/// Behavior is independent of both reference indices and the actor's flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachmentWords {
    pub child_reference: u32,
    pub parent_reference: u32,
    pub behavior: u32,
}

impl From<Attachment> for AttachmentWords {
    fn from(binding: Attachment) -> Self {
        Self {
            child_reference: binding.child_reference,
            parent_reference: binding.parent_reference,
            behavior: binding.behavior,
        }
    }
}

impl Resident {
    /// Current supplied words, including those retained after detachment.
    /// An actor without an acquired producer returns None; this does not
    /// manufacture constructor defaults or acknowledge initialization.
    pub fn attachment_words(&self) -> Option<AttachmentWords> {
        self.attachment
            .map(AttachmentWords::from)
            .or(self.retained_attachment_words)
    }
}

impl Session {
    /// Clear the typed parent while preserving its supplied binding words.
    /// A matching presentation mirror is cleared with it; an independent
    /// presentation relationship is preserved. No state callback, wield cleanup,
    /// physical impulse, network notice or primary phase is executed here.
    pub fn clear_actor_parent(&mut self, id: Id) -> Result<(), &'static str> {
        self.get(id)
            .ok_or("parent binding actor absent or foreign")?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        if let Some(binding) = actor.attachment.take() {
            actor.retained_attachment_words = Some(binding.into());
            if actor.presentation_parent == Some(binding.parent) {
                actor.presentation_parent = None;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        arithmetic::Precision,
        object_lists::DirectFlags,
        state_storage::{Profile, Seed},
    };

    fn actor(session: &mut Session) -> Id {
        let program = giants_formats::class_program::parse(&[0x1e, 18, 0xff]).unwrap();
        let seed = Seed::from_program_for(&program, Profile::Debug, Precision::Double53).unwrap();
        session
            .register(583, &seed, &[], DirectFlags::default())
            .unwrap()
    }

    #[test]
    fn detached_words_survive_repeated_clear_rebinding_and_owner_cleanup() {
        let mut session = Session::new(991);
        let parent = actor(&mut session);
        let child = actor(&mut session);
        assert_eq!(session.get(child).unwrap().attachment_words(), None);
        let before = session
            .get(child)
            .unwrap()
            .seed
            .storage
            .data()
            .unwrap()
            .to_vec();
        let binding = Attachment {
            parent,
            child_reference: u32::MAX,
            parent_reference: 0x8000_0001,
            behavior: 33,
        };
        {
            let child = session.residents.get_mut(&child.serial).unwrap();
            child.attachment = Some(binding);
            child.presentation_parent = Some(parent);
        }
        session.clear_actor_parent(child).unwrap();
        let detached = session.get(child).unwrap();
        assert!(detached.attachment.is_none());
        assert!(detached.presentation_parent.is_none());
        assert_eq!(detached.attachment_words(), Some(binding.into()));
        assert_eq!(detached.seed.storage.data().unwrap(), before);
        session.clear_actor_parent(child).unwrap();
        assert_eq!(
            session.get(child).unwrap().attachment_words(),
            Some(binding.into())
        );
        let replacement = Attachment {
            parent,
            child_reference: 17,
            parent_reference: 2,
            behavior: 19,
        };
        session.residents.get_mut(&child.serial).unwrap().attachment = Some(replacement);
        assert_eq!(
            session.get(child).unwrap().attachment_words(),
            Some(replacement.into())
        );
        session.clear_actor_parent(child).unwrap();
        assert_eq!(
            session.get(child).unwrap().attachment_words(),
            Some(replacement.into())
        );
        let mut foreign = Session::new(992);
        let foreign_id = actor(&mut foreign);
        assert!(session.clear_actor_parent(foreign_id).is_err());
        assert_eq!(
            session.get(child).unwrap().attachment_words(),
            Some(replacement.into())
        );
        session.clear();
        assert!(session.clear_actor_parent(child).is_err());
        let next = actor(&mut session);
        assert_eq!(session.get(next).unwrap().attachment_words(), None);
    }

    #[test]
    fn parent_clear_preserves_independent_presentation_parent() {
        let mut session = Session::new(993);
        let parent = actor(&mut session);
        let presentation = actor(&mut session);
        let child = actor(&mut session);
        let resident = session.residents.get_mut(&child.serial).unwrap();
        resident.attachment = Some(Attachment {
            parent,
            child_reference: 0,
            parent_reference: 1,
            behavior: 0,
        });
        resident.presentation_parent = Some(presentation);
        session.clear_actor_parent(child).unwrap();
        assert_eq!(
            session.get(child).unwrap().presentation_parent,
            Some(presentation)
        );
    }
}
