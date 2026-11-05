//! Read-only presentation of a completed owned Mecc update. No movement,
//! animation clock, reference update or completion marker is produced here.
use crate::resident_states::{Id, Session};

pub struct Frame {
    pub owner: Id,
    pub number: u64,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub mass: f32,
    pub health: crate::mecc_damage::State,
    pub jet: crate::mecc_jet::State,
    pub object_flags: u32,
    pub children: Vec<crate::mecc_jet_presentation::Frame>,
    pub world_weapon: Option<crate::mecc_wield_pose::Frame>,
    /// Inspection view only; this does not admit a firing phase or a subjective muzzle.
    pub world_aim: Option<crate::mecc_aim::Target>,
    pub angles: [f32; 3],
    pub scale: f32,
    pub look_pitch: f32,
    pub look_height: f32,
    pub model_rates: [f32; 4],
    pub channels: crate::animation_channels::Channels,
    pub positions: Vec<[f32; 3]>,
    pub geometry: crate::catalog_registry::Location,
    pub affine: [f32; 16],
    pub camera: crate::mecc_camera::Pose,
    /// Produced by the owning driver after its actual primary and held pass.
    pub camera_cache: Option<crate::mecc_camera_update::Output>,
    pub references: Vec<crate::procedural_aim::Matrix>,
    pub stamp: u32,
}

/// The session's shared model bank is updated after the real owned primary;
/// actor state, clocks, references and completion markers remain unchanged.
pub fn sample(
    session: &mut Session,
    actor: Id,
    number: u64,
    first_person: bool,
) -> Result<Frame, String> {
    if session.actor_world_tick(actor)?.phase() != crate::actor_world_tick::Phase::Finished
        || session.actor_world_flags(actor)? & 0x40 == 0
        || session.mecc_physical_cycle_failure(actor).is_some()
    {
        return Err("Mecc presentation requires a completed owned primary/world frame".into());
    }
    let view = session.mecc_physical_cycle(actor)?;
    if view.number != number || number == 0 {
        return Err("Mecc presentation frame is absent or stale".into());
    }
    let resident = session.get(actor).ok_or("Mecc presentation owner absent")?;
    let fields = resident
        .construction_fields
        .as_ref()
        .ok_or("Mecc presentation fields absent")?;
    let channels = resident
        .channels
        .ok_or("Mecc presentation channels absent")?;
    if channels != fields.channels {
        return Err("Mecc presentation channel ownership differs".into());
    }
    let spec = session
        .specification(actor)
        .ok_or("Mecc presentation specification absent")?;
    let stamp = resident.mecc_animation_stamp;
    let payload = resident
        .seed
        .storage
        .data()
        .ok_or("Mecc presentation payload absent")?;
    if payload.len() != 524 {
        return Err("Mecc presentation payload profile changed".into());
    }
    let word = |at| u32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
    let scalar = |at| f32::from_bits(word(at));
    let health = session.mecc_health(actor)?;
    let mass = session.mecc_physics(actor)?.mass();
    let jet = crate::mecc_jet::State {
        fuel: scalar(0x4c),
        boost: scalar(0xc0),
        boost_cooldown: scalar(0xc4),
        ignition_delay: scalar(0x138),
        boost_active: word(0xc8) != 0,
    };
    let geometry = spec
        .models
        .first()
        .and_then(Option::as_ref)
        .ok_or("Mecc presentation primary model absent")?
        .geometry_source
        .clone();
    let view_affine = view.affine.ok_or("Mecc presentation affine absent")?.matrix;
    let affine = session.actor_current_affine(actor)?.matrix;
    if affine.map(f32::to_bits) != view_affine.map(f32::to_bits) {
        return Err("Mecc presentation affine differs from its completed cycle".into());
    }
    let camera = crate::mecc_camera::pose(
        fields.position,
        fields.angles[0],
        view.look_pitch,
        first_person,
    )?;
    let references = session.current_reference_matrices(actor)?.to_vec();
    let mut frame = Frame {
        owner: actor,
        number,
        position: fields.position,
        velocity: view.velocity,
        mass,
        health,
        jet,
        object_flags: fields.object_flags,
        children: Vec::new(),
        world_weapon: None,
        world_aim: None,
        angles: fields.angles,
        scale: fields.scale,
        look_pitch: view.look_pitch,
        look_height: f32::from_bits(fields.configuration[22]),
        model_rates: [
            view.model_rates[0],
            view.model_rates[1],
            view.model_rates[2],
            view.angular_rate,
        ],
        channels,
        positions: Vec::new(),
        geometry,
        affine,
        camera,
        camera_cache: None,
        references,
        stamp,
    };
    session.prepare_actor_positions(actor, 0, stamp, false)?;
    frame.positions = session
        .actor_model_positions(actor, 0)?
        .ok_or("Mecc presentation positions were not produced")?
        .to_vec();
    Ok(frame)
}
