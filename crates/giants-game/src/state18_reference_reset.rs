//! Acquired action-gated reference-reset routing. The reset body, current
//! preferred relationship, list ownership and position writer remain services.
#[derive(Clone, Copy, Debug)]
pub struct Preferred<K> {
    pub identity: K,
    pub state: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct Candidate<K> {
    pub identity: K,
    pub context: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan<K> {
    Unchanged,
    Reset(K),
    Fallback,
}
pub trait Services<K> {
    /// Execute the actual reset/constructor/lifetime operation for this pair.
    fn reset(&mut self, source: K, point: K) -> Result<(), String>;
    /// Only current position is requested here, not an invented history/cache reset.
    fn position(&mut self, source: K, position: [f32; 3]) -> Result<(), String>;
}
impl<K: Copy> Plan<K> {
    pub fn apply(self, source: K, services: &mut impl Services<K>) -> Result<(), String> {
        match self {
            Self::Unchanged => Ok(()),
            Self::Reset(point) => services.reset(source, point),
            Self::Fallback => services.position(source, [0., 0., 200.]),
        }
    }
}
/// Metadata is resolved lazily. A preferred state32/82 wins regardless of its
/// context; otherwise the first matching context in the declared list wins.
/// Neither candidate flags nor fallback states introduce an additional filter.
pub fn choose<K: Copy>(
    action: u32,
    context: u32,
    preferred: impl FnOnce() -> Result<Option<Preferred<K>>, String>,
    population: impl IntoIterator<Item = Result<Candidate<K>, String>>,
) -> Result<Plan<K>, String> {
    if action != 10 {
        return Ok(Plan::Unchanged);
    }
    if let Some(p) = preferred()?
        && matches!(p.state, 32 | 82)
    {
        return Ok(Plan::Reset(p.identity));
    }
    for candidate in population {
        let candidate = candidate?;
        if candidate.context == context {
            return Ok(Plan::Reset(candidate.identity));
        }
    }
    Ok(Plan::Fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inactive_action_and_preferred_point_do_not_enter_unused_resolvers() {
        let forbidden = || std::iter::from_fn(|| panic!("unused fallback metadata requested"));
        assert_eq!(
            choose::<u32>(
                0x100000a,
                0,
                || panic!("inactive preferred read"),
                forbidden()
            )
            .unwrap(),
            Plan::Unchanged
        );
        assert_eq!(
            choose(
                10,
                0x80000001,
                || Ok(Some(Preferred {
                    identity: 2,
                    state: 32
                })),
                forbidden(),
            )
            .unwrap(),
            Plan::Reset(2)
        );
    }
}
