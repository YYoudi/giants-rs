//! Selection for the carried-actor branch of the ordinary backpack command.
//! Earlier vehicle/base gates, equipment dropping and the detach transition are
//! separate. Iterate the caller's current ordered list, never class templates.
#[derive(Clone, Copy, Debug)]
pub struct Candidate {
    pub identity: u64,
    pub class: i32,
    pub object_flags: u32,
    pub binding: Option<u64>,
    pub parent: Option<u64>,
    pub action: u32,
    pub payload_word_54: u32,
}

/// The owner has already passed the upstream base/vehicle input gates. A
/// returned identity requests a real detach service; it does not execute it.
pub fn select(owner: u64, response_flags: u32, ordered: &[Candidate]) -> Option<u64> {
    if response_flags & 0x80_0000 == 0 {
        return None;
    }
    ordered
        .iter()
        .find(|actor| {
            actor.object_flags & 0x8000 == 0
                && matches!(actor.class, 11 | 584 | 742)
                && (actor.binding == Some(owner) || actor.parent == Some(owner))
                && actor.action != 5
                && actor.object_flags & 0x400 == 0
                && actor.payload_word_54 == 0
        })
        .map(|actor| actor.identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uses_live_ownership_and_source_order_without_unlocking_scripted_passenger() {
        let candidate = Candidate {
            identity: 1,
            class: 11,
            object_flags: 0,
            binding: None,
            parent: Some(99),
            action: 3,
            payload_word_54: 0,
        };
        let scripted = Candidate {
            class: 583,
            ..candidate
        };
        let protected = Candidate {
            object_flags: 0x400,
            ..candidate
        };
        let other_owner = Candidate {
            parent: Some(100),
            ..candidate
        };
        let second = Candidate {
            identity: 2,
            binding: Some(99),
            parent: None,
            ..candidate
        };
        assert_eq!(select(99, 0, &[candidate]), None);
        assert_eq!(
            select(99, 0x80_0000, &[scripted, protected, other_owner]),
            None
        );
        assert_eq!(
            select(99, 0x80_0000, &[scripted, second, candidate]),
            Some(2)
        );
        assert_eq!(select(99, 0x80_0000, &[candidate, second]), Some(1));
    }
}
