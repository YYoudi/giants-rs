//! Source-declared Delphi mission staging through the acquired cold factory.
//! Projection, relocation equivalence and the live controller stay explicit.
use crate::{
    animation_channels::Blending, delphi_player::Attempt, delphi_special, instances::Plan,
    player_start::Declared, resident_states::Session,
};
use giants_formats::{
    global_data::GlobalData, mission::Mission, weapons::Catalog, world::WorldConfig,
};

pub struct Factory {
    start: Declared,
    special: delphi_special::Factory,
}
impl Factory {
    pub fn load(
        plan: &Plan,
        world: &WorldConfig,
        mission: &Mission,
        globals: &GlobalData,
        weapons: &Catalog,
    ) -> Result<Self, String> {
        let start = crate::player_start::declared_for_kind(plan, world, mission, 3)?;
        let special = delphi_special::Factory::load(plan.resources(), globals, weapons)?;
        Ok(Self { start, special })
    }
    pub fn declared_start(&self) -> &Declared {
        &self.start
    }
    pub fn construct_declared(&self, session: &mut Session, blending: Blending) -> Attempt {
        self.special
            .construct_declared_player_fields(session, &self.start, blending)
    }
}
