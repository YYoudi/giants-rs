//! Owned pre-physical target publication. Required providers can borrow the
//! actual Session; neither geometry nor camera/activation services are implied.
use super::*;
use crate::delphi_local_target as target;

/// Bind required services against the actual Session. Owner input fields stay
/// fixed during the caller; point/link outputs use the supplied mutable values.
/// External effects remain owned by Session, including after an error.
pub trait Providers {
    type Bound<'a>: target::World<Id, Id>
    where
        Self: 'a;
    fn bind<'a>(
        &'a mut self,
        session: &'a mut Session,
        owner: Id,
        number: u64,
    ) -> Result<Self::Bound<'a>, String>;
}

pub struct Context<'a> {
    pub number: u64,
    /// Actual selected identity, including a different class or no player.
    pub current_player: Option<Id>,
    pub suppressed: bool,
    /// Incoming shared display word; no constructor/default value is inferred.
    pub terrain_selected: u32,
    pub globals: &'a giants_formats::global_data::GlobalData,
    pub exponent: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Running,
    Prepared,
    Consumed,
    Failed,
}
#[derive(Clone, Debug)]
pub struct View {
    pub number: u64,
    pub phase: Phase,
    pub state: target::State<Id, Id>,
    pub failure: Option<String>,
}
impl View {
    pub(super) fn references(&self, id: Id) -> bool {
        [
            self.state.candidate,
            self.state.highlighted,
            self.state.pending,
            self.state.item_target,
            self.state.proxy,
        ]
        .contains(&Some(id))
    }
    pub(super) fn check_physical(&self, number: u64) -> Result<(), String> {
        if self.phase != Phase::Prepared || self.failure.is_some() || self.number != number {
            return Err("Delphi physical entry requires its prepared local target phase".into());
        }
        if [
            self.state.candidate,
            self.state.highlighted,
            self.state.pending,
            self.state.item_target,
            self.state.proxy,
        ]
        .into_iter()
        .any(|id| id.is_some())
        {
            return Err("Delphi physical target/proxy relationship consumers unprepared".into());
        }
        Ok(())
    }
}

