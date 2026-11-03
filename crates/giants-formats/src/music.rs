//! Music table declarations and the measured headerless PCM upload profile.
//! Transition policy, device buffering and mission selection are separate.
use crate::{Error, slice, u32_at};

#[derive(Debug, PartialEq, Eq)]
pub enum Entry {
    Track(String),
    Stop,
    Jump(String),
}
#[derive(Debug)]
pub struct Song {
    pub source_offset: usize,
    pub name: String,
    pub entries: Vec<Entry>,
}
impl Song {
    /// Packed runtime representation produced by the measured table loader.
    pub fn packed_entries(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for entry in &self.entries {
            match entry {
                Entry::Stop => out.push(1),
                Entry::Jump(name) => {
                    out.push(2);
                    out.extend(name.as_bytes());
                    out.push(0);
                }
                Entry::Track(name) => {
                    out.extend(name.as_bytes());
                    out.push(0);
                }
            }
        }
        out.push(0);
        out
    }
}
#[derive(Debug)]
pub struct Link {
    pub from: String,
    pub to: String,
}
#[derive(Debug)]
pub struct Table {
    /// Source order and repeated offsets/names are significant; no deduplication.
    pub songs: Vec<Song>,
    pub transitions: Vec<Link>,
    pub groups: Vec<Link>,
}
impl Table {
    /// Transition destinations absent from ordinary packed song prefixes become
    /// tracks of the native loader's extra internal song, in declaration order.
    /// Preserve repeated destinations rather than silently deduplicating them.
    pub fn transition_tracks(&self) -> Vec<&str> {
        self.transitions
            .iter()
            .filter(|link| {
                !self.songs.iter().any(|song| {
                    song.entries
                        .iter()
                        .take_while(|entry| matches!(entry, Entry::Track(_)))
                        .any(|entry| matches!(entry, Entry::Track(name) if name.eq_ignore_ascii_case(&link.to)))
                })
            })
            .map(|link| link.to.as_str())
            .collect()
    }
}
fn name(data: &[u8], at: usize) -> Result<String, Error> {
    let field = slice(data, at, 16)?;
    let end = field
        .iter()
        .position(|b| *b == 0)
        .ok_or(Error("unterminated music name"))?;
    if end == 0
        || !field[..end]
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
    {
        return Err(Error("music name outside measured identifier profile"));
    }
    Ok(String::from_utf8(field[..end].to_vec()).unwrap())
}
pub fn parse(data: &[u8]) -> Result<Table, Error> {
    if data.len() > 4 * 1024 * 1024 {
        return Err(Error("music table budget"));
    }
    let songs = u32_at(data, 0)? as usize;
    let transitions = u32_at(data, 4)? as usize;
    let groups = u32_at(data, 8)? as usize;
    if songs == 0 || songs >= 500 || transitions > 16384 || groups > 16384 {
        return Err(Error("music table counts outside supported budgets"));
    }
    let pool = 12 + songs * 4 + (transitions + groups) * 32;
    slice(data, 0, pool)?;
    let mut result = Vec::with_capacity(songs);
    for index in 0..songs {
        let source_offset = u32_at(data, 12 + index * 4)? as usize;
        if source_offset < pool {
            return Err(Error("music song points into table headers"));
        }
        let song_name = name(data, source_offset)?;
        let mut at = source_offset + 16;
        let mut entries = Vec::new();
        loop {
            let opcode = slice(data, at, 1)?[0];
            if opcode == 0 {
                break;
            }
            if entries.len() >= 8192 {
                return Err(Error("music song entry budget"));
            }
            entries.push(match opcode {
                1 => {
                    at += 1;
                    Entry::Stop
                }
                2 => {
                    let target = name(data, at + 1)?;
                    at += 17;
                    Entry::Jump(target)
                }
                _ => {
                    let track = name(data, at)?;
                    at += 16;
                    Entry::Track(track)
                }
            });
        }
        result.push(Song {
            source_offset,
            name: song_name,
            entries,
        });
    }
    let mut links = Vec::with_capacity(transitions + groups);
    for index in 0..transitions + groups {
        let at = 12 + songs * 4 + index * 32;
        links.push(Link {
            from: name(data, at)?,
            to: name(data, at + 16)?,
        });
    }
    let group_links = links.split_off(transitions);
    Ok(Table {
        songs: result,
        transitions: links,
        groups: group_links,
    })
}

/// Native MDVInit's PCM format, not a guess from a RAW filename or its contents.
pub const CHANNELS: u16 = 2;
pub const SAMPLE_RATE: u32 = 22050;
pub const FRAME_BYTES: usize = 4;
pub fn parse_raw(data: &[u8]) -> Result<crate::wav::Pcm16, Error> {
    if data.is_empty() || data.len() > 128 * 1024 * 1024 || !data.len().is_multiple_of(FRAME_BYTES)
    {
        return Err(Error("music PCM outside complete stereo frame profile"));
    }
    Ok(crate::wav::Pcm16 {
        channels: CHANNELS,
        sample_rate: SAMPLE_RATE,
        samples: data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes(*b))
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixed(s: &[u8]) -> Vec<u8> {
        let mut b = vec![0; 16];
        b[..s.len()].copy_from_slice(s);
        b
    }
    #[test]
    fn ordered_duplicate_songs_control_tokens_and_bounds() {
        let mut b = Vec::new();
        for w in [2u32, 1, 1, 84, 84] {
            b.extend(w.to_le_bytes());
        }
        for s in [b"segment".as_slice(), b"next", b"song", b"group"] {
            b.extend(fixed(s));
        }
        b.extend(fixed(b"song"));
        b.extend(fixed(b"segment"));
        b.push(2);
        b.extend(fixed(b"next"));
        b.extend([1, 0]);
        let t = parse(&b).unwrap();
        assert_eq!(t.songs.len(), 2);
        assert_eq!(t.songs[0].source_offset, t.songs[1].source_offset);
        assert_eq!(
            t.songs[0].entries,
            vec![
                Entry::Track("segment".into()),
                Entry::Jump("next".into()),
                Entry::Stop
            ]
        );
        assert_eq!(t.songs[0].packed_entries(), b"segment\0\x02next\0\x01\0");
        assert_eq!(t.transitions[0].from, "segment");
        assert_eq!(t.transition_tracks(), ["next"]);
        assert_eq!(t.groups[0].to, "group");
        assert!(parse(&b[..b.len() - 1]).is_err());
        b[12..16].copy_from_slice(&12u32.to_le_bytes());
        assert!(parse(&b).is_err());
    }
    #[test]
    fn raw_preserves_signed_interleaved_samples_and_rejects_partial_frames() {
        let bytes = [0, 128, 255, 127, 255, 255, 1, 0];
        let p = parse_raw(&bytes).unwrap();
        assert_eq!((p.channels, p.sample_rate), (2, 22050));
        assert_eq!(p.samples, [i16::MIN, i16::MAX, -1, 1]);
        assert!(parse_raw(&[]).is_err());
        assert!(parse_raw(&bytes[..6]).is_err());
    }
}
