//! Required shared-world guards, actual controller publication and live audio.
//! Prototype PS2 `hurt_object`/`mecc_shot` remain unknown search relationships.
//! Wiring consumers does not establish availability of the missing world owner.
use giants_game::{
    audio_parameters::Random,
    enemy_projectile::{ObjectImpact, ObjectImpactServices},
    resident_states::{Id, MeccEnemyHitGates, MeccEnemyHitServices, Session},
};

pub(in crate::mission_runtime) trait Services:
    MeccEnemyHitServices + giants_game::actor_retirement::Services
{
    fn world_suppressed(&mut self, session: &Session) -> Result<bool, String>;
}

/// Legacy probes supply retirement only. Do not turn them into accepted
/// controller/audio/world services when they reach an actual actor hit.
#[cfg(test)]
pub(in crate::mission_runtime) struct RetirementOnly<'a, D>(pub &'a mut D);
#[cfg(test)]
impl<D> MeccEnemyHitServices for RetirementOnly<'_, D> {
    fn gates(&mut self, _: &Session, _: Id) -> Result<MeccEnemyHitGates, String> {
        Err("Ripper hit requires actual shared-world receiver guards".into())
    }
    fn recoil(
        &mut self,
        _: &Session,
        _: giants_game::controlled_mecc::HitResponse,
    ) -> Result<(), String> {
        Err("retirement-only Ripper probe has no controlled Mecc receiver".into())
    }
    fn voice(&mut self, _: &Session, _: Id, _: u32) -> Result<(), String> {
        Err("retirement-only Ripper probe has no local voice device".into())
    }
}
#[cfg(test)]
impl<D: giants_game::actor_retirement::Services> giants_game::actor_retirement::Services
    for RetirementOnly<'_, D>
{
    fn release(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
        self.0.release(session, id)
    }
}
#[cfg(test)]
impl<D: giants_game::actor_retirement::Services> Services for RetirementOnly<'_, D> {
    fn world_suppressed(&mut self, _: &Session) -> Result<bool, String> {
        Err("Ripper hit requires actual shared-world failure/completion guards".into())
    }
}

pub(in crate::mission_runtime) struct Normal<'a, 'p, D> {
    pub owner: Id,
    pub receiver: crate::mecc_dev_scene::EnemyReceiver<'p>,
    pub device: &'a mut D,
}
impl<D: giants_game::resident_states::MeccImpactVoice> MeccEnemyHitServices for Normal<'_, '_, D> {
    fn gates(&mut self, session: &Session, target: Id) -> Result<MeccEnemyHitGates, String> {
        if target != self.owner || session.get(target).is_none() {
            return Err("Ripper receiver differs from the current normal Mecc owner".into());
        }
        Err("Ripper hit requires actual shared-world edit/health-suppression guards".into())
    }
    fn recoil(
        &mut self,
        session: &Session,
        response: giants_game::controlled_mecc::HitResponse,
    ) -> Result<(), String> {
        self.receiver
            .accept_enemy_hit_response(session, self.owner, response)
    }
    fn voice(&mut self, session: &Session, target: Id, cue: u32) -> Result<(), String> {
        if target != self.owner {
            return Err(
                "Ripper local voice target differs from the current controlled Mecc".into(),
            );
        }
        self.device.play(session, target, cue)
    }
}
impl<D: giants_game::actor_retirement::Services> giants_game::actor_retirement::Services
    for Normal<'_, '_, D>
{
    fn release(&mut self, session: &mut Session, id: Id) -> Result<(), String> {
        self.device.release(session, id)
    }
}
impl<D: giants_game::actor_retirement::Services + giants_game::resident_states::MeccImpactVoice>
    Services for Normal<'_, '_, D>
{
    fn world_suppressed(&mut self, session: &Session) -> Result<bool, String> {
        if session.get(self.owner).is_none() {
            return Err("Ripper shared-world request lost its controlled Mecc".into());
        }
        // The campaign's portable counter and a living Mecc cannot substitute
        // for this native owner. No clear snapshot or readiness is fabricated.
        Err("Ripper hit requires actual shared-world failure/completion guards".into())
    }
}

pub(super) struct Object<'a, D> {
    pub device: &'a mut D,
    pub step: u64,
    pub source: Id,
}
impl<D: Services> ObjectImpactServices for Object<'_, D> {
    fn world_suppressed(&mut self, session: &Session) -> Result<bool, String> {
        self.device.world_suppressed(session)
    }
    fn hurt(
        &mut self,
        session: &mut Session,
        request: ObjectImpact,
        random: &mut Random,
    ) -> Result<(), String> {
        if request.source != self.source {
            return Err("Ripper object continuation changed its actual source".into());
        }
        session.damage_mecc_from_enemy_object(request, self.step, random, self.device)
    }
    fn terminal(&mut self, session: &mut Session, target: Id) -> Result<(), String> {
        if session.enemy_object_terminal_no_effect(self.source, target)? {
            Ok(())
        } else {
            Err("Ripper object hit requires its actual mission/credit terminal consumer".into())
        }
    }
}
