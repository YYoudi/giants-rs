//! Exact-D history/reset pair used inside surfacing. No ground correction,
//! transform invalidation, primary transition, or external service is implied.
#[derive(Clone, Copy, Debug)]
pub struct State {
    pub position: [f32; 3],
    pub current: [f32; 3],
    pub previous: [f32; 3],
    pub rates: [f32; 4],
    pub previous_rates: [f32; 4],
    pub velocity: [f32; 3],
    pub previous_velocity: [f32; 3],
}
pub fn reset(mut state: State) -> State {
    state.previous = state.current;
    state.current = state.position;
    state.rates = [0.; 4];
    state.previous_rates = [0.; 4];
    state.velocity = [0.; 3];
    state.previous_velocity = [0.; 3];
    state
}
