//! Normal partial-Mecc collection. Campaign quota execution is deliberately not
//! admitted: there is no active objective VM in this scene preview. The -1
//! no-credit branch is a preview policy, not reconstructed mission startup.
use super::*;
use giants_game::{audio_parameters::Random, audio_sequence::Route, mecc_collection};

pub(super) struct Collection {
    world: mecc_collection::World,
    progress: giants_game::mission_notice::State,
    pub count: u64,
    step: u64,
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
    ) -> Result<(), String> {
        if self.collection.is_none() {
            let current = session.mecc_meat(frame.controlled)?;
            self.collection = Some(Collection {
                world: mecc_collection::World::new(frame.controlled, -1., 0., current)?,
                progress: Default::default(),
                count: 0,
                step: 0,
            });
        }
        let collection = self.collection.as_mut().unwrap();
        if collection.step.checked_add(1) != Some(frame.step) {
            return Err("collection requires contiguous ordinary controller steps".into());
        }
        session.controlled_mecc_motion(frame.controlled, frame.step)?;
        let count = session.collect_nearby_vimp_chunks(
            frame.controlled,
            frame.seconds,
            mecc_collection::Services {
                world: &mut collection.world,
                progress: &mut collection.progress,
                context: None,
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
                "mission_meat_collected ticket={} chunks={count} total_chunks={} carried={} source_scan=true source_callback=true objective_credit=false audio_queued=true",
                frame.ticket,
                collection.count,
                session.mecc_meat(frame.controlled)?
            );
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
