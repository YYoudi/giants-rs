//! Actual class230 mode71 marker relation, not a retirement acknowledgement.
//! Prototype PS2 `ripper_move` remains an unknown attribution for the PC caller.
use super::*;
use crate::ripper_primary_turn::{FrameInput, Outcome, Services, State};
type Effect = crate::attached_effect_context::Id;
struct Bridge<'a, S> {
    session: &'a mut Session,
    owner: Id,
    services: &'a mut S,
}
impl<S: Services<Id, Effect>> Services<Id, Effect> for Bridge<'_, S> {
    fn owner_sound(
        &mut self,
        s: &State<Id, Effect>,
        cue: u32,
        r: &mut crate::audio_parameters::Random,
    ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
        self.services.owner_sound(s, cue, r)
    }
    fn remove_effect(
        &mut self,
        s: &State<Id, Effect>,
        e: Effect,
        r: &mut crate::audio_parameters::Random,
    ) -> Result<(), String> {
        self.services.remove_effect(s, e, r)
    }
    fn mark_surface_marker_retirement(&mut self, _: &State<Id, Effect>) -> Result<(), String> {
        self.session
            .mark_ripper_surface_marker_retirement(self.owner)
    }
}
impl Session {
    pub fn update_ripper_variant_retreat(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        random: &mut crate::audio_parameters::Random,
        services: &mut impl Services<Id, Effect>,
    ) -> Result<Outcome, String> {
        self.update_ripper_owned_primary(
            id,
            input,
            random,
            services,
            |session, machine, input, random, services| {
                let mut bridge = Bridge {
                    session,
                    owner: id,
                    services,
                };
                machine.advance_variant_retreat(input, random, &mut bridge)
            },
        )
    }
}
