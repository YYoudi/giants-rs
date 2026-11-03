//! Read-only FX consumer/mesh evidence at the capture boundary, not pixels.
use super::*;

#[derive(Resource)]
pub(crate) struct EffectIsolation {
    pub offset: usize,
    pub hold_on_activation: bool,
    pub tracer_only: bool,
}

/// Diagnostic projection only: keep simulation and source permission unchanged,
/// but suppress meshes outside one source effect root before visibility checks.
pub(crate) fn isolate_effect_meshes(
    selected: Res<EffectIsolation>,
    scene: Res<Scene>,
    mut cinematic: ResMut<crate::cinematic_camera::Cinematic>,
    mut meshes: Query<(Entity, &mut Visibility), With<Mesh3d>>,
    mut reported: Local<bool>,
    mut exit: MessageWriter<bevy::app::AppExit>,
) {
    if !cinematic
        .program()
        .commands
        .iter()
        .any(|c| c.opcode == 39 && c.offset == selected.offset)
    {
        error!("FX isolation requires an effect command in the active director");
        exit.write(bevy::app::AppExit::Error(std::num::NonZeroU8::MIN));
        return;
    }
    let mut entities = scene.draws.iter().filter(|draw| {
        matches!(&scene.instances[draw.instance].source, Source::Effect { request, .. } if request.offset == selected.offset)
    }).map(|draw| draw.entity).collect::<BTreeSet<_>>();
    if selected.tracer_only {
        entities.clear();
    }
    if let Some(entity) = scene.tracers.entity() {
        entities.insert(entity);
    }
    let mut selected_admitted = false;
    for (entity, mut visibility) in &mut meshes {
        if !entities.contains(&entity) {
            *visibility = Visibility::Hidden;
        } else if *visibility != Visibility::Hidden {
            selected_admitted = true;
        }
    }
    if !*reported {
        println!(
            "cinematic_fx_isolation sequence={} offset={} reserved_mesh_entities={} simulation_unchanged=true native_occlusion_preserved=false pixels_verified=false",
            cinematic.sequence(),
            selected.offset,
            entities.len()
        );
        *reported = true;
    }
    if selected.hold_on_activation && selected_admitted && cinematic.animation_dt() > 0. && !cinematic.paused() && scene.instances.iter().any(|instance| {
        matches!(&instance.source, Source::Effect { request, .. } if request.offset == selected.offset)
            && !instance.parts.is_empty() && instance.visible(&cinematic)
            && instance.transparency().is_none_or(|t| t < 1.)
    }) {
        println!("cinematic_fx_activation_hold sequence={} offset={} tick={} seconds={} native_timing_verified=false viewer_dt_bits={}",
            cinematic.sequence(), selected.offset, cinematic.tick(), cinematic.director_seconds(), cinematic.animation_dt().to_bits());
        println!("cinematic_inspection_hold sequence={} tick={} seconds={} native_timing_verified=false",
            cinematic.sequence(), cinematic.tick(), cinematic.director_seconds());
        cinematic.suspend(true);
    }
}

fn runtime_state(instance: &Instance) -> (&'static str, Option<bool>, Option<bool>) {
    if let Some(state) = &instance.mixed {
        (
            "mixed",
            Some(state.presentation.is_some()),
            state.presentation.map(|p| p.expired),
        )
    } else if let Some(state) = &instance.emitted {
        (
            "emitted",
            Some(state.presentation.is_some()),
            state.presentation.map(|p| p.expired),
        )
    } else if let Some(state) = &instance.burst {
        (
            "burst",
            Some(state.playback.is_some()),
            state.playback.as_ref().map(|p| p.expired),
        )
    } else if let Some(state) = &instance.projectile {
        (
            "projectile",
            Some(state.playback.matrix.is_some()),
            Some(state.playback.expired),
        )
    } else if let Some(state) = &instance.timed {
        ("timed", Some(state.visual.is_some()), Some(state.expired))
    } else if let Some(state) = &instance.released {
        (
            "released",
            Some(state.playback.visual.is_some()),
            Some(state.playback.expired),
        )
    } else {
        ("unclassified", None, None)
    }
}

#[derive(Default)]
struct Count {
    templates: usize,
    playback_present: usize,
    playback_absent: usize,
    playback_unknown: usize,
    expired: usize,
    registered_meshes: usize,
    meshes: usize,
    enabled: usize,
    prepared: usize,
}

