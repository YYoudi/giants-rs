//! Ordinary offline Vimp damage with no source actor or impact point. The
//! stationary impulse prefix is empty; blood, sounds and death services run.
use crate::{
    audio_parameters::Random, blood_particles, resident_states::Id, vimp_chunks, vimp_death_proxy,
};
use giants_formats::global_data::GlobalData;

#[derive(Clone, Copy, Debug)]
pub struct DeathEffect {
    pub position: [f32; 3],
    /// Source death-program request, flags9, rotation [angle, 0, 0].
    pub angle: f32,
}
/// The provider must consume the source death program, or report failure.
/// A recorded zero-return request sink is only an explicit test boundary.
pub trait Effects {
    fn death(&mut self, effect: DeathEffect, random: &mut Random) -> Result<(), String>;
}
pub trait Sounds {
    fn positional(
        &mut self,
        selector: u32,
        position: [f32; 3],
        random: &mut Random,
    ) -> Result<(), String>;
    fn attached(&mut self, selector: u32, actor: Id, random: &mut Random) -> Result<(), String>;
}
pub trait Blood {
    fn hit(
        &mut self,
        session: &crate::resident_states::Session,
        owner: Id,
        actor: blood_particles::Actor,
        point: Option<[f32; 3]>,
        random: &mut Random,
    ) -> Result<usize, String>;
}
impl Blood for blood_particles::Pool {
    fn hit(
        &mut self,
        _: &crate::resident_states::Session,
        _: Id,
        actor: blood_particles::Actor,
        point: Option<[f32; 3]>,
        random: &mut Random,
    ) -> Result<usize, String> {
        if point.is_some() {
            return Err("impact blood requires an attached-wound owner".into());
        }
        self.emit(actor, random)
    }
}
pub struct AttachedBlood<'a> {
    pub particles: &'a mut blood_particles::Pool,
    pub wounds: &'a mut crate::blood_wounds::Pool<Id, String>,
}
impl Blood for AttachedBlood<'_> {
    fn hit(
        &mut self,
        session: &crate::resident_states::Session,
        owner: Id,
        actor: blood_particles::Actor,
        point: Option<[f32; 3]>,
        random: &mut Random,
    ) -> Result<usize, String> {
        if let Some(point) = point {
            self.wounds
                .attach(owner, point, session.vimp_wound_pose(owner)?)?;
            Ok(0)
        } else {
            self.particles.emit(actor, random)
        }
    }
}
/// Source-bearing ordinary partial-pistol hit. Serial is not a Session actor.
pub struct PartialPistolHit {
    pub shooter: Id,
    pub hit: crate::mecc_pistol::WorldHit,
}
pub struct Services<'a, 'r> {
    pub effects: &'a mut dyn Effects,
    pub sounds: &'a mut dyn Sounds,
    pub blood: &'a mut dyn Blood,
    pub chunks: &'a mut vimp_chunks::Factory<'r>,
    pub proxy: &'a vimp_death_proxy::Factory,
    pub globals: &'a GlobalData,
}
#[derive(Debug)]
pub enum Outcome {
    Ignored,
    Hurt { blood_particles: usize },
    Died { chunks: Vec<Id>, proxy: Option<Id> },
}
pub(crate) fn choice(random: &mut Random) -> u32 {
    u32::from(((random.next_15() * 100) >> 15) >= 50)
}
