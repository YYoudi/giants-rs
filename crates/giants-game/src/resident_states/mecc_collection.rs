//! Partial-controller consumer of the acquired collection phase. This is not
//! admission of the complete native Mecc primary callback.
use super::*;

impl Session {
    /// Current source-owned carried meat, shared with collection and HUD.
    pub fn mecc_meat(&self, id: Id) -> Result<f32, String> {
        let actor = self.get(id).ok_or("meat owner absent")?;
        if actor.class != 2 || !actor.mecc_special.is_some_and(|s| s.player_fields) {
            return Err("meat requires an initialized Mecc player".into());
        }
        let body = actor.seed.storage.data().ok_or("meat payload absent")?;
        let value = f32::from_le_bytes(body[0x50..0x54].try_into().unwrap());
        if !value.is_finite() {
            return Err("nonfinite carried meat".into());
        }
        Ok(value)
    }

    /// Prototype PS2 search key: `mecc_check_food_chunk_collect`. The exact
    /// PC block attribution is probable; see docs/SYMBOL_NAVIGATION.md.
    /// The native quarter-second XY scan and callback are reused unchanged.
    pub fn collect_nearby_vimp_chunks(
        &mut self,
        collector: Id,
        seconds: f32,
        services: crate::mecc_collection::Services<'_>,
        random: &mut crate::audio_parameters::Random,
    ) -> Result<usize, String> {
        let actor = self.get(collector).ok_or("collection player absent")?;
        if actor.controlled_motion.is_none() || actor.class != 2 {
            return Err("partial collection requires published ordinary Mecc motion".into());
        }
        self.mecc_meat(collector)?;
        let body = actor
            .seed
            .storage
            .data()
            .ok_or("collection payload absent")?;
        let timer = f32::from_le_bytes(body[0x1d4..0x1d8].try_into().unwrap());
        let position = actor
            .construction_fields
            .as_ref()
            .ok_or("collection pose absent")?
            .position;
        let candidates = self
            .list(0)
            .into_iter()
            .filter_map(|id| {
                let a = self.get(id)?;
                if !(1117..=1119).contains(&a.class) {
                    return None;
                }
                let f = a.construction_fields.as_ref()?;
                Some(crate::mecc_periodic_scan::Candidate {
                    id,
                    class: a.class,
                    flags: f.object_flags,
                    control: f.control_flags,
                    position: f.position,
                })
            })
            .collect::<Vec<_>>();
        let plan = crate::mecc_periodic_scan::plan(timer, seconds, position, &candidates)?;
        self.residents
            .get_mut(&collector.serial)
            .unwrap()
            .seed
            .storage
            .data_mut()
            .unwrap()[0x1d4..0x1d8]
            .copy_from_slice(&plan.timer.to_le_bytes());
        let mut collected = 0;
        for item in plan.selected {
            collected += usize::from(self.collect_vimp_chunk(
                collector,
                item,
                crate::mecc_collection::Services {
                    world: services.world,
                    progress: services.progress,
                    context: services.context,
                    catalog: services.catalog,
                    current_player: services.current_player,
                    now: services.now,
                    offline: services.offline,
                    audio_key: services.audio_key,
                    audio: &mut *services.audio,
                },
                random,
            )?);
        }
        Ok(collected)
    }
}
