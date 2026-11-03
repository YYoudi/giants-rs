//! Music graph preparation and segment-boundary scheduling. No audio device,
//! buffering, wall-clock cadence or mission-driven request selection.
use giants_formats::music::{Entry, Song, Table};

#[derive(Debug)]
pub struct Prepared {
    pub songs: Vec<Song>,
    pub transitions: Vec<[i32; 4]>,
    pub groups: Vec<Vec<usize>>,
    pub routes: Vec<Vec<i32>>,
    /// Some native initialization bytes are left untouched. Unknown cells stay
    /// None unless the caller supplies their actual backing storage explicitly.
    pub internal_targets: Vec<Option<i32>>,
}
fn locate(songs: &[Song], name: &str) -> Option<[i32; 2]> {
    for (song, data) in songs.iter().enumerate() {
        for (segment, entry) in data.entries.iter().enumerate() {
            match entry {
                Entry::Track(track) if track.eq_ignore_ascii_case(name) => {
                    return Some([song as i32, segment as i32]);
                }
                Entry::Track(_) => {}
                _ => break,
            }
        }
    }
    None
}
impl Prepared {
    pub fn new(table: Table) -> Result<Self, &'static str> {
        Self::with_internal_storage(table, None)
    }
    pub fn with_internal_storage(
        mut table: Table,
        storage: Option<&[u8]>,
    ) -> Result<Self, &'static str> {
        let n = table.songs.len() + 1;
        if !(2..=500).contains(&n) {
            return Err("music graph count outside supported profile");
        }
        let extra: Vec<_> = table
            .transition_tracks()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut transitions: Vec<_> = table
            .transitions
            .iter()
            .map(|link| {
                let from = locate(&table.songs, &link.from).unwrap_or([-1, -1]);
                let to = locate(&table.songs, &link.to).unwrap_or([-1, -1]);
                [from[0], from[1], to[0], to[1]]
            })
            .collect();
        table.songs.push(Song {
            source_offset: usize::MAX,
            name: "*TransitionSong".into(),
            entries: extra.into_iter().map(Entry::Track).collect(),
        });
        let internal = n - 1;
        let mut segment = 0;
        for (transition, link) in transitions.iter_mut().zip(&table.transitions) {
            if transition[2] == -1 {
                transition[2] = internal as i32;
                transition[3] = segment;
                segment += 1;
            }
            if transition[0] == -1 {
                let from =
                    locate(&table.songs, &link.from).ok_or("music transition source unresolved")?;
                transition[0] = from[0];
                transition[1] = from[1];
            }
        }
        let cells = segment as usize;
        if storage.is_some_and(|bytes| bytes.len() != cells * 4) {
            return Err("music internal backing storage length mismatch");
        }
        let mut internal_targets = Vec::with_capacity(cells);
        for index in 0..cells {
            let mut bytes = [0u8; 4];
            let mut known = true;
            for (byte, value) in bytes.iter_mut().enumerate() {
                let offset = index * 4 + byte;
                if offset < cells {
                    *value = 0xff;
                } else if let Some(storage) = storage {
                    *value = storage[offset];
                } else {
                    known = false;
                }
            }
            internal_targets.push(known.then(|| i32::from_le_bytes(bytes)));
        }
        let mut routes = vec![vec![-1; n]; n];
        for &[from, at, to, _] in &transitions {
            if from as usize == internal {
                internal_targets[at as usize] = Some(to);
            }
            if from as usize != internal && to as usize != internal {
                routes[from as usize][to as usize] = to;
            }
        }
        // Ordered, in-place closure. Keep the last qualifying intermediate in
        // a previously unknown cell; this is not a shortest-path algorithm.
        loop {
            let mut changed = false;
            for from in 0..n {
                for to in 0..n {
                    if routes[from][to] != -1 {
                        continue;
                    }
                    for via in 0..n {
                        if routes[from][via] != -1 && routes[via][to] != -1 {
                            routes[from][to] = routes[from][via];
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let mut result = Self {
            songs: table.songs,
            transitions,
            groups: vec![vec![]; n],
            routes,
            internal_targets,
        };
        for group in table.groups {
            let from = result
                .song(&group.from)
                .ok_or("music group source unresolved")?;
            let to = result
                .song(&group.to)
                .ok_or("music group member unresolved")?;
            result.groups[from].push(to);
        }
        Ok(result)
    }
    pub fn song(&self, name: &str) -> Option<usize> {
        self.songs
            .iter()
            .position(|song| song.name.eq_ignore_ascii_case(name))
    }
    fn validate(&self, state: State) -> Result<(), &'static str> {
        if [state.current, state.pending]
            .iter()
            .any(|v| *v < -1 || *v >= self.songs.len() as i32)
            || !(-1..=8192).contains(&state.segment)
        {
            return Err("music state outside prepared scheduling profile");
        }
        Ok(())
    }
    /// Choose a requested group member or song at a source transition point.
    pub fn choose(&self, state: State, desired: usize) -> Result<Option<usize>, &'static str> {
        self.validate(state)?;
        if state.current < 0 || desired >= self.songs.len() {
            return Err("music transition endpoint absent");
        }
        let mut from = state.current as usize;
        if self.groups[from].is_empty()
            && let Some(group) = self
                .groups
                .iter()
                .position(|members| members.contains(&from))
        {
            from = group;
        }
        let mut best = None;
        for &[source, segment, target, _] in &self.transitions {
            if source != from as i32
                || !(target == desired as i32 || self.groups[desired].contains(&(target as usize)))
            {
                continue;
            }
            let delta = segment - state.segment;
            let replace = match best {
                None => true,
                Some((old, _)) => {
                    if delta >= 0 {
                        old < 0 || delta < old
                    } else {
                        old < 0 && delta < old
                    }
                }
            };
            if replace {
                best = Some((delta, target as usize));
            }
        }
        Ok(best.map(|(_, song)| song))
    }
    /// Unknown requests leave scheduling unchanged; no playback success implied.
    pub fn request(&self, state: &mut State, name: &str) -> Result<bool, &'static str> {
        self.validate(*state)?;
        let Some(song) = self.song(name) else {
            return Ok(false);
        };
        if state.current == -1 {
            state.current = song as i32;
            state.pending = -1;
            state.segment = -1;
        } else {
            state.pending = song as i32;
        }
        Ok(true)
    }
    /// Advance only when the caller reaches a real segment boundary. This does
    /// not predict that boundary from nominal track duration or a render tick.
    pub fn next<'a>(&'a self, state: &mut State) -> Result<Next<'a>, &'static str> {
        self.validate(*state)?;
        if state.current == -1 {
            return Ok(Next::Idle);
        }
        if state.pending != -1 {
            if state.pending == state.current {
                state.pending = -1;
            } else {
                state.pending = self
                    .choose(*state, state.pending as usize)?
                    .map_or(-1, |s| s as i32);
                if state.pending != -1 {
                    let internal = self.songs.len() - 1;
                    let destination = if state.current as usize == internal {
                        state.routed_target
                    } else {
                        let target = self.routes[state.current as usize][state.pending as usize];
                        state.routed_target = target;
                        target
                    };
                    for &[from, at, to, segment] in &self.transitions {
                        if from != state.current || at != state.segment {
                            continue;
                        }
                        let indirect = if to != destination && to as usize == internal {
                            self.internal_targets[segment as usize]
                                .ok_or("music internal route depends on unwritten backing bytes")?
                                == destination
                        } else {
                            false
                        };
                        if to == destination {
                            state.current = to;
                            state.segment = segment - 1;
                            if to == state.pending {
                                state.pending = -1;
                            }
                            break;
                        } else if indirect {
                            state.current = internal as i32;
                            state.routed_target = destination;
                            state.segment = segment - 1;
                            break;
                        }
                    }
                }
            }
        }
        state.segment += 1;
        let mut song = &self.songs[state.current as usize];
        let limit = song
            .entries
            .iter()
            .take_while(|e| matches!(e, Entry::Track(_)))
            .count();
        let position = (state.segment as usize).min(limit);
        let entry = if position == song.entries.len() {
            state.segment = 0;
            song.entries.first()
        } else {
            song.entries.get(position)
        };
        match entry {
            Some(Entry::Stop) => {
                state.current = -1;
                state.pending = -1;
                state.segment = -1;
                Ok(Next::Stopped)
            }
            Some(Entry::Jump(name)) => {
                let target = self.song(name).ok_or("music jump target unresolved")?;
                state.current = target as i32;
                state.segment = 0;
                song = &self.songs[target];
                match song.entries.first() {
                    Some(Entry::Track(track)) => Ok(Next::Track(track)),
                    _ => Err("music jump starts outside track profile"),
                }
            }
            Some(Entry::Track(track)) => Ok(Next::Track(track)),
            None => Err("empty music song has no track"),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub current: i32,
    pub pending: i32,
    pub segment: i32,
    pub routed_target: i32,
}
impl Default for State {
    fn default() -> Self {
        Self {
            current: -1,
            pending: -1,
            segment: -1,
            routed_target: 0,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum Next<'a> {
    Idle,
    Stopped,
    Track(&'a str),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unwritten_internal_storage_requires_explicit_input() {
        let table = || Table {
            songs: vec![song("ordinary", vec![Entry::Track("base".into())])],
            transitions: vec![giants_formats::music::Link {
                from: "base".into(),
                to: "extra".into(),
            }],
            groups: vec![],
        };
        let unknown = Prepared::new(table()).unwrap();
        assert_eq!(unknown.internal_targets, [None]);
        let supplied = Prepared::with_internal_storage(table(), Some(&[0x5a; 4])).unwrap();
        assert_eq!(supplied.internal_targets, [Some(0x5a5a5aff)]);
        assert!(Prepared::with_internal_storage(table(), Some(&[0; 3])).is_err());
        let mut state = State {
            current: 0,
            pending: 1,
            segment: 0,
            routed_target: 0,
        };
        assert!(unknown.next(&mut state).is_err());
    }
    fn song(name: &str, entries: Vec<Entry>) -> Song {
        Song {
            source_offset: 0,
            name: name.into(),
            entries,
        }
    }
    #[test]
    fn implicit_wrap_explicit_stop_jump_and_unknown_request() {
        let p = Prepared::new(Table {
            songs: vec![
                song("loop", vec![Entry::Track("one".into())]),
                song("stop", vec![Entry::Track("two".into()), Entry::Stop]),
                song(
                    "jump",
                    vec![Entry::Track("three".into()), Entry::Jump("stop".into())],
                ),
            ],
            transitions: vec![],
            groups: vec![],
        })
        .unwrap();
        let mut s = State::default();
        assert_eq!(p.next(&mut s), Ok(Next::Idle));
        assert!(p.request(&mut s, "LOOP").unwrap());
        assert_eq!(p.next(&mut s), Ok(Next::Track("one")));
        assert_eq!(p.next(&mut s), Ok(Next::Track("one")));
        assert_eq!(s.segment, 0);
        let before = s;
        assert!(!p.request(&mut s, "missing").unwrap());
        assert_eq!(s, before);
        s = State::default();
        p.request(&mut s, "jump").unwrap();
        assert_eq!(p.next(&mut s), Ok(Next::Track("three")));
        assert_eq!(p.next(&mut s), Ok(Next::Track("two")));
        assert_eq!(s.current, 1);
        assert_eq!(p.next(&mut s), Ok(Next::Stopped));
        assert_eq!(s.current, -1);
    }
}
