//! Owned Mecc render-model selection after the completed outer camera pass.
//! Quality/forced-slot settings are explicit inspection inputs. Visibility,
//! part/material admission and actual graphics submission remain separate.
use crate::{
    mecc_world_camera::{Pass, Publication},
    resident_states::{Id, Session},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub quality: usize,
    pub forced: i32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            quality: 2,
            forced: -1,
        }
    }
}

pub struct Selection {
    source: Id,
    number: u64,
    slot: usize,
    specification: Arc<crate::specification::Prepared>,
    positions: Option<Vec<[f32; 3]>>,
}
impl Selection {
    pub fn source(&self) -> Id {
        self.source
    }
    pub fn number(&self) -> u64 {
        self.number
    }
    pub fn slot(&self) -> usize {
        self.slot
    }
    pub fn model(&self) -> Option<&crate::scene_resources::CpuModel> {
        self.specification
            .models
            .get(self.slot)
            .and_then(Option::as_ref)
    }
    pub fn positions(&self) -> Option<&[[f32; 3]]> {
        self.positions.as_deref()
    }
}

pub fn prepare(
    session: &mut Session,
    camera: &Publication<Id>,
    settings: Settings,
) -> Result<Selection, String> {
    let source = camera.source();
    let number = camera.frame();
    let actor = session.get(source).ok_or("draw source absent or foreign")?;
    if actor.class != 2
        || actor.seed.storage.profile() != crate::state_storage::Profile::Debug
        || actor.seed.storage.state() != 2
        || actor.attachment.is_some()
        || camera.pass() != Pass::Outer
        || session.actor_world_tick(source)?.phase() != crate::actor_world_tick::Phase::Finished
        || session.mecc_physical_cycle(source)?.number != number
        || session.mecc_physical_cycle_failure(source).is_some()
        || settings.quality > 3
        || settings.forced >= 10
    {
        return Err(
            "draw model requires its completed owned Mecc/camera frame and settings".into(),
        );
    }
    let fields = actor
        .construction_fields
        .as_ref()
        .ok_or("draw fields absent")?;
    let specification = session
        .specification(source)
        .ok_or("draw specification absent")?
        .clone();
    let slot = specification
        .metadata
        .selection
        .table()?
        .select_positioned(
            crate::lod::PositionInput {
                object_position: fields.position,
                camera_position: camera.output().viewport_origin,
                offset: 0.,
                camera_factor: crate::lod::threshold(camera.output().pose.vertical_fov, 1.)?,
                object_scale: fields.scale,
                quality: settings.quality,
                forced: settings.forced,
                allow_missing: false,
            },
            crate::arithmetic::Precision::Double53,
        )?;
    let slot = usize::try_from(slot).map_err(|_| "draw selector returned a negative model")?;
    let epoch = actor.mecc_animation_stamp;
    let positions = if specification.models.get(slot).is_some_and(Option::is_some) {
        session.prepare_actor_positions(source, slot, epoch, false)?;
        Some(
            session
                .actor_model_positions(source, slot)?
                .ok_or("selected draw positions absent")?
                .to_vec(),
        )
    } else {
        None
    };
    Ok(Selection {
        source,
        number,
        slot,
        specification,
        positions,
    })
}
