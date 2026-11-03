//! Opt-in CPU render-stage intervals and timestamped GPU pass diagnostics.
//! Checkpoints measure intervals between stage boundaries, not exclusive work.
use bevy::{
    prelude::*,
    render::{Render, RenderApp, RenderSystems},
};

#[derive(Resource, Default)]
struct Stages {
    previous: Option<std::time::Instant>,
    frame: u64,
}
const NAMES: [&str; 10] = [
    "begin",
    "assets",
    "meshes",
    "views",
    "queue",
    "sort",
    "resources",
    "flush_bind_groups",
    "render",
    "cleanup",
];
fn checkpoint<const N: usize>(mut stages: ResMut<Stages>) {
    let now = std::time::Instant::now();
    if N != 0
        && let Some(start) = stages.previous
        && stages.frame.is_multiple_of(30)
    {
        info!(
            "render_stage_profile frame={} stage={} ms={}",
            stages.frame,
            NAMES[N],
            (now - start).as_secs_f64() * 1000.
        );
    }
    stages.previous = Some(now);
    if N == 9 {
        stages.frame += 1;
    }
}
fn passes(diagnostics: Res<bevy::diagnostic::DiagnosticsStore>, mut frame: Local<u64>) {
    *frame += 1;
    if !frame.is_multiple_of(30) {
        return;
    }
    for diagnostic in diagnostics.iter() {
        let path = diagnostic.path().to_string();
        if (path.ends_with("elapsed_cpu") || path.ends_with("elapsed_gpu"))
            && let Some(value) = diagnostic.value()
        {
            info!(
                "render_pass_profile frame={} path={} ms={}",
                *frame, path, value
            );
        }
    }
}
fn asset_changes<A: Asset>(
    events: Option<Res<Messages<AssetEvent<A>>>>,
    assets: Option<Res<Assets<A>>>,
    mut cursor: Local<bevy::ecs::message::MessageCursor<AssetEvent<A>>>,
    mut sample: Local<(u64, usize, usize)>,
) {
    sample.0 += 1;
    if let Some(events) = events {
        for event in cursor.read(&events) {
            match event {
                AssetEvent::Modified { .. } => sample.1 += 1,
                AssetEvent::Added { .. } => sample.2 += 1,
                _ => {}
            }
        }
    }
    if sample.0.is_multiple_of(30) {
        info!(
            "asset_profile frame={} family={} live={} modified={} added={} window_frames=30",
            sample.0,
            std::any::type_name::<A>(),
            assets.map_or(0, |a| a.len()),
            sample.1,
            sample.2
        );
        sample.1 = 0;
        sample.2 = 0;
    }
}
fn transparent_batches(
    phases: Res<
        bevy::render::render_phase::ViewSortedRenderPhases<
            bevy::core_pipeline::core_3d::Transparent3d,
        >,
    >,
    mut frame: Local<u64>,
) {
    *frame += 1;
    if !frame.is_multiple_of(30) {
        return;
    }
    let mut items = 0;
    let mut batches = 0;
    let mut instances = 0;
    for phase in phases.values() {
        items += phase.items.len();
        let mut entries = phase.items.values();
        while let Some(item) = entries.next() {
            let count = item.batch_range.len();
            if count != 0 {
                batches += 1;
                instances += count;
                // SortedRenderPhase::render_range advances by the batch length,
                // not one entry. Follower entries retain nonempty ranges too.
                for _ in 1..count {
                    entries.next();
                }
            }
        }
    }
    info!(
        "transparent_batch_profile_v2 frame={} items={} draw_batches={} instances={}",
        *frame, items, batches, instances
    );
}
pub fn install(app: &mut App) {
    app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin)
        .add_systems(Update, passes)
        .add_systems(
            Last,
            (
                asset_changes::<crate::prop_scene::Material>,
                asset_changes::<crate::scenery_scene::Material>,
                asset_changes::<crate::actor_scene::Material>,
                asset_changes::<crate::actor_scene::LitMaterial>,
                asset_changes::<crate::gun_scene::Material>,
                asset_changes::<crate::sword_scene::Material>,
                asset_changes::<crate::world_scene::WorldColorMaterial>,
                asset_changes::<bevy::render::storage::ShaderBuffer>,
                asset_changes::<Mesh>,
                asset_changes::<Image>,
            ),
        );
    let render = app.sub_app_mut(RenderApp);
    render.init_resource::<Stages>();
    render.add_systems(
        Render,
        transparent_batches
            .after(RenderSystems::PrepareBindGroups)
            .before(RenderSystems::Render),
    );
    // Checkpoints are ordered between the engine's existing stage boundaries.
    // They include intervening parallel work and are not exclusive per-system costs.
    render.add_systems(
        Render,
        checkpoint::<0>
            .after(RenderSystems::ExtractCommands)
            .before(RenderSystems::PrepareAssets),
    );
    render.add_systems(
        Render,
        checkpoint::<1>
            .after(RenderSystems::PrepareAssets)
            .before(RenderSystems::PrepareMeshes),
    );
    render.add_systems(
        Render,
        checkpoint::<2>
            .after(RenderSystems::PrepareMeshes)
            .before(RenderSystems::CreateViews),
    );
    render.add_systems(
        Render,
        checkpoint::<3>
            .after(RenderSystems::PrepareViews)
            .before(RenderSystems::Queue),
    );
    render.add_systems(
        Render,
        checkpoint::<4>
            .after(RenderSystems::Queue)
            .before(RenderSystems::PhaseSort),
    );
    render.add_systems(
        Render,
        checkpoint::<5>
            .after(RenderSystems::PhaseSort)
            .before(RenderSystems::Prepare),
    );
    render.add_systems(
        Render,
        checkpoint::<6>
            .after(RenderSystems::PrepareResources)
            .before(RenderSystems::PrepareResourcesFlush),
    );
    render.add_systems(
        Render,
        checkpoint::<7>
            .after(RenderSystems::PrepareBindGroups)
            .before(RenderSystems::Render),
    );
    render.add_systems(
        Render,
        checkpoint::<8>
            .after(RenderSystems::Render)
            .before(RenderSystems::Cleanup),
    );
    render.add_systems(
        Render,
        checkpoint::<9>
            .after(RenderSystems::Cleanup)
            .before(RenderSystems::PostCleanup),
    );
}
