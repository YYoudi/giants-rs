//! Reuse existing scenery consumers in owned mission previews. No readiness ack.
use bevy::{asset::AssetId, prelude::*};
use std::collections::BTreeSet;
pub(crate) struct Prepared {
    decorations: Option<crate::decoration_scene::Scene>,
    foliage: Option<crate::foliage_scene::Scene>,
}
impl Prepared {
    pub(crate) fn load(
        root: &std::path::Path,
        plan: &giants_game::instances::Plan,
        world: &giants_game::WorldAsset,
    ) -> Self {
        let decorations = crate::decoration_scene::Scene::load(root, plan)
            .map_err(|e| println!("mission_dressing_omission family=decorations reason={e}"))
            .ok();
        let foliage = crate::foliage_scene::Scene::load(root, world, plan)
            .map_err(|e| println!("mission_dressing_omission family=foliage reason={e}"))
            .ok();
        Self {
            decorations,
            foliage,
        }
    }
    pub(crate) fn covered(&self) -> BTreeSet<(bool, usize)> {
        let mut keys = BTreeSet::new();
        if let Some(s) = &self.decorations {
            keys.extend(s.source_keys());
        }
        if let Some(s) = &self.foliage {
            keys.extend(s.source_keys());
        }
        keys
    }
}
pub(crate) struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<State>().add_systems(
            Update,
            update
                .after(crate::mission_showcase::update)
                .before(crate::source_view::ViewUpdate)
                .run_if(crate::mecc_session::frontend_available),
        );
    }
}
macro_rules! assets {
    ($($field:ident:$ty:ty),*) => {
        #[derive(Default)] struct Gpu { $( $field:BTreeSet<AssetId<$ty>>, )* }
        impl Gpu {
            fn snapshot(world:&World)->Self { Self { $( $field:world.resource::<Assets<$ty>>().ids().collect(), )* } }
            fn collect(&mut self,before:&Self,world:&World) { $( self.$field.extend(world.resource::<Assets<$ty>>().ids().filter(|id|!before.$field.contains(id))); )* }
            fn release(&self,world:&mut World) { $( for id in &self.$field { world.resource_mut::<Assets<$ty>>().remove(*id); } )* }
            fn remaining(&self,world:&World)->usize { 0 $( + self.$field.iter().filter(|id|world.resource::<Assets<$ty>>().contains(**id)).count() )* }
        }
    }
}
assets!(meshes:Mesh,images:Image,materials:crate::prop_scene::Material,buffers:bevy::render::storage::ShaderBuffer);
struct Mounted {
    ticket: u64,
    entities: Vec<Entity>,
    gpu: Gpu,
}
#[derive(Resource, Default)]
struct State {
    mounted: Option<Mounted>,
}
fn update(world: &mut World) {
    let wanted = world
        .resource::<crate::mission_runtime::Runtime>()
        .active_ticket();
    if world
        .resource::<State>()
        .mounted
        .as_ref()
        .is_some_and(|m| Some(m.ticket) != wanted)
    {
        let mounted = world.resource_mut::<State>().mounted.take().unwrap();
        for entity in &mounted.entities {
            world.despawn(*entity);
        }
        world.remove_resource::<crate::foliage_scene::Scene>();
        world.remove_resource::<crate::decoration_scene::Scene>();
        mounted.gpu.release(world);
        let remaining = mounted.gpu.remaining(world);
        assert_eq!(remaining, 0);
        println!(
            "mission_dressing_released ticket={} entities={} remaining_assets={remaining}",
            mounted.ticket,
            mounted.entities.len()
        );
    }
    let Some(ticket) = wanted else { return };
    if !world.resource::<crate::mission_showcase::State>().enabled()
        || world
            .resource::<crate::mission_environment::State>()
            .mounted_ticket()
            != Some(ticket)
    {
        return;
    }
    if world.resource::<State>().mounted.is_none() {
        let Some(prepared) = world
            .resource_mut::<crate::mission_runtime::Runtime>()
            .take_dressing(ticket)
        else {
            return;
        };
        let before = Gpu::snapshot(world);
        let old: BTreeSet<_> = world.query::<Entity>().iter(world).collect();
        if let Some(scene) = prepared.decorations {
            world.insert_resource(scene);
            world
                .run_system_cached(crate::decoration_scene::setup)
                .expect("owned decorations");
        }
        if let Some(scene) = prepared.foliage {
            world.insert_resource(scene);
            world
                .run_system_cached(crate::foliage_scene::setup)
                .expect("owned foliage");
        }
        let entities: Vec<_> = world
            .query_filtered::<Entity, With<Mesh3d>>()
            .iter(world)
            .filter(|e| !old.contains(e))
            .collect();
        for entity in &entities {
            world
                .entity_mut(*entity)
                .insert(crate::mission_runtime::MissionOwned(ticket));
        }
        let mut gpu = Gpu::default();
        gpu.collect(&before, world);
        println!(
            "mission_dressing_mounted ticket={ticket} entities={} duplicates_excluded=true",
            entities.len()
        );
        world.resource_mut::<State>().mounted = Some(Mounted {
            ticket,
            entities,
            gpu,
        });
    }
    if world.contains_resource::<crate::foliage_scene::Scene>() {
        world
            .run_system_cached(crate::foliage_scene::update)
            .expect("mission foliage preview update");
        let ids: Vec<_> = world
            .resource::<crate::foliage_scene::Scene>()
            .mesh_ids()
            .collect();
        world
            .resource_mut::<State>()
            .mounted
            .as_mut()
            .unwrap()
            .gpu
            .meshes
            .extend(ids);
    }
}
