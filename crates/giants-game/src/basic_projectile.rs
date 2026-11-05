//! Acquired D basic construction for class38, declarations17/42. Launch,
//! collision callbacks, named-effect activation and live hit dispatch are
//! separate operations; a constructed projectile is not an eligible hit source.
use crate::{
    actor_fields::Fields,
    projectile_definition::Definition,
    state_storage::{Profile, Seed},
};
use std::sync::Arc;
pub const LIST_MASK: u32 = 0x21;
#[derive(Debug)]
pub struct Metadata {
    pub definition: Arc<Definition>,
    /// Source constructor argument stored in header18c; semantics remain a
    /// caller contract, not an inferred faction or native pointer.
    pub dispatch_context: u32,
    pub camera_origin: [f32; 3],
    pub initial_matrix: [f32; 16],
    /// Two native code pointers are represented by their acquired family,
    /// never copied into the byte payload as executable addresses.
    pub callback_family: CallbackFamily,
    pub constructor_effect: Option<String>,
    pub launch: Option<Launch>,
}
#[derive(Clone, Debug)]
pub struct Launch {
    /// Historical launch parent, retained after its temporary binding ends.
    pub parent: crate::resident_states::Id,
    pub attachment_parent: Option<crate::resident_states::Id>,
    /// Attribution written by the firing wrapper after launch. This is neither
    /// the launch parent nor the optional tracking actor.
    pub source: Option<crate::resident_states::Id>,
    /// Most recent supplied mesh-list query, not a completed hit callback.
    pub mesh_contacts: Option<crate::projectile_contacts::Query<crate::resident_states::Id>>,
    /// Callback attempt only; Applied does not complete effects or retirement.
    pub actor_hit: ActorHit,
    pub tracking_actor: Option<crate::resident_states::Id>,
    pub reference: i32,
    pub reference_selector: i32,
    pub straight: crate::projectile_launch::Straight,
    pub velocity: [f32; 3],
    pub previous_velocity: [f32; 3],
    pub ground: f32,
    pub ground_plane: [f32; 4],
    pub current_matrix: crate::procedural_aim::Matrix,
    pub bounds: [f32; 6],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorHit {
    Pending,
    Started,
    Suppressed,
    Applied { target: crate::resident_states::Id },
}
/// The caller supplies the current point returned by its reference service.
/// It must not substitute an unrelated camera origin for the muzzle point.
pub struct LaunchInput {
    pub parent: crate::resident_states::Id,
    pub tracking_actor: Option<crate::resident_states::Id>,
    pub reference: i32,
    pub resolved_reference: [f32; 3],
    pub target: [f32; 3],
}
/// Result of the acquired terrain portion of state77, before pose and hit
/// callbacks. A terrain hit is not an acknowledgement of actor damage.
pub struct TerrainStage {
    pub sweep: Option<crate::segment_queries::TerrainHit>,
    pub swept_endpoint: [f32; 3],
    pub physical: crate::physical_ground::Output,
    pub hit: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallbackFamily {
    BasicContactsPending,
}
pub struct Prepared {
    pub seed: Seed,
    pub fields: Fields,
    pub metadata: Metadata,
}
pub fn prepare(
    seed: &Seed,
    fields: &Fields,
    definition: Arc<Definition>,
    dispatch_context: u32,
    camera_origin: [f32; 3],
) -> Result<Prepared, String> {
    if seed.storage.profile() != Profile::Debug
        || seed.storage.state() != 6
        || !seed.diagnostics.is_empty()
        || !seed.unresolved.is_empty()
        || fields.class != 38
        || definition.class() != 38
        || !matches!(definition.id, 17 | 42)
        || fields.configuration[20] != 6
        || !camera_origin.into_iter().all(f32::is_finite)
    {
        return Err(
            "basic projectile outside acquired D class38/state6/declaration17/42 construction"
                .into(),
        );
    }
    let mut seed = seed.instantiate()?;
    let transition = seed.storage.transition(77)?;
    seed.transitions.push(transition);
    let bytes = seed.storage.data_mut().ok_or("state77 payload missing")?;
    if bytes.len() != 500 {
        return Err("D basic projectile requires 500-byte state77 storage".into());
    }
    let put = |bytes: &mut [u8], at: usize, value: u32| {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes())
    };
    let flags = definition.flags();
    put(bytes, 0, 1 | if flags & 2 == 0 { 0x20 } else { 0 });
    put(bytes, 0x1ac, u32::MAX);
    for (from, to) in [
        (0x54, 0x24),
        (0x58, 0x20),
        (0x5c, 0x28),
        (0x60, 0x2c),
        (0x64, 0x4c),
        (0x70, 0x50),
    ] {
        put(bytes, to, definition.word(from)?);
    }
    bytes[0x58..0x1ac].copy_from_slice(definition.bytes());
    bytes[0x1e0] = definition.id as u8;
    for (i, value) in camera_origin.into_iter().enumerate() {
        put(bytes, 0x1e8 + i * 4, value.to_bits());
    }
    let mut fields = fields.clone();
    fields.dispatch_context = dispatch_context;
    let matrix = crate::object_transform::euler(fields.position, fields.angles, fields.scale)
        .map_err(str::to_owned)?;
    fields.object_flags |= 0x20;
    fields.configuration[14] |= 0x600 | if flags & 0x4000 != 0 { 0x20000 } else { 0 };
    fields.configuration[20] = 77;
    let constructor_effect = definition.effect(0).map(str::to_owned);
    Ok(Prepared {
        seed,
        fields,
        metadata: Metadata {
            definition,
            dispatch_context,
            camera_origin,
            initial_matrix: matrix,
            callback_family: CallbackFamily::BasicContactsPending,
            constructor_effect,
            launch: None,
        },
    })
}
