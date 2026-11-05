//! Ticket-owned terminal effects and the bounded persistent attached queue.
//! Persistent previews do not acknowledge world/campaign readiness or fidelity.
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
    persistent: Vec<Option<giants_game::cinematic_persistent::Bindings>>,
    persistent_queue: Option<(usize, giants_game::cinematic_persistent::Queue)>,
}
impl State {
    pub(crate) fn ticket(&self) -> u64 {
        self.ticket
    }
    pub(crate) fn advanced_persistent_objects(&self) -> usize {
        self.persistent_queue
            .as_ref()
            .map_or(0, |(_, queue)| queue.advanced_objects())
    }
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
            persistent: vec![],
            persistent_queue: None,
        }
    }
    pub(crate) fn with_persistent(
        mut self,
        bindings: Vec<Option<giants_game::cinematic_persistent::Bindings>>,
    ) -> Self {
        self.persistent = bindings;
        self
    }
    pub(crate) fn persistent_frame(
        &self,
        director: usize,
        offset: usize,
    ) -> Option<Option<giants_game::cinematic_persistent::Frame>> {
        if !self.persistent.get(director)?.as_ref()?.owns(offset) {
            return None;
        }
        Some(
            self.persistent_queue
                .as_ref()
                .filter(|(d, _)| *d == director)
                .and_then(|(_, queue)| queue.frame(offset)),
        )
    }
    fn release_persistent(&mut self) {
        if let Some((director, mut queue)) = self.persistent_queue.take() {
            let objects = queue
                .release()
                .expect("source-marked persistent queue release");
            println!(
                "mission_persistent_released ticket={} director={director} retired_objects={objects} remaining_objects={} remaining_contexts={} lifetime_children=0",
                self.ticket,
                queue.objects(),
                queue.contexts()
            );
        }
    }
    fn select_persistent(&mut self, director: usize) {
        if self
            .persistent_queue
            .as_ref()
            .is_some_and(|(d, _)| *d != director)
        {
            self.release_persistent();
        }
        if self.persistent_queue.is_none()
            && let Some(Some(binding)) = self.persistent.get(director)
        {
            self.persistent_queue = Some((director, binding.queue(self.ticket)));
        }
    }
    fn dispatch_persistent(
        &mut self,
        director: usize,
        offset: usize,
        seconds: f32,
    ) -> Result<bool, String> {
        let Some(binding) = self.persistent.get(director).and_then(Option::as_ref) else {
            return Ok(false);
        };
        let (_, queue) = self
            .persistent_queue
            .as_mut()
            .ok_or("persistent queue absent")?;
        queue.dispatch(binding, offset, seconds)?;
        println!(
            "mission_persistent_spawn ticket={} director={director} offset={offset} objects={} contexts={} source_marked=true readiness_acknowledged=false",
            self.ticket,
            queue.objects(),
            queue.contexts()
        );
        Ok(true)
    }
    fn update_persistent(&mut self, seconds: f32, delta: f32) -> Result<(), String> {
        if let Some((director, queue)) = &mut self.persistent_queue {
            queue.update(
                self.persistent[*director]
                    .as_ref()
                    .ok_or("persistent binding absent")?,
                seconds,
                delta,
            )?;
        }
        Ok(())
    }
    pub(crate) fn report_release(mut self) {
        self.release_persistent();
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
        state.release_persistent();
        state.active.clear();
        state.revision = 0;
        state.generation = cinematic.generation();
        state.dispatched = 0;
        state.retired = 0;
    }
    state.select_persistent(cinematic.director_index());
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
                        let reason = error.clone();
                        state.dispatch_persistent(
                            cinematic.director_index(),
                            event.offset,
                            cinematic.event_seconds(),
                        )?;
                        println!(
                            "mission_effect_dispatch_deferred ticket={} director={} offset={} reason={reason:?} success=false",
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
        state.update_persistent(cinematic.pose_seconds(), cinematic.animation_dt())?;
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
