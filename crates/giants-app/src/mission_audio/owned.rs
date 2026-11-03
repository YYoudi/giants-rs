//! Ready local voices for the owned Mecc driver. Reservations consume no RNG,
//! remain muted/paused, and are not playback acknowledgements.
use super::*;
pub(super) const SOUNDS: [u32; 4] = [0x5b0, 0x5b1, 0x5b2, 0x5b3];
pub(super) fn requested() -> bool {
    std::env::var("GIANTS_MISSION_MECC_AUDIO").as_deref() == Ok("reserved")
        && std::env::var("GIANTS_MISSION_MECC_BACKEND").as_deref() == Ok("owned-inspection")
}
pub(super) fn reserve(world: &mut World, mounted: &mut Mounted) -> Result<(), String> {
    for sound in SOUNDS {
        let count = mounted
            .voices
            .values()
            .filter(|v| v.sound == sound && v.owned_reservation && v.deferred_parameters)
            .count();
        for _ in count..2 {
            let source = mounted
                .sources
                .get(&sound)
                .ok_or("owned jet source absent")?;
            if source.words[0] & !1 != 0 || source.rate == 0 {
                return Err("owned jet source outside local PCM profile".into());
            }
            let looping = source.words[0] & 1 != 0;
            let id = mounted
                .next
                .checked_add(1)
                .ok_or("owned voice identity exhausted")?;
            let entity = world
                .spawn((
                    crate::mission_runtime::MissionOwned(mounted.ticket),
                    AudioPlayer::new(source.handle.clone()),
                    PlaybackSettings {
                        speed: 1.,
                        volume: Volume::Linear(0.),
                        paused: true,
                        muted: true,
                        ..if looping {
                            PlaybackSettings::LOOP
                        } else {
                            PlaybackSettings::ONCE
                        }
                    },
                ))
                .id();
            mounted.next = id;
            mounted.voices.insert(
                id,
                Voice {
                    owned_reservation: true,
                    deferred_parameters: true,
                    actor_owner: None,
                    actor_moved: false,
                    spatial: None,
                    entity,
                    sound,
                    gain: 0.,
                    frequency: 0,
                    frames: source.frames,
                    fraction: 1.,
                    ready: false,
                    started: false,
                    looping,
                },
            );
            println!(
                "mission_audio_reserved ticket={} voice={id} sound={sound} rng_consumed=false paused=true muted=true",
                mounted.ticket
            );
        }
    }
    Ok(())
}

