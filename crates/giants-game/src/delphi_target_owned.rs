//! Selected-owner held-ray candidate query with actual Session identities and
//! common current geometry. Range is an explicit upstream boundary; this does
//! not produce terrain/readiness state or publish a movement target.
use crate::{
    delphi_target_predicate::{self, Candidate},
    mecc_target_owned::{GeometryServices, Metadata},
    mecc_target_query as query,
    resident_states::{Id, Session},
    scene_resources::SceneResources,
};
pub struct Input {
    /// Selected local Delphi owner, also the query's current-player identity.
    pub owner: Id,
    pub number: u64,
    pub normalization_threshold: f32,
    /// Actual range supplied by the upstream producer, not a guessed weapon value.
    pub range: f32,
    pub vertex_epoch: u32,
    pub force_vertices: bool,
}
pub struct Output {
    pub target: Option<Id>,
    pub point: [f32; 3],
    pub geometry_visits: Vec<Id>,
    pub mesh_visits: Vec<Id>,
}
struct Services<'a> {
    geometry: GeometryServices<'a>,
    owner: Id,
    position: [f32; 3],
    second_column: [f32; 3],
}
impl query::Services for Services<'_> {
    fn predicate(&mut self, key: u32) -> Result<bool, String> {
        let id = self.geometry.identity(key)?;
        let session = self.geometry.session();
        session.check_target_geometry(id)?;
        let fields = session
            .get(id)
            .and_then(|a| a.construction_fields.as_ref())
            .ok_or("Delphi candidate owned header absent")?;
        delphi_target_predicate::accepts(
            Candidate {
                flags: fields.object_flags,
                personality: fields.configuration[14],
                position: fields.position,
                is_current_player: id == self.owner,
            },
            self.position,
            self.second_column,
        )
    }
    fn header(&mut self, _: u32) -> Result<query::Header, String> {
        Err("Delphi policy does not consume a Mecc context header".into())
    }
    fn geometry(&mut self, key: u32) -> Result<query::Geometry, String> {
        self.geometry.geometry(key)
    }
    fn mesh(&mut self, key: u32) -> Result<crate::object_segment::Mesh, String> {
        self.geometry.mesh(key)
    }
}
impl Session {
    /// Query current candidates from the published completed-frame held cache.
    /// This leaves target links/readiness untouched and refuses missing geometry.
    pub fn query_delphi_target(
        &mut self,
        resources: &SceneResources,
        input: Input,
    ) -> Result<Output, String> {
        self.check_target_geometry(input.owner)?;
        let owner = self.get(input.owner).ok_or("Delphi target owner absent")?;
        if owner.class != 3
            || owner.seed.storage.state() != 3
            || owner.attachment.is_some()
            || input.vertex_epoch == 0
        {
            return Err(
                "Delphi target query requires its selected unparented owner and vertex epoch"
                    .into(),
            );
        }
        let ray =
            self.delphi_target_ray(input.owner, input.number, input.normalization_threshold)?;
        let position = owner
            .construction_fields
            .as_ref()
            .ok_or("Delphi target owner header absent")?
            .position;
        let matrix = self.actor_current_affine(input.owner)?.matrix;
        let Metadata {
            actors,
            identities,
            order,
        } = self.target_query_metadata(input.owner)?;
        let key = identities
            .iter()
            .find_map(|(&key, &id)| (id == input.owner).then_some(key))
            .ok_or("Delphi target owner absent from common population")?;
        let mut services = Services {
            geometry: GeometryServices::new(
                self,
                resources,
                &identities,
                input.vertex_epoch,
                input.force_vertices,
            ),
            owner: input.owner,
            position,
            second_column: [matrix[1], matrix[5], matrix[9]],
        };
        let result = query::query_with(
            query::Input {
                current_player: Some(key),
                context: 0,
                predicate: query::Predicate::Provided,
                origin: ray.origin,
                direction: ray.direction,
                range: input.range,
            },
            &actors,
            &order,
            &mut services,
        )?;
        let target = result
            .actor
            .map(|key| services.geometry.identity(key))
            .transpose()?;
        let (geometry_visits, mesh_visits) = services.geometry.visits();
        Ok(Output {
            target,
            point: result.point,
            geometry_visits,
            mesh_visits,
        })
    }
}
