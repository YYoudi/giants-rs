//! Acquired post-countdown and record-advance orchestration. Storage identities,
//! strings, player keys and every side-effecting service belong to the caller.
//! This does not provide a mission scheduler, UI transport or save/load backend.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Controls {
    pub notice_shown: u32,
    pub first_visit: u32,
    pub music_mode: i32,
    pub transition_timers: [f32; 2],
    pub selected_player: u32,
}

/// Reads must resolve the actual retained mission handle. Text values are owned
/// references/handles, not an instruction to copy or retire the underlying text.
/// Providers may replace current mission/player state during callbacks.
pub trait Services {
    type Mission: Copy;
    type Text;
    type Selection;

    fn controls(&self) -> &Controls;
    fn controls_mut(&mut self) -> &mut Controls;
    fn current_mission(&self) -> Result<Self::Mission, String>;
    fn index(&self, mission: Self::Mission) -> Result<i32, String>;
    fn set_index(&mut self, mission: Self::Mission, index: i32) -> Result<(), String>;
    fn level(&self, mission: Self::Mission) -> Result<i32, String>;
    fn kind(&self, mission: Self::Mission, index: i32) -> Result<u32, String>;
    fn entry_text(&self, mission: Self::Mission, index: i32) -> Result<Self::Text, String>;
    /// `None` means the actual completion text starts with a null byte.
    fn completion_text(
        &self,
        mission: Self::Mission,
        previous: i32,
    ) -> Result<Option<Self::Text>, String>;
    fn world(&self) -> i32;
    fn controlled_player_key(&self) -> Result<u32, String>;
    fn progress(&self, world: i32, index: i32) -> Result<i32, String>;
    fn set_progress(&mut self, world: i32, index: i32, value: i32) -> Result<(), String>;
    fn set_required(&mut self, mission: Self::Mission, value: i32) -> Result<(), String>;
    fn set_completed(&mut self, mission: Self::Mission, value: i32) -> Result<(), String>;

    fn prepare_record(&mut self) -> Result<(), String>;
    fn message(&mut self, text: Self::Text) -> Result<(), String>;
    fn begin_countdown(&mut self) -> Result<(), String>;
    fn record_event(&mut self) -> Result<(), String>;
    fn reset_music(&mut self) -> Result<(), String>;
    fn resolve_selection(&mut self, text: Self::Text) -> Result<Self::Selection, String>;
    fn queue_selection(&mut self, selection: Self::Selection) -> Result<(), String>;
    fn commit_selection(&mut self) -> Result<(), String>;
    fn prepare_next(&mut self) -> Result<(), String>;
    fn select_next(&mut self, world: i32, index: i32, player: u32) -> Result<(), String>;
    fn finalize_progress(&mut self) -> Result<(), String>;
}

/// The countdown has already published its expiry and suppression-release writes.
/// Completed state/services survive later failure. The enclosing world owner must
/// quarantine a failed turn rather than replaying its completed prefix.
pub fn after_expiry(services: &mut impl Services) -> Result<bool, String> {
    if services.controls().notice_shown == 0 {
        let mission = services.current_mission()?;
        let previous = services.index(mission)?.wrapping_sub(1);
        if let Some(text) = services.completion_text(mission, previous)? {
            services.controls_mut().notice_shown = 1;
            services.message(text)?;
            return Ok(false);
        }
    }
    services.controls_mut().music_mode = 100;
    services.reset_music()?;
    let mission = services.current_mission()?;
    let world = services.world();
    let index = services.index(mission)?;
    let level = services.level(mission)?;
    if services.progress(world, index)? < level {
        services.set_progress(world, index, level)?;
    }
    let mut next = index;
    while next < 32 {
        if services.kind(mission, next)? == 2 {
            break;
        }
        next += 1;
    }
    services.prepare_next()?;
    if next < 32 {
        // The scan result is retained across this callback; the world and player
        // are read again after it, as in the native caller.
        services.select_next(services.world(), next, services.controlled_player_key()?)?;
    }
    services.finalize_progress()?;
    advance_record(services)
}

