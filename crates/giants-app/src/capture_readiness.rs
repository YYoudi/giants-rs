//! Capture preparation receipt, not native fidelity or successful source loading.
use bevy::{
    asset::AssetId,
    pbr::{PreparedMaterial, RenderMaterialInstances, RenderMeshInstances},
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
        erased_render_asset::ErasedRenderAssets,
        mesh::RenderMesh,
        render_asset::RenderAssets,
        render_resource::{CachedPipelineState, PipelineCache},
        sync_world::MainEntity,
    },
};
use std::sync::{Arc, Mutex};

#[derive(Resource, Clone, Default)]
pub struct Desired {
    active: bool,
    meshes: Vec<(Entity, AssetId<Mesh>)>,
    text_image: Option<AssetId<Image>>,
}
impl Desired {
    pub fn contains_mesh_entity(&self, entity: Entity) -> bool {
        self.meshes
            .iter()
            .any(|(candidate, _)| *candidate == entity)
    }
}
#[derive(Clone, Default)]
struct Receipt {
    meshes: Vec<(Entity, AssetId<Mesh>)>,
    text_image: Option<AssetId<Image>>,
    missing_text_image: bool,
    missing_meshes: usize,
    missing_materials: usize,
    missing_instances: usize,
    waiting_pipelines: usize,
    failed_pipelines: usize,
    pipelines: usize,
}
impl Receipt {
    fn ready_for(&self, desired: &Desired) -> bool {
        desired.active
            && self.meshes == desired.meshes
            && self.text_image == desired.text_image
            && !self.missing_text_image
            && self.missing_meshes == 0
            && self.missing_materials == 0
            && self.missing_instances == 0
            && self.waiting_pipelines == 0
            && self.failed_pipelines == 0
            && self.pipelines != 0
    }
}
#[derive(Resource, Clone, Default)]
pub struct Prepared(Arc<Mutex<Receipt>>);
impl Prepared {
    pub fn ready_for(&self, desired: &Desired) -> bool {
        self.0
            .lock()
            .expect("Capture preparation receipt")
            .ready_for(desired)
    }
}
pub fn sample_expected(
    capture: Res<crate::Capture>,
    mut desired: ResMut<Desired>,
    meshes: Query<(Entity, &Mesh3d, &ViewVisibility)>,
    text_image: Res<crate::cinematic_text_scene::CaptureImage>,
) {
    desired.active = capture.path.is_some();
    if !desired.active {
        return;
    }
    desired.text_image = text_image.0.as_ref().map(Handle::id);
    desired.meshes = meshes
        .iter()
        .filter(|(_, _, visible)| visible.get())
        .map(|(entity, mesh, _)| (entity, mesh.id()))
        .collect();
    desired
        .meshes
        .sort_unstable_by_key(|(entity, _)| entity.to_bits());
}
fn extract(
    mut commands: Commands,
    desired: Extract<Res<Desired>>,
    prepared: Extract<Res<Prepared>>,
) {
    commands.insert_resource(desired.clone());
    commands.insert_resource(prepared.clone());
}
type PreparationCounts = (usize, usize, usize, usize, usize, usize);
#[derive(bevy::ecs::system::SystemParam)]
struct GpuAssets<'w> {
    images: Res<'w, RenderAssets<bevy::render::texture::GpuImage>>,
    meshes: Res<'w, RenderAssets<RenderMesh>>,
    mesh_instances: Res<'w, RenderMeshInstances>,
    instances: Res<'w, RenderMaterialInstances>,
    materials: Res<'w, ErasedRenderAssets<PreparedMaterial>>,
}
fn inspect(
    desired: Res<Desired>,
    prepared: Res<Prepared>,
    assets: GpuAssets,
    pipelines: Res<PipelineCache>,
    mut previous: Local<Option<PreparationCounts>>,
) {
    if !desired.active {
        return;
    }
    let mut receipt = Receipt {
        meshes: desired.meshes.clone(),
        text_image: desired.text_image,
        missing_text_image: desired
            .text_image
            .is_some_and(|id| assets.images.get(id).is_none()),
        ..default()
    };
    for (entity, mesh) in &desired.meshes {
        receipt.missing_meshes += usize::from(assets.meshes.get(*mesh).is_none());
        receipt.missing_instances += usize::from(
            assets
                .mesh_instances
                .mesh_asset_id(MainEntity::from(*entity))
                != Some(*mesh),
        );
        let material = assets.instances.instances.get(&MainEntity::from(*entity));
        receipt.missing_materials +=
            usize::from(material.is_none_or(|m| assets.materials.get(m.asset_id).is_none()));
    }
    receipt.waiting_pipelines = pipelines.waiting_pipelines().count();
    for pipeline in pipelines.pipelines() {
        receipt.pipelines += 1;
        receipt.failed_pipelines +=
            usize::from(matches!(pipeline.state, CachedPipelineState::Err(_)));
    }
    let state = (
        receipt.meshes.len(),
        receipt.missing_meshes,
        receipt.missing_materials,
        receipt.waiting_pipelines,
        receipt.failed_pipelines,
        receipt.missing_instances,
    );
    if *previous != Some(state) {
        println!(
            "capture_preparation visible_meshes={} missing_gpu_meshes={} missing_gpu_materials={} waiting_pipelines={} failed_pipelines={} missing_gpu_instances={} source_completeness_verified=false",
            state.0, state.1, state.2, state.3, state.4, state.5
        );
        *previous = Some(state);
    }
    *prepared.0.lock().expect("Capture preparation receipt") = receipt;
}
pub fn install(app: &mut App) {
    app.init_resource::<Desired>()
        .init_resource::<Prepared>()
        .add_systems(
            PostUpdate,
            sample_expected.after(bevy::camera::visibility::VisibilitySystems::CheckVisibility),
        );
    app.sub_app_mut(RenderApp)
        .add_systems(ExtractSchedule, extract)
        .add_systems(
            Render,
            inspect
                .after(RenderSystems::PrepareBindGroups)
                .before(RenderSystems::Render),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ready_pipelines_do_not_ack_missing_materials_or_a_changed_visible_set() {
        let mut world = World::new();
        let first = world.spawn_empty().id();
        let second = world.spawn_empty().id();
        let mesh = Handle::<Mesh>::default().id();
        let mut desired = Desired {
            active: true,
            meshes: vec![(first, mesh)],
            ..default()
        };
        let mut receipt = Receipt {
            meshes: desired.meshes.clone(),
            pipelines: 1,
            missing_materials: 1,
            ..default()
        };
        assert!(!receipt.ready_for(&desired));
        receipt.missing_materials = 0;
        assert!(receipt.ready_for(&desired));
        desired.meshes = vec![(second, mesh)];
        assert!(!receipt.ready_for(&desired));
        receipt.meshes = desired.meshes.clone();
        receipt.failed_pipelines = 1;
        assert!(!receipt.ready_for(&desired));
        receipt.failed_pipelines = 0;
        receipt.missing_instances = 1;
        assert!(!receipt.ready_for(&desired));
        receipt.missing_instances = 0;
        receipt.missing_meshes = 1;
        assert!(!receipt.ready_for(&desired));
        receipt.missing_meshes = 0;
        desired.text_image = Some(Handle::<Image>::default().id());
        assert!(!receipt.ready_for(&desired));
        receipt.text_image = desired.text_image;
        receipt.missing_text_image = true;
        assert!(!receipt.ready_for(&desired));
        receipt.missing_text_image = false;
        assert!(receipt.ready_for(&desired));
    }
}
