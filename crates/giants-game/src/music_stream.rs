//! In-memory music ring transport over an explicit track provider. No device,
//! render-clock advancement or implicit song selection is performed here.
use crate::{
    music_buffer::{self, Plan},
    music_schedule::{Next, Prepared, State},
};
use std::io::Read;

pub trait Tracks {
    type File: Read;
    fn open(&mut self, name: &str) -> Result<(Self::File, u32), String>;
}
pub struct Stream<P: Tracks> {
    graph: std::sync::Arc<Prepared>,
    schedule: State,
    provider: P,
    file: Option<P::File>,
    track: Option<String>,
    file_offset: u32,
    file_size: u32,
    write_offset: u32,
    started: bool,
    ring: Vec<u8>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Transfer {
    Idle,
    Wait,
    Stopped,
    Read {
        track: String,
        opened: bool,
        offset: u32,
        bytes: u32,
        received: u32,
        file_finished: bool,
        start_playback: bool,
        warning: Option<String>,
    },
}
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    Need(String),
    Open {
        message: String,
        stop_playback: bool,
    },
    Format {
        message: String,
        stop_playback: bool,
    },
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Need(message) | Self::Open { message, .. } | Self::Format { message, .. } => {
                f.write_str(message)
            }
        }
    }
}
impl std::error::Error for Failure {}
impl<P: Tracks> Stream<P> {
    pub fn new(
        graph: std::sync::Arc<Prepared>,
        provider: P,
        ring: Vec<u8>,
    ) -> Result<Self, &'static str> {
        if ring.len() != music_buffer::CAPACITY as usize {
            return Err("music ring length mismatch");
        }
        Ok(Self {
            graph,
            schedule: State::default(),
            provider,
            file: None,
            track: None,
            file_offset: 0,
            file_size: 0,
            write_offset: 0,
            started: false,
            ring,
        })
    }
    pub fn request(&mut self, name: &str) -> Result<bool, &'static str> {
        let idle = self.schedule.current == -1;
        let recognized = self.graph.request(&mut self.schedule, name)?;
        if idle && recognized {
            self.write_offset = 0;
        }
        Ok(recognized)
    }
    /// Logical stop request; the device adapter must separately apply Stop.
    pub fn stop(&mut self) {
        self.schedule.current = -1;
        self.schedule.pending = -1;
        self.schedule.segment = -1;
        self.file = None;
        self.write_offset = 0;
        self.started = false;
    }
    pub fn ring(&self) -> &[u8] {
        &self.ring
    }
    pub fn schedule(&self) -> State {
        self.schedule
    }
    /// Current source-visible transport fields, not an audible-playback claim.
    pub fn fields(&self) -> [u32; 5] {
        [
            u32::from(self.file.is_some()),
            self.file_offset,
            self.file_size,
            self.write_offset,
            u32::from(self.started),
        ]
    }
    pub fn refill(
        &mut self,
        play_cursor: u32,
        device_write_cursor: u32,
    ) -> Result<Transfer, Failure> {
        if self.schedule.current == -1 {
            return Ok(Transfer::Idle);
        }
        let mut opened = false;
        if self.file.is_none() {
            let mut next = self.schedule;
            let selected = self
                .graph
                .next(&mut next)
                .map_err(|e| Failure::Need(e.into()))?;
            self.schedule = next;
            let name = match selected {
                Next::Idle => return Ok(Transfer::Idle),
                Next::Stopped => {
                    self.stop();
                    return Ok(Transfer::Stopped);
                }
                Next::Track(name) => name.to_owned(),
            };
            let (file, size) = match self.provider.open(&name) {
                Ok(value) => value,
                Err(error) => {
                    let stop_playback = self.started;
                    self.stop();
                    return Err(Failure::Open {
                        message: format!("music open {name}: {error}"),
                        stop_playback,
                    });
                }
            };
            if size > 128 * 1024 * 1024 || !size.is_multiple_of(4) {
                let stop_playback = self.started;
                self.stop();
                return Err(Failure::Format {
                    message: "music track outside bounded stereo PCM profile".into(),
                    stop_playback,
                });
            }
            self.file = Some(file);
            self.track = Some(name);
            self.file_offset = 0;
            self.file_size = size;
            opened = true;
        }
        let plan = music_buffer::plan(music_buffer::Input {
            playing: self.started,
            write_offset: self.write_offset,
            play_cursor,
            device_write_cursor,
            file_offset: self.file_offset,
            file_size: self.file_size,
        })
        .map_err(|e| Failure::Need(e.into()))?;
        let Plan::Read {
            offset,
            bytes,
            next_offset,
            file_finished,
            start_playback,
        } = plan
        else {
            return Ok(Transfer::Wait);
        };
        let destination = &mut self.ring[offset as usize..(offset + bytes) as usize];
        let (received, warning) = match self
            .file
            .as_mut()
            .ok_or_else(|| Failure::Need("music file disappeared".into()))?
            .read(destination)
        {
            Ok(received) => (
                received,
                if received != bytes as usize {
                    Some(format!("music short read: {received}/{bytes}"))
                } else {
                    None
                },
            ),
            Err(error) => (0, Some(format!("music read failed: {error}"))),
        };
        // The original logs a failed/short read but advances its requested-byte
        // fields. Preserve that contract and expose the warning to the adapter.
        self.file_offset += bytes;
        self.write_offset = next_offset;
        if file_finished {
            self.file = None;
        }
        if start_playback {
            self.started = true;
        }
        Ok(Transfer::Read {
            track: self
                .track
                .clone()
                .ok_or_else(|| Failure::Need("music track absent".into()))?,
            opened,
            offset,
            bytes,
            received: received as u32,
            file_finished,
            start_playback,
            warning,
        })
    }
}
/// Explicit unique-candidate filesystem provider. Ambiguous names are rejected;
/// this does not infer general native search precedence from filename matches.
#[derive(Clone)]
pub struct Files {
    paths: std::sync::Arc<std::collections::BTreeMap<String, std::path::PathBuf>>,
}
impl Files {
    pub fn scan(root: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut paths = std::collections::BTreeMap::new();
        for folder in std::fs::read_dir(root.join("Music"))? {
            let folder = folder?.path();
            if !folder.is_dir() {
                continue;
            }
            for entry in std::fs::read_dir(folder)? {
                let path = entry?.path();
                if !path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("raw"))
                {
                    continue;
                }
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or("music name outside UTF8 profile")?
                    .to_ascii_lowercase();
                if paths.insert(name, path).is_some() {
                    return Err("ambiguous music filename".into());
                }
            }
        }
        Ok(Self {
            paths: std::sync::Arc::new(paths),
        })
    }
}
impl Tracks for Files {
    type File = std::fs::File;
    fn open(&mut self, name: &str) -> Result<(Self::File, u32), String> {
        let path = self
            .paths
            .get(&name.to_ascii_lowercase())
            .ok_or_else(|| format!("music candidate missing: {name}"))?;
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let size = u32::try_from(file.metadata().map_err(|e| e.to_string())?.len())
            .map_err(|_| "music size overflow")?;
        Ok((file, size))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Provider;
    #[test]
    fn open_failure_exposes_device_stop_and_preserves_buffer() {
        struct Large;
        impl Tracks for Large {
            type File = std::io::Cursor<Vec<u8>>;
            fn open(&mut self, name: &str) -> Result<(Self::File, u32), String> {
                if name == "missing" {
                    return Err("fixture missing".into());
                }
                Ok((std::io::Cursor::new(vec![7; 0x20000]), 0x20000))
            }
        }
        let graph = Prepared::new(giants_formats::music::Table {
            songs: vec![giants_formats::music::Song {
                source_offset: 0,
                name: "song".into(),
                entries: vec![
                    giants_formats::music::Entry::Track("large".into()),
                    giants_formats::music::Entry::Track("missing".into()),
                ],
            }],
            transitions: vec![],
            groups: vec![],
        })
        .unwrap();
        let mut stream =
            Stream::new(std::sync::Arc::new(graph), Large, vec![0xa5; 0x40000]).unwrap();
        stream.request("song").unwrap();
        stream.refill(0, 0).unwrap();
        assert_eq!(stream.fields()[4], 1);
        assert!(matches!(
            stream.refill(0, 0),
            Err(Failure::Open {
                stop_playback: true,
                ..
            })
        ));
        assert_eq!(stream.fields(), [0, 0x20000, 0x20000, 0, 0]);
        assert_eq!(stream.schedule().current, -1);
        assert!(stream.ring()[..0x20000].iter().all(|b| *b == 7));
    }
    #[test]
    fn unsupported_internal_route_does_not_commit_a_segment() {
        let graph = Prepared::new(giants_formats::music::Table {
            songs: vec![giants_formats::music::Song {
                source_offset: 0,
                name: "song".into(),
                entries: vec![giants_formats::music::Entry::Track("track".into())],
            }],
            transitions: vec![giants_formats::music::Link {
                from: "track".into(),
                to: "extra".into(),
            }],
            groups: vec![],
        })
        .unwrap();
        let mut stream =
            Stream::new(std::sync::Arc::new(graph), Provider, vec![0xa5; 0x40000]).unwrap();
        stream.request("song").unwrap();
        stream.refill(0, 0).unwrap();
        stream.request("*TransitionSong").unwrap();
        let before = stream.schedule();
        assert!(matches!(stream.refill(0, 0), Err(Failure::Need(_))));
        assert_eq!(stream.schedule(), before);
    }
    impl Tracks for Provider {
        type File = std::io::Cursor<Vec<u8>>;
        fn open(&mut self, name: &str) -> Result<(Self::File, u32), String> {
            if name == "missing" {
                return Err("missing fixture".into());
            }
            Ok((std::io::Cursor::new(vec![1, 2, 3, 4]), 8))
        }
    }
    #[test]
    fn short_read_keeps_unwritten_bytes_and_exposes_requested_advance() {
        let p = Prepared::new(giants_formats::music::Table {
            songs: vec![giants_formats::music::Song {
                source_offset: 0,
                name: "song".into(),
                entries: vec![
                    giants_formats::music::Entry::Track("track".into()),
                    giants_formats::music::Entry::Stop,
                ],
            }],
            transitions: vec![],
            groups: vec![],
        })
        .unwrap();
        let mut s = Stream::new(
            std::sync::Arc::new(p),
            Provider,
            vec![0xa5; music_buffer::CAPACITY as usize],
        )
        .unwrap();
        s.request("song").unwrap();
        let transfer = s.refill(0, 0).unwrap();
        assert!(matches!(
            transfer,
            Transfer::Read {
                bytes: 8,
                received: 4,
                warning: Some(_),
                ..
            }
        ));
        assert_eq!(&s.ring()[..8], &[1, 2, 3, 4, 0xa5, 0xa5, 0xa5, 0xa5]);
        assert_eq!(s.fields(), [0, 8, 8, 8, 0]);
        assert_eq!(s.refill(0, 0).unwrap(), Transfer::Stopped);
        assert_eq!(s.schedule().current, -1);
    }
}
