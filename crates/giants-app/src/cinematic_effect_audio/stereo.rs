//! Documented DirectSound gain/pan interpretation, not a driver mixer clone.
use bevy::{
    audio::{ChannelCount, Decodable, SampleRate, Source},
    prelude::*,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

#[derive(Clone, Debug)]
pub(super) struct Controls(Arc<AtomicU64>);
impl Controls {
    pub fn new(volume: i32, pan: i32) -> Result<Self, &'static str> {
        let value = Self(Arc::new(AtomicU64::new(0)));
        value.set(volume, pan)?;
        Ok(value)
    }
    pub fn set(&self, volume: i32, pan: i32) -> Result<(), &'static str> {
        if !(-10000..=0).contains(&volume) || !(-10000..=10000).contains(&pan) {
            return Err("sound device control outside documented range");
        }
        let left = 10f64.powf(f64::from(volume - pan.max(0)) / 2000.) as f32;
        let right = 10f64.powf(f64::from(volume + pan.min(0)) / 2000.) as f32;
        self.0.store(
            u64::from(left.to_bits()) | (u64::from(right.to_bits()) << 32),
            Ordering::Release,
        );
        Ok(())
    }
    fn gains(&self) -> [f32; 2] {
        let value = self.0.load(Ordering::Acquire);
        [
            f32::from_bits(value as u32),
            f32::from_bits((value >> 32) as u32),
        ]
    }
}
#[derive(Asset, TypePath, Clone, Debug)]
pub(super) struct StereoSource {
    pub samples: Arc<[i16]>,
    pub channels: u16,
    pub rate: u32,
    pub controls: Controls,
}
impl StereoSource {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !matches!(self.channels, 1 | 2)
            || self.rate == 0
            || self.samples.is_empty()
            || !self
                .samples
                .len()
                .is_multiple_of(usize::from(self.channels))
        {
            return Err("sound PCM outside complete mono/stereo profile");
        }
        Ok(())
    }
}
pub(super) struct Decoder {
    source: StereoSource,
    cursor: usize,
    gains: [f32; 2],
}
impl Decodable for StereoSource {
    type Decoder = Decoder;
    fn decoder(&self) -> Decoder {
        self.validate().expect("prepared complete PCM source");
        Decoder {
            source: self.clone(),
            cursor: 0,
            gains: self.controls.gains(),
        }
    }
}
impl Iterator for Decoder {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let frame = self.cursor / 2;
        let channel = self.cursor % 2;
        let input = frame * usize::from(self.source.channels)
            + if self.source.channels == 1 {
                0
            } else {
                channel
            };
        let sample = *self.source.samples.get(input)?;
        if channel == 0 {
            self.gains = self.source.controls.gains();
        }
        self.cursor += 1;
        Some(f32::from(sample) / 32768. * self.gains[channel])
    }
}
impl Source for Decoder {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.source.samples.len() / usize::from(self.source.channels) * 2 - self.cursor)
    }
    fn channels(&self) -> ChannelCount {
        ChannelCount::new(2).unwrap()
    }
    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(self.source.rate).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            (self.source.samples.len() / usize::from(self.source.channels)) as f64
                / f64::from(self.source.rate),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stereo_frame_updates_are_coherent_and_invalid_controls_preserve_previous_state() {
        let controls = Controls::new(0, 0).unwrap();
        let source = StereoSource {
            samples: Arc::from([16384, 16384, 16384]),
            channels: 1,
            rate: 22050,
            controls: controls.clone(),
        };
        let mut decoder = source.decoder();
        assert_eq!(decoder.next(), Some(0.5));
        controls.set(-2000, 6000).unwrap();
        assert_eq!(decoder.next(), Some(0.5)); // Remaining right sample of old frame.
        let left = decoder.next().unwrap();
        let right = decoder.next().unwrap();
        assert!((left - 0.00005).abs() < 1e-10);
        assert!((right - 0.05).abs() < 1e-8);
        assert!(controls.set(-10001, 0).is_err());
        assert_eq!(decoder.next(), Some(left));
        assert_eq!(decoder.next(), Some(right));
        assert_eq!(decoder.next(), None);
        assert_eq!(decoder.current_span_len(), Some(0));
    }
    #[test]
    fn stereo_preserves_source_channels_and_duration_without_creating_a_device() {
        let source = StereoSource {
            samples: Arc::from([i16::MIN, i16::MAX, 8192, -8192]),
            channels: 2,
            rate: 2,
            controls: Controls::new(0, 0).unwrap(),
        };
        let decoder = source.decoder();
        assert_eq!(decoder.total_duration(), Some(Duration::from_secs(1)));
        assert_eq!(
            decoder.collect::<Vec<_>>(),
            [-1., 32767. / 32768., 0.25, -0.25]
        );
    }
}
