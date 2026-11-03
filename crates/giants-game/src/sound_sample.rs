//! Decoded sample preparation before a real audio device creates its handle.
//! The checked native profile is canonical PCM16 RIFF with fmt16 then data.
use giants_formats::wav::Pcm16;

pub struct Prepared {
    pub descriptor: [u32; 6],
    pub pcm: Pcm16,
}

impl Prepared {
    /// Scalar arguments after the context, sample identity and data pointer.
    /// No device or runtime instance is represented by this projection.
    pub fn creation_words(&self) -> [u32; 6] {
        [
            (self.pcm.samples.len() * 2) as u32,
            16,
            u32::from(self.pcm.channels),
            self.pcm.sample_rate,
            u32::from(self.pcm.channels) * 2,
            0,
        ]
    }
}

pub fn prepare(
    controls: u32,
    descriptor: [u32; 6],
    wave: &[u8],
) -> Result<Option<Prepared>, &'static str> {
    // The native disabled branch does not inspect the record or waveform.
    if controls & 1 == 0 {
        return Ok(None);
    }
    if wave.get(12..20) != Some(b"fmt \x10\0\0\0") || wave.get(36..40) != Some(b"data") {
        return Err("sound sample layout outside checked native preload profile");
    }
    let pcm = giants_formats::wav::parse(wave).map_err(|e| e.0)?;
    let data_bytes = u32::from_le_bytes(
        wave.get(40..44)
            .ok_or("truncated sample payload size")?
            .try_into()
            .unwrap(),
    ) as usize;
    if data_bytes != pcm.samples.len() * 2 || wave.len() != 44 + data_bytes {
        return Err("sound sample payload outside checked native preload profile");
    }
    Ok(Some(Prepared {
        descriptor: crate::sound_descriptor::prepare(descriptor)?,
        pcm,
    }))
}

#[cfg(test)]
mod tests {
    use super::prepare;

    fn wave() -> Vec<u8> {
        let mut wave = b"RIFF".to_vec();
        wave.extend(44u32.to_le_bytes());
        wave.extend(b"WAVEfmt ");
        wave.extend(16u32.to_le_bytes());
        wave.extend(1u16.to_le_bytes());
        wave.extend(2u16.to_le_bytes());
        wave.extend(22050u32.to_le_bytes());
        wave.extend(88200u32.to_le_bytes());
        wave.extend(4u16.to_le_bytes());
        wave.extend(16u16.to_le_bytes());
        wave.extend(b"data");
        wave.extend(8u32.to_le_bytes());
        for sample in [i16::MIN, 7, -9, i16::MAX] {
            wave.extend(sample.to_le_bytes());
        }
        wave
    }

    #[test]
    fn disabled_preload_does_not_admit_invalid_data_or_claim_a_handle() {
        for controls in [0, 2, 0xfffffffe] {
            assert!(prepare(controls, [u32::MAX; 6], &[]).unwrap().is_none());
        }
        let raw = wave();
        let prepared = prepare(3, [0; 6], &raw).unwrap().unwrap();
        assert_eq!(prepared.creation_words(), [8, 16, 2, 22050, 4, 0]);
        assert_eq!(prepared.pcm.samples, [i16::MIN, 7, -9, i16::MAX]);
        assert_eq!(
            prepared.descriptor[4..],
            [10f32.to_bits(), 150f32.to_bits()]
        );
        for end in 0..raw.len() {
            assert!(prepare(1, [0; 6], &raw[..end]).is_err());
        }
        let mut extra = raw.clone();
        extra.splice(36..36, *b"JUNK\0\0\0\0");
        let size = extra.len() as u32 - 8;
        extra[4..8].copy_from_slice(&size.to_le_bytes());
        assert!(giants_formats::wav::parse(&extra).is_ok());
        assert!(prepare(1, [0; 6], &extra).is_err());
    }
}