/// Complete record-advance branch. The boolean is the acquired caller return,
/// not a claim that a mission/campaign has finished or its services are bound.
pub fn advance_record(services: &mut impl Services) -> Result<bool, String> {
    let controls = services.controls_mut();
    controls.notice_shown = 0;
    controls.first_visit = 0;
    let mission = services.current_mission()?;
    if services.index(mission)? >= 27 {
        return Ok(false);
    }
    services.prepare_record()?;
    let mission = services.current_mission()?;
    let index = services.index(mission)?;
    match services.kind(mission, index)? {
        1 => {
            services.message(services.entry_text(mission, index)?)?;
            let mission = services.current_mission()?;
            let next = services.index(mission)?.wrapping_add(1);
            services.set_index(mission, next)?;
            Ok(true)
        }
        2 => {
            services.begin_countdown()?;
            Ok(true)
        }
        3 => {
            let controls = services.controls_mut();
            controls.music_mode = 100;
            controls.transition_timers = [0.; 2];
            services.reset_music()?;
            services.controls_mut().selected_player = services.controlled_player_key()?;
            let mission = services.current_mission()?;
            let text = services.entry_text(mission, services.index(mission)?)?;
            let selection = services.resolve_selection(text)?;
            let mission = services.current_mission()?;
            services.set_index(mission, 27)?;
            services.queue_selection(selection)?;
            services.commit_selection()?;
            Ok(true)
        }
        4 => {
            services.set_index(mission, index.wrapping_add(1))?;
            services.record_event()?;
            Ok(false)
        }
        _ => {
            services.set_required(mission, 9_999_999)?;
            let mission = services.current_mission()?;
            services.set_completed(mission, 1)?;
            Ok(false)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    struct Provider {
        c: Controls,
        current: usize,
        indexes: [i32; 2],
        kind: u32,
        fail_message: bool,
        fail_queue: bool,
        calls: Vec<&'static str>,
    }
    impl Provider {
        fn new() -> Self {
            Self {
                c: Controls {
                    notice_shown: 7,
                    first_visit: 9,
                    music_mode: 3,
                    transition_timers: [2., 3.],
                    selected_player: 19,
                },
                current: 0,
                indexes: [0, 2],
                kind: 1,
                fail_message: false,
                fail_queue: false,
                calls: Vec::new(),
            }
        }
    }
    impl Services for Provider {
        type Mission = usize;
        type Text = (usize, i32);
        type Selection = u32;
        fn controls(&self) -> &Controls {
            &self.c
        }
        fn controls_mut(&mut self) -> &mut Controls {
            &mut self.c
        }
        fn current_mission(&self) -> Result<usize, String> {
            Ok(self.current)
        }
        fn index(&self, mission: usize) -> Result<i32, String> {
            Ok(self.indexes[mission])
        }
        fn set_index(&mut self, mission: usize, index: i32) -> Result<(), String> {
            self.indexes[mission] = index;
            Ok(())
        }
        fn level(&self, _: usize) -> Result<i32, String> {
            Err("unexpected level read".into())
        }
        fn kind(&self, _: usize, _: i32) -> Result<u32, String> {
            Ok(self.kind)
        }
        fn entry_text(&self, mission: usize, index: i32) -> Result<Self::Text, String> {
            Ok((mission, index))
        }
        fn completion_text(
            &self,
            mission: usize,
            previous: i32,
        ) -> Result<Option<Self::Text>, String> {
            Ok(Some((mission, previous)))
        }
        fn world(&self) -> i32 {
            0
        }
        fn controlled_player_key(&self) -> Result<u32, String> {
            Ok(77)
        }
        fn progress(&self, _: i32, _: i32) -> Result<i32, String> {
            Err("unexpected progress read".into())
        }
        fn set_progress(&mut self, _: i32, _: i32, _: i32) -> Result<(), String> {
            Err("unexpected progress write".into())
        }
        fn set_required(&mut self, _: usize, _: i32) -> Result<(), String> {
            Err("unexpected count write".into())
        }
        fn set_completed(&mut self, _: usize, _: i32) -> Result<(), String> {
            Err("unexpected count write".into())
        }
        fn prepare_record(&mut self) -> Result<(), String> {
            self.calls.push("prepare");
            Ok(())
        }
        fn message(&mut self, text: Self::Text) -> Result<(), String> {
            self.calls.push("message");
            assert_eq!(text.0, 0);
            self.current = 1;
            if self.fail_message {
                Err("message transport unavailable".into())
            } else {
                Ok(())
            }
        }
        fn begin_countdown(&mut self) -> Result<(), String> {
            Err("unexpected countdown".into())
        }
        fn record_event(&mut self) -> Result<(), String> {
            Err("unexpected event".into())
        }
        fn reset_music(&mut self) -> Result<(), String> {
            self.calls.push("music");
            self.current = 1;
            Ok(())
        }
        fn resolve_selection(&mut self, text: Self::Text) -> Result<u32, String> {
            self.calls.push("resolve");
            assert_eq!(text, (1, 2));
            self.current = 0;
            Ok(42)
        }
        fn queue_selection(&mut self, selection: u32) -> Result<(), String> {
            self.calls.push("queue");
            assert_eq!(selection, 42);
            if self.fail_queue {
                Err("selection queue unavailable".into())
            } else {
                Ok(())
            }
        }
        fn commit_selection(&mut self) -> Result<(), String> {
            Err("unexpected commit".into())
        }
        fn prepare_next(&mut self) -> Result<(), String> {
            Err("unexpected preparation".into())
        }
        fn select_next(&mut self, _: i32, _: i32, _: u32) -> Result<(), String> {
            Err("unexpected selection".into())
        }
        fn finalize_progress(&mut self) -> Result<(), String> {
            Err("unexpected finalize".into())
        }
    }

    #[test]
    fn record_message_increments_the_replacement_only_after_success() {
        for fail in [false, true] {
            let mut p = Provider::new();
            p.fail_message = fail;
            let result = advance_record(&mut p);
            assert_eq!(result.is_err(), fail);
            assert_eq!(p.calls, ["prepare", "message"]);
            assert_eq!(p.current, 1);
            assert_eq!(p.indexes, [0, if fail { 2 } else { 3 }]);
            assert_eq!((p.c.notice_shown, p.c.first_visit), (0, 0));
        }
    }

    #[test]
    fn failed_expiry_notice_keeps_marker_without_advancing_records() {
        let mut p = Provider::new();
        p.indexes[0] = 1;
        p.c.notice_shown = 0;
        p.fail_message = true;
        assert!(after_expiry(&mut p).is_err());
        assert_eq!(p.calls, ["message"]);
        assert_eq!(p.c.notice_shown, 1);
        assert_eq!(p.c.first_visit, 9);
        assert_eq!(p.c.music_mode, 3);
        assert_eq!(p.indexes, [1, 2]);
        assert_eq!(p.current, 1);
    }

    #[test]
    fn failed_queue_keeps_music_player_and_post_resolve_record_writes() {
        let mut p = Provider::new();
        p.kind = 3;
        p.fail_queue = true;
        assert!(advance_record(&mut p).is_err());
        assert_eq!(p.calls, ["prepare", "music", "resolve", "queue"]);
        assert_eq!(p.c.music_mode, 100);
        assert_eq!(p.c.transition_timers, [0.; 2]);
        assert_eq!(p.c.selected_player, 77);
        assert_eq!(p.indexes, [27, 2]);
    }
}
