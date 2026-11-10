//! Acquired handoff between the current reference and two held slots.
//! Captured caller decisions/counters and all required backends remain explicit;
//! this does not produce those decisions or admit a live Kabuto controller.
use crate::{
    actor_attachment_bind::Input as Binding, actor_reference_detach,
    kabuto_contact::Request as ActionRequest, kabuto_geometry_mode::NotificationContext,
    kabuto_reference_release::Notice,
};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug)]
pub struct State<K> {
    pub reference: Option<K>,
    pub held: [Option<K>; 2],
    pub reference_scale: f32,
    pub held_scales: [f32; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// Full caller-local decision; not an inferred current actor action.
    pub decision: u32,
    /// Signed caller-local counters; their live animation producer is unbound.
    pub previous: i32,
    pub current: i32,
    pub finished: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct CreatedNotice {
    pub scale: f32,
    pub owner_word: u32,
    pub type_id: u32,
    pub parent_reference: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    Processed,
    ContinueCaller,
}

/// Required mutable backend boundaries for the Session composition. These
/// requests are not implemented by a default successful provider.
#[derive(Clone, Copy, Debug)]
pub enum Request {
    Construct {
        parameter: u32,
        type_id: u32,
    },
    Created(CreatedNotice),
    Released(Notice<crate::resident_states::Id>),
    Bind {
        child: crate::resident_states::Id,
        input: Binding,
    },
    ResetState18(crate::resident_states::Id),
    Action(crate::kabuto_contact::Request<crate::resident_states::Id>),
    SetOwner {
        actor: crate::resident_states::Id,
        word: u32,
    },
}

#[derive(Clone, Copy, Debug)]
pub enum Response {
    Complete,
    Actor(crate::resident_states::Id),
}

pub trait World {
    fn session_mode(&self, session: &crate::resident_states::Session) -> Result<u32, String>;
    fn network<'a>(
        &'a self,
        session: &crate::resident_states::Session,
        source: crate::resident_states::Id,
    ) -> Result<NotificationContext<'a>, String>;
    fn owner_word(
        &self,
        session: &crate::resident_states::Session,
        actor: crate::resident_states::Id,
    ) -> Result<u32, String>;
    /// Observe the actual registered owners. Completed effects survive errors;
    /// construction must return an actor owned by this Session. Binding,
    /// transport, state18 reset and action bodies remain required services.
    fn execute(
        &mut self,
        session: &mut crate::resident_states::Session,
        source: crate::resident_states::Id,
        request: Request,
    ) -> Result<Response, String>;
}

pub trait Services<K>: actor_reference_detach::Services<K, State<K>> {
    fn session_mode(&self) -> Result<u32, String>;
    fn notification_context(&self, source: K) -> Result<NotificationContext<'_>, String>;
    fn physical_flags(&self, actor: K) -> Result<u32, String>;
    fn type_id(&self, actor: K) -> Result<u32, String>;
    fn owner_word(&self, actor: K) -> Result<u32, String>;
    fn set_owner_word(
        &mut self,
        published: &mut State<K>,
        actor: K,
        value: u32,
    ) -> Result<(), String>;
    fn construct(
        &mut self,
        published: &mut State<K>,
        source: K,
        parameter: u32,
        type_id: u32,
    ) -> Result<K, String>;
    /// The acquired request declares 21 transport bytes, but only thirteen
    /// initialized bytes are established here. Remaining bytes stay unknown;
    /// this semantic request does not invent packet padding or delivery.
    fn notify_created(
        &mut self,
        published: &mut State<K>,
        source: K,
        notice: CreatedNotice,
    ) -> Result<(), String>;
    fn notify_release(
        &mut self,
        published: &mut State<K>,
        source: K,
        notice: Notice<K>,
    ) -> Result<(), String>;
    fn bind(
        &mut self,
        published: &mut State<K>,
        source: K,
        child: K,
        input: Binding,
    ) -> Result<(), String>;
    fn action(&mut self, published: &mut State<K>, request: ActionRequest<K>)
    -> Result<(), String>;
}

fn notifying<K: Copy>(source: K, services: &impl Services<K>) -> Result<bool, String> {
    if services.session_mode()? != 4 {
        return Ok(false);
    }
    services
        .notification_context(source)?
        .eligible()
        .map_err(str::to_owned)
}

/// Caller storage must be published before each service and reread afterwards.
/// Services may mutate the supplied state; final stores retain the original
/// selected slot and captured replacement identity. Errors preserve effects.
pub fn advance<K: Copy>(
    state: &mut State<K>,
    source: K,
    input: Input,
    services: &mut impl Services<K>,
) -> Result<Next, String> {
    if input.decision == 11 {
        if input.current < 4800 || state.reference.is_none() {
            return Ok(Next::ContinueCaller);
        }
        let Some(index) = state.held.iter().position(Option::is_none) else {
            return Ok(Next::ContinueCaller);
        };
        state.held[index] = state.reference;
        state.held_scales[index] = state.reference_scale;
        state.reference = None;
        let held = state.held[index].ok_or("held reference absent after publication")?;
        if services.session_mode()? == 4 && services.physical_flags(held)? & 1 != 0 {
            let type_id = services.type_id(held)?;
            let scale = state.held_scales[index];
            let created = services.construct(state, source, 0, type_id)?;
            if notifying(source, services)? {
                let owner_word = services.owner_word(created)?;
                services.notify_created(
                    state,
                    source,
                    CreatedNotice {
                        scale,
                        owner_word,
                        type_id,
                        parent_reference: index as u8,
                    },
                )?;
            } else {
                services.set_owner_word(state, created, 0)?;
            }
            services.bind(
                state,
                source,
                created,
                Binding {
                    scale_factor: scale,
                    parent_reference: index as u32,
                    notify: false,
                },
            )?;
            let scale = state.held_scales[index];
            let old = state.held[index].ok_or("held reference absent before release")?;
            if notifying(source, services)? {
                let target_owner_word = services.owner_word(old)?;
                services.notify_release(
                    state,
                    source,
                    Notice {
                        captured_target: old,
                        captured_scale: scale,
                        target_owner_word,
                    },
                )?;
            }
            actor_reference_detach::advance(old, scale, state, services)?;
            let old = state.held[index].ok_or("held reference absent before action")?;
            if services.session_mode()? != 4 || notifying(source, services)? {
                services.action(
                    state,
                    ActionRequest {
                        action: 5,
                        amount: 10000.,
                        owner: Some(source),
                        peer: old,
                        trailing: [0; 2],
                    },
                )?;
            }
            state.held[index] = Some(created);
        } else {
            services.bind(
                state,
                source,
                held,
                Binding {
                    scale_factor: 1.,
                    parent_reference: index as u32,
                    notify: true,
                },
            )?;
        }
        return Ok(Next::Processed);
    }
    if input.decision != 10 {
        return Ok(Next::ContinueCaller);
    }
    let Some(index) = state.held.iter().position(Option::is_some) else {
        return Ok(Next::ContinueCaller);
    };
    if input.previous < 4160 && input.current >= 4160 {
        let held = state.held[index].ok_or("held reference absent before return")?;
        state.reference = Some(held);
        state.reference_scale = state.held_scales[index];
        services.bind(
            state,
            source,
            held,
            Binding {
                scale_factor: 1.,
                parent_reference: 3,
                notify: true,
            },
        )?;
        state.held[index] = None;
    }
    Ok(if input.finished {
        Next::ContinueCaller
    } else {
        Next::Processed
    })
}
