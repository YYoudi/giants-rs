//! Actual radial-list registrations and live owned health. Prototype PS2
//! `hurt_object` / `hurt_object_do_multi` remain unknown attribution candidates.
//! Native identity and prior aliases remain in the private lookup record.
use super::*;
use crate::radial_damage::{Actor, Hit, Input, Machine, Services};

/// The caller must bind the actual world producers. No default global gate or
/// elapsed time is inferred from the existence of a projectile or a menu scene.
pub struct Environment {
    pub cross_context: bool,
    pub vimp_alert_range: f32,
    pub elapsed_seconds: f32,
}

/// Required attributed hit consumers, run against the same current Session.
/// Computing a radial amount does not acknowledge a damage callback.
pub trait Hits {
    fn damage(&mut self, session: &mut Session, hit: Hit<Id>) -> Result<(), String>;
}

// The original radial head is the first additional list, not common list0.
const LIST: u32 = 1;

impl Session {
    /// Exact single-player passive case of the direct-hit wrapper. State0 has
    /// no hurt callback; class219/state48 uses the empty shared callback. With
    /// a living, unmarked recipient, neither callback, kill-credit nor terminal
    /// counter tail can change the actor. This result is invariant under the
    /// wrapper's global guards, so no unknown guard value is supplied here.
    ///
    /// True means a verified no-effect hit, never applied damage or an available
    /// Mecc/Vimp/Ripper hit backend. False requires the actual other consumer.
    /// The native network path is outside this explicitly single-player API.
    pub fn enemy_radial_passive_no_effect_singleplayer(
        &self,
        hit: Hit<Id>,
    ) -> Result<bool, String> {
        let shot = self
            .get(hit.source)
            .ok_or("passive radial source absent or foreign")?;
        let metadata = shot
            .enemy_projectile
            .as_ref()
            .ok_or("passive hit requires an actual enemy projectile")?;
        let fields = shot
            .construction_fields
            .as_ref()
            .ok_or("passive radial source fields absent")?;
        let launch = metadata
            .launch
            .as_ref()
            .ok_or("passive radial source launch absent")?;
        let owner = self
            .get(launch.source)
            .ok_or("passive radial Ripper owner absent")?;
        let owner_fields = owner
            .construction_fields
            .as_ref()
            .ok_or("passive radial owner fields absent")?;
        if hit.kind != 2
            || hit.tag != 0
            || !hit.amount.is_finite()
            || !hit.point.into_iter().all(f32::is_finite)
            || !shot.acquired_construction
            || shot.seed.storage.profile() != Profile::Debug
            || !matches!(shot.class, 90 | 232)
            || shot.seed.storage.state() != 6
            || shot.seed.storage.data().is_none_or(|b| b.len() != 36)
            || metadata.resource_class != shot.class
            || metadata.phase != crate::enemy_projectile::Phase::ImpactServices
            || metadata.failure.is_some()
            || shot.attachment.is_some()
            || shot.presentation_parent.is_some()
            || fields.class != shot.class
            || fields.configuration[20] != 6
            || fields.position.map(f32::to_bits) != hit.point.map(f32::to_bits)
            || !owner.acquired_construction
            || owner.seed.storage.profile() != Profile::Debug
            || !matches!(owner.class, 73 | 230)
            || owner.seed.storage.state() != 23
            || owner_fields.class != owner.class
            || owner_fields.configuration[20] != 23
        {
            return Err("passive hit differs from its canonical single-player enemy impact".into());
        }
        let target = self
            .get(hit.target)
            .ok_or("passive radial recipient absent or foreign")?;
        if !target.acquired_construction || target.seed.storage.profile() != Profile::Debug {
            return Err("passive radial recipient construction not acquired".into());
        }
        if !(target.seed.storage.state() == 0
            || target.class == 219 && target.seed.storage.state() == 48)
        {
            return Ok(false);
        }
        let actual = self.radial_actor(hit.target)?;
        let [damage, limit] = actual.health.ok_or("passive header health not acquired")?;
        if ![damage, limit].into_iter().all(f32::is_finite) {
            return Err("passive header health outside finite profile".into());
        }
        Ok(damage < limit && actual.active & 0x8000 == 0)
    }

