//! Persistent source state, weapon bindings and bounded child countdown.
//! Full behavior callbacks and general visibility policy remain separate.
use bevy::prelude::*;
#[derive(Resource)]
pub struct Scene {
    pub plan: giants_game::instances::Plan,
    pub children: giants_game::child_creation::InitialPlan,
    pub runtime: giants_game::child_creation::Runtime,
    pub weapons: Result<giants_game::weapons::Registry, String>,
    pub targets: Option<giants_game::target_runtime::Runtime>,
}
impl Scene {
    pub fn load(
        root: &std::path::Path,
        world: &giants_game::WorldAsset,
        mission: Option<&giants_game::mission_loading::PreparedMission>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let plan = match mission {
            Some(packet) => packet.instance_plan(root, "English")?,
            None => giants_game::instances::Plan::load_first(root, world)?,
        };
        let source_errors = plan.entries.iter().filter(|p| p.source.is_err()).count();
        println!(
            "source_mission_exclusion_requests={} unresolved_children_or_owners={} runtime_executed=false",
            plan.mission_exclusion_results.len(),
            plan.mission_exclusion_results
                .iter()
                .filter(|r| r.result.is_err())
                .count()
        );
        for result in &plan.mission_exclusion_results {
            if let Err(reason) = &result.result {
                println!(
                    "source_mission_exclusion_unresolved offset={} owner={:?} reason={reason}",
                    result.source_offset, result.object
                );
            }
        }
        let world_errors = plan
            .world_entries
            .iter()
            .filter(|p| p.class_input().is_err())
            .count();
        let template_errors = plan
            .state_templates()
            .values()
            .filter(|s| s.is_err())
            .count();
        println!(
            "source_instances={} class_state_templates={} source_instance_errors={} class_state_template_errors={} instance_behavior_callbacks=false",
            plan.entries.len(),
            plan.state_templates().len(),
            source_errors,
            template_errors
        );
        println!(
            "source_world_declarations={} source_world_class_errors={} world_pose_callbacks=false",
            plan.world_entries.len(),
            world_errors
        );
        let children = giants_game::child_creation::InitialPlan::prepare_enabled(&plan, 0);
        let runtime = giants_game::child_creation::Runtime::construct(&children, &plan);
        let weapons = giants_game::weapons::Registry::load(root, &children, &plan);
        match &weapons {
            Err(e) => println!("source_weapon_catalog_error={e}"),
            Ok(w) => {
                let resolved = w
                    .bindings
                    .iter()
                    .flat_map(|b| &b.effects)
                    .filter(|e| e.commands.is_ok())
                    .count();
                let effects = w.bindings.iter().flat_map(|b| &b.effects).count();
                println!(
                    "source_weapon_catalog records={} projectiles={} icons={} unprojected_records={} ignored_records={} inactive_records={}",
                    w.catalog.records.len(),
                    w.catalog.projectiles.len(),
                    w.catalog.icons.len(),
                    w.catalog.unprojected_records,
                    w.catalog.ignored_records,
                    w.catalog.inactive_records
                );
                println!(
                    "source_weapon_bindings={} binding_errors={} registered_classes={} effect_references={} resolved_effects={} full_projectile_execution=false",
                    w.bindings.len(),
                    w.errors.len(),
                    w.bindings.iter().filter(|b| b.class_registered).count(),
                    effects,
                    resolved
                );
                for b in &w.bindings {
                    println!(
                        "source_weapon_binding origin={} offset={} selector={} parameter={} class={} interval_bits={}",
                        if b.parent.0 { "world" } else { "mission" },
                        b.parent.1,
                        b.selector,
                        b.parameter,
                        b.class,
                        b.interval.to_bits()
                    );
                }
            }
        }
        // This independent initial inspection profile shares the existing
        // model warmup delta. Zero phase is explicit and is not native timing.
        let phase = 0.;
        let delta = giants_game::model_pose::BENCHMARK_INITIAL_STEP;
        let targets = weapons.as_ref().ok().map(|weapons| {
            giants_game::target_runtime::Runtime::initialize(
                &children, &plan, weapons, phase, delta,
            )
        });
        if let Some(targets) = &targets {
            println!(
                "source_target_profile phase_bits={} delta_bits={} native_phase_aligned=false",
                phase.to_bits(),
                delta.to_bits()
            );
            println!(
                "source_targets={} target_errors={} references={} simulation=initial_idle_only rendering=false",
                targets.instances.len(),
                targets.errors.len(),
                targets
                    .instances
                    .iter()
                    .map(|t| t.references.len())
                    .sum::<usize>()
            );
            for t in &targets.instances {
                println!(
                    "source_initial_target origin={} offset={} selector={} response_bits={} point_bits={:?}",
                    if t.parent.0 { "world" } else { "mission" },
                    t.parent.1,
                    t.selector,
                    t.response.to_bits(),
                    t.target.map(f32::to_bits)
                );
            }
            for error in &targets.errors {
                println!("source_target_error={error}");
            }
        }
        println!(
            "source_runtime_children={} child_construction_errors={} scope=partial_constructor_profile effect_callbacks=false",
            runtime.instances.len(),
            runtime.errors.len()
        );
        for e in &runtime.errors {
            println!("source_child_construction_error={e}");
        }
        let scene = Self {
            plan,
            children,
            runtime,
            weapons,
            targets,
        };
        println!(
            "source_initial_child_requests={} parent_pairs={} child_classes={} child_class_errors={} child_plan_errors={} child_runtime_execution=initial_profile secondary_flags_profile=0",
            scene.children.pairs.len() * 2,
            scene.children.pairs.len(),
            scene.children.classes.len(),
            scene
                .children
                .classes
                .values()
                .filter(|c| c.is_err())
                .count(),
            scene.children.diagnostics.len()
        );
        for pair in &scene.children.pairs {
            println!(
                "source_child_pair origin={} offset={} parent_class={} children={:?} parameter={} selector={} allocation_flags={:08x}",
                if pair.parent.0 { "world" } else { "mission" },
                pair.parent.1,
                pair.parent_class,
                pair.children,
                pair.variant.parameter,
                pair.variant.selector,
                pair.allocation_flags,
            );
        }
        Ok(scene)
    }
}

