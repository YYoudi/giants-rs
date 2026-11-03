//! Bounded RIFF/WAVE PCM16 reader. Other codecs are rejected explicitly.
use crate::{Error, slice, u32_at};
pub struct Pcm16 {
    pub channels: u16,
    pub sample_rate: u32,
    pub samples: Vec<i16>,
}
fn u16_at(data: &[u8], at: usize) -> Result<u16, Error> {
    Ok(u16::from_le_bytes(slice(data, at, 2)?.try_into().unwrap()))
}
pub fn parse(data: &[u8]) -> Result<Pcm16, Error> {
    if data.len() > 128 * 1024 * 1024
        || slice(data, 0, 4)? != b"RIFF"
        || slice(data, 8, 4)? != b"WAVE"
        || u32_at(data, 4)? as usize + 8 != data.len()
    {
        return Err(Error("invalid PCM RIFF framing"));
    }
    let mut at = 12;
    let mut format = None;
    let mut payload = None;
    while at < data.len() {
        let tag = slice(data, at, 4)?;
        let size = u32_at(data, at + 4)? as usize;
        let chunk = slice(data, at + 8, size)?;
        match tag {
            b"fmt " => {
                if format.is_some()
                    || size != 16
                    || u16_at(chunk, 0)? != 1
                    || u16_at(chunk, 14)? != 16
                {
                    return Err(Error("unsupported PCM format"));
                }
                let channels = u16_at(chunk, 2)?;
                let rate = u32_at(chunk, 4)?;
                if !(1..=2).contains(&channels)
                    || !(8000..=192000).contains(&rate)
                    || u16_at(chunk, 12)? != channels * 2
                    || u32_at(chunk, 8)? != rate * u32::from(channels) * 2
                {
                    return Err(Error("inconsistent PCM format"));
                }
                format = Some((channels, rate));
            }
            b"data" => {
                if payload.is_some() || size == 0 {
                    return Err(Error("duplicate or empty PCM data"));
                }
                payload = Some(chunk);
            }
            _ => {}
        }
        at += 8 + size;
        if !size.is_multiple_of(2) {
            slice(data, at, 1)?;
            at += 1;
        }
    }
    let (channels, sample_rate) = format.ok_or(Error("missing PCM format"))?;
    let payload = payload.ok_or(Error("missing PCM data"))?;
    if !payload.len().is_multiple_of(usize::from(channels) * 2) {
        return Err(Error("incomplete PCM frame"));
    }
    let samples = payload
        .as_chunks::<2>()
        .0
        .iter()
        .map(|v| i16::from_le_bytes(*v))
        .collect();
    Ok(Pcm16 {
        channels,
        sample_rate,
        samples,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pcm_fields_samples_and_truncations() {
        let mut b = b"RIFF".to_vec();
        b.extend(42u32.to_le_bytes());
        b.extend(b"WAVEfmt ");
        b.extend(16u32.to_le_bytes());
        for x in [1u16, 1] {
            b.extend(x.to_le_bytes());
        }
        b.extend(22050u32.to_le_bytes());
        b.extend(44100u32.to_le_bytes());
        b.extend(2u16.to_le_bytes());
        b.extend(16u16.to_le_bytes());
        b.extend(b"data");
        b.extend(6u32.to_le_bytes());
        for x in [i16::MIN, 0, i16::MAX] {
            b.extend(x.to_le_bytes());
        }
        let p = parse(&b).unwrap();
        assert_eq!(p.sample_rate, 22050);
        assert_eq!(p.channels, 1);
        assert_eq!(p.samples, [i16::MIN, 0, i16::MAX]);
        for end in 0..b.len() {
            assert!(parse(&b[..end]).is_err());
        }
        for (at, value) in [(20, 3), (22, 0), (32, 1), (34, 8)] {
            let mut x = b.clone();
            x[at..at + 2].copy_from_slice(&u16::to_le_bytes(value));
            assert!(parse(&x).is_err());
        }
    }
}