#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct OwnedAudio<'w, 's> {
    engine: ResMut<'w, Engine>,
    sinks: Query<'w, 's, &'static mut AudioSink>,
    commands: Commands<'w, 's>,
}
impl OwnedAudio<'_, '_> {
    pub(crate) fn failure(&self, ticket: u64) -> Option<String> {
        (self.engine.mounted.is_none() && self.engine.attempted == Some(ticket))
            .then(|| "owned audio source preparation failed".into())
    }
    pub(crate) fn quarantine(&mut self, ticket: u64) {
        let Some(mounted) = self.engine.mounted.as_mut().filter(|m| m.ticket == ticket) else {
            return;
        };
        let ids = mounted
            .voices
            .iter()
            .filter_map(|(&id, v)| v.owned_reservation.then_some(id))
            .collect::<Vec<_>>();
        for id in &ids {
            let voice = mounted.voices.remove(id).unwrap();
            if let Ok(sink) = self.sinks.get_mut(voice.entity) {
                sink.stop();
            }
            self.commands.entity(voice.entity).despawn();
        }
        println!(
            "mission_owned_audio_quarantine ticket={ticket} stopped_or_discarded={} scope=owned_reservations",
            ids.len()
        );
    }
    pub(crate) fn ready(&self, ticket: u64) -> bool {
        self.engine.mounted.as_ref().is_some_and(|m| {
            m.ticket == ticket
                && SOUNDS.iter().all(|sound| {
                    m.voices.values().any(|v| {
                        v.sound == *sound
                            && v.owned_reservation
                            && v.deferred_parameters
                            && self.sinks.contains(v.entity)
                    })
                })
        })
    }
    pub(crate) fn with_backend<T>(
        &mut self,
        ticket: u64,
        effects: f32,
        operation: impl FnOnce(&mut dyn Backend<Handle = u32>) -> Result<T, String>,
    ) -> Result<T, String> {
        if !self.ready(ticket) {
            return Err("owned audio reservations have no actual ready sinks".into());
        }
        if !effects.is_finite() || !(0. ..=1.).contains(&effects) {
            return Err("owned audio gain outside finite unit profile".into());
        }
        let mounted = self.engine.mounted.as_mut().unwrap();
        let mut device = ReservedDevice {
            mounted,
            sinks: &mut self.sinks,
            commands: &mut self.commands,
            effects,
        };
        operation(&mut device)
    }
}
struct ReservedDevice<'a, 'w, 's> {
    mounted: &'a mut Mounted,
    sinks: &'a mut Query<'w, 's, &'static mut AudioSink>,
    commands: &'a mut Commands<'w, 's>,
    effects: f32,
}
impl Backend for ReservedDevice<'_, '_, '_> {
    type Handle = u32;
    fn stop_sound(&mut self, sound: u32, owner: u64) -> Result<(), String> {
        if owner != self.mounted.ticket || !SOUNDS.contains(&sound) {
            return Err("owned audio root or sound mismatch".into());
        }
        for voice in self
            .mounted
            .voices
            .values()
            .filter(|v| v.owned_reservation && v.started && v.sound == sound)
        {
            if let Ok(sink) = self.sinks.get_mut(voice.entity) {
                sink.stop();
            }
        }
        Ok(())
    }
    fn play(&mut self, sound: u32, owner: u64, route: Route) -> Result<u32, String> {
        if route != Route::Local || owner != self.mounted.ticket {
            return Err("owned audio route or root mismatch".into());
        }
        let id = self
            .mounted
            .voices
            .iter()
            .find_map(|(&id, v)| {
                (v.sound == sound
                    && v.owned_reservation
                    && v.deferred_parameters
                    && self.sinks.contains(v.entity))
                .then_some(id)
            })
            .ok_or("owned sound has no reserved ready voice")?;
        let source = self
            .mounted
            .sources
            .get(&sound)
            .ok_or("owned source disappeared")?;
        let seed = self.mounted.audio_random.0;
        let parameters = audio_parameters::prepare(
            source.words,
            source.rate,
            &mut self.mounted.audio_random,
            Precision::Double53,
        )?;
        if parameters.frequency == 0 {
            return Err("owned frequency is zero".into());
        }
        let voice = self.mounted.voices.get_mut(&id).unwrap();
        let mut sink = self
            .sinks
            .get_mut(voice.entity)
            .map_err(|_| "owned ready sink disappeared")?;
        let gain = audio_parameters::instance_gain(
            self.effects,
            parameters.gain,
            1.,
            1.,
            Precision::Double53,
        )?;
        sink.set_speed(parameters.frequency as f32 / source.rate as f32);
        sink.set_volume(Volume::Linear(if muted() { 0. } else { gain }));
        if muted() {
            sink.mute();
        } else {
            sink.unmute();
        }
        sink.play();
        voice.gain = parameters.gain;
        voice.frequency = parameters.frequency;
        if !voice.ready {
            self.mounted.ready += 1;
            println!(
                "mission_audio_reservation_ready ticket={} voice={id} sound={sound} paused=true",
                self.mounted.ticket
            );
        }
        voice.ready = true;
        voice.started = true;
        voice.deferred_parameters = false;
        self.mounted.started += 1;
        println!(
            "mission_owned_audio_started ticket={} voice={id} sound={sound} frequency={} seed_before={seed} seed_after={} actual_sink=true muted={}",
            self.mounted.ticket,
            parameters.frequency,
            self.mounted.audio_random.0,
            muted()
        );
        Ok(id)
    }
    fn duration(&mut self, id: u32) -> Result<f32, String> {
        let v = self
            .mounted
            .voices
            .get(&id)
            .ok_or("owned duration voice missing")?;
        if !v.owned_reservation || !v.started || v.frequency == 0 || !self.sinks.contains(v.entity)
        {
            return Err("owned duration needs a started real voice".into());
        }
        Ok((v.frames as f64 / f64::from(v.frequency)) as f32)
    }
    fn volume(&mut self, id: u32, fraction: f32) -> Result<(), String> {
        if id == 0 {
            return Ok(());
        }
        let v = self
            .mounted
            .voices
            .get_mut(&id)
            .ok_or("owned fade voice missing")?;
        if !v.owned_reservation || !v.started {
            return Err("owned fade voice not started".into());
        }
        let mut sink = self
            .sinks
            .get_mut(v.entity)
            .map_err(|_| "owned fade sink missing")?;
        let gain = audio_parameters::instance_gain(
            self.effects,
            v.gain,
            1.,
            fraction,
            Precision::Double53,
        )?;
        sink.set_volume(Volume::Linear(if muted() { 0. } else { gain }));
        v.fraction = fraction;
        Ok(())
    }
    fn stop(&mut self, id: u32) -> Result<(), String> {
        if self
            .mounted
            .voices
            .get(&id)
            .is_some_and(|v| !v.owned_reservation)
        {
            return Err("foreign voice in owned audio cleanup".into());
        }
        if let Some(v) = self.mounted.voices.remove(&id) {
            if let Ok(sink) = self.sinks.get_mut(v.entity) {
                sink.stop();
            }
            self.commands.entity(v.entity).despawn();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
