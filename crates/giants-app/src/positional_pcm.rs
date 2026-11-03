//! Stereo adapter for the measured software pan, with no second spatial law.
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

#[derive(Clone)]
pub struct Pan(Arc<AtomicU64>);
impl Default for Pan {
    fn default() -> Self {
        Self(Arc::new(AtomicU64::new(pack([1., 1.]))))
    }
}
fn pack(g: [f32; 2]) -> u64 {
    u64::from(g[0].to_bits()) | (u64::from(g[1].to_bits()) << 32)
}
impl Pan {
    /// DirectSound SetPan uses hundredths of a decibel and leaves the opposite
    /// channel at unity. This adapter applies that API convention to PCM.
    pub fn set(&self, units: i32) -> Result<(), String> {
        if !(-6000..=6000).contains(&units) {
            return Err("pan outside measured interval".into());
        }
        let attenuation = 10f64.powf(-f64::from(units.abs()) / 2000.) as f32;
        self.0.store(
            pack(if units > 0 {
                [attenuation, 1.]
            } else {
                [1., attenuation]
            }),
            Ordering::Relaxed,
        );
        Ok(())
    }
    fn get(&self) -> [f32; 2] {
        let g = self.0.load(Ordering::Relaxed);
        [f32::from_bits(g as u32), f32::from_bits((g >> 32) as u32)]
    }
}
#[derive(Asset, TypePath, Clone)]
pub struct Pcm {
    samples: Arc<[i16]>,
    channels: u16,
    rate: u32,
    pub pan: Pan,
}
impl Pcm {
    pub fn new(samples: Arc<[i16]>, channels: u16, rate: u32) -> Result<Self, String> {
        if !(1..=2).contains(&channels)
            || !(8000..=192000).contains(&rate)
            || samples.is_empty()
            || !samples.len().is_multiple_of(usize::from(channels))
        {
            return Err("invalid positional PCM".into());
        }
        Ok(Self {
            samples,
            channels,
            rate,
            pan: Pan::default(),
        })
    }
}
pub struct Decoder {
    source: Pcm,
    at: usize,
    gains: [f32; 2],
}
impl Iterator for Decoder {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let frame = self.at / 2;
        if frame >= self.source.samples.len() / usize::from(self.source.channels) {
            return None;
        }
        let channel = self.at % 2;
        if channel == 0 {
            self.gains = self.source.pan.get();
        }
        let index = frame * usize::from(self.source.channels)
            + if self.source.channels == 2 {
                channel
            } else {
                0
            };
        self.at += 1;
        Some(f32::from(self.source.samples[index]) / 32768. * self.gains[channel])
    }
}
impl Source for Decoder {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.source.samples.len() / usize::from(self.source.channels) * 2 - self.at)
    }
    fn channels(&self) -> ChannelCount {
        ChannelCount::new(2).unwrap()
    }
    fn sample_rate(&self) -> SampleRate {
        SampleRate::new(self.source.rate).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            self.source.samples.len() as f64
                / f64::from(self.source.channels)
                / f64::from(self.source.rate),
        ))
    }
}
impl Decodable for Pcm {
    type Decoder = Decoder;
    fn decoder(&self) -> Decoder {
        Decoder {
            source: self.clone(),
            at: 0,
            gains: [1.; 2],
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mono_stereo_and_frame_coherent_live_pan() {
        let pcm = Pcm::new(vec![16384, 8192].into(), 1, 8000).unwrap();
        pcm.pan.set(6000).unwrap();
        let mut d = pcm.decoder();
        assert!((d.next().unwrap() - 0.0005).abs() < 1e-8);
        pcm.pan.set(-6000).unwrap();
        assert_eq!(d.next(), Some(0.5));
        assert_eq!(d.next(), Some(0.25));
        assert!((d.next().unwrap() - 0.00025).abs() < 1e-8);
        assert_eq!(d.next(), None);
        let pcm = Pcm::new(vec![16384, -8192].into(), 2, 8000).unwrap();
        assert_eq!(pcm.decoder().collect::<Vec<_>>(), vec![0.5, -0.25]);
        assert!(pcm.pan.set(6001).is_err());
    }
    #[test]
    #[ignore = "Requires real audio device; only synthetic zero PCM, no window"]
    fn real_sink_admits_custom_stereo_paused_and_cancels() {
        use bevy::audio::{AddAudioSource, AudioPlugin, GlobalVolume, Volume};
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            AudioPlugin {
                global_volume: GlobalVolume::new(Volume::Linear(0.)),
                ..default()
            },
        ));
        app.add_audio_source::<Pcm>();
        let pcm = Pcm::new(vec![0; 8000].into(), 1, 8000).unwrap();
        let pan = pcm.pan.clone();
        let handle = app.world_mut().resource_mut::<Assets<Pcm>>().add(pcm);
        let entity = app
            .world_mut()
            .spawn((
                AudioPlayer(handle.clone()),
                PlaybackSettings {
                    paused: true,
                    ..PlaybackSettings::ONCE
                },
            ))
            .id();
        let start = std::time::Instant::now();
        while app.world().get::<AudioSink>(entity).is_none()
            && start.elapsed() < Duration::from_secs(5)
        {
            app.update();
            std::thread::sleep(Duration::from_millis(5));
        }
        let sink = app.world().get::<AudioSink>(entity).expect("no real sink");
        assert!(sink.is_paused());
        pan.set(6000).unwrap();
        pan.set(-6000).unwrap();
        let mut sink = app.world_mut().get_mut::<AudioSink>(entity).unwrap();
        sink.set_volume(Volume::Linear(0.25));
        sink.play();
        sink.pause();
        assert!(sink.is_paused());
        sink.stop();
        app.world_mut().despawn(entity);
        app.world_mut()
            .resource_mut::<Assets<Pcm>>()
            .remove(handle.id());
        assert!(app.world().resource::<Assets<Pcm>>().is_empty());
    }
}
