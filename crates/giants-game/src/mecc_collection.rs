//! Offline unparented Mecc collection services. Mission gates/context and audio
//! are authoritative inputs; network ownership and chunk updates stay separate.
use crate::{audio_parameters::Random, audio_sequence::Route, mission_notice, resident_states::Id};
use giants_formats::gtext::Catalog;

pub trait Audio {
    /// Use the same random stream if a loaded sound bank consumes variation.
    fn play(
        &mut self,
        sound: u32,
        owner: u64,
        route: Route,
        random: &mut Random,
    ) -> Result<(), String>;
}
#[derive(Clone, Debug)]
pub struct World {
    pub(crate) scope: Id,
    pub(crate) quota: f32,
    pub(crate) credited: f32,
    pub(crate) hud: f32,
    pub(crate) failure: Option<String>,
}
impl World {
    pub fn new(scope: Id, quota: f32, credited: f32, hud: f32) -> Result<Self, String> {
        if ![quota, credited, hud].into_iter().all(f32::is_finite) {
            return Err("collection world requires finite supplied state".into());
        }
        Ok(Self {
            scope,
            quota,
            credited,
            hud,
            failure: None,
        })
    }
    pub fn words(&self) -> [u32; 4] {
        [
            self.quota.to_bits(),
            self.credited.to_bits(),
            self.hud.to_bits(),
            u32::from(self.failure.is_some()),
        ]
    }
    pub(crate) fn credit(
        &mut self,
        amount: f32,
        progress: &mut mission_notice::State,
        context: Option<mission_notice::Context<'_>>,
        catalog: &Catalog,
        messages: &mut crate::mission_messages::Queue,
        now: f32,
    ) -> Result<(), String> {
        if self.quota == -1. {
            return Ok(());
        }
        let next = (f64::from(self.credited) + f64::from(amount)) as f32;
        if !next.is_finite() {
            return Err("collection quota overflow".into());
        }
        self.credited = next;
        if self.quota > self.credited {
            return Ok(());
        }
        self.quota = -1.;
        let context = context.ok_or("collection milestone requires real mission context")?;
        if context.gates.suppress.into_iter().any(|v| v != 0) || context.gates.time > 0. {
            return Ok(());
        }
        // The pending pickup word belongs to a different producer. Share only
        // the actual context counter, leaving pending untouched.
        progress.counter = progress.counter.wrapping_add(1);
        let mut key = context.prefix.to_vec();
        key.extend_from_slice((progress.counter as i32).to_string().as_bytes());
        let request = mission_notice::Lookup { key };
        if let Some(notice) = request.resolve(catalog.get(&request.key))? {
            messages.post_notice(&notice, now)?;
        }
        Ok(())
    }
}
pub struct Services<'a> {
    pub world: &'a mut World,
    pub progress: &'a mut mission_notice::State,
    pub context: Option<mission_notice::Context<'a>>,
    pub catalog: &'a Catalog,
    pub current_player: Option<Id>,
    pub now: f32,
    pub offline: bool,
    pub audio_key: u64,
    pub audio: &'a mut dyn Audio,
}

/// All providers for the composed countdown/jet/proximity/secondary cycle.
pub struct CycleServices<'a, 'r, 'c> {
    pub jet: crate::mecc_jet_owned::Services<'a, 'r>,
    pub collection: Services<'c>,
}