    /// Read the current header and acquired health owner, never a render pose or
    /// a cached population census. Unsupported health stays absent; traversal
    /// refuses it only if the native eligibility path actually requires it.
    pub fn radial_actor(&self, id: Id) -> Result<Actor<Id>, String> {
        let actor = self.get(id).ok_or("radial actor absent or foreign")?;
        if !actor.acquired_construction || actor.seed.storage.profile() != Profile::Debug {
            return Err("radial actor construction not acquired".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("radial header fields absent")?;
        let state = u32::from(actor.seed.storage.state());
        let health = match (actor.class, state) {
            (2, 2) => self.mecc_health(id).ok().map(|h| [h.damage, h.limit]),
            (73 | 230, 23) => self
                .state23_snapshot(id)
                .ok()
                .map(|h| [h.damage, f32::from_bits(fields.configuration[12])]),
            (8, 21) => actor
                .vimp
                .map(|v| [v.damage, f32::from_bits(fields.configuration[12])]),
            (_, 0) | (219, 48) => actor
                .passive_damage
                .map(|damage| [damage, f32::from_bits(fields.configuration[12])]),
            _ => None,
        };
        Ok(Actor {
            id,
            class: actor.class,
            state,
            context: fields.dispatch_context,
            active: fields.object_flags,
            personality: fields.configuration[14],
            position: fields.position,
            health,
        })
    }

    /// Required-service radial continuation of an actual state6 ground impact.
    /// The enclosing impact owner retains failures/prefixes; the machine cannot
    /// replay a completed or partially failed traversal. This does not supply
    /// missing world globals, hit callbacks, FX, sound or retirement services.
    pub fn dispatch_enemy_projectile_radial(
        &mut self,
        request: crate::enemy_projectile::RadialRequest,
        environment: Environment,
        machine: &mut Machine,
        hits: &mut impl Hits,
    ) -> Result<(), String> {
        let actor = self
            .get(request.source)
            .ok_or("radial source absent or foreign")?;
        let metadata = actor
            .enemy_projectile
            .as_ref()
            .ok_or("radial enemy source absent")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("radial source fields absent")?;
        let body = actor
            .seed
            .storage
            .data()
            .ok_or("radial source payload absent")?;
        let launch = metadata
            .launch
            .as_ref()
            .ok_or("radial source launch absent")?;
        if !actor.acquired_construction
            || actor.seed.storage.profile() != Profile::Debug
            || actor.seed.storage.state() != 6
            || !matches!(actor.class, 90 | 232)
            || body.len() != 36
            || metadata.resource_class != actor.class
            || metadata.phase != crate::enemy_projectile::Phase::ImpactServices
            || metadata.failure.is_some()
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || request.context != launch.dispatch_context
            || request.position.map(f32::to_bits) != fields.position.map(f32::to_bits)
            || request.parameters
                != [12, 20, 16, 28, 32]
                    .map(|at| u32::from_le_bytes(body[at..at + 4].try_into().unwrap()))
            || request.other.is_some()
        {
            return Err("radial request differs from its actual ground-impact owner".into());
        }
        machine.run(
            Input {
                source: request.source,
                other: request.other,
                position: request.position,
                parameters: request.parameters.map(f32::from_bits),
                context: request.context,
                cross_context: environment.cross_context,
                vimp_alert_range: environment.vimp_alert_range,
            },
            &mut World {
                session: self,
                hits,
                elapsed_seconds: environment.elapsed_seconds,
                cursor: None,
            },
        )
    }
}

struct World<'a, H> {
    session: &'a mut Session,
    hits: &'a mut H,
    elapsed_seconds: f32,
    cursor: Option<crate::object_links::LinkId>,
}
impl<H: Hits> World<'_, H> {
    fn current(&self) -> Result<Option<Id>, String> {
        self.cursor
            .map(|node| {
                let serial = self.session.links.node_object(LIST, node)?;
                Ok(Id {
                    ticket: self.session.ticket,
                    serial,
                })
            })
            .transpose()
    }
}
impl<H: Hits> Services<Id> for World<'_, H> {
    fn first(&mut self) -> Result<Option<Id>, String> {
        self.cursor = self.session.links.first_node(LIST);
        self.current()
    }
    fn next(&mut self, id: Id) -> Result<Option<Id>, String> {
        if self.current()? != Some(id) {
            return Err("radial current registration changed or retired".into());
        }
        self.cursor = self.session.links.next_node(LIST, self.cursor.unwrap())?;
        self.current()
    }
    fn actor(&mut self, id: Id) -> Result<Actor<Id>, String> {
        if self.current()? != Some(id) {
            return Err("radial actor differs from current registration".into());
        }
        self.session.radial_actor(id)
    }
    fn alert_vimp(&mut self, target: Id, source: Id) -> Result<(), String> {
        self.session
            .record_vimp_radial_signal(target, source, self.elapsed_seconds)
    }
    fn damage(&mut self, hit: Hit<Id>) -> Result<(), String> {
        self.hits.damage(self.session, hit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct FailedHit(Vec<Hit<Id>>);
    impl Hits for FailedHit {
        fn damage(&mut self, _: &mut Session, hit: Hit<Id>) -> Result<(), String> {
            self.0.push(hit);
            Err("required attributed hit consumer unavailable".into())
        }
    }

    #[test]
    fn radial_cursor_uses_additional_list_and_retains_post_callback_edge() {
        let mut session = Session::new(990);
        // Registration fixtures only: no acquired actor or damage is claimed.
        let id = |serial| Id {
            ticket: 990,
            serial,
        };
        session
            .links
            .insert(0, 100, crate::object_links::Position::Tail);
        for serial in [1, 2] {
            session
                .links
                .insert(LIST, serial, crate::object_links::Position::Tail);
        }
        let mut hits = FailedHit(vec![]);
        let mut world = World {
            session: &mut session,
            hits: &mut hits,
            elapsed_seconds: 0.,
            cursor: None,
        };
        assert_eq!(world.first().unwrap(), Some(id(1)));
        // A callback prepending a new registration must not restart traversal.
        world
            .session
            .links
            .insert(LIST, 3, crate::object_links::Position::Head);
        assert_eq!(world.next(id(1)).unwrap(), Some(id(2)));
        world
            .session
            .links
            .insert(LIST, 4, crate::object_links::Position::Tail);
        assert_eq!(world.next(id(2)).unwrap(), Some(id(4)));
        assert!(world.next(id(1)).is_err());
        world.session.links.remove_membership(LIST, 4);
        assert!(world.next(id(4)).is_err());
        assert!(world.hits.0.is_empty());
    }

    #[test]
    #[ignore = "Requires GIANTS_NPC_ROOT; real owned Vimp/list/signal providers and failing captured hit, not impact or gameplay acceptance"]
    fn source_radial_world_alerts_actual_vimp_before_required_hit_failure() {
        let (mut session, _, _, _, children, _) =
            super::super::vimp_history_tests::source_births(true);
        let source = children[1];
        let first = session.list(LIST)[0];
        let actor = session.radial_actor(first).unwrap();
        assert_eq!((actor.class, actor.state), (8, 21));
        assert_eq!(
            actor.health.unwrap()[0],
            session.vimp(first).unwrap().damage
        );
        let source_position = session
            .get(source)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .position;
        let before_health = session.vimp(first).unwrap().damage;
        let mut machine = Machine::default();
        let mut hits = FailedHit(vec![]);
        let input = Input {
            source,
            other: None,
            position: actor.position,
            parameters: [1., 100., 2., -1., 100.],
            context: actor.context,
            cross_context: true,
            vimp_alert_range: 100.,
        };
        {
            let mut world = World {
                session: &mut session,
                hits: &mut hits,
                elapsed_seconds: 2.5,
                cursor: None,
            };
            assert!(machine.run(input, &mut world).is_err());
            assert_eq!(machine.phase(), crate::radial_damage::Phase::Failed);
            assert!(machine.run(input, &mut world).is_err());
        }
        assert_eq!(hits.0.len(), 1);
        assert_eq!(hits.0[0].target, first);
        assert_eq!(hits.0[0].source, source);
        assert_eq!(session.vimp(first).unwrap().damage, before_health);
        let body = session.get(first).unwrap().seed.storage.data().unwrap();
        assert_eq!(
            f32::from_le_bytes(body[0xc4..0xc8].try_into().unwrap()),
            2.5
        );
        for (i, value) in source_position.into_iter().enumerate() {
            assert_eq!(
                u32::from_le_bytes(body[0xc8 + 4 * i..0xcc + 4 * i].try_into().unwrap()),
                value.to_bits()
            );
        }
        assert!(Session::new(661).radial_actor(first).is_err());
        session.clear();
        assert!(session.radial_actor(first).is_err());
    }
}
