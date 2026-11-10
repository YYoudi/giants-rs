//! Acquired profile-family/view selection. Coefficients, input production and
//! projection remain required services; no original profile tables are embedded.

mod executable;
pub use executable::{ExecutableCatalog, Record};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Model2,
    Model2Variant,
    Model1,
    Model3,
    Model1076,
    Model7,
    Model6,
    Model4,
    Default,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    family: Family,
    view: u8,
}
impl Selection {
    pub fn family(self) -> Family {
        self.family
    }
    pub fn view(self) -> u8 {
        self.view
    }
    /// Query external reviewed data on each invocation. Selecting a family does
    /// not supply coefficients or certify their binary/version identity.
    pub fn resolve(self, catalog: &impl Catalog) -> Result<crate::camera_mode::Profile, String> {
        catalog.profile(self)
    }
}
pub trait Catalog {
    fn profile(&self, selection: Selection) -> Result<crate::camera_mode::Profile, String>;
}

pub struct Input<K> {
    pub owner: Option<K>,
    pub controlled: Option<K>,
    pub view: u32,
    pub scaled_input: bool,
}
pub trait World<K> {
    fn object_flags(&self, actor: K) -> Result<u32, String>;
    fn class(&self, actor: K) -> Result<i32, String>;
    fn control_flags(&self, actor: K) -> Result<u32, String>;
    fn parent(&self, actor: K) -> Result<Option<K>, String>;
    fn state_kind(&self, actor: K) -> Result<i32, String>;
}

/// Only queried branches require fields. In particular the controlled predicate
/// uses state kind, and a parent's substituted class is not recursively tested
/// for the owner's variant. Unknown classes follow the acquired default family.
pub fn select<K: Copy>(input: Input<K>, world: &impl World<K>) -> Result<Selection, String> {
    let mut class = 0;
    if let Some(owner) = input.owner
        && world.object_flags(owner)? & 0x8000 == 0
    {
        class = world.class(owner)?;
        if class == 2 {
            if world.control_flags(owner)? & 0x20000 != 0 {
                class = 991;
            }
        } else if class == 3
            && let Some(parent) = world.parent(owner)?
            && world.state_kind(parent)? == 90
        {
            class = world.class(parent)?;
        }
    }
    let view = if input.scaled_input
        && let Some(controlled) = input.controlled
        && matches!(world.state_kind(controlled)?, 1..=3)
    {
        1
    } else {
        input.view
    };
    if view > 2 {
        return Err("camera profile view outside acquired table".into());
    }
    Ok(Selection {
        family: match class {
            2 => Family::Model2,
            991 => Family::Model2Variant,
            1 => Family::Model1,
            3 => Family::Model3,
            1076 => Family::Model1076,
            7 => Family::Model7,
            6 => Family::Model6,
            4 => Family::Model4,
            _ => Family::Default,
        },
        view: view as u8,
    })
}

/// Borrow current Session headers and the checked owner/mode states for each
/// query. No persistent profile snapshot or default coefficients are introduced.
pub fn from_session(
    session: &crate::resident_states::Session,
    owners: &crate::camera_owner::State<crate::resident_states::Id>,
    modes: &crate::camera_mode::State,
    controlled: Option<crate::resident_states::Id>,
    scaled_input: bool,
) -> Result<Selection, String> {
    if modes.failure().is_some() {
        return Err("camera profile mode state is quarantined".into());
    }
    select(
        Input {
            owner: owners.owner_from_session(session)?,
            controlled,
            view: modes.view(),
            scaled_input,
        },
        session,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fields;
    impl World<u32> for Fields {
        fn object_flags(&self, actor: u32) -> Result<u32, String> {
            Ok(if actor == 0 { 0x8000 } else { 0 })
        }
        fn class(&self, actor: u32) -> Result<i32, String> {
            match actor {
                1 => Ok(3),
                2 => Ok(2),
                _ => Err("unexpected class query".into()),
            }
        }
        fn control_flags(&self, _: u32) -> Result<u32, String> {
            Err("parent variant must not be queried".into())
        }
        fn parent(&self, actor: u32) -> Result<Option<u32>, String> {
            assert_eq!(actor, 1);
            Ok(Some(2))
        }
        fn state_kind(&self, actor: u32) -> Result<i32, String> {
            match actor {
                2 => Ok(90),
                3 => Ok(2),
                _ => Err("controlled actor unavailable".into()),
            }
        }
    }
    #[test]
    fn ignored_owner_and_forced_view_do_not_consume_unneeded_fields() {
        let input = || Input {
            owner: Some(0),
            controlled: Some(3),
            view: u32::MAX,
            scaled_input: true,
        };
        let selected = select(input(), &Fields).unwrap();
        assert_eq!((selected.family(), selected.view()), (Family::Default, 1));
        assert!(
            select(
                Input {
                    scaled_input: false,
                    ..input()
                },
                &Fields
            )
            .is_err()
        );
        assert!(
            select(
                Input {
                    controlled: Some(99),
                    ..input()
                },
                &Fields
            )
            .is_err()
        );
        let selected = select(
            Input {
                controlled: Some(99),
                scaled_input: false,
                view: 0,
                ..input()
            },
            &Fields,
        )
        .unwrap();
        assert_eq!(selected.view(), 0);
    }
    #[test]
    fn parent_class_substitution_does_not_reenter_variant_or_invent_coefficients() {
        let selected = select(
            Input {
                owner: Some(1),
                controlled: None,
                view: 2,
                scaled_input: false,
            },
            &Fields,
        )
        .unwrap();
        assert_eq!((selected.family(), selected.view()), (Family::Model2, 2));
        struct Missing;
        impl Catalog for Missing {
            fn profile(&self, _: Selection) -> Result<crate::camera_mode::Profile, String> {
                Err("external coefficient catalog unavailable".into())
            }
        }
        assert!(selected.resolve(&Missing).is_err());
    }
}