fn indexed(body: &[u8], base: i64, index: i32, stride: i64) -> Result<u32, String> {
    let offset = base + i64::from(index) * stride;
    let offset = usize::try_from(offset).map_err(|_| "Delphi target selection before payload")?;
    let bytes = body
        .get(offset..offset + 4)
        .ok_or("Delphi target selection beyond payload")?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

impl Session {
    /// Acquired visibility store on an actual constructed proxy. This neither
    /// prepares its matrix nor admits its target relationship or renderer route.
    pub fn hide_delphi_target_proxy(&mut self, proxy: Id) -> Result<(), String> {
        if proxy_invalid(self, Some(proxy)) {
            return Err("Delphi target visibility outside acquired proxy profile".into());
        }
        self.residents
            .get_mut(&proxy.serial)
            .unwrap()
            .construction_fields
            .as_mut()
            .ok_or("Delphi proxy fields absent")?
            .object_flags |= 0x40000000;
        Ok(())
    }
    pub fn delphi_local_target(&self, owner: Id) -> Option<&View> {
        self.get(owner)?.delphi_local_target.as_ref()
    }
    /// Execute the real caller kernel over this owner's current fields. This
    /// opt-in component precedes the next physical step; it is not a full primary.
    /// Prefix writes and provider effects survive errors, without replay. The
    /// existing physical entry consumes a completed, link-free phase once; linked
    /// consumers and movement activation remain required services.
    pub fn advance_delphi_local_target(
        &mut self,
        owner: Id,
        context: Context<'_>,
        providers: &mut impl Providers,
    ) -> Result<View, String> {
        if self.delphi_target_running {
            return Err("Delphi local target phase is already running".into());
        }
        if let Some(current) = context.current_player {
            self.get(current)
                .ok_or("Delphi selected player absent or foreign")?;
        }
        let actor = self
            .get(owner)
            .ok_or("Delphi target owner absent or foreign")?;
        let cycle = actor
            .delphi_physical
            .as_ref()
            .ok_or("Delphi physical cycle unprepared")?;
        if actor.class != 3
            || actor.seed.storage.state() != 3
            || actor.seed.storage.profile() != Profile::Debug
            || actor.attachment.is_some()
            || actor.presentation_parent.is_some()
            || cycle.view.failure.is_some()
            || cycle.view.number.checked_add(1) != Some(context.number)
            || actor.delphi_local_target.as_ref().is_some_and(|v| {
                v.failure.is_some() || v.phase != Phase::Consumed || v.number != cycle.view.number
            })
        {
            return Err("Delphi local target entry is failed, replayed or not contiguous".into());
        }
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("Delphi target fields absent")?;
        let body = actor
            .seed
            .storage
            .data()
            .ok_or("Delphi target payload absent")?;
        if body.len() != 4096
            || [0xc, 0x50, 0xf44, 0xf48, 0xf8c]
                .into_iter()
                .any(|at| crate::delphi_physical::word(body, at) != 0)
        {
            return Err("Delphi target relationships require typed owned slots".into());
        }
        let word = |at| crate::delphi_physical::word(body, at);
        let scalar = |at| f32::from_bits(word(at));
        let point = |at| std::array::from_fn(|i| scalar(at + i * 4));
        let old = actor.delphi_local_target.as_ref().map(|v| &v.state);
        let mut state = target::State {
            action_available: word(0xf3c),
            item_available: word(0xf38),
            movement_ready: word(0xf40),
            point_available: word(0xf58),
            highlighted: old.and_then(|s| s.highlighted),
            item_target: old.and_then(|s| s.item_target),
            terrain_selected: context.terrain_selected,
            cooldown: scalar(0xda8),
            controls: fields.control_flags,
            proxy: old.and_then(|s| s.proxy),
            candidate: old.and_then(|s| s.candidate),
            pending: old.and_then(|s| s.pending),
            point: point(0x1c),
            terrain_point: point(0xf4c),
            aim_point: point(0xfc4),
        };
        let local = context.current_player == Some(owner);
        let enabled = local
            && fields.response_flags & 0x04000000 == 0
            && !context.suppressed
            && word(0x5c) != 0x4d;
        let selected = word(0xdb4) as i32;
        let selected_item = word(0xdb0) as i32;
        let input = target::Input {
            local_owner: local,
            response_flags: fields.response_flags,
            suppressed: context.suppressed,
            animation: word(0x5c) as i32,
            selected,
            selected_kind: if enabled {
                indexed(body, 0xd70, selected, 8)? as i32
            } else {
                0
            },
            selected_item,
            item_kind: if enabled {
                indexed(body, 0xd3c, selected_item, 12)? as i32
            } else {
                0
            },
            pending_seconds: scalar(0xf98),
            position: fields.position,
            scale: fields.scale,
            reference_scale: scalar(0xfac),
            height_limit: f32::from_bits(fields.configuration[22]),
            reserve: scalar(0x28),
            readiness: crate::delphi_target_readiness::Parameters::from_globals(
                context.globals,
                context.exponent,
            )?,
        };
        self.residents
            .get_mut(&owner.serial)
            .unwrap()
            .delphi_local_target = Some(View {
            number: context.number,
            phase: Phase::Running,
            state: state.clone(),
            failure: None,
        });
        self.delphi_target_running = true;
        let result = if !local {
            Ok(())
        } else {
            match providers.bind(self, owner, context.number) {
                Ok(mut world) => target::advance(&mut state, input, &mut world),
                Err(error) => Err(error),
            }
        };
        self.delphi_target_running = false;
        // A provider can cancel the mission. Never restore a retired owner or
        // attach its old state to a newly constructed actor of the same class.
        let actor = self
            .get(owner)
            .ok_or("Delphi target owner retired during its phase")?;
        if actor.class != 3
            || actor.seed.storage.state() != 3
            || actor.seed.storage.profile() != Profile::Debug
            || actor
                .seed
                .storage
                .data()
                .is_none_or(|body| body.len() != 4096)
        {
            return Err("Delphi target owner changed its acquired storage during its phase".into());
        }
        let identities = [
            state.candidate,
            state.highlighted,
            state.pending,
            state.item_target,
            state.proxy,
        ];
        let result = result.and_then(|()| {
            if identities
                .into_iter()
                .flatten()
                .any(|id| self.get(id).is_none())
            {
                Err("Delphi local target returned a foreign or retired identity".into())
            } else if proxy_invalid(self, state.proxy) {
                Err("Delphi local target returned an unacquired proxy role".into())
            } else {
                Ok(())
            }
        });
        let view = View {
            number: context.number,
            phase: if result.is_ok() {
                Phase::Prepared
            } else {
                Phase::Failed
            },
            state,
            failure: result.as_ref().err().cloned(),
        };
        let actor = self.residents.get_mut(&owner.serial).unwrap();
        actor.construction_fields.as_mut().unwrap().control_flags = view.state.controls;
        let body = actor.seed.storage.data_mut().unwrap();
        for (at, value) in [
            (0xf3c, view.state.action_available),
            (0xf38, view.state.item_available),
            (0xf40, view.state.movement_ready),
            (0xf58, view.state.point_available),
            (0xda8, view.state.cooldown.to_bits()),
        ] {
            body[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        for (at, point) in [
            (0x1c, view.state.point),
            (0xf4c, view.state.terrain_point),
            (0xfc4, view.state.aim_point),
        ] {
            for (i, value) in point.into_iter().enumerate() {
                body[at + i * 4..at + i * 4 + 4].copy_from_slice(&value.to_bits().to_le_bytes());
            }
        }
        actor.delphi_local_target = Some(view.clone());
        result?;
        Ok(view)
    }
}

fn proxy_invalid(session: &Session, proxy: Option<Id>) -> bool {
    proxy.is_some_and(|id| {
        session.get(id).is_none_or(|actor| {
            !actor.acquired_construction
                || actor.class != 0x3d4
                || actor.seed.storage.profile() != Profile::Debug
                || actor.seed.storage.state() != 78
        })
    })
}

#[cfg(test)]
mod tests;
