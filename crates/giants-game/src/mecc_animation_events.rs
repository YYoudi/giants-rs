//! Shared registered-class event state with a real deferred-context provider.
//! This staged projection is not an interleaved primary or active FX update.
use crate::{
    actor_fields::Fields,
    animation_channels::Channels,
    animation_event_dispatch::{Anchor, Context, Request, State, Window},
    attached_effect_context::{Creation, Pool},
    audio_parameters::Random,
    procedural_aim::Matrix,
    resident_states::Id,
    specification::Prepared,
};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) struct Input<'a> {
    pub owner: Id,
    pub fields: &'a Fields,
    pub channels: &'a Channels,
    pub references: &'a [Matrix],
    pub affine: Option<[f32; 16]>,
    pub material: Material<'a>,
}
#[derive(Clone, Copy)]
pub(crate) enum Material<'a> {
    Unknown,
    Acquired(Option<&'a str>),
}

#[derive(Clone)]
pub(crate) struct Bank {
    specification: Arc<Prepared>,
    tracks: BTreeMap<u8, Vec<State>>,
    pistol: Option<crate::mecc_pistol_draw::Actions>,
}
impl std::fmt::Debug for Bank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeccEventBank")
            .field("tracks", &self.tracks.len())
            .finish()
    }
}
impl Bank {
    pub fn new(specification: Arc<Prepared>) -> Self {
        let tracks = specification
            .metadata
            .events
            .iter()
            .map(|track| (track.slot, track.events.iter().map(State::from).collect()))
            .collect();
        Self {
            specification,
            tracks,
            pistol: None,
        }
    }
    pub fn belongs_to(&self, specification: &Arc<Prepared>) -> bool {
        Arc::ptr_eq(&self.specification, specification)
    }
    pub fn kinds(&self, slot: u8) -> Result<Vec<u32>, String> {
        Ok(self
            .tracks
            .get(&slot)
            .ok_or("event track absent")?
            .iter()
            .map(|e| e.kind)
            .collect())
    }
    pub fn pistol(
        &mut self,
        catalog: &giants_formats::weapons::Catalog,
    ) -> Result<&mut crate::mecc_pistol_draw::Actions, String> {
        if self.pistol.is_none() {
            self.pistol = Some(crate::mecc_pistol_draw::Actions::new(catalog)?);
        }
        let actions = self.pistol.as_mut().unwrap();
        actions.bind(catalog)?;
        Ok(actions)
    }
    #[cfg(test)]
    pub fn pistol_kinds(&self) -> Option<[u32; 3]> {
        self.pistol
            .as_ref()
            .map(crate::mecc_pistol_draw::Actions::event_kinds)
    }
    /// Supplemental actions follow class events within the same ANM pass.
    pub fn dispatch_pistol(
        &mut self,
        input: Input<'_>,
        mut replace: impl FnMut(u8, i32) -> Result<(), String>,
    ) -> Result<(), String> {
        let slot =
            u8::try_from(input.channels[0][0]).map_err(|_| "pistol event clip outside table")?;
        let duration = self
            .specification
            .animations
            .get(&slot)
            .ok_or("pistol event descriptor absent")?
            .template
            .duration;
        let window = Window::classify(
            f32::from_bits(input.channels[0][2]),
            f32::from_bits(input.channels[0][1]),
            duration,
        )
        .map_err(|e| e.to_string())?;
        let context = Context {
            wet: input.fields.object_flags & 0x100000 != 0,
            material: match input.material {
                Material::Unknown => None,
                Material::Acquired(value) => value,
            },
            position: input.fields.position,
            world: None,
            references: &[],
            contact_scalar: 0.,
            cinematic: false,
        };
        self.pistol
            .as_mut()
            .ok_or("active pistol action bank absent")?
            .dispatch(window, &context, |reference, class| {
                if !context.wet && matches!(input.material, Material::Unknown) {
                    return Err("pistol event requires its actual contact material producer".into());
                }
                replace(reference, class)
            })
    }
    pub fn dispatch(
        &mut self,
        input: Input<'_>,
        contexts: &mut Pool<Id>,
        random: &mut Random,
    ) -> Result<u32, String> {
        let Input {
            owner,
            fields,
            channels,
            references,
            affine,
            material,
        } = input;
        let slot = u8::try_from(channels[0][0]).map_err(|_| "event clip outside source table")?;
        let duration = self
            .specification
            .animations
            .get(&slot)
            .ok_or("event descriptor absent")?
            .template
            .duration;
        let window = Window::classify(
            f32::from_bits(channels[0][2]),
            f32::from_bits(channels[0][1]),
            duration,
        )
        .map_err(|e| e.to_string())?;
        let points = references
            .iter()
            .map(|m| [m.values[3], m.values[7], m.values[11]])
            .collect::<Vec<_>>();
        let context = Context {
            wet: fields.object_flags & 0x100000 != 0,
            material: match material {
                Material::Unknown => None,
                Material::Acquired(value) => value,
            },
            position: fields.position,
            world: affine.map(|m| m[..12].try_into().unwrap()),
            references: &points,
            contact_scalar: 0.,
            cinematic: false,
        };
        let events = self.tracks.get_mut(&slot).ok_or("event table absent")?;
        for event in events.iter_mut() {
            let dispatched = event
                .dispatch(window, &context)
                .map_err(|e| e.to_string())?;
            if dispatched.fired && !context.wet && matches!(material, Material::Unknown) {
                return Err("dry event requires its actual contact material producer".into());
            }
            if dispatched.configuration_override.is_some() {
                return Err("event configuration service missing".into());
            }
            for request in dispatched.requests {
                match request {
                    Request::Effect {
                        flags: 0x18,
                        target,
                        anchor: Anchor::Position(position),
                    } => {
                        let (creation, _) =
                            contexts.prepare_positional_offset(target, position, random)?;
                        if target.is_some() && !matches!(creation, Creation::Prepared(_)) {
                            return Err("required positional event context absent".into());
                        }
                    }
                    Request::Effect {
                        flags: 0x14,
                        target,
                        anchor: Anchor::OwnerReference(reference),
                    } => {
                        let (creation, _) = contexts.prepare_offset(
                            target,
                            owner,
                            fields.object_flags,
                            reference as u32,
                            random,
                        )?;
                        if target.is_some() && !matches!(creation, Creation::Prepared(_)) {
                            return Err("required event effect context absent".into());
                        }
                    }
                    _ => return Err("event requires a real unsupported service".into()),
                }
            }
        }
        Ok(events.len() as u32)
    }
}
