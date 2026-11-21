//! Owned query/filter composition after the resource checkpoint. Prototype PS2
//! `kabuto_move` remains an unknown name route, not a signature/layout match.
//! Ray, bounds, clock, geometry and world publication remain required producers.
use crate::{
    kabuto_action_select::Scratch,
    kabuto_reference_target::State,
    resident_states::{Id, Session},
};

pub use crate::kabuto_resource_phase::Clock;
pub struct Geometry {
    pub height: f32,
    pub axis: [f32; 3],
}
pub trait Services {
    fn bounds(
        &mut self,
        session: &Session,
        target: Id,
    ) -> Result<crate::kabuto_reference_target::Bounds, String>;
    fn clock(&mut self, session: &Session, owner: Id) -> Result<Clock, String>;
    fn announce(&mut self, session: &mut Session, owner: Id) -> Result<(), String>;
    fn reset_marker(&mut self, session: &mut Session, owner: Id) -> Result<(), String>;
    fn parent(&mut self, session: &Session, target: Id) -> Result<Option<Id>, String>;
    fn geometry(&mut self, session: &Session, owner: Id) -> Result<Geometry, String>;
}
/// A snapshot of a completed component, not enduring lifetime/admission proof.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub owner: Id,
    pub number: u64,
    pub target: State<Id>,
    pub caller_scratch: Scratch,
}
/// Empty values mean no invocation or producer, never successful native setup.
#[derive(Default)]
pub(crate) struct Phase {
    pub targets: Option<Targets>,
    pub entered: Option<u64>,
    pub running: bool,
    pub completed: Option<u64>,
    pub caller: Option<Scratch>,
    pub clock_word: Option<u32>,
}
#[derive(Clone, Copy)]
pub(crate) struct Targets {
    pub raw: Option<Id>,
    pub retained: Option<Id>,
}
