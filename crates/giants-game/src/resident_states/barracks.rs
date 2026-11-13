//! Canonical state98 initial branch. Prototype PS2 `barracks_logic` remains an
//! unknown-attribution route. Later spawning/AI and world phases remain separate.
use super::{Id, Session};
use crate::building_barracks::{self, InitialParameters, State};

#[derive(Debug)]
pub(super) struct Owned {
    pub(super) state: State<Id>,
    failure: Option<String>,
    running: bool,
}
/// Real hit attribution and the current source alert-radius global. Supplying
/// an arbitrary actor as the player is not an ordinary input integration.
#[derive(Clone, Copy, Debug)]
pub struct Attack {
    pub amount: f32,
    pub hit_owner: Option<Id>,
    pub player: Id,
    pub radius_base: f32,
}
pub trait Effects {
    fn object_sound(&mut self, owner: Id, cue: u32) -> Result<(), String>;
    fn explosion(
        &mut self,
        kind: building_barracks::Explosion,
        point: [f32; 3],
    ) -> Result<(), String>;
    fn positional_sound(&mut self, cue: u32, point: [f32; 3]) -> Result<(), String>;
    fn mission_trigger(&mut self, owner: Id, state: &State<Id>) -> Result<(), String>;
}
struct HitServices<'a, E> {
    session: &'a mut Session,
    id: Id,
    effects: &'a mut E,
}
impl<E: Effects> building_barracks::Services<Id> for HitServices<'_, E> {
    fn alert_team(&mut self, state: &State<Id>, radius: f32) -> Result<(), String> {
        self.session.alert_team_from_building(
            self.id,
            radius,
            state.target.ok_or("barracks alert target absent")?,
        )
    }
    fn alert_rippers(&mut self, state: &State<Id>, radius: f32) -> Result<(), String> {
        self.session.alert_rippers_from_building(
            self.id,
            radius,
            state.target.ok_or("barracks alert target absent")?,
        )
    }
    fn object_sound(&mut self, cue: u32) -> Result<(), String> {
        self.effects.object_sound(self.id, cue)
    }
    fn explosion(
        &mut self,
        kind: building_barracks::Explosion,
        point: [f32; 3],
    ) -> Result<(), String> {
        self.effects.explosion(kind, point)
    }
    fn positional_sound(&mut self, cue: u32, point: [f32; 3]) -> Result<(), String> {
        self.effects.positional_sound(cue, point)
    }
    fn mission_trigger(&mut self, state: &State<Id>) -> Result<(), String> {
        self.effects.mission_trigger(self.id, state)
    }
    fn retire(&mut self) -> Result<(), String> {
        // Acquired generic retirement callback: mark only. Real list/resource
        // destruction remains a separate lifetime operation, not fabricated here.
        self.session
            .residents
            .get_mut(&self.id.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .unwrap()
            .object_flags |= 0x8000;
        Ok(())
    }
}
struct Sound<'a, F> {
    id: Id,
    request: &'a mut F,
}
impl<F: FnMut(Id, u32) -> Result<(), String>> building_barracks::InitialSound<Id> for Sound<'_, F> {
    fn object_sound(&mut self, _: &State<Id>, cue: u32) -> Result<(), String> {
        (self.request)(self.id, cue)
    }
}
impl Session {
    /// Execute the acquired initial branch once on its actual mission owner.
    /// `request_sound` must accept the real object-owned request; returning Ok
    /// does not acknowledge a backend voice. Neither this method nor a retained
    /// initial state acknowledges subsequent AI or an actor world tick.
    pub fn initialize_barracks(
        &mut self,
        id: Id,
        stock: InitialParameters,
        suppressed: bool,
        request_sound: &mut impl FnMut(Id, u32) -> Result<(), String>,
    ) -> Result<bool, String> {
        let actor = self.get(id).ok_or("barracks owner absent or foreign")?;
        if actor.barracks.is_some() {
            return Err("barracks initial attempt already consumed; do not replay".into());
        }
        if !actor.acquired_construction
            || actor.seed.storage.state() != 98
            || !matches!(actor.class, 1060 | 1226)
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || actor.world_tick_running
            || self.specification(id).is_none()
            || self.state_resources(id).is_none()
        {
            return Err("barracks initialization requires its acquired cold owner".into());
        }
        if suppressed {
            return Ok(false);
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("barracks fields absent")?;
        let mut channels = actor.channels.ok_or("barracks channels absent")?;
        let mut state = State {
            position: fields.position,
            target: None,
            phase: 0,
            selector: fields.action as i32,
            damage: 0.,
            limit: f32::from_bits(fields.configuration[12]),
            mark_flags: fields.response_flags,
            remaining: 0,
        };
        let result = building_barracks::initialize(
            &mut state,
            actor.class,
            stock,
            false,
            &mut channels,
            &mut Sound {
                id,
                request: request_sound,
            },
        );
        let actor = self.residents.get_mut(&id.serial).unwrap();
        // Failures retain completed numeric writes and prevent a repeated sound
        // request. The actor identity, resources and registrations stay owned.
        actor.channels = Some(channels);
        actor.construction_fields.as_mut().unwrap().action = state.selector as u32;
        actor.barracks = Some(Owned {
            state,
            failure: result.as_ref().err().cloned(),
            running: false,
        });
        result
    }
    /// Initialized source state, independent of later primary/world readiness.
    /// A failed initial request is not exposed as a usable damage owner.
    pub fn barracks_state(&self, id: Id) -> Result<&State<Id>, String> {
        let owner = self
            .get(id)
            .and_then(|a| a.barracks.as_ref())
            .ok_or("barracks not initialized or foreign")?;
        if let Some(failure) = &owner.failure {
            return Err(format!("barracks initial failure: {failure}"));
        }
        if owner.running {
            return Err("barracks callback already running".into());
        }
        Ok(&owner.state)
    }
    /// Ordered shot against the canonical state and actual registered actors.
    /// Every required effect/audio/mission consumer must be admitted by the
    /// ordinary caller. A failed prefix remains owned and cannot be replayed.
    pub fn hit_barracks(
        &mut self,
        id: Id,
        attack: Attack,
        effects: &mut impl Effects,
    ) -> Result<bool, String> {
        self.get(attack.player)
            .ok_or("barracks actual player absent or foreign")?;
        if let Some(owner) = attack.hit_owner {
            self.get(owner)
                .ok_or("barracks actual shooter absent or foreign")?;
        }
        let mut state = *self.barracks_state(id)?;
        let fields = self
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .ok_or("barracks current fields absent")?;
        if fields.object_flags & 0x8000 != 0 {
            return Ok(false);
        }
        state.position = fields.position;
        state.selector = fields.action as i32;
        state.mark_flags = fields.response_flags;
        state.limit = f32::from_bits(fields.configuration[12]);
        self.residents
            .get_mut(&id.serial)
            .unwrap()
            .barracks
            .as_mut()
            .unwrap()
            .running = true;
        let result = building_barracks::hit(
            &mut state,
            attack.amount,
            attack.hit_owner,
            attack.player,
            attack.radius_base,
            &mut HitServices {
                session: self,
                id,
                effects,
            },
        );
        let owned = self
            .residents
            .get_mut(&id.serial)
            .unwrap()
            .barracks
            .as_mut()
            .unwrap();
        owned.state = state;
        owned.failure = result.as_ref().err().cloned();
        owned.running = false;
        result
    }
    /// Separate numeric health-animation suffix. This does not execute the
    /// building's later spawning/AI primary or acknowledge a world frame.
    pub fn refresh_barracks_health(
        &mut self,
        id: Id,
        thresholds: crate::building_damage::Thresholds,
    ) -> Result<f32, String> {
        if self
            .get(id)
            .and_then(|a| a.construction_fields.as_ref())
            .is_some_and(|f| f.object_flags & 0x8000 != 0)
        {
            return Err("retired barracks cannot execute a later health suffix".into());
        }
        let state = self.barracks_state(id)?;
        let health = building_barracks::health_frame(state.damage, state.limit, thresholds)?;
        let actor = self.residents.get_mut(&id.serial).unwrap();
        let channels = actor
            .channels
            .as_mut()
            .ok_or("barracks health channels absent")?;
        channels[0][1] = health.fraction.to_bits();
        channels[0][8] |= 0x80;
        if health.terminal {
            actor.barracks.as_mut().unwrap().state.remaining = 0;
        }
        Ok(health.fraction)
    }
}
