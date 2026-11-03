//! Pending numeric sound requests. No handle, playback or stop is acknowledged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub command_offset: usize,
    pub opcode: u32,
    pub actor: Option<usize>,
    pub sound_id: u32,
}
impl Request {
    pub fn decode(c: &giants_formats::flick::Command) -> Result<Option<Self>, &'static str> {
        let (actor, sound_id) = match (c.opcode, c.words.as_slice()) {
            (13, [slot, _]) if (*slot as i32) < 0 => return Ok(None),
            (13, [slot, sound]) if *slot < 32 => (Some(*slot as usize), *sound),
            (27, [sound]) if (*sound as i32) < 0 => return Ok(None),
            (27, [sound]) => (None, *sound),
            (13 | 27, _) => return Err("numeric sound request outside operand profile"),
            _ => return Ok(None),
        };
        if sound_id >= 2000 {
            return Err("numeric sound outside table capacity");
        }
        Ok(Some(Self {
            command_offset: c.offset,
            opcode: c.opcode,
            actor,
            sound_id,
        }))
    }
}
