//! Source-dispatched, ticket-owned terminal effects. Other programs are refused
//! during binding; no particle emitter or gameplay readiness is inferred.
use bevy::prelude::*;
use giants_game::{
    cinematic_fx::{Bindings, TerminalProxy},
    flick::EventKind,
};
#[derive(Resource)]
pub(crate) struct State {
    ticket: u64,
    bindings: Vec<Result<Bindings, String>>,
    active: Vec<TerminalProxy>,
    revision: u64,
    generation: u64,
    dispatched: usize,
    retired: usize,
}
impl State {
    pub(crate) fn settled(&self) -> bool {
        self.active.is_empty()
            && self.bindings.iter().all(Result::is_ok)
            && self.dispatched
                == self
                    .bindings
                    .iter()
                    .filter_map(|b| b.as_ref().ok())
                    .map(Bindings::len)
                    .sum::<usize>()
    }
    pub(crate) fn new(ticket: u64, bindings: Vec<Result<Bindings, String>>) -> Self {
        Self {
            ticket,
            bindings,
            active: Vec::new(),
            revision: 0,
            generation: 0,
            dispatched: 0,
            retired: 0,
        }
    }
    pub(crate) fn report_release(&self) {
        println!(
            "mission_effects_released ticket={} dispatched={} retired={} active={} queue_owned=true",
            self.ticket,
            self.dispatched,
            self.retired,
            self.active.len()
        );
    }
}
pub(crate) fn update(
    owner: Res<crate::mission_opening::Owner>,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    mut state: ResMut<State>,
    mut exit: MessageWriter<AppExit>,
    mut showcase: Option<ResMut<crate::mission_showcase::State>>,
) {
    assert_eq!(owner.0, state.ticket);
    if cinematic.paused() {
        return;
    }
    if state.generation != cinematic.generation() {
        state.active.clear();
        state.revision = 0;
        state.generation = cinematic.generation();
        state.dispatched = 0;
        state.retired = 0;
    }
    let result = (|| -> Result<(), String> {
        if state.revision != cinematic.event_revision() {
            for event in cinematic
                .events()
                .iter()
                .filter(|e| e.kind == EventKind::Effect)
            {
                let binding = state
                    .bindings
                    .get(cinematic.director_index())
                    .ok_or("effect director not bound")?;
                let binding = match binding {
                    Ok(binding) => binding,
                    Err(error) if showcase.as_ref().is_some_and(|s| s.enabled()) => {
                        println!(
                            "mission_effect_dispatch_deferred ticket={} director={} offset={} reason={error:?} success=false",
                            owner.0,
                            cinematic.director_index(),
                            event.offset
                        );
                        continue;
                    }
                    Err(error) => return Err(error.clone()),
                };
                if let Some(proxy) = binding.instantiate(event.offset, cinematic.event_seconds())? {
                    if state
                        .active
                        .iter()
                        .any(|p| p.placement.slot == proxy.placement.slot)
                    {
                        return Err(
                            "effect slot replaced before its previous terminal retirement".into(),
                        );
                    }
                    println!(
                        "mission_effect_spawn ticket={} director={} offset={} slot={} seconds_bits={} position_bits={:?} cursor={} active={} profile=mode1_fx02",
                        owner.0,
                        cinematic.director_index(),
                        event.offset,
                        proxy.placement.slot,
                        cinematic.event_seconds().to_bits(),
                        proxy.placement.position.map(f32::to_bits),
                        proxy.cursor(),
                        proxy.active()
                    );
                    state.active.push(proxy);
                } else {
                    println!(
                        "mission_effect_inactive ticket={} director={} offset={} slot_negative=true",
                        owner.0,
                        cinematic.director_index(),
                        event.offset
                    );
                }
                state.dispatched += 1;
            }
            state.revision = cinematic.event_revision();
        }
        for proxy in &mut state.active {
            proxy.update().map_err(str::to_owned)?;
            println!(
                "mission_effect_update ticket={} slot={} cursor={} active={} emitted_objects=0 profile=mode1_fx02",
                owner.0,
                proxy.placement.slot,
                proxy.cursor(),
                proxy.active()
            );
        }
        let before = state.active.len();
        state.active.retain(TerminalProxy::active);
        state.retired += before - state.active.len();
        Ok(())
    })();
    if let Err(error) = result {
        error!("Mission effect dispatch failed: {error}");
        if !showcase.as_mut().is_some_and(|s| s.playback_failed(&error)) {
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::State;
    #[test]
    fn pending_effect_binding_cannot_acknowledge_movie_completion() {
        let state = State::new(7, vec![Err("unknown actor service".into())]);
        assert!(!state.settled());
        assert!(State::new(7, vec![]).settled());
    }
}
