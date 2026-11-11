//! Ordinary source collection. An admitted campaign owner supplies the actual
//! quota, shared counter and language context. Inspection/unsupported objectives
//! retain the explicit -1 no-credit preview policy.
use super::*;
use giants_game::{audio_parameters::Random, audio_sequence::Route, mecc_collection};

pub(super) struct Collection {
    world: mecc_collection::World,
    progress: giants_game::mission_notice::State,
    pub count: u64,
    step: u64,
    objective: bool,
}
pub(in crate::mission_runtime) struct Tick {
    pub controlled: Id,
    pub ticket: u64,
    pub step: u64,
    pub seconds: f32,
    pub now: f32,
}
struct Sounds<'a> {
    owner: Id,
    key: u64,
    events: &'a mut Vec<crate::mission_audio::Event>,
}
impl mecc_collection::Audio for Sounds<'_> {
    fn play(&mut self, sound: u32, owner: u64, route: Route, _: &mut Random) -> Result<(), String> {
        if owner != self.key || route != Route::Local {
            return Err("normal collection sound requires the real local player".into());
        }
        self.events.push(crate::mission_audio::Event::ActorSound {
            owner: self.owner,
            sound,
            kind: giants_game::actor_sound::Kind::Attachment,
        });
        Ok(())
    }
}
impl State {
    pub(in crate::mission_runtime) fn collected_chunks(&self) -> u64 {
        self.collection.as_ref().map_or(0, |c| c.count)
    }
    pub(in crate::mission_runtime) fn collect(
        &mut self,
        session: &mut Session,
        frame: Tick,
        random: &mut Random,
        events: &mut Vec<crate::mission_audio::Event>,
        mut campaign: Option<&mut super::super::campaign::State>,
    ) -> Result<(), String> {
        if self.collection.is_none() {
            let current = session.mecc_meat(frame.controlled)?;
            self.collection = Some(Collection {
                world: mecc_collection::World::new(
                    frame.controlled,
                    campaign
                        .as_ref()
                        .and_then(|c| c.program.quota)
                        .map_or(-1., |v| v as f32),
                    0.,
                    current,
                )?,
                progress: Default::default(),
                count: 0,
                step: 0,
                objective: campaign.is_some(),
            });
        }
        let collection = self.collection.as_mut().unwrap();
        if let Some(campaign) = campaign.as_ref()
            && !collection.objective
        {
            if collection.count != 0 {
                return Err("campaign cannot adopt earlier uncredited inspection pickups".into());
            }
            collection.world = mecc_collection::World::new(
                frame.controlled,
                campaign.program.quota.map_or(-1., |v| v as f32),
                0.,
                session.mecc_meat(frame.controlled)?,
            )?;
            collection.objective = true;
        }
        if collection.step.checked_add(1) != Some(frame.step) {
            return Err("collection requires contiguous ordinary controller steps".into());
        }
        session.controlled_mecc_motion(frame.controlled, frame.step)?;
        let objective = campaign.is_some();
        let (progress, context) = match campaign.as_mut() {
            Some(c) => (
                &mut c.progress,
                Some(giants_game::mission_notice::Context {
                    prefix: &c.program.prefix,
                    gates: giants_game::mission_notice::Gates {
                        suppress: [0, 0, u32::from(c.completed)],
                        time: 0.,
                    },
                }),
            ),
            None => (&mut collection.progress, None),
        };
        let count = session.collect_nearby_vimp_chunks(
            frame.controlled,
            frame.seconds,
            mecc_collection::Services {
                world: &mut collection.world,
                progress,
                context,
                catalog: &self.collection_catalog,
                current_player: Some(frame.controlled),
                now: frame.now,
                offline: true,
                audio_key: frame.ticket,
                audio: &mut Sounds {
                    owner: frame.controlled,
                    key: frame.ticket,
                    events,
                },
            },
            random,
        )?;
        collection.count += count as u64;
        collection.step = frame.step;
        if count != 0 {
            println!(
                "mission_meat_collected ticket={} chunks={count} total_chunks={} carried={} source_scan=true source_callback=true objective_credit={objective} audio_queued=true",
                frame.ticket,
                collection.count,
                session.mecc_meat(frame.controlled)?
            );
        }
        if let Some(campaign) = campaign {
            campaign.advance(
                session,
                frame.controlled,
                &self.collection_catalog,
                frame.now,
            )?;
        }
        Ok(())
    }
    pub(in crate::mission_runtime) fn meat_target(
        &self,
        session: &Session,
        position: [f32; 3],
    ) -> Option<[f32; 3]> {
        session
            .list(0)
            .into_iter()
            .filter_map(|id| {
                session.vimp_chunk(id)?;
                let f = session.get(id)?.construction_fields.as_ref()?;
                (f.object_flags & 0x8000 == 0 && f.control_flags & 1 != 0).then_some(f.position)
            })
            .min_by(|a, b| {
                let d = |p: &[f32; 3]| (p[0] - position[0]).hypot(p[1] - position[1]);
                d(a).total_cmp(&d(b))
            })
    }
}
