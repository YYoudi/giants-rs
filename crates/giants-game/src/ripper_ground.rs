//! Static exact-D terrain producer for the ordinary primary. Other services
//! remain mandatory and are forwarded unchanged. Dynamic ripple is excluded
//! by the supplied StaticTerrainQuery, never silently approximated here.
use crate::{
    audio_parameters::Random,
    enemy_projectile::{EffectOutcome, SoundOutcome},
    ripper_primary_turn::{Services, State},
    state23_animation::FireOutcome,
    terrain_queries::StaticTerrainQuery,
};

pub struct StaticGround<'a, 'terrain, S> {
    pub query: &'a StaticTerrainQuery<'terrain>,
    pub services: &'a mut S,
}
impl<K, E, S: Services<K, E>> Services<K, E> for StaticGround<'_, '_, S> {
    fn ground_normal_z(&mut self, state: &State<K, E>) -> Result<f32, String> {
        self.services.observe_ground(state);
        Ok(self
            .query
            .sample([state.entry.position[0], state.entry.position[1]])?
            .plane[2])
    }
    fn observe_ground(&mut self, state: &State<K, E>) {
        self.services.observe_ground(state);
    }
    fn prepare_awakening_dirt(
        &mut self,
        state: &State<K, E>,
        random: &mut Random,
    ) -> Result<Option<E>, String> {
        self.services.prepare_awakening_dirt(state, random)
    }
    fn surface_popup(
        &mut self,
        state: &State<K, E>,
        random: &mut Random,
    ) -> Result<EffectOutcome, String> {
        self.services.surface_popup(state, random)
    }
    fn reset_surface_motion(&mut self, state: &State<K, E>) -> Result<(), String> {
        self.services.reset_surface_motion(state)
    }
    fn construct_surface_marker(&mut self, state: &State<K, E>) -> Result<K, String> {
        self.services.construct_surface_marker(state)
    }
    fn stop_owner_sound(
        &mut self,
        state: &State<K, E>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.services.stop_owner_sound(state, cue, random)
    }
    fn remove_effect(
        &mut self,
        state: &State<K, E>,
        effect: E,
        random: &mut Random,
    ) -> Result<(), String> {
        self.services.remove_effect(state, effect, random)
    }
    fn owner_sound(
        &mut self,
        state: &State<K, E>,
        cue: u32,
        random: &mut Random,
    ) -> Result<SoundOutcome, String> {
        self.services.owner_sound(state, cue, random)
    }
    fn fire(
        &mut self,
        state: &State<K, E>,
        target: [f32; 3],
        vertical_spread_percent: f32,
        random: &mut Random,
    ) -> Result<FireOutcome, String> {
        self.services
            .fire(state, target, vertical_spread_percent, random)
    }
}
