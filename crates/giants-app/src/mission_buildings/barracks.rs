//! Ordinary state98 initial branch and real object-bound audio requests.
//! Damage uses canonical Session state and the common geometry/effect owners.
//! Later spawning/AI and full world-update admission remain separate.
//! Prototype PS2 `sound_play_obj` / `sound_play_3dpos` are unknown-attribution
//! navigation routes for the independently acquired PC absent-descriptor gates.
use giants_game::{
    building_barracks::InitialParameters,
    declared_residents::{Key, Scope},
    resident_states::Session,
};

pub(crate) struct Initial {
    keys: Vec<Key>,
    opaque_words: Vec<Option<u32>>,
    stock: InitialParameters,
    consumed: bool,
    failure: Option<String>,
    alert_keys: Vec<Key>,
}

impl Initial {
    pub(crate) fn prepare(
        plan: &giants_game::instances::Plan,
        globals: &giants_formats::global_data::GlobalData,
    ) -> Result<Self, String> {
        let world = plan
            .world_entries
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e.declaration.object_type, 1060 | 1226))
            .map(|(index, _)| Key {
                scope: Scope::World,
                index,
            });
        let mission = plan
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e.serialized.object_type, 1060 | 1226))
            .map(|(index, _)| Key {
                scope: Scope::Mission,
                index,
            });
        let keys: Vec<_> = world.chain(mission).collect();
        let opaque_words = keys
            .iter()
            .map(|key| {
                if key.scope == Scope::World {
                    Ok(plan.world_entries[key.index]
                        .class_input()?
                        .mission_word_174)
                } else {
                    Ok(plan.entries[key.index]
                        .source
                        .as_ref()
                        .map_err(Clone::clone)?
                        .mission_word_174)
                }
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut alert_keys = Vec::new();
        for (key, class) in plan
            .world_entries
            .iter()
            .enumerate()
            .map(|(index, e)| {
                (
                    Key {
                        scope: Scope::World,
                        index,
                    },
                    e.declaration.object_type,
                )
            })
            .chain(plan.entries.iter().enumerate().map(|(index, e)| {
                (
                    Key {
                        scope: Scope::Mission,
                        index,
                    },
                    e.serialized.object_type,
                )
            }))
            .filter(|_| !keys.is_empty())
        {
            let program = plan
                .resources()
                .classes
                .models
                .first_program(i32::try_from(class).map_err(|_| "alert source class overflow")?)
                .ok_or("alert source class metadata absent")?
                .program
                .as_ref()
                .map_err(Clone::clone)?;
            let state = program
                .records
                .iter()
                .rfind(|r| r.opcode == 0x1e)
                .map(|r| {
                    r.bytes
                        .get(1)
                        .copied()
                        .ok_or("alert source state record incomplete")
                })
                .transpose()?
                .unwrap_or(0);
            let class = i32::try_from(class).map_err(|_| "alert source class overflow")?;
            let flags = giants_game::object_lists::DirectFlags::project(class, program);
            if (class == 215 || state == 23)
                && giants_game::object_lists::membership(class, u32::from(state), flags) & 1 != 0
            {
                alert_keys.push(key);
            }
        }
        Ok(Self {
            keys,
            opaque_words,
            stock: InitialParameters::load(globals)?,
            consumed: false,
            failure: None,
            alert_keys,
        })
    }
    pub(crate) fn start(
        &mut self,
        session: &mut Session,
        events: &mut Vec<crate::mission_audio::Event>,
    ) -> Result<bool, String> {
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        if self.consumed {
            return Ok(false);
        }
        for key in &self.alert_keys {
            let id = session
                .declaration(*key)
                .ok_or_else(|| format!("building alert source owner missing: {key:?}"))?;
            if !session.list(1).contains(&id) {
                return Err("building alert source owner missing its constructor list".into());
            }
        }
        let owners = self
            .keys
            .iter()
            .zip(&self.opaque_words)
            .map(|(key, expected)| -> Result<_, String> {
                let id = session
                    .declaration(*key)
                    .ok_or_else(|| format!("barracks source owner missing: {key:?}"))?;
                if session
                    .get(id)
                    .ok_or("barracks source resident absent")?
                    .declared_word_174
                    != *expected
                {
                    return Err("barracks source opaque declaration word was not retained".into());
                }
                Ok(id)
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Exact authored keys, including declarations that failed construction:
        // a partial resident list cannot turn into an empty successful start.
        self.consumed = true;
        for owner in owners {
            let result =
                session.initialize_barracks(owner, self.stock, false, &mut |owner, sound| {
                    events.push(crate::mission_audio::Event::ActorSound {
                        owner,
                        sound,
                        kind: giants_game::actor_sound::Kind::Owner,
                    });
                    Ok(())
                });
            if let Err(error) = result {
                self.failure = Some(error.clone());
                return Err(error);
            }
            println!(
                "mission_barracks_initialized owner={owner:?} selector={} remaining={} initial_branch=true audio_request=true backend_acknowledged=false later_primary=false",
                session.barracks_state(owner)?.selector,
                session.barracks_state(owner)?.remaining
            );
        }
        Ok(true)
    }
}

struct Feedback<'a> {
    runtime: &'a mut crate::mission_building_effects::Runtime,
    ticket: u64,
    selection: giants_game::effect_initialization::Selection,
    random: &'a mut giants_game::audio_parameters::Random,
    sounds: &'a mut Vec<crate::mission_audio::Event>,
}
impl giants_game::resident_states::BarracksEffects for Feedback<'_> {
    fn object_sound(
        &mut self,
        owner: giants_game::resident_states::Id,
        cue: u32,
    ) -> Result<(), String> {
        self.sounds.push(crate::mission_audio::Event::ActorSound {
            owner,
            sound: cue,
            kind: giants_game::actor_sound::Kind::Owner,
        });
        Ok(())
    }
    fn explosion(
        &mut self,
        kind: giants_game::building_barracks::Explosion,
        point: [f32; 3],
    ) -> Result<(), String> {
        self.sounds.extend(self.runtime.emit_barracks(
            self.ticket,
            kind,
            point,
            self.selection,
            self.random,
        )?);
        Ok(())
    }
    fn positional_sound(&mut self, cue: u32, point: [f32; 3]) -> Result<(), String> {
        self.sounds.push(crate::mission_audio::Event::NpcPosition {
            sound: cue,
            position: point,
        });
        Ok(())
    }
    fn mission_trigger(
        &mut self,
        _: giants_game::resident_states::Id,
        _: &giants_game::building_barracks::State<giants_game::resident_states::Id>,
    ) -> Result<(), String> {
        Err("barracks source mission-death counter is not bound".into())
    }
}
impl super::Owner {
    fn barracks_receipt(
        &mut self,
        ticket: u64,
        object: u32,
        session: &Session,
        fraction: f32,
        sounds: Vec<crate::mission_audio::Event>,
    ) -> Result<super::HitResult, String> {
        let id = session
            .declaration(self.declaration)
            .ok_or("barracks source identity absent")?;
        let state = session.barracks_state(id)?;
        let fields = session
            .get(id)
            .unwrap()
            .construction_fields
            .as_ref()
            .ok_or("barracks current fields absent")?;
        if fields.class != self.fields.class
            || fields.position != self.fields.position
            || fields.angles != self.fields.angles
            || fields.scale.to_bits() != self.fields.scale.to_bits()
        {
            return Err("barracks current pose disagrees with its declared static owner".into());
        }
        if state.mark_flags & 15 == 2 && !matches!(state.selector, 4 | 5) && state.remaining > 0 {
            return Err("barracks mission-death owner required before damage admission".into());
        }
        self.fields = fields.clone();
        self.damage = super::House {
            damage: state.damage,
            limit: state.limit,
        };
        self.fraction = fraction.to_bits();
        let stage = self
            .class
            .stages
            .get(&self.fraction)
            .ok_or("barracks source health geometry absent")?
            .clone();
        let retired = fields.object_flags & 0x8000 != 0;
        Ok(super::HitResult {
            physical: self.physical,
            environment: self.environment,
            object,
            active: fields.object_flags,
            triangles: if retired {
                Vec::new()
            } else {
                stage.placed(self.pose)?
            },
            identities: stage.identities.clone(),
            update: super::Update {
                ticket,
                key: self.key,
                class: fields.class,
                geometry: (!retired).then_some(stage),
            },
            sounds,
            replacement: None,
        })
    }
}
impl super::World {
    pub(crate) fn is_barracks(&self, object: u32) -> bool {
        self.owners
            .get(&object)
            .is_some_and(|o| matches!(o.class.policy, super::Policy::Barracks))
    }
    pub(crate) fn initial_barracks_updates(
        &mut self,
        ticket: u64,
        session: &Session,
    ) -> Result<Vec<super::HitResult>, String> {
        self.owners
            .iter_mut()
            .filter(|(_, o)| matches!(o.class.policy, super::Policy::Barracks))
            .map(|(&object, owner)| {
                let id = session
                    .declaration(owner.declaration)
                    .ok_or("barracks initial owner absent")?;
                let fraction = f32::from_bits(
                    session
                        .get(id)
                        .and_then(|a| a.channels)
                        .ok_or("barracks initial channels absent")?[0][1],
                );
                owner.barracks_receipt(ticket, object, session, fraction, Vec::new())
            })
            .collect()
    }
    pub(crate) fn hit_barracks(
        &mut self,
        ticket: u64,
        hit: giants_game::mecc_pistol::WorldHit,
        camera: [f32; 3],
        session: &mut Session,
        player: giants_game::resident_states::Id,
        random: &mut giants_game::audio_parameters::Random,
    ) -> Result<Option<super::HitResult>, String> {
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        let result: Result<Option<super::HitResult>, String> = (|| {
            let extra = self
                .owners
                .values()
                .filter(|o| session.declaration(o.declaration).is_none())
                .count();
            let count = session
                .counts()
                .residents
                .checked_add(extra)
                .and_then(|n| n.checked_add(self.effects.active()))
                .ok_or("barracks owned population overflow")?;
            let owner = self
                .owners
                .get_mut(&hit.object)
                .ok_or("barracks collision owner absent")?;
            let id = session
                .declaration(owner.declaration)
                .ok_or("barracks source owner absent")?;
            let state = *session.barracks_state(id)?;
            if state.mark_flags & 15 == 2 && !matches!(state.selector, 4 | 5) && state.remaining > 0
            {
                return Err("barracks mission-death owner required before damage admission".into());
            }
            let mut sounds = Vec::new();
            let accepted = session.hit_barracks(
                id,
                giants_game::resident_states::BarracksAttack {
                    amount: hit.damage,
                    hit_owner: Some(player),
                    player,
                    radius_base: self.barracks_radius,
                },
                &mut Feedback {
                    runtime: &mut self.effects,
                    ticket,
                    random,
                    sounds: &mut sounds,
                    selection: giants_game::effect_initialization::Selection {
                        active_objects: u32::try_from(count)
                            .map_err(|_| "barracks owned population overflow")?,
                        quality: self.quality,
                        lite_distance: self.lite_distance,
                        camera,
                        recipient_position: state.position,
                    },
                },
            )?;
            if !accepted {
                return Ok(None);
            }
            let retired = session
                .get(id)
                .unwrap()
                .construction_fields
                .as_ref()
                .unwrap()
                .object_flags
                & 0x8000
                != 0;
            let fraction = if retired {
                0.
            } else {
                session.refresh_barracks_health(id, self.thresholds)?
            };
            let result = owner.barracks_receipt(ticket, hit.object, session, fraction, sounds)?;
            println!(
                "mission_barracks_hit ticket={ticket} object={} owner={id:?} damage={} limit={} fraction={fraction} retired={retired} canonical_owner=true ordinary_callback=true later_primary=false",
                hit.object, owner.damage.damage, owner.damage.limit
            );
            Ok(Some(result))
        })();
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; actual authored owners, stock and audio resources, not device/GPU/normal hit acceptance"]
    fn installed_barracks_initial_queues_real_owner_sounds_once() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_BUILDING_ROOT").unwrap());
        let menu = giants_game::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let packet =
            giants_game::mission_loading::prepare(&root, menu.request("Story2", 1).unwrap())
                .unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let globals =
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
                .unwrap();
        let mut initial = Initial::prepare(&plan, &globals).unwrap();
        assert_eq!(initial.keys.len(), 2);
        let mut session = Session::new(412);
        let mut events = Vec::new();
        assert!(initial.start(&mut session, &mut events).is_err());
        assert!(events.is_empty());
        giants_game::declared_residents::construct_acquired(&plan, &mut session);
        initial.start(&mut session, &mut events).unwrap();
        assert_eq!(events.len(), 2);
        let resources = plan.resources();
        let registry =
            giants_game::gameplay_sound_bank::Registry::prepare(&resources.classes.visits, false)
                .unwrap();
        let (language, _) = giants_game::gameplay_sound_bank::voice_language(
            &resources.archives,
            &resources.language,
        )
        .unwrap();
        for event in &events {
            let crate::mission_audio::Event::ActorSound { owner, sound, kind } = event else {
                panic!("initial sound lost its object owner");
            };
            assert_eq!(*kind, giants_game::actor_sound::Kind::Owner);
            assert_eq!(session.barracks_state(*owner).unwrap().phase, 1);
            assert!(session.actor_world_tick(*owner).is_err());
            let source = registry
                .resolve(&resources.archives, &language, *sound)
                .unwrap()
                .unwrap();
            assert!(!source.pcm.samples.is_empty());
        }
        initial.start(&mut session, &mut events).unwrap();
        assert_eq!(events.len(), 2);
        session.clear();
        for event in events {
            let crate::mission_audio::Event::ActorSound { owner, .. } = event else {
                unreachable!()
            };
            assert!(session.barracks_state(owner).is_err());
        }
    }
    #[test]
    #[ignore = "requires GIANTS_BUILDING_ROOT; installed cold mission metadata/initial branches, not GPU or mission acceptance"]
    fn installed_following_mission_barracks_retain_opaque_command7_and_initialize() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_BUILDING_ROOT").unwrap());
        let globals =
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
                .unwrap();
        for (world, index) in [("Story2", 3), ("Story4", 0)] {
            let menu = giants_game::mission_selection::MissionMenu::parse(
                &std::fs::read(root.join(format!("Bin/w_{world}.bin"))).unwrap(),
                0,
                true,
            )
            .unwrap();
            let packet =
                giants_game::mission_loading::prepare(&root, menu.request(world, index).unwrap())
                    .unwrap();
            let plan = packet.instance_plan(&root, "English").unwrap();
            let mut session = Session::new(413);
            let report = giants_game::declared_residents::construct_acquired(&plan, &mut session);
            let mut count = 0;
            for (index, entry) in plan.entries.iter().enumerate() {
                let source = entry.source.as_ref().unwrap();
                if !matches!(source.class, 1060 | 1226) {
                    continue;
                }
                let key = Key {
                    scope: Scope::Mission,
                    index,
                };
                let record = report.records.iter().find(|r| r.key == key).unwrap();
                let id = *record.result.as_ref().unwrap();
                assert_eq!(session.declaration(key), Some(id));
                assert_eq!(
                    session.get(id).unwrap().declared_word_174,
                    source.mission_word_174
                );
                if source.mission_word_174.is_some() {
                    count += 1;
                }
            }
            assert!(count > 0, "expected actual source command7 in {world}");
            let mut initial = Initial::prepare(&plan, &globals).unwrap();
            let mut events = vec![];
            assert!(initial.start(&mut session, &mut events).unwrap());
            for key in &initial.keys {
                let id = session.declaration(*key).unwrap();
                assert_eq!(session.barracks_state(id).unwrap().phase, 1);
                // Retention does not admit the still-unbound later AI walk.
                assert!(session.actor_world_tick(id).is_err());
            }
            assert!(!initial.start(&mut session, &mut events).unwrap());
            session.clear();
            assert_eq!(session.declaration_count(), 0);
        }
    }
}
