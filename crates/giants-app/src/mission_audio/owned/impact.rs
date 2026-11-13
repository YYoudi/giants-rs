//! Real reserved positional voices for acquired owned impact callbacks.
//! Main secondary damage/world composition remains a separate caller.
use super::*;
use giants_game::{
    positional_impact,
    resident_states::{Id, Session},
};

/// Reserve paused/muted PCM with no parameter draws or playback acknowledgement.
pub(crate) fn reserve(
    world: &mut World,
    mounted: &mut Mounted,
    cues: &[u32],
) -> Result<(), String> {
    if cues.len() > 32 {
        return Err("impact reservation exceeds bounded voice-source budget".into());
    }
    for &sound in cues {
        if !ensure_actor_source(world, mounted, sound)? {
            continue;
        }
        if mounted
            .voices
            .values()
            .filter(|v| {
                v.sound == sound
                    && v.owned_reservation
                    && v.deferred_parameters
                    && v.spatial.is_some()
            })
            .count()
            >= 2
        {
            continue;
        }
        let source = mounted.sources.get(&sound).unwrap();
        if source.words[0] != 0 {
            return Err("impact reservation requires acquired one-shot source".into());
        }
        let pcm =
            crate::positional_pcm::Pcm::new(source.pcm.clone(), source.channels, source.rate)?;
        let pan = pcm.pan.clone();
        let handle = world
            .resource_mut::<Assets<crate::positional_pcm::Pcm>>()
            .add(pcm);
        let id = mounted
            .next
            .checked_add(1)
            .ok_or("impact voice identity exhausted")?;
        let entity = world
            .spawn((
                crate::mission_runtime::MissionOwned(mounted.ticket),
                AudioPlayer(handle.clone()),
                PlaybackSettings {
                    paused: true,
                    muted: true,
                    volume: Volume::Linear(0.),
                    ..PlaybackSettings::ONCE
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
                local_owner: None,
                actor_moved: false,
                spatial: Some(SpatialVoice {
                    handle,
                    pan,
                    position: [0.; 3],
                    minimum: 0.,
                    maximum: 0.,
                    gain: 0.,
                }),
                entity,
                sound,
                gain: 0.,
                frequency: 0,
                frames: source.frames,
                fraction: 1.,
                ready: false,
                started: false,
                looping: false,
            },
        );
    }
    Ok(())
}
impl OwnedAudio<'_, '_> {
    pub(crate) fn impact_ready(&self, ticket: u64, cues: &[u32]) -> bool {
        self.engine.mounted.as_ref().is_some_and(|m| {
            m.ticket == ticket
                && cues.iter().all(|sound| {
                    m.unavailable.contains(sound)
                        || m.voices.values().any(|v| {
                            v.sound == *sound
                                && v.owned_reservation
                                && v.deferred_parameters
                                && v.spatial.is_some()
                                && self.sinks.contains(v.entity)
                        })
                })
        })
    }
    pub(crate) fn with_impact<T>(
        &mut self,
        ticket: u64,
        effects: f32,
        listener: [f32; 16],
        local: Id,
        operation: impl FnOnce(&mut Device<'_, '_, '_>) -> Result<T, String>,
    ) -> Result<T, String> {
        if !effects.is_finite()
            || !(0. ..=1.).contains(&effects)
            || !listener.into_iter().all(f32::is_finite)
        {
            return Err("impact audio requires finite listener and gain".into());
        }
        if local.ticket() != ticket {
            return Err("impact local actor belongs to another mission ticket".into());
        }
        let mounted = self
            .engine
            .mounted
            .as_mut()
            .filter(|m| m.ticket == ticket)
            .ok_or("impact audio ticket absent or foreign")?;
        let assets = self
            .positional_assets
            .as_mut()
            .ok_or("impact positional asset storage absent")?;
        operation(&mut Device {
            mounted,
            sinks: &mut self.sinks,
            commands: &mut self.commands,
            effects,
            listener,
            local,
            assets,
        })
    }
}
pub(crate) struct Device<'a, 'w, 's> {
    mounted: &'a mut Mounted,
    sinks: &'a mut Query<'w, 's, &'static mut AudioSink>,
    commands: &'a mut Commands<'w, 's>,
    effects: f32,
    listener: [f32; 16],
    local: Id,
    assets: &'a mut Assets<crate::positional_pcm::Pcm>,
}
impl giants_game::resident_states::MeccImpactVoice for Device<'_, '_, '_> {
    fn play(&mut self, session: &Session, target: Id, cue: u32) -> Result<(), String> {
        // Reborrow the same live catalog, sinks, commands and RNG exclusively.
        // The local voice adapter retains its routing/source/device checks.
        let mut backend = ReservedDevice {
            mounted: self.mounted,
            sinks: self.sinks,
            commands: self.commands,
            effects: self.effects,
            prepared_sound: None,
        };
        damage::SharedBackend::new(&mut backend)
            .voice(self.local, self.local.ticket())
            .play(session, target, cue)
    }
}
impl Device<'_, '_, '_> {
    fn root(&self, s: &Session, id: Id) -> Result<Id, String> {
        match s.sound_route(
            id,
            self.local,
            giants_game::actor_sound::Kind::Attachment,
            true,
        )? {
            Some(giants_game::actor_sound::Route::Attached(Some(root))) => Ok(root),
            _ => Err("impact audio requires acquired positional actor root".into()),
        }
    }
    fn stop_ids(&mut self, ids: Vec<u32>) -> Result<(), String> {
        for id in ids {
            let v = self
                .mounted
                .voices
                .get(&id)
                .ok_or("impact voice disappeared")?;
            let sink = self
                .sinks
                .get_mut(v.entity)
                .map_err(|_| "started impact voice lost its sink")?;
            sink.stop();
            let v = self.mounted.voices.remove(&id).unwrap();
            if let Some(spatial) = v.spatial {
                self.assets.remove(spatial.handle.id());
            }
            self.commands.entity(v.entity).despawn();
        }
        Ok(())
    }
}
impl positional_impact::Cue for Device<'_, '_, '_> {
    fn play(
        &mut self,
        s: &mut Session,
        id: Id,
        sound: u32,
        _vm_random: &mut Random,
    ) -> Result<(), String> {
        let actor = s.get(id).ok_or("impact audio actor absent or foreign")?;
        if s.positional_impact(id).is_none()
            || actor
                .construction_fields
                .as_ref()
                .is_none_or(|f| f.object_flags & 0xc000 != 0)
        {
            return Err("impact playback requires its live owned effect actor".into());
        }
        let root = self.root(s, id)?;
        if self.mounted.unavailable.contains(&sound) {
            return Ok(());
        }
        let voice = self
            .mounted
            .voices
            .iter()
            .find_map(|(&key, v)| {
                (v.sound == sound
                    && v.owned_reservation
                    && v.deferred_parameters
                    && v.spatial.is_some()
                    && self.sinks.contains(v.entity))
                .then_some(key)
            })
            .ok_or("impact source has no actual ready reserved voice")?;
        let source = self
            .mounted
            .sources
            .get(&sound)
            .ok_or("impact source absent")?;
        if source.words[0] != 0 {
            return Err("impact sound flags outside acquired one-shot profile".into());
        }
        let position = s
            .get(root)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("impact audio owner fields absent")?
            .position;
        let audio_seed_before = self.mounted.audio_random.0;
        let parameters = audio_parameters::prepare(
            source.words,
            source.rate,
            &mut self.mounted.audio_random,
            Precision::Double53,
        )?;
        if parameters.frequency == 0 {
            return Err("impact frequency is zero".into());
        }
        let spatial = giants_game::sound_spatial::update(giants_game::sound_spatial::Input {
            flags: 0x2800e,
            hardware: false,
            fixed_position: [0.; 3],
            owner_position: position,
            previous: giants_game::sound_spatial::State {
                position,
                pan: 0.,
                attenuation: 0.,
            },
            distances: [parameters.minimum_distance, parameters.maximum_distance],
            matrix: self.listener,
        })?;
        let gain = audio_parameters::instance_gain(
            self.effects,
            parameters.gain,
            spatial.attenuation,
            1.,
            Precision::Double53,
        )?;
        let v = self.mounted.voices.get_mut(&voice).unwrap();
        let spatial_voice = v.spatial.as_mut().unwrap();
        spatial_voice
            .pan
            .set(giants_game::sound_device::pan_units(spatial.pan)?)?;
        let mut sink = self
            .sinks
            .get_mut(v.entity)
            .map_err(|_| "ready impact sink disappeared")?;
        sink.set_speed(parameters.frequency as f32 / source.rate as f32);
        sink.set_volume(Volume::Linear(if muted() { 0. } else { gain }));
        if muted() {
            sink.mute()
        } else {
            sink.unmute()
        }
        sink.play();
        v.actor_owner = Some(root);
        v.deferred_parameters = false;
        v.started = true;
        if !v.ready {
            self.mounted.ready += 1;
        }
        v.ready = true;
        v.gain = parameters.gain;
        v.frequency = parameters.frequency;
        spatial_voice.position = position;
        spatial_voice.minimum = parameters.minimum_distance;
        spatial_voice.maximum = parameters.maximum_distance;
        spatial_voice.gain = spatial.attenuation;
        self.mounted.started += 1;
        self.mounted.actor_started += 1;
        println!(
            "mission_owned_impact_audio_started ticket={} owner={root:?} sound={sound} voice={voice} actual_sink=true frequency={} sample_gain_bits={} gain_bits={} attenuation_bits={} pan_bits={} audio_seed_before={audio_seed_before} audio_seed={} vm_rng_untouched=true",
            self.mounted.ticket,
            parameters.frequency,
            parameters.gain.to_bits(),
            gain.to_bits(),
            spatial.attenuation.to_bits(),
            spatial.pan.to_bits(),
            self.mounted.audio_random.0
        );
        Ok(())
    }
}
impl positional_impact::Stop for Device<'_, '_, '_> {
    fn stop(&mut self, s: &mut Session, id: Id, sound: u32) -> Result<(), String> {
        let root = self.root(s, id)?;
        let ids: Vec<_> = self
            .mounted
            .voices
            .iter()
            .filter_map(|(&key, v)| {
                (v.actor_owner == Some(root) && v.sound == sound).then_some(key)
            })
            .collect();
        self.stop_ids(ids)
    }
}
impl giants_game::actor_retirement::Services for Device<'_, '_, '_> {
    fn release(&mut self, s: &mut Session, id: Id) -> Result<(), String> {
        s.get(id)
            .ok_or("impact retirement audio owner absent or foreign")?;
        let ids: Vec<_> = self
            .mounted
            .voices
            .iter()
            .filter_map(|(&key, v)| (v.actor_owner == Some(id)).then_some(key))
            .collect();
        // A just-requested injury voice may still await its first AudioSink.
        // Cancel that owned reservation without acknowledging playback; a
        // started voice still requires its real sink before retirement.
        for key in &ids {
            let voice = &self.mounted.voices[key];
            if voice.started && !self.sinks.contains(voice.entity) {
                return Err("started actor voice lost its retirement sink".into());
            }
        }
        if self
            .mounted
            .pending
            .as_ref()
            .is_some_and(|p| matches!(p.action, Action::Actor { owner, .. } if owner == id))
        {
            self.mounted.pending = None;
        }
        for key in ids {
            let voice = self.mounted.voices.remove(&key).unwrap();
            if let Ok(sink) = self.sinks.get_mut(voice.entity) {
                sink.stop();
            }
            if let Some(spatial) = voice.spatial {
                self.assets.remove(spatial.handle.id());
            }
            self.commands.entity(voice.entity).despawn();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
