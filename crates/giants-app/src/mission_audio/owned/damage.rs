//! Local Mecc voice transport sharing the actual reserved jet backend. Every
//! backend operation borrows it exclusively; native no-play needs no borrow.
use super::*;
use giants_game::resident_states::{Id, MeccImpactVoice, Session};
use std::cell::{RefCell, RefMut};

pub(crate) struct SharedBackend<'a> {
    backend: RefCell<&'a mut dyn Backend<Handle = u32>>,
}
impl<'a> SharedBackend<'a> {
    pub(crate) fn new(backend: &'a mut dyn Backend<Handle = u32>) -> Self {
        Self {
            backend: RefCell::new(backend),
        }
    }
    fn borrow(&self) -> Result<RefMut<'_, &'a mut dyn Backend<Handle = u32>>, String> {
        self.backend
            .try_borrow_mut()
            .map_err(|_| "nested owned audio backend operation".into())
    }
    pub(crate) fn jet(&self) -> Jet<'_, 'a> {
        Jet(self)
    }
    pub(crate) fn voice(&self, controlled: Id, ticket: u64) -> LocalVoice<'_, 'a> {
        self.voice_with_playback(
            controlled,
            ticket,
            giants_game::startup_playback::Playback::default(),
        )
    }
    pub(crate) fn voice_with_playback(
        &self,
        controlled: Id,
        ticket: u64,
        playback: giants_game::startup_playback::Playback,
    ) -> LocalVoice<'_, 'a> {
        LocalVoice {
            shared: self,
            controlled,
            ticket,
            playback,
        }
    }
}
pub(crate) struct Jet<'a, 'b>(&'a SharedBackend<'b>);
impl Backend for Jet<'_, '_> {
    type Handle = u32;
    fn stop_sound(&mut self, sound: u32, owner: u64) -> Result<(), String> {
        self.0.borrow()?.stop_sound(sound, owner)
    }
    fn play(&mut self, sound: u32, owner: u64, route: Route) -> Result<u32, String> {
        self.0.borrow()?.play(sound, owner, route)
    }
    fn duration(&mut self, id: u32) -> Result<f32, String> {
        self.0.borrow()?.duration(id)
    }
    fn volume(&mut self, id: u32, value: f32) -> Result<(), String> {
        self.0.borrow()?.volume(id, value)
    }
    fn stop(&mut self, id: u32) -> Result<(), String> {
        self.0.borrow()?.stop(id)
    }
}
pub(crate) struct LocalVoice<'a, 'b> {
    shared: &'a SharedBackend<'b>,
    controlled: Id,
    ticket: u64,
    playback: giants_game::startup_playback::Playback,
}
impl MeccImpactVoice for LocalVoice<'_, '_> {
    fn play(&mut self, session: &Session, source: Id, sound: u32) -> Result<(), String> {
        if self.controlled.ticket() != self.ticket
            || source.ticket() != self.ticket
            || !DAMAGE_SOUNDS.contains(&sound)
            || session.get(source).is_none_or(|actor| actor.class != 2)
        {
            return Err(
                "Mecc voice requires the current owned source, root and cue profile".into(),
            );
        }
        let route = session.sound_route_with_playback(
            source,
            self.controlled,
            giants_game::actor_sound::Kind::Attachment,
            true,
            self.playback,
        )?;
        if route.is_none() && self.playback.suppressed() {
            // Native no-play returns before descriptor/device work. Successful
            // cue handling here acknowledges suppression, not a playback start.
            return Ok(());
        }
        if route != Some(giants_game::actor_sound::Route::Local(self.controlled)) {
            return Err("Mecc positional voice route remains unbound".into());
        }
        // Zero is a true catalog/source miss. The backend logs it separately;
        // read/decode or absent-ready-device failures remain errors.
        self.shared
            .borrow()?
            .play(sound, self.ticket, Route::Local)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
