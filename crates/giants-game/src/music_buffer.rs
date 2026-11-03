//! Native music refill decisions. Device cursors and open-file state are inputs;
//! this neither opens a track nor reports audio-device success.
pub const CAPACITY: u32 = 0x40000;
pub const MAX_READ: u32 = 0x20000;
pub const BATCH_MIN: u32 = 0x10000;
#[derive(Debug, Clone, Copy)]
pub struct Input {
    pub playing: bool,
    pub write_offset: u32,
    pub play_cursor: u32,
    pub device_write_cursor: u32,
    pub file_offset: u32,
    pub file_size: u32,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Plan {
    Wait,
    Read {
        offset: u32,
        bytes: u32,
        next_offset: u32,
        file_finished: bool,
        start_playback: bool,
    },
}
pub fn plan(input: Input) -> Result<Plan, &'static str> {
    if input.write_offset >= CAPACITY
        || input.file_offset > input.file_size
        || input.file_size > 128 * 1024 * 1024
        || (input.playing
            && (input.play_cursor >= CAPACITY || input.device_write_cursor >= CAPACITY))
    {
        return Err("music refill outside bounded cursor/file profile");
    }
    let play = if input.playing {
        input.play_cursor
    } else {
        CAPACITY
    };
    let device_write = if input.playing {
        input.device_write_cursor
    } else {
        CAPACITY
    };
    let mut available = if play >= input.write_offset {
        play - input.write_offset
    } else {
        CAPACITY - input.write_offset
    };
    if play < input.write_offset && device_write > input.write_offset {
        return Ok(Plan::Wait);
    }
    if available < BATCH_MIN
        && input.file_offset + available < input.file_size
        && input.write_offset + available < CAPACITY
    {
        return Ok(Plan::Wait);
    }
    available = available
        .min(MAX_READ)
        .min(input.file_size - input.file_offset);
    let end = input.write_offset + available;
    let next_offset = if end >= CAPACITY { 0 } else { end };
    Ok(Plan::Read {
        offset: input.write_offset,
        bytes: available,
        next_offset,
        file_finished: input.file_offset + available >= input.file_size,
        start_playback: !input.playing && next_offset >= MAX_READ,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_batch_end_of_file_and_device_write_guard() {
        let mut i = Input {
            playing: false,
            write_offset: 0,
            play_cursor: 0,
            device_write_cursor: 0,
            file_offset: 0,
            file_size: 300000,
        };
        assert_eq!(
            plan(i),
            Ok(Plan::Read {
                offset: 0,
                bytes: MAX_READ,
                next_offset: MAX_READ,
                file_finished: false,
                start_playback: true
            })
        );
        i.playing = true;
        i.write_offset = 100000;
        i.play_cursor = 100001;
        i.device_write_cursor = 100002;
        assert_eq!(plan(i), Ok(Plan::Wait));
        i.file_offset = i.file_size - 1;
        assert_eq!(
            plan(i),
            Ok(Plan::Read {
                offset: 100000,
                bytes: 1,
                next_offset: 100001,
                file_finished: true,
                start_playback: false
            })
        );
        i.play_cursor = 99999;
        i.device_write_cursor = 100001;
        assert_eq!(plan(i), Ok(Plan::Wait));
        i.file_offset = i.file_size + 1;
        assert!(plan(i).is_err());
    }
}
