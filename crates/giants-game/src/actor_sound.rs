//! Original owner/attachment routing, before the actual sound backend.
//! Descriptor readiness and suppression are supplied. Bounded/cycle failures
//! are independent-runtime guards, not successful native requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Owner,
    Attachment,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route<K> {
    Local(K),
    Attached(Option<K>),
}
pub fn resolve<K: Copy + Eq>(
    kind: Kind,
    owner: Option<K>,
    local_player: Option<K>,
    suppressed: bool,
    descriptor_ready: bool,
    mut parent: impl FnMut(K) -> Result<Option<K>, &'static str>,
) -> Result<Option<Route<K>>, &'static str> {
    if suppressed || !descriptor_ready {
        return Ok(None);
    }
    if kind == Kind::Owner {
        return Ok(Some(Route::Attached(owner)));
    }
    let Some(mut root) = owner else {
        return Ok(None);
    };
    let mut seen = Vec::new();
    loop {
        if seen.contains(&root) || seen.len() >= 256 {
            return Err("actor sound attachment cycle or traversal budget");
        }
        seen.push(root);
        match parent(root)? {
            Some(next) => root = next,
            None => break,
        }
    }
    Ok(Some(if Some(root) == local_player {
        Route::Local(root)
    } else {
        Route::Attached(Some(root))
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_direct_owner_and_refuses_stale_or_cyclic_attachments() {
        let parent = |id| match id {
            1 => Ok(Some(2)),
            2 => Ok(None),
            _ => Err("stale"),
        };
        assert_eq!(
            resolve(Kind::Attachment, Some(1), Some(2), false, true, parent),
            Ok(Some(Route::Local(2)))
        );
        assert_eq!(
            resolve(Kind::Owner, Some(1), Some(2), false, true, parent),
            Ok(Some(Route::Attached(Some(1))))
        );
        assert_eq!(
            resolve(Kind::Owner, None, Some(2), false, true, parent),
            Ok(Some(Route::Attached(None)))
        );
        assert!(resolve(Kind::Attachment, Some(3), Some(2), false, true, parent).is_err());
        assert!(
            resolve(Kind::Attachment, Some(1), Some(2), false, true, |_| Ok(
                Some(1)
            ))
            .is_err()
        );
        assert_eq!(
            resolve(Kind::Attachment, Some(3), Some(2), true, true, parent),
            Ok(None)
        );
    }
}
