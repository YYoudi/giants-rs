//! Explicit owner/profile inspection after a supplied native mode-1 command.
//! No floor/chase/mesh provider is fabricated. This is the unclipped position
//! kernel, not ordinary camera scheduling, startup or initializer admission.
use crate::{
    camera_initial, camera_mode, camera_owner, camera_profile, camera_projection, kabuto_size,
    mecc_camera,
    resident_states::{Id, Session},
};

pub struct Prepared {
    pub owner: Id,
    pub pose: mecc_camera::Pose,
    pub projection: camera_projection::Output,
    signature: [u32; 11],
}
fn signature(session: &Session, owner: Id) -> Result<[u32; 11], String> {
    let actor = session
        .get(owner)
        .ok_or("Kabuto camera inspection owner absent or foreign")?;
    let group = session
        .kabuto_group(owner)
        .ok_or("Kabuto camera inspection group absent")?;
    if actor.class != 1
        || actor.seed.storage.profile() != crate::state_storage::Profile::Debug
        || actor.seed.storage.state() != 1
        || actor.attachment.is_some()
        || group.primary_started
        || group.cleaned_up()
        || !session.has_cold_kabuto_player_fields(owner)
    {
        return Err("Kabuto camera inspection requires its owned cold player".into());
    }
    let f = actor
        .construction_fields
        .as_ref()
        .ok_or("Kabuto camera inspection header absent")?;
    if f.object_flags & 0x8000 != 0 {
        return Err("Kabuto camera inspection owner marked".into());
    }
    let gain = kabuto_size::camera_gain(
        actor
            .seed
            .storage
            .data()
            .ok_or("Kabuto camera payload absent")?,
    )?;
    Ok([
        f.position[0].to_bits(),
        f.position[1].to_bits(),
        f.position[2].to_bits(),
        f.angles[0].to_bits(),
        session.camera_look_angle(owner)?.to_bits(),
        f.scale.to_bits(),
        f.object_flags,
        f.class as u32,
        u32::from(actor.seed.storage.state()),
        f.control_flags,
        gain.to_bits(),
    ])
}
impl Prepared {
    /// The caller supplies a compatible external executable and an explicit
    /// inspected owner. Cold metadata is an experiment context, not proof of
    /// the actual state after native mission-entry writers or a warm reset.
    pub fn after_mode_one(
        session: &mut Session,
        owner: Id,
        executable: &std::path::Path,
        viewport: [f32; 2],
    ) -> Result<Self, String> {
        signature(session, owner)?;
        let catalog = camera_profile::ExecutableCatalog::read(executable)?;
        let cold = camera_initial::Cold::read(executable)?;
        let owners = camera_owner::State::new(Some(owner), cold.reference_status);
        let mut modes = cold.modes()?;
        let mut projection = None;
        session.change_camera_mode(
            &mut modes,
            &owners,
            camera_mode::Command {
                request: 1,
                extended: cold.extended_modes,
                controlled: Some(owner),
                scaled_input: cold.scaled_input,
            },
            &catalog,
            &mut |fov| {
                projection = Some(camera_projection::build(fov, cold.projection(viewport))?);
                Ok(())
            },
        )?;
        let record = catalog.record(camera_profile::from_session(
            session,
            &owners,
            &modes,
            Some(owner),
            cold.scaled_input,
        )?)?;
        let signature = signature(session, owner)?;
        let f = session
            .get(owner)
            .unwrap()
            .construction_fields
            .as_ref()
            .unwrap();
        let pose = record
            .camera
            .for_kabuto(f32::from_bits(signature[10]))?
            .unclipped_pose(f.position, f.angles[0], session.camera_look_angle(owner)?)?;
        Ok(Self {
            owner,
            pose,
            projection: projection.ok_or("Kabuto projection request absent")?,
            signature,
        })
    }
    pub fn current(&self, session: &Session) -> bool {
        signature(session, self.owner).is_ok_and(|s| s == self.signature)
    }
}
