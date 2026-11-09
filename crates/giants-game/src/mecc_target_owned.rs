//! Session-backed local targeting and typed publication. Owned context producers
//! are available; special headers and camera/viewport timing remain incomplete.
use crate::{
    first_person::Camera,
    mecc_target_query as query,
    resident_states::{Id, Session},
    scene_resources::SceneResources,
};
use std::collections::BTreeMap;

pub trait Headers {
    fn header(&mut self, session: &Session, root: Id) -> Result<query::Header, String>;
}
/// Current context from the acquired owned constructors and callback publications.
/// The class805 special header remains unavailable; its predicate still refuses
/// when that value is required. This does not create missing mission actors.
pub struct OwnedHeaders;
impl Headers for OwnedHeaders {
    fn header(&mut self, session: &Session, root: Id) -> Result<query::Header, String> {
        session.mecc_target_header(root)
    }
}
pub struct Input<'a> {
    pub source: Id,
    pub camera: Camera,
    pub viewport_origin: [f32; 3],
    pub viewport_forward: [f32; 3],
    pub definition: &'a crate::projectile_definition::Definition,
    pub vertex_epoch: u32,
    pub force_vertices: bool,
}
pub(crate) struct Metadata {
    pub actors: BTreeMap<u32, query::Actor>,
    pub identities: BTreeMap<u32, Id>,
    pub order: Vec<u32>,
}
pub struct Output {
    pub target: Option<Id>,
    pub point: [f32; 3],
    pub near_terrain: bool,
    pub ray: crate::mecc_aim::Ray,
    pub geometry_visits: Vec<Id>,
    pub mesh_visits: Vec<Id>,
}
/// The local prepass publishes three initially equal relationships. Raw payload
/// pointer slots stay normalized; later callbacks must use these owned identities.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Published {
    /// Payload relationships at 0x14, 0xe0 and 0x120, in that order.
    pub links: [Option<Id>; 3],
    pub point: [f32; 3],
    pub near_terrain: bool,
    /// Supplied collision-vertex epoch, not an inferred simulation frame.
    pub vertex_epoch: u32,
}
struct Services<'a, H> {
    geometry: GeometryServices<'a>,
    headers: &'a mut H,
}
/// Shared current geometry provider; character policies remain outside it.
pub(crate) struct GeometryServices<'a> {
    session: &'a mut Session,
    source: &'a SceneResources,
    identities: &'a BTreeMap<u32, Id>,
    epoch: u32,
    force: bool,
    visits: Vec<Id>,
    meshes: Vec<Id>,
}
impl<H: Headers> query::Services for Services<'_, H> {
    fn header(&mut self, key: u32) -> Result<query::Header, String> {
        self.headers
            .header(self.geometry.session(), self.geometry.identity(key)?)
    }
    fn geometry(&mut self, key: u32) -> Result<query::Geometry, String> {
        self.geometry.geometry(key)
    }
    fn mesh(&mut self, key: u32) -> Result<crate::object_segment::Mesh, String> {
        self.geometry.mesh(key)
    }
}
impl<'a> GeometryServices<'a> {
    pub(crate) fn new(
        session: &'a mut Session,
        source: &'a SceneResources,
        identities: &'a BTreeMap<u32, Id>,
        epoch: u32,
        force: bool,
    ) -> Self {
        Self {
            session,
            source,
            identities,
            epoch,
            force,
            visits: Vec::new(),
            meshes: Vec::new(),
        }
    }
    pub(crate) fn session(&self) -> &Session {
        self.session
    }
    pub(crate) fn identity(&self, key: u32) -> Result<Id, String> {
        self.identities
            .get(&key)
            .copied()
            .ok_or("target owned identity absent".into())
    }
    pub(crate) fn visits(self) -> (Vec<Id>, Vec<Id>) {
        (self.visits, self.meshes)
    }
    pub(crate) fn geometry(&mut self, key: u32) -> Result<query::Geometry, String> {
        let id = self.identity(key)?;
        self.session.check_target_geometry(id)?;
        let actor = self
            .session
            .get(id)
            .ok_or("target geometry actor removed")?;
        let fields = actor
            .construction_fields
            .as_ref()
            .ok_or("target geometry fields absent")?;
        let spec = self
            .session
            .specification(id)
            .ok_or("target geometry specification absent")?
            .clone();
        let slot = actor.channels.ok_or("target geometry channels absent")?[0][0];
        let bounds =
            crate::resident_environment::Resources::load(self.source, &spec)?.bounds(slot)?;
        let values = bounds.current_auxiliary.unwrap_or(bounds.primary);
        let affine = self.session.actor_current_affine(id)?;
        let pose =
            crate::contact_pose::Cached::new(fields.object_flags, affine.flags, affine.matrix)?;
        self.visits.push(id);
        Ok(query::Geometry {
            pose,
            bounds: crate::volume_queries::Bounds {
                min: values[..3].try_into().unwrap(),
                max: values[3..].try_into().unwrap(),
            },
            mesh: None,
        })
    }
    pub(crate) fn mesh(&mut self, key: u32) -> Result<crate::object_segment::Mesh, String> {
        let id = self.identity(key)?;
        let spec = self
            .session
            .specification(id)
            .ok_or("target mesh specification absent")?;
        let slot = spec.metadata.selection.table()?.collision_slot()?;
        self.session
            .prepare_actor_positions(id, slot, self.epoch, self.force)?;
        let mesh = self.session.actor_current_contact_mesh(id)?;
        self.meshes.push(id);
        Ok(mesh)
    }
}
pub(crate) fn run<H: Headers>(
    session: &mut Session,
    source: &SceneResources,
    input: Input<'_>,
    terrain: &crate::terrain_queries::StaticTerrainQuery<'_>,
    headers: &mut H,
) -> Result<Output, String> {
    let owner = session
        .get(input.source)
        .ok_or("local targeting source absent or foreign")?;
    if owner.class != 2
        || owner.seed.storage.state() != 2
        || owner.attachment.is_some()
        || input.definition.id != 17
        || input.vertex_epoch == 0
    {
        return Err(
            "owned local targeting requires its normal Mecc/pistol and explicit vertex epoch"
                .into(),
        );
    }
    let header = headers.header(session, input.source)?;
    let Metadata {
        actors,
        identities,
        order,
    } = session.target_query_metadata(input.source)?;
    let key = identities
        .iter()
        .find_map(|(&key, &id)| (id == input.source).then_some(key))
        .ok_or("target source absent from common population")?;
    let mut services = Services {
        geometry: GeometryServices::new(
            session,
            source,
            &identities,
            input.vertex_epoch,
            input.force_vertices,
        ),
        headers,
    };
    let result = query::local_with(
        query::Input {
            current_player: Some(key),
            context: header.context,
            predicate: query::Predicate::OfflinePistol,
            origin: input.viewport_origin,
            direction: input.viewport_forward,
            range: f32::from_bits(input.definition.word(0x2c)?),
        },
        input.camera,
        terrain,
        &actors,
        &order,
        &mut services,
    )?;
    let (geometry_visits, mesh_visits) = services.geometry.visits();
    Ok(Output {
        target: result.target.actor.map(|id| identities[&id]),
        point: result.target.point,
        near_terrain: result.near_terrain,
        ray: result.ray,
        geometry_visits,
        mesh_visits,
    })
}
