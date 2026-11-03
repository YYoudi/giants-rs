//! Required services for the acquired normal offline Mecc primary profile.
//! This does not admit actors, run their children or acknowledge a whole world.
pub struct Services<'a, 'r, 'c> {
    pub animation: crate::mecc_animation_phase::Services<'a>,
    pub jet: crate::mecc_jet_owned::Services<'a, 'r>,
    /// Required if the actual periodic scan selects an eligible owned item.
    pub collection: Option<crate::mecc_collection::Services<'c>>,
}
