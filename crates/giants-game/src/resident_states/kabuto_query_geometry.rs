use super::*;
use crate::{kabuto_geometry_mode as mode, kabuto_query_geometry as geometry};

fn word(p: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(p[at..at + 4].try_into().unwrap())
}
impl Session {
    /// Current owned scalar/mode state, including notification prefix stores.
    /// This diagnostic read supplies no query/frame or primary admission.
    pub fn kabuto_query_geometry_mode_state(&self, owner: Id) -> Result<mode::State<Id>, String> {
        self.kabuto_reference_state(owner)?;
        let a = self.get(owner).unwrap();
        let g = a.kabuto_group.as_ref().unwrap();
        let p = a.seed.storage.data().unwrap();
        if word(p, 0x30c) != 0
            || g.query_geometry
                .target
                .is_some_and(|id| id.ticket() != owner.ticket() || self.get(id).is_none())
        {
            return Err("Kabuto geometry target is untyped, absent or foreign".into());
        }
        Ok(mode::State {
            flags: word(p, 0),
            duration: f32::from_bits(word(p, 0x224)),
            mode: p[0x228],
            scalar: g
                .query_geometry
                .header
                .ok_or("Kabuto geometry header is retired")?
                .weight,
            target: g.query_geometry.target,
            target_status: word(p, 0x310),
        })
    }
    fn publish_kabuto_query_mode(&mut self, owner: Id, state: mode::State<Id>) {
        // Only called inside the checked synchronous mode phase. Its observers
        // receive an immutable Session and cannot replace actor/group storage.
        let a = self.residents.get_mut(&owner.serial).unwrap();
        let p = a.seed.storage.data_mut().unwrap();
        for (at, value) in [
            (0, state.flags),
            (0x224, state.duration.to_bits()),
            (0x310, state.target_status),
        ] {
            p[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        p[0x228] = state.mode;
        let g = a.kabuto_group.as_mut().unwrap();
        g.query_geometry.header.as_mut().unwrap().weight = state.scalar;
        g.query_geometry.target = state.target;
    }
    pub(crate) fn captured_kabuto_query_geometry(
        &self,
        owner: Id,
        resources: &Resources,
        number: u64,
    ) -> Result<geometry::View, String> {
        self.captured_kabuto_query_feedback(owner, resources, number)?;
        let phase = &self.kabuto_group(owner).unwrap().query_geometry;
        if phase.running || phase.completed != Some(number) || phase.header.is_none() {
            return Err("Kabuto query geometry is not completed on this owner/frame".into());
        }
        self.kabuto_query_geometry_mode_state(owner)?;
        phase
            .view
            .clone()
            .ok_or_else(|| "Kabuto query geometry view absent".into())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn advance_kabuto_query_geometry(
        &mut self,
        owner: Id,
        resources: &Resources,
        number: u64,
        extra: &crate::kabuto_auxiliary::ExtraFactory,
        frame: geometry::Frame<'_>,
        animation: &mut crate::actor_animation_phase::Services<'_>,
        services: &mut impl geometry::Services,
    ) -> Result<geometry::View, String> {
        self.captured_kabuto_query_feedback(owner, resources, number)?;
        let query = self.captured_kabuto_query_frame(owner, resources, number)?;
        let g = self.kabuto_group(owner).unwrap();
        if g.query_geometry.entered.is_some()
            || g.query_geometry.running
            || g.action_select_entered.is_some()
            || g.held_reference_entered.is_some()
            || g.animation_commit_entered.is_some()
            || g.primary_tail_entered.is_some()
        {
            return Err("Kabuto query geometry requires unconsumed ordered work".into());
        }
        if !frame.global_seconds.is_finite() || frame.global_seconds < 0. {
            return Err("Kabuto query geometry frame outside finite clock profile".into());
        }
        let source_word = self
            .get(owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap()
            .dispatch_context;
        if frame.network.owner_word != source_word {
            return Err(
                "Kabuto mode notification context differs from its actual actor word".into(),
            );
        }
        let mut state = self.kabuto_query_geometry_mode_state(owner)?;
        let controls = u32::from_le_bytes(g.motion.commands[4..8].try_into().unwrap());
        let target = query
            .target
            .accepted
            .map(|id| {
                let a = self.get(id).ok_or("Kabuto mode target absent")?;
                let f = a
                    .construction_fields
                    .as_ref()
                    .ok_or("Kabuto mode target fields absent")?;
                Ok::<_, String>(mode::Target {
                    identity: id,
                    owner_word: f.dispatch_context,
                })
            })
            .transpose()?;
        let phase = &mut self
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap()
            .query_geometry;
        phase.entered = Some(number);
        phase.running = true;
        let result: Result<geometry::View, String> = (|| {
            mode::advance(
                &mut state,
                mode::Input {
                    control_flags: controls,
                    delta: frame.global_seconds,
                    target,
                },
                frame.network,
                |event, seen| {
                    self.publish_kabuto_query_mode(owner, *seen);
                    services.notify(self, owner, event);
                },
            )
            .map_err(str::to_owned)?;
            self.publish_kabuto_query_mode(owner, state);
            let g = self
                .residents
                .get_mut(&owner.serial)
                .unwrap()
                .kabuto_group
                .as_mut()
                .unwrap();
            g.query_geometry.mode_completed = Some(number);
            let model = g.motion.model;
            let contact = g.contact;
            let affine = self
                .get(owner)
                .unwrap()
                .affine_cache
                .as_ref()
                .map(|a| a.value);
            let transition = self.advance_kabuto_geometry_transition_with_world(
                owner,
                extra,
                crate::kabuto_geometry_transition::Frame {
                    mode: state.mode,
                    global_seconds: frame.global_seconds,
                    areas: frame.areas,
                    model,
                    affine,
                    support: contact.support,
                    support_part: contact.support_part,
                    materials: frame.materials,
                },
                animation,
                |session, event| services.sound(session, event),
            )?;
            // Native header disabling precedes selected-owner and camera reads.
            self.residents
                .get_mut(&owner.serial)
                .unwrap()
                .kabuto_group
                .as_mut()
                .unwrap()
                .query_geometry
                .header
                .as_mut()
                .unwrap()
                .enabled = false;
            let controlled = services.controlled(self)?;
            if controlled.is_some_and(|id| id.ticket() != owner.ticket() || self.get(id).is_none())
            {
                return Err("Kabuto geometry controlled owner is foreign or absent".into());
            }
            let current = self.kabuto_query_geometry_mode_state(owner)?;
            let camera = if controlled == Some(owner) && current.mode >= 1 {
                Some(services.camera(self, owner)?)
            } else {
                None
            };
            let previous = self
                .kabuto_group(owner)
                .unwrap()
                .query_geometry
                .header
                .unwrap();
            let header = crate::kabuto_geometry_state::prepare_with::<
                crate::kabuto_geometry_state::Body,
                String,
            >(previous, current.mode, camera, || {
                let affine = self.actor_current_affine(owner)?;
                let attachment = services.attachment(self, owner)?;
                let f = self
                    .get(owner)
                    .unwrap()
                    .construction_fields
                    .as_ref()
                    .unwrap();
                let scale = f.scale;
                let target_bounds = current
                    .target
                    .map(|id| {
                        let bounds = services.bounds(self, id)?;
                        Ok::<_, String>((bounds.lower, bounds.upper))
                    })
                    .transpose()?;
                Ok(crate::kabuto_geometry_state::Body {
                    matrix: crate::procedural_aim::Matrix {
                        flags: affine.flags,
                        values: affine.matrix,
                    },
                    attachment,
                    scale,
                    target_bounds,
                })
            })?;
            self.residents
                .get_mut(&owner.serial)
                .unwrap()
                .kabuto_group
                .as_mut()
                .unwrap()
                .query_geometry
                .header = Some(header);
            let mode = self.kabuto_query_geometry_mode_state(owner)?.mode;
            Ok(geometry::View {
                owner,
                number,
                mode,
                header,
                transition,
                next: if mode == 0 {
                    geometry::Next::ReactionAdmission
                } else {
                    geometry::Next::PrimaryTailRequired
                },
            })
        })();
        let g = self
            .residents
            .get_mut(&owner.serial)
            .unwrap()
            .kabuto_group
            .as_mut()
            .unwrap();
        g.query_geometry.running = false;
        match &result {
            Ok(view) => {
                g.query_geometry.completed = Some(number);
                g.query_geometry.view = Some(view.clone());
            }
            Err(error) => {
                g.geometry_transition_failure
                    .get_or_insert_with(|| error.clone());
            }
        }
        result?;
        self.captured_kabuto_query_geometry(owner, resources, number)
    }
}
