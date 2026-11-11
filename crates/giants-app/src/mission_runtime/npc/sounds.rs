//! NPC requests use the common owned sound transport. Only its actual AudioSink
//! consumers acknowledge a voice; this queue does not advance an audio RNG.
use crate::mission_audio::Event;
use giants_game::{audio_parameters::Random, resident_states::Id};

#[derive(Default)]
pub(super) struct Sounds(pub Vec<Event>);
impl giants_game::positional_effect_audio::Service for Sounds {
    fn request(&mut self, sound: u32, position: [f32; 3], _: &mut Random) -> Result<(), String> {
        if !position.into_iter().all(f32::is_finite) {
            return Err("NPC positional sound is nonfinite".into());
        }
        self.0.push(Event::NpcPosition { sound, position });
        Ok(())
    }
}
impl giants_game::vimp_hurt::Sounds for Sounds {
    fn stop(&mut self, owner: Id, sound: u32, _: &mut Random) -> Result<(), String> {
        self.0.push(Event::StopActorSound { owner, sound });
        Ok(())
    }
}
impl giants_game::vimp_damage::Sounds for Sounds {
    fn positional(
        &mut self,
        sound: u32,
        position: [f32; 3],
        random: &mut Random,
    ) -> Result<(), String> {
        giants_game::positional_effect_audio::Service::request(self, sound, position, random)
    }
    fn attached(&mut self, owner_sound: u32, owner: Id, _: &mut Random) -> Result<(), String> {
        self.0.push(Event::ActorSound {
            owner,
            sound: owner_sound,
            kind: giants_game::actor_sound::Kind::Owner,
        });
        Ok(())
    }
}
