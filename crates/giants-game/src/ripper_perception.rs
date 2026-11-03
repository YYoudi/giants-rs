//! Ordered D state23 target lookup. Source fields, contact-list order and
//! previous target identity are inputs; this is not a complete behavior tick.
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub struct Candidate<K> {
    pub id: K,
    pub class: i32,
    pub state: u32,
    pub active: u32,
    pub personality: u32,
    pub control: u32,
    pub position: [f32; 3],
    pub damage: f32,
    pub limit: f32,
    /// The native lookup tests pointer presence only, without dereferencing it.
    pub parent_present: bool,
}
pub struct Query<K> {
    pub position: [f32; 3],
    pub retained_timer: f32,
    pub range: f32,
    pub previous: Option<K>,
}
pub struct Frame<'a, K> {
    actors: BTreeMap<K, &'a Candidate<K>>,
    order: Vec<K>,
}
impl<'a, K: Copy + Ord> Frame<'a, K> {
    pub(crate) fn candidates(&self) -> impl Iterator<Item = &Candidate<K>> {
        self.actors.values().copied()
    }
    pub fn candidate(&self, id: K) -> Result<&Candidate<K>, &'static str> {
        self.actors
            .get(&id)
            .copied()
            .ok_or("perception identity absent from current frame")
    }
    /// Apply the lookup to the owned combat objective, which the hurt callback
    /// can also set from the attacker. The auxiliary `target` is a distinct
    /// relation and must not be overwritten by this lookup.
    pub fn refresh_state23(
        &self,
        actor: &mut crate::state23_damage::State<K>,
        retained_timer: f32,
        range: f32,
    ) -> Result<Option<K>, &'static str> {
        if !matches!(actor.class, 73 | 230) {
            return Err("perception combat update requires the acquired state23 class profile");
        }
        let selected = self.select(Query {
            position: actor.position,
            retained_timer,
            range,
            previous: actor.retaliation,
        })?;
        actor.retaliation = selected;
        Ok(selected)
    }
    pub fn new(actors: &'a [Candidate<K>], order: &[K]) -> Result<Self, &'static str> {
        let mut index = BTreeMap::new();
        for actor in actors {
            if index.insert(actor.id, actor).is_some() {
                return Err("duplicate perception identity");
            }
        }
        if order.iter().any(|id| !index.contains_key(id)) {
            return Err("perception order contains an absent identity");
        }
        Ok(Self {
            actors: index,
            order: order.to_vec(),
        })
    }
    pub fn select(&self, query: Query<K>) -> Result<Option<K>, &'static str> {
        if !query
            .position
            .into_iter()
            .chain([query.retained_timer, query.range])
            .all(f32::is_finite)
        {
            return Err("perception query outside finite profile");
        }
        let retained = match query.previous {
            Some(id) => {
                let actor = self
                    .actors
                    .get(&id)
                    .ok_or("previous perception identity absent")?;
                (actor.active & 0x8000 == 0).then_some(id)
            }
            None => None,
        };
        let mut best = (f64::from(query.range) * f64::from(query.range)) as f32;
        let mut selected = None;
        for id in &self.order {
            let actor = self.actors[id];
            if actor.active & 0x4000_c000 != 0 {
                continue;
            }
            if !actor.damage.is_finite() || !actor.limit.is_finite() {
                return Err("perception health outside finite profile");
            }
            if actor.damage >= actor.limit
                || (actor.personality & 1 == 0 && actor.state != 85)
                || actor.class == 0
                || actor.parent_present
                || (actor.state == 2
                    && actor.control & 0x0200_0000 != 0
                    && query.retained_timer < 4.)
            {
                continue;
            }
            if !actor.position.into_iter().all(f32::is_finite) {
                return Err("perception candidate position outside finite profile");
            }
            let d: [f64; 3] = std::array::from_fn(|i| {
                f64::from(query.position[i]) - f64::from(actor.position[i])
            });
            let mut distance = (d[0] * d[0] + d[1] * d[1]) + d[2] * d[2];
            if Some(*id) == retained {
                distance *= 0.5;
            }
            // The native threshold is stored as f32 after each accepted
            // candidate. Equality selects the later list element.
            if distance <= f64::from(best) {
                best = distance as f32;
                selected = Some(*id);
            }
        }
        Ok(selected.or(retained))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn actor(id: u32, x: f32) -> Candidate<u32> {
        Candidate {
            id,
            class: 2,
            state: 2,
            active: 0,
            personality: 1,
            control: 0,
            position: [x, 0., 0.],
            damage: 0.,
            limit: 100.,
            parent_present: false,
        }
    }
    fn query(previous: Option<u32>) -> Query<u32> {
        Query {
            position: [0.; 3],
            retained_timer: 0.,
            range: 10.,
            previous,
        }
    }
    #[test]
    fn boundary_ties_follow_list_order_and_previous_gets_squared_distance_bias() {
        let actors = [actor(1, 10.), actor(2, -10.)];
        assert_eq!(
            Frame::new(&actors, &[1, 2])
                .unwrap()
                .select(query(None))
                .unwrap(),
            Some(2)
        );
        assert_eq!(
            Frame::new(&actors, &[2, 1])
                .unwrap()
                .select(query(None))
                .unwrap(),
            Some(1)
        );
        assert_eq!(
            Frame::new(&actors, &[1, 2])
                .unwrap()
                .select(query(Some(1)))
                .unwrap(),
            Some(1)
        );
    }
    #[test]
    fn failed_search_retains_prior_even_when_it_would_fail_candidate_filters() {
        let mut actors = [actor(1, 100.)];
        actors[0].damage = 100.;
        assert_eq!(
            Frame::new(&actors, &[1])
                .unwrap()
                .select(query(Some(1)))
                .unwrap(),
            Some(1)
        );
        actors[0].active = 0x8000;
        assert_eq!(
            Frame::new(&actors, &[])
                .unwrap()
                .select(query(Some(1)))
                .unwrap(),
            None
        );
    }
    #[test]
    fn cloaked_mecc_requires_four_and_parented_candidates_are_skipped() {
        let mut actors = [actor(1, 1.)];
        actors[0].control = 0x0200_0000;
        let f = Frame::new(&actors, &[1]).unwrap();
        assert_eq!(f.select(query(None)).unwrap(), None);
        assert_eq!(
            f.select(Query {
                retained_timer: 4.,
                ..query(None)
            })
            .unwrap(),
            Some(1)
        );
        actors[0].parent_present = true;
        assert_eq!(
            Frame::new(&actors, &[1])
                .unwrap()
                .select(Query {
                    retained_timer: 4.,
                    ..query(None)
                })
                .unwrap(),
            None
        );
    }
    #[test]
    fn unknown_identities_do_not_silently_become_an_empty_target() {
        let actors = [actor(1, 1.)];
        assert!(Frame::new(&actors, &[2]).is_err());
        assert!(
            Frame::new(&actors, &[1])
                .unwrap()
                .select(query(Some(2)))
                .is_err()
        );
    }
    #[test]
    fn combat_objective_update_preserves_auxiliary_target_and_failed_lookup_is_atomic() {
        let candidates = [actor(1, 1.)];
        let frame = Frame::new(&candidates, &[1]).unwrap();
        let mut combat = crate::state23_damage::State {
            id: 9,
            class: 73,
            limit: 40.,
            damage: 0.,
            mode: 31,
            action: 0,
            flags: 0,
            personality: 0,
            control: 0,
            timer: 0.,
            position: [0.; 3],
            parent: None,
            owner: None,
            retaliation: None,
            target: Some(crate::state23_damage::Target {
                id: 7,
                state: 47,
                flags: 0,
            }),
            group: None,
            group_index: 0,
            effect: None,
        };
        let secondary = combat.target;
        assert_eq!(
            frame.refresh_state23(&mut combat, 0., 10.).unwrap(),
            Some(1)
        );
        assert_eq!(combat.retaliation, Some(1));
        assert_eq!(combat.target, secondary);
        combat.retaliation = Some(99);
        let before = combat.clone();
        assert!(frame.refresh_state23(&mut combat, 0., 10.).is_err());
        assert_eq!(combat, before);
    }
}