/// Logical objects start hidden; rendering and effect callbacks remain separate.
#[derive(Component)]
pub struct RuntimeChild(pub giants_game::child_creation::ChildId);

pub fn spawn_children(mut commands: Commands, scene: Res<Scene>) {
    for child in &scene.runtime.instances {
        commands.spawn((
            RuntimeChild(child.id),
            Name::new(format!("Initial child {}", child.id.0)),
        ));
    }
}

pub fn report_children(
    scene: Res<Scene>,
    children: Query<&RuntimeChild>,
    mut reported: Local<bool>,
) {
    if *reported {
        return;
    }
    let ids: std::collections::BTreeSet<_> = children.iter().map(|c| c.0.0).collect();
    let expected: std::collections::BTreeSet<_> =
        scene.runtime.instances.iter().map(|c| c.id.0).collect();
    assert_eq!(children.iter().count(), expected.len());
    assert_eq!(ids, expected);
    println!("source_weapon_registry_retained={}", scene.weapons.is_ok());
    println!(
        "source_target_registry_retained={}",
        scene.targets.is_some()
    );
    println!(
        "source_runtime_children_ecs={} ids_unique=true scope=partial_constructor_profile effects=false",
        ids.len()
    );
    *reported = true;
}

/// Inspection playback supplies its elapsed delta; this does not certify normal
/// game cadence or construct native slow-area lifetimes and firing triggers.
pub fn step_children(time: Res<Time>, mut scene: ResMut<Scene>, mut reported: Local<bool>) {
    let pairs = scene.runtime.step_children(time.delta_secs(), &[]);
    if !*reported {
        let hidden = scene
            .runtime
            .instances
            .iter()
            .filter(|c| c.fields.active & 0x40000000 != 0)
            .count();
        println!(
            "source_child_countdown_pairs={pairs} hidden_children={hidden} clock=viewer_delta local_slow_areas=0 full_effect_callbacks=false"
        );
        *reported = true;
    }
}
