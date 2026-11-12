//! Positive-hit source gate in the acquired PC Mecc callback. Prototype PS2
//! search key `mecc_shot` remains an unknown routine correspondence. This does
//! not supply global suppression, equipment, hit reactions or a damage backend.

#[derive(Clone, Copy, Debug)]
pub struct Actor<I> {
    pub id: I,
    pub class: i32,
    pub state: u32,
    pub context: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Source<I> {
    pub actor: Actor<I>,
    /// Exactly the current callback-owner link, not a recursive parent walk.
    pub owner: Option<Actor<I>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gate<I> {
    /// Permits health arithmetic only. A denied health change must still run
    /// the remaining native callback: flags, impulse and feedback are separate.
    pub damage_allowed: bool,
    /// Candidate for the separate empty associated-player retaliation slot.
    /// This result does not overwrite an existing relation or publish a hit.
    pub retaliation: Option<I>,
}

/// Single-player source selection for a nonnegative amount, after the actual
/// callback entry guards. Multiplayer overrides remain outside this contract.
/// State2/44 same-context sources are suppressed, with two exact self-source
/// class exceptions; even those exceptions clear the retaliation candidate.
pub fn singleplayer<I: Copy + Eq>(recipient: Actor<I>, source: Option<Source<I>>) -> Gate<I> {
    let Some(source) = source else {
        return Gate {
            damage_allowed: true,
            retaliation: None,
        };
    };
    let owner = source.owner.unwrap_or(source.actor);
    if owner.context == recipient.context && matches!(owner.state, 2 | 44) {
        Gate {
            damage_allowed: owner.id == recipient.id && matches!(source.actor.class, 822 | 808),
            retaliation: None,
        }
    } else {
        Gate {
            damage_allowed: true,
            retaliation: Some(owner.id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn actor(id: u32, state: u32, context: u32) -> Actor<u32> {
        Actor {
            id,
            class: 73,
            state,
            context,
        }
    }
    #[test]
    fn current_owner_controls_team_filter_and_only_exact_self_classes_bypass_it() {
        let recipient = actor(1, 2, 0);
        let mut source = Source {
            actor: actor(2, 23, 1),
            owner: Some(actor(3, 44, 0)),
        };
        assert_eq!(
            singleplayer(recipient, Some(source)),
            Gate {
                damage_allowed: false,
                retaliation: None
            }
        );
        source.owner = Some(actor(3, 23, 0));
        assert_eq!(
            singleplayer(recipient, Some(source)),
            Gate {
                damage_allowed: true,
                retaliation: Some(3)
            }
        );
        source.owner = Some(recipient);
        for class in [822, 808] {
            source.actor.class = class;
            assert_eq!(
                singleplayer(recipient, Some(source)),
                Gate {
                    damage_allowed: true,
                    retaliation: None
                }
            );
        }
        source.actor.class = 232;
        assert!(!singleplayer(recipient, Some(source)).damage_allowed);
        source.owner = Some(actor(1, 2, 1));
        assert_eq!(
            singleplayer(recipient, Some(source)),
            Gate {
                damage_allowed: true,
                retaliation: Some(1)
            }
        );
    }
}
