//! Dynamic use of the common cinematic renderers, owned by one mission ticket.
//! These are partial renderers, not a substitute for campaign readiness.
use bevy::{asset::AssetId, prelude::*};
use std::collections::HashSet;

pub(crate) struct Prepared {
    actors: crate::actor_scene::Scene,
    gun: crate::gun_scene::Scene,
    staff: crate::attachment_scene::Scene,
    spear: crate::spear_scene::Scene,
    sword: Option<crate::sword_scene::Scene>,
    rigid: crate::rigid_attachment_scene::Scene,
    cube: bool,
    effects: Vec<Result<giants_game::cinematic_fx::Bindings, String>>,
    persistent: Vec<Option<giants_game::cinematic_persistent::Bindings>>,
    timed: Vec<Option<giants_game::cinematic_timed::Bindings>>,
    timed_world: Option<std::sync::Arc<giants_game::cinematic_timed::world::Prepared>>,
}
impl Prepared {
    pub(crate) fn prepare_timed_world(
        &mut self,
        plan: &giants_game::instances::Plan,
        terrain: &giants_formats::gti::Terrain,
    ) -> Result<(), String> {
        if self.timed.iter().any(Option::is_some) {
            self.timed_world = Some(std::sync::Arc::new(
                giants_game::cinematic_timed::world::Prepared::load(plan, terrain)?,
            ));
        }
        Ok(())
    }
    pub(crate) fn effects_bound(&self) -> bool {
        self.effects.iter().enumerate().all(|(director, terminal)| {
            terminal.is_ok()
                || self.timed_world.is_some()
                    && self
                        .timed
                        .get(director)
                        .and_then(Option::as_ref)
                        .is_some_and(|binding| {
                            self.rigid.covers_canonical_effects(director, binding)
                        })
        })
    }
    pub(crate) fn load(
        root: &std::path::Path,
        cinematic: &crate::cinematic_camera::Cinematic,
        resources: &giants_game::scene_resources::SceneResources,
        config: &giants_formats::world::WorldConfig,
    ) -> Result<Self, String> {
        let result = (|| -> Result<Self, Box<dyn std::error::Error>> {
            let actors = crate::actor_scene::Scene::load(root, cinematic, None, resources, config)?;
            let gun = crate::gun_scene::Scene::load(root, cinematic, None, config)?;
            let staff = crate::attachment_scene::Scene::load(root, cinematic, None)?;
            let spear = crate::spear_scene::Scene::load(root, cinematic, None, config)?;
            let sword = crate::sword_scene::Scene::load(root, cinematic, None, config)?;
            let sword = sword.has_geometry().then_some(sword);
            let mut owners =
                crate::cinematic_scene::AttachmentOwners::new(cinematic.segments().len());
            for director in 0..cinematic.segments().len() {
                owners.add(director, gun.owned_slots(director), "gun")?;
                owners.add(director, staff.owned_slots(director), "staff")?;
                owners.add(director, spear.owned_slots(director), "spear")?;
                if let Some(sword) = &sword {
                    owners.add(director, sword.owned_slots(director), "sword")?;
                }
            }
            let rigid = crate::rigid_attachment_scene::Scene::load(
                root, cinematic, None, resources, &actors, &owners, config,
            )?;
            let cube = actors.needs_cube() || rigid.needs_cube() || sword.is_some();
            let effects = cinematic
                .segments()
                .iter()
                .map(|segment| {
                    let path = giants_game::flick::sequence_path(root, segment.sequence())
                        .map_err(str::to_owned)?;
                    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                    let binding = giants_game::cinematic_fx::Bindings::load(resources, &bytes);
                    if let Err(error) = &binding {
                        println!(
                            "mission_effect_binding_deferred sequence={} reason={error}",
                            segment.sequence()
                        );
                    }
                    Ok(binding)
                })
                .collect::<Result<Vec<_>, String>>()?;
            let persistent = cinematic.segments().iter().map(|segment| {
                let path = giants_game::flick::sequence_path(root, segment.sequence())?;
                let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
                let binding = giants_game::cinematic_persistent::Bindings::load(resources, &bytes).ok();
                if binding.is_some() {
                    println!("mission_persistent_binding sequence={} source_profile=attached_constant readiness_acknowledged=false", segment.sequence());
                }
                Ok(binding)
            }).collect::<Result<Vec<_>, String>>()?;
            let timed = cinematic
                .segments()
                .iter()
                .map(|segment| {
                    let bytes =
                        std::fs::read(giants_game::flick::sequence_path(root, segment.sequence())?)
                            .map_err(|e| e.to_string())?;
                    Ok(giants_game::cinematic_timed::Bindings::load(resources, &bytes).ok())
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(Self {
                actors,
                gun,
                staff,
                spear,
                sword,
                rigid,
                cube,
                effects,
                persistent,
                timed,
                timed_world: None,
            })
        })();
        result.map_err(|e| e.to_string())
    }
}

macro_rules! gpu_assets {
    ($($field:ident : $ty:ty),* $(,)?) => {
        #[derive(Default)]
        struct Gpu { $( $field: HashSet<AssetId<$ty>>, )* }
        impl Gpu {
            fn snapshot(world: &World) -> Self {
                Self { $( $field: world.resource::<Assets<$ty>>().ids().collect(), )* }
            }
            fn collect_new(&mut self, before: &Self, world: &World) {
                $( self.$field.extend(world.resource::<Assets<$ty>>().ids().filter(|id| !before.$field.contains(id))); )*
            }
            fn release(&self, world: &mut World) {
                $( for id in &self.$field { world.resource_mut::<Assets<$ty>>().remove(*id); } )*
            }
            fn count(&self) -> usize { 0 $( + self.$field.len() )* }
            fn remaining(&self, world: &World) -> usize {
                0 $( + self.$field.iter().filter(|id| world.resource::<Assets<$ty>>().contains(**id)).count() )*
            }
        }
    }
}
gpu_assets! {
    meshes: Mesh, images: Image,
    actors: crate::actor_scene::Material,
    lit: crate::actor_scene::LitMaterial,
    context: crate::body_context::Material,
    gun: crate::gun_scene::Material,
    staff: crate::attachment_scene::Material,
    spear: crate::spear_scene::Material,
    sword: crate::sword_scene::Material,
    rigid: crate::rigid_attachment_scene::Material,
    buffers: bevy::render::storage::ShaderBuffer,
}
#[derive(Component)]
struct Owned(pub u64);
#[derive(Resource, Default)]
struct State {
    mounted: Option<Mounted>,
}
struct Mounted {
    ticket: u64,
    prepared: Option<Prepared>,
    gpu: Gpu,
    cube_started: bool,
    ready: bool,
    director: Option<usize>,
}
pub(crate) struct Plugin;
impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        macro_rules! material {
            ($ty:ty) => {
                if !app.is_plugin_added::<MaterialPlugin<$ty>>() {
                    app.add_plugins(MaterialPlugin::<$ty>::default());
                }
            };
        }
        material!(crate::actor_scene::Material);
        material!(crate::actor_scene::LitMaterial);
        material!(crate::body_context::Material);
        material!(crate::gun_scene::Material);
        material!(crate::attachment_scene::Material);
        material!(crate::spear_scene::Material);
        material!(crate::sword_scene::Material);
        material!(crate::rigid_attachment_scene::Material);
        app.init_resource::<State>().add_systems(
            Update,
            draw.after(crate::mission_opening::OpeningUpdate)
                .before(crate::mission_player::PlayerUpdate)
                .before(crate::source_view::ViewUpdate),
        );
    }
}
pub(crate) fn mount(world: &mut World, ticket: u64, prepared: Prepared) {
    assert!(world.resource::<State>().mounted.is_none());
    world.resource_mut::<State>().mounted = Some(Mounted {
        ticket,
        prepared: Some(prepared),
        gpu: Gpu::default(),
        cube_started: false,
        ready: false,
        director: None,
    });
}
pub(crate) fn ready(world: &World) -> bool {
    world
        .get_resource::<State>()
        .and_then(|s| s.mounted.as_ref())
        .is_some_and(|m| m.ready)
}
pub(crate) fn drawn(world: &World) -> bool {
    world
        .get_resource::<State>()
        .and_then(|s| s.mounted.as_ref())
        .is_some_and(|m| m.ready && m.director.is_some())
}
pub(crate) fn prepare_gpu(world: &mut World) {
    let Some(mounted) = world.resource::<State>().mounted.as_ref() else {
        return;
    };
    if mounted.ready {
        return;
    }
    let ticket = mounted.ticket;
    let need_cube = mounted.prepared.as_ref().is_some_and(|p| p.cube);
    if need_cube && !mounted.cube_started {
        crate::mission_environment::prepare_reflection(world, ticket)
            .expect("owned selected-world reflection profile");
        let mut state = world.resource_mut::<State>();
        let mounted = state.mounted.as_mut().unwrap();
        // The environment owns this shared image beyond opening cleanup.
        // Its ID must not enter the film's destructive GPU ownership set.
        mounted.cube_started = true;
        println!(
            "mission_film_cube ticket={ticket} started=true source=selected_world capture_input=false owner=environment borrowed=true"
        );
        return;
    }
    // Even an opening with no reflective actor must wait for the world's cube.
    // Its completion restores the view and invokes the common renderer setup;
    // installing/consuming actor pose inputs before that boundary is unsafe.
    if world
        .get_resource::<crate::reflection_cube::Build>()
        .map_or(need_cube, |b| !b.playback_ready())
    {
        return;
    }
    let root = world
        .resource::<crate::frontend_scene::Scene>()
        .data_root()
        .to_owned();
    crate::cinematic_text_scene::mount(world, ticket, &root)
        .expect("selected mission text ownership");
    let prepared = world
        .resource_mut::<State>()
        .mounted
        .as_mut()
        .unwrap()
        .prepared
        .take()
        .unwrap();
    world.insert_resource(prepared.actors);
    world.insert_resource(prepared.gun);
    world.insert_resource(prepared.staff);
    world.insert_resource(prepared.spear);
    world.insert_resource(prepared.rigid);
    world.insert_resource(
        crate::mission_effects::State::new(ticket, prepared.effects)
            .with_persistent(prepared.persistent)
            .with_timed(prepared.timed, prepared.timed_world),
    );
    if let Some(sword) = prepared.sword {
        world.insert_resource(sword);
    }
    world
        .resource_mut::<State>()
        .mounted
        .as_mut()
        .unwrap()
        .ready = true;
    println!(
        "mission_film_prepared ticket={ticket} renderers=common_actor_and_attachments cube={need_cube} campaign_ready=false"
    );
}
pub(crate) fn draw(world: &mut World) {
    if !crate::mecc_session::frontend_available(world) {
        return;
    }
    if !ready(world) || !world.contains_resource::<crate::cinematic_camera::Cinematic>() {
        return;
    }
    let director = world
        .resource::<crate::cinematic_camera::Cinematic>()
        .director_index();
    let mounted = world.resource::<State>().mounted.as_ref().unwrap();
    let ticket = mounted.ticket;
    let setup = mounted.director != Some(director);
    let first = mounted.director.is_none();
    // Particle/tracer renderers can allocate during an ordinary update, long
    // after setup. Adopt only this draw's additions, including those late births;
    // resources that another producer installed beforehand remain outside it.
    let before = Gpu::snapshot(world);
    let previous: HashSet<_> = world.query::<Entity>().iter(world).collect();
    macro_rules! run {
        ($system:path) => {
            world.run_system_cached($system).expect(stringify!($system));
        };
    }
    if setup {
        if first {
            run!(crate::actor_scene::setup);
            run!(crate::gun_scene::setup);
            run!(crate::attachment_scene::setup);
            run!(crate::spear_scene::setup);
            run!(crate::rigid_attachment_scene::setup);
            if world.contains_resource::<crate::sword_scene::Scene>() {
                run!(crate::sword_scene::setup);
            }
        } else {
            run!(crate::actor_scene::switch);
            run!(crate::gun_scene::switch);
            run!(crate::attachment_scene::switch);
            run!(crate::spear_scene::switch);
            run!(crate::rigid_attachment_scene::switch);
            if world.contains_resource::<crate::sword_scene::Scene>() {
                run!(crate::sword_scene::switch);
            }
        }
    }
    run!(crate::actor_scene::update);
    run!(crate::gun_scene::update);
    run!(crate::attachment_scene::update);
    run!(crate::spear_scene::update);
    run!(crate::rigid_attachment_scene::update);
    if world.contains_resource::<crate::sword_scene::Scene>() {
        run!(crate::sword_scene::update);
    }
    run!(crate::cinematic_visibility::update);
    let entities = adopt_draws(world, ticket, &previous);
    world.resource_scope::<State, _>(|world, mut state| {
        let mounted = state.mounted.as_mut().unwrap();
        let previous_assets = mounted.gpu.count();
        mounted.gpu.collect_new(&before, world);
        mounted.director = Some(director);
        if setup || entities != 0 || mounted.gpu.count() != previous_assets {
            println!("mission_film_draw ticket={ticket} director={director} new_entities={} cumulative_assets={} ready_event=false", entities, mounted.gpu.count());
        }
    });
}
fn adopt_draws(world: &mut World, ticket: u64, before: &HashSet<Entity>) -> usize {
    let candidates: Vec<_> = world
        .query_filtered::<Entity, With<Mesh3d>>()
        .iter(world)
        .filter(|e| !before.contains(e))
        .collect();
    for entity in &candidates {
        world.entity_mut(*entity).insert(Owned(ticket));
    }
    candidates.len()
}
pub(crate) fn release(world: &mut World) {
    let Some(mut state) = world.get_resource_mut::<State>() else {
        return;
    };
    let Some(mounted) = state.mounted.take() else {
        return;
    };
    crate::cinematic_text_scene::release(world, mounted.ticket);
    let entities: Vec<_> = world
        .query::<(Entity, &Owned)>()
        .iter(world)
        .filter(|(_, t)| t.0 == mounted.ticket)
        .map(|(e, _)| e)
        .collect();
    for entity in &entities {
        world.despawn(*entity);
    }
    world.remove_resource::<crate::actor_scene::Scene>();
    world.remove_resource::<crate::gun_scene::Scene>();
    world.remove_resource::<crate::attachment_scene::Scene>();
    world.remove_resource::<crate::spear_scene::Scene>();
    world.remove_resource::<crate::rigid_attachment_scene::Scene>();
    world.remove_resource::<crate::sword_scene::Scene>();
    if let Some(effects) = world.remove_resource::<crate::mission_effects::State>() {
        effects.report_release(&mut world.resource_mut::<crate::mission_runtime::Runtime>());
    }
    mounted.gpu.release(world);
    let remaining = mounted.gpu.remaining(world);
    assert_eq!(remaining, 0);
    println!(
        "mission_film_released ticket={} entities={} assets={} remaining_assets={remaining}",
        mounted.ticket,
        entities.len(),
        mounted.gpu.count()
    );
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires GIANTS_MISSION_OWNED_ROOT; preparation is not graphical film acceptance"]
    fn installed_wife_timed_world_and_render_owners_prepare_together() {
        let root = std::path::PathBuf::from(std::env::var_os("GIANTS_MISSION_OWNED_ROOT").unwrap());
        let menu = giants_game::mission_selection::MissionMenu::parse(
            &std::fs::read(root.join("Bin/w_Story2.bin")).unwrap(),
            0,
            true,
        )
        .unwrap();
        let index = menu
            .entries
            .iter()
            .position(|entry| entry.name == "2b_1_savewife")
            .unwrap();
        let packet =
            giants_game::mission_loading::prepare(&root, menu.request("Story2", index).unwrap())
                .unwrap();
        let plan = packet.instance_plan(&root, "English").unwrap();
        let world = packet.world_asset(&root).unwrap();
        let selection = packet.completion(&root).unwrap();
        let (cinematic, _) = crate::cinematic_camera::Cinematic::load(
            &root,
            crate::cinematic_camera::Load {
                tick: None,
                benchmark: false,
                directors: 1,
                sequence: selection.director.as_deref(),
                full_chain: true,
                preview: true,
            },
        )
        .unwrap();
        let mut prepared =
            super::Prepared::load(&root, &cinematic, plan.resources(), &world.config).unwrap();
        prepared.prepare_timed_world(&plan, &world.terrain).unwrap();
        assert_eq!(prepared.timed.len(), 6);
        for director in [2, 3] {
            let binding = prepared.timed[director]
                .as_ref()
                .expect("source shotgun family");
            let mut count = 0;
            for offset in binding.offsets() {
                if binding.role(offset, 829).is_some() {
                    assert_eq!(binding.role(offset, 828), Some(false));
                    count += 1;
                }
            }
            assert!(count > 0);
        }
        assert!(!prepared.timed_world.as_ref().unwrap().objects.is_empty());
        assert!(prepared.effects_bound());
        // A prepared simulation cannot stand in for the missing actual world.
        prepared.timed_world = None;
        assert!(!prepared.effects_bound());
    }
    use super::*;
    fn cached_probe() {}
    #[test]
    fn release_preserves_shared_buffers_later_assets_and_cached_systems() {
        let mut world = World::new();
        world.init_resource::<State>();
        macro_rules! assets { ($($ty:ty),* $(,)?) => { $(world.init_resource::<Assets<$ty>>();)* }; }
        assets!(
            Mesh,
            Image,
            crate::actor_scene::Material,
            crate::actor_scene::LitMaterial,
            crate::body_context::Material,
            crate::gun_scene::Material,
            crate::attachment_scene::Material,
            crate::spear_scene::Material,
            crate::sword_scene::Material,
            crate::rigid_attachment_scene::Material,
            bevy::render::storage::ShaderBuffer
        );
        world.init_resource::<crate::source_view::SourceView>();
        let shared = world
            .resource::<crate::source_view::SourceView>()
            .buffer
            .clone();
        let before = Gpu::snapshot(&world);
        let entities: HashSet<_> = world.query::<Entity>().iter(&world).collect();
        world.run_system_cached(cached_probe).unwrap();
        let image = world.resource_mut::<Assets<Image>>().add(Image::default());
        let draw = world.spawn(Mesh3d::default()).id();
        assert_eq!(adopt_draws(&mut world, 7, &entities), 1);
        let mut gpu = Gpu::default();
        gpu.collect_new(&before, &world);
        let unrelated = world.resource_mut::<Assets<Image>>().add(Image::default());
        // A later ordinary draw owns its new particles, while an intervening
        // producer's image remains outside the film's snapshot delta.
        let later_before = Gpu::snapshot(&world);
        let later_entities = world.query::<Entity>().iter(&world).collect();
        let late_image = world.resource_mut::<Assets<Image>>().add(Image::default());
        let late_draw = world.spawn(Mesh3d::default()).id();
        assert_eq!(adopt_draws(&mut world, 7, &later_entities), 1);
        gpu.collect_new(&later_before, &world);
        world.resource_mut::<State>().mounted = Some(Mounted {
            ticket: 7,
            prepared: None,
            gpu,
            cube_started: false,
            ready: true,
            director: Some(0),
        });
        crate::cinematic_text_scene::mount(
            &mut world,
            7,
            std::path::Path::new("first-installation"),
        )
        .unwrap();
        // A returning frontend may already have installed its own producer.
        let mut borrowed = crate::reflection_cube::Build::new(None);
        borrowed.set_source(
            unrelated.clone(),
            &mut world.resource_mut::<Assets<Image>>(),
        );
        let borrowed_image = borrowed.handle().unwrap();
        world.insert_resource(borrowed);
        release(&mut world);
        release(&mut world);
        crate::cinematic_text_scene::mount(
            &mut world,
            8,
            std::path::Path::new("second-installation"),
        )
        .unwrap();
        assert!(world.get_entity(draw).is_err());
        assert!(world.get_entity(late_draw).is_err());
        assert!(!world.resource::<Assets<Image>>().contains(late_image.id()));
        assert!(world.contains_resource::<crate::reflection_cube::Build>());
        assert_eq!(
            world.resource::<crate::reflection_cube::Build>().handle(),
            Some(borrowed_image.clone())
        );
        assert!(
            world
                .resource::<Assets<Image>>()
                .contains(borrowed_image.id())
        );
        assert!(!world.resource::<Assets<Image>>().contains(image.id()));
        assert!(world.resource::<Assets<Image>>().contains(unrelated.id()));
        assert!(
            world
                .resource::<Assets<bevy::render::storage::ShaderBuffer>>()
                .contains(shared.id())
        );
        world.run_system_cached(cached_probe).unwrap();
    }
}
