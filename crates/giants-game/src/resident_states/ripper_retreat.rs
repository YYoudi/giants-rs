//! Actual class73 mode43/45 / class230 mode71 marker relation, not cleanup acceptance.
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
    fn prepare_awakening_dirt(
        &mut self,
        s: &State<Id, Effect>,
        r: &mut crate::audio_parameters::Random,
    ) -> Result<Option<Effect>, String> {
        self.services.prepare_awakening_dirt(s, r)
    }
    fn ground_normal_z(&mut self, s: &State<Id, Effect>) -> Result<f32, String> {
        self.services.ground_normal_z(s)
    }
    fn observe_ground(&mut self, s: &State<Id, Effect>) {
        self.services.observe_ground(s)
    }
    fn stop_owner_sound(
        &mut self,
        s: &State<Id, Effect>,
        cue: u32,
        r: &mut crate::audio_parameters::Random,
    ) -> Result<crate::enemy_projectile::SoundOutcome, String> {
        self.services.stop_owner_sound(s, cue, r)
    }
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
    pub fn update_ripper_classic_orbit_on_static_terrain(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        parameters: crate::ripper_primary_turn::ClassicOrbitParameters,
        terrain: &crate::terrain_queries::StaticTerrainQuery<'_>,
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
                machine.advance_classic_orbit(
                    input,
                    parameters,
                    random,
                    &mut crate::ripper_ground::StaticGround {
                        query: terrain,
                        services: &mut bridge,
                    },
                )
            },
        )
    }
    pub fn update_ripper_retreat(
        &mut self,
        id: Id,
        input: FrameInput<'_, Id>,
        buried_speed_percent: f32,
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
                machine.advance_retreat(input, buried_speed_percent, random, &mut bridge)
            },
        )
    }
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