impl Scene {
    /// Retain the CPU mesh attributes actually submitted by the vertical route.
    /// Ordinary-route entities share these same handles and are not duplicated.
    pub fn capture_vertical_buffers(
        &self,
        cinematic: &crate::cinematic_camera::Cinematic,
        meshes: &bevy::prelude::Assets<Mesh>,
    ) {
        use bevy::mesh::VertexAttributeValues::{Float32x2, Float32x3, Float32x4};
        for draw in self.draws.iter().filter(|draw| draw.route == 2) {
            let instance = &self.instances[draw.instance];
            let Some(released) = &instance.released else {
                continue;
            };
            if released.routes.is_none() || released.playback.visual.is_none() {
                continue;
            }
            let Source::Effect { request, .. } = &instance.source else {
                continue;
            };
            let Some(mesh) = meshes.get(&draw.mesh) else {
                continue;
            };
            let (Some(Float32x3(position)), Some(Float32x2(uv)), Some(Float32x4(color))) = (
                mesh.attribute(Mesh::ATTRIBUTE_POSITION),
                mesh.attribute(Mesh::ATTRIBUTE_UV_0),
                mesh.attribute(Mesh::ATTRIBUTE_COLOR),
            ) else {
                continue;
            };
            let words = position
                .iter()
                .zip(uv.iter())
                .zip(color.iter())
                .flat_map(|((p, uv), color)| {
                    p.iter()
                        .chain(uv.iter())
                        .chain(color.iter())
                        .map(|v| v.to_bits())
                })
                .collect::<Vec<_>>();
            println!(
                "cinematic_particle_vertices sequence={} offset={} part={} words={:?} cpu_mesh_only=true native_pixels_verified=false",
                cinematic.sequence(),
                request.offset,
                draw.part,
                words
            );
        }
    }
    pub fn capture_effect_entities(
        &self,
        cinematic: &crate::cinematic_camera::Cinematic,
        mut observe: impl FnMut(Entity, &Handle<Mesh>) -> Option<(bool, bool)>,
    ) {
        let reached = cinematic.segments()[cinematic.director_index()]
            .effect_requests()
            .iter()
            .map(|(_, request)| request.offset)
            .collect::<BTreeSet<_>>();
        let mut groups = BTreeMap::<(usize, i32, &'static str), Count>::new();
        for (index, instance) in self.instances.iter().enumerate() {
            let Source::Effect { request, class, .. } = &instance.source else {
                continue;
            };
            let (family, present, expired) = runtime_state(instance);
            if let Some(released) = &instance.released
                && released.routes.is_some()
                && let Some(words) = released.vertical_state_words()
            {
                println!(
                    "cinematic_particle_state sequence={} offset={} class={} state_words={:?} loop_bits={} native_cadence_verified=false native_pixels_verified=false",
                    cinematic.sequence(),
                    request.offset,
                    class,
                    words,
                    instance
                        .animation
                        .as_ref()
                        .map_or(0, |a| a.clock.seconds.to_bits())
                );
            }
            let count = groups.entry((request.offset, *class, family)).or_default();
            count.templates += 1;
            count.playback_present += usize::from(present == Some(true));
            count.playback_absent += usize::from(present == Some(false));
            count.playback_unknown += usize::from(present.is_none());
            count.expired += usize::from(expired == Some(true));
            for draw in self.draws.iter().filter(|draw| draw.instance == index) {
                count.registered_meshes += 1;
                if let Some((enabled, prepared)) = observe(draw.entity, &draw.mesh) {
                    count.meshes += 1;
                    count.enabled += usize::from(enabled);
                    count.prepared += usize::from(prepared);
                }
            }
        }
        for command in cinematic
            .program()
            .commands
            .iter()
            .filter(|command| command.opcode == 39)
        {
            let templates = groups
                .iter()
                .filter(|((offset, _, _), _)| *offset == command.offset)
                .map(|(_, count)| count.templates)
                .sum::<usize>();
            println!(
                "cinematic_fx_entity_root sequence={} director={} tick={} offset={} source_request_reached={} rigid_templates={} rigid_consumer_only=true pixels_verified=false",
                cinematic.sequence(),
                cinematic.director_index(),
                cinematic.tick(),
                command.offset,
                reached.contains(&command.offset),
                templates
            );
        }
        for ((offset, class, family), count) in groups {
            println!(
                "cinematic_fx_entity_snapshot sequence={} director={} tick={} offset={} class={} family={} source_request_reached={} templates={} playback_present={} playback_absent={} playback_unknown={} expired={} registered_meshes={} matching_mesh_entities={} local_render_enabled={} gpu_prepared_entities={} rigid_consumer_only=true pixels_verified=false",
                cinematic.sequence(),
                cinematic.director_index(),
                cinematic.tick(),
                offset,
                class,
                family,
                reached.contains(&offset),
                count.templates,
                count.playback_present,
                count.playback_absent,
                count.playback_unknown,
                count.expired,
                count.registered_meshes,
                count.meshes,
                count.enabled,
                count.prepared
            );
        }
    }
}
