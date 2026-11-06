//! Debug Mecc body/subjective submission policy with caller-owned world inputs.
//! This orders draw callbacks; it does not prepare the subjective actor, choose
//! LODs/materials, draw geometry or certify a completed Session post.
//! The held-parent helper is a separate world-pose phase, not drawing.

#[derive(Clone, Copy, Debug)]
pub struct Parent {
    pub class: u32,
    pub control_flags: u32,
}

/// The ordinary actor must be a Mecc (class 2). Other character profiles remain
/// separate: their payload and display conditions are not imported here.
#[derive(Clone, Copy, Debug)]
pub struct Input {
    pub class: u32,
    pub subjective_active: bool,
    /// Identity equality with the controlled actor, not camera-mode selection.
    pub controlled: bool,
    pub control_flags: u32,
    pub parent: Option<Parent>,
    pub ordinary_prelude_flags: u8,
    pub display_flags: u8,
    pub height: f32,
    pub ordinary_prelude_inhibited: bool,
    pub auxiliary_active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actor {
    Body,
    Subjective,
    Auxiliary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// The separately required ordinary prelude callback remains external.
    OrdinaryPrelude,
    Prepare(Actor),
    Submit(Actor),
}

/// Preserve callback order and completed side effects on failure. The caller
/// must quarantine a failed pass rather than replay this stateless dispatcher.
pub fn submit(
    input: Input,
    mut operation: impl FnMut(Operation) -> Result<(), String>,
) -> Result<(), String> {
    if input.class != 2 {
        return Err("Mecc draw policy requires the class-2 profile".into());
    }
    if !input.subjective_active {
        if input.ordinary_prelude_flags & 0x40 == 0
            && input.display_flags & 0x10 == 0
            && input.height > -61.1_f32
            && !input.ordinary_prelude_inhibited
        {
            operation(Operation::OrdinaryPrelude)?;
        }
        return operation(Operation::Submit(Actor::Body));
    }
    if !input.controlled {
        if input
            .parent
            .is_some_and(|p| p.class == 2 && p.control_flags & 0x20000 != 0)
        {
            return Ok(());
        }
        operation(Operation::Prepare(Actor::Body))?;
        return operation(Operation::Submit(Actor::Body));
    }
    if input.control_flags & 0x2020000 != 0 {
        return Ok(());
    }
    operation(Operation::Prepare(Actor::Subjective))?;
    operation(Operation::Submit(Actor::Subjective))?;
    if input.auxiliary_active {
        operation(Operation::Prepare(Actor::Auxiliary))?;
        operation(Operation::Submit(Actor::Auxiliary))?;
    }
    Ok(())
}

/// The actual held-object pose provider owns its parent slot and may detach it.
pub trait HeldServices {
    type Id: Copy + PartialEq;
    fn parent(&self) -> Option<Self::Id>;
    fn camera_owner(&mut self) -> Result<Option<Self::Id>, String>;
    fn set_parent(&mut self, parent: Self::Id);
    fn update_pose(&mut self) -> Result<(), String>;
}

/// Redirect only the active camera owner's held object, unless inhibited.
/// A detached parent stays absent; any remaining parent is restored to the
/// original identity. A failed provider retains its completed mutations.
pub fn update_held_pose<K: Copy + PartialEq>(
    subjective_active: bool,
    inhibited: bool,
    subjective: K,
    services: &mut impl HeldServices<Id = K>,
) -> Result<(), String> {
    let Some(parent) = services.parent() else {
        return Ok(());
    };
    // The native getter executes even when display is inactive or inhibited.
    let owner = services.camera_owner()?;
    if !subjective_active || inhibited || owner != Some(parent) {
        return services.update_pose();
    }
    services.set_parent(subjective);
    services.update_pose()?;
    if services.parent().is_some() {
        services.set_parent(parent);
    }
    Ok(())
}

/// Historical alias retained for earlier evidence. This updates a held pose;
/// it does not submit geometry or draw the held object.
#[deprecated(note = "This is a world-pose wrapper; use update_held_pose")]
pub fn submit_held<K: Copy + PartialEq>(
    subjective_active: bool,
    inhibited: bool,
    subjective: K,
    services: &mut impl HeldServices<Id = K>,
) -> Result<(), String> {
    update_held_pose(subjective_active, inhibited, subjective, services)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrelated_character_refusal_and_late_failure_preserve_callback_boundary() {
        let input = Input {
            class: 2,
            subjective_active: true,
            controlled: true,
            control_flags: 0,
            parent: None,
            ordinary_prelude_flags: 0,
            display_flags: 0,
            height: 0.,
            ordinary_prelude_inhibited: false,
            auxiliary_active: true,
        };
        let mut completed = Vec::new();
        assert!(
            submit(Input { class: 3, ..input }, |op| {
                completed.push(op);
                Ok(())
            })
            .is_err()
        );
        assert!(completed.is_empty());
        assert!(
            submit(input, |op| {
                completed.push(op);
                if op == Operation::Submit(Actor::Subjective) {
                    Err("injected submission failure".into())
                } else {
                    Ok(())
                }
            })
            .is_err()
        );
        assert_eq!(
            completed,
            [
                Operation::Prepare(Actor::Subjective),
                Operation::Submit(Actor::Subjective)
            ]
        );
    }

    #[test]
    fn held_provider_failure_retains_its_detachment() {
        struct Held(Option<u32>, usize);
        impl HeldServices for Held {
            type Id = u32;
            fn parent(&self) -> Option<u32> {
                self.0
            }
            fn camera_owner(&mut self) -> Result<Option<u32>, String> {
                self.1 += 1;
                Ok(Some(7))
            }
            fn set_parent(&mut self, parent: u32) {
                self.0 = Some(parent);
            }
            fn update_pose(&mut self) -> Result<(), String> {
                assert_eq!(self.0, Some(9));
                self.0 = None;
                Err("injected held pose failure".into())
            }
        }
        let mut held = Held(Some(7), 0);
        assert!(update_held_pose(true, false, 9, &mut held).is_err());
        assert_eq!(held.0, None);
        assert_eq!(held.1, 1);
    }
}
