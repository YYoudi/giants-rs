//! Persistent source lens state, presented as a byte-space additive veil.
use bevy::prelude::*;
use giants_game::{animation::Evaluator, sun_glare, terrain_visibility};

#[derive(Resource)]
pub struct State {
    intensity: f32,
    generation: u64,
    last_clock: Option<u32>,
    frozen_applied: bool,
    steps: usize,
}
fn step(
    world: &giants_game::WorldAsset,
    intensity: f32,
    camera: &giants_game::animation::Camera,
    dt: f32,
) -> Result<f32, Box<dyn std::error::Error>> {
    let parameters = world
        .config
        .sun_glare
        .as_ref()
        .ok_or("Missing lens parameters")?;
    let ray = sun_glare::ray(
        world.sun,
        camera.origin,
        [camera.view[2], camera.view[6], camera.view[10]],
    )?;
    let clear = terrain_visibility::clear(&world.terrain, &ray)?;
    Ok(sun_glare::advance(intensity, dt, ray.dot, clear, parameters.exponent, 2.)?.0)
}
impl State {
    pub fn load(
        root: &std::path::Path,
        world: &giants_game::WorldAsset,
        cinematic: &crate::cinematic_camera::Cinematic,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // The terrain-only visibility profile is validated on these six directors.
        for segment in cinematic.segments() {
            if !(1..=6).any(|i| {
                segment
                    .sequence()
                    .eq_ignore_ascii_case(&format!("story4_t_square_{i}"))
            }) {
                return Err("Lens visibility beyond the measured six-director profile".into());
            }
        }
        let mut intensity = 0.;
        let mut steps = 0;
        if !cinematic.realtime() {
            let mut sequence = "story4_t_square_1".to_string();
            let mut reached = false;
            for _ in 0..6 {
                let animation = giants_game::animation::load_director(root, &sequence)?;
                let selected = sequence.eq_ignore_ascii_case(cinematic.sequence());
                let [start, end, tpf, fps] = animation.timeline;
                let stop = if selected { cinematic.tick() } else { end };
                let samples = ((f64::from(stop - start) / f64::from(tpf) / f64::from(fps)) * 60.)
                    .ceil() as usize;
                for sample in 0..=samples {
                    let tick = (f64::from(start)
                        + sample as f64 / 60. * f64::from(tpf) * f64::from(fps))
                        as i32;
                    let camera = Evaluator {
                        animation: &animation,
                    }
                    .camera(tick.min(stop))?;
                    intensity = step(world, intensity, &camera, 1. / 60.)?;
                    steps += 1;
                }
                if selected {
                    reached = true;
                    break;
                }
                let program = giants_formats::flick::parse(&std::fs::read(
                    giants_game::flick::sequence_path(root, &sequence)?,
                )?)?;
                sequence = program
                    .next
                    .ok_or("Lens warmup chain ended before selected director")?;
            }
            if !reached {
                return Err("Lens warmup did not reach selected director".into());
            }
        }
        println!(
            "cinematic_glare_ready intensity_bits={} canonical_warmup_steps={} visibility=source_terrain_only history={}",
            intensity.to_bits(),
            steps,
            if cinematic.realtime() {
                "actual_runtime_steps"
            } else {
                "canonical_60hz_inspection_not_native_history"
            }
        );
        Ok(Self {
            intensity,
            generation: cinematic.generation(),
            last_clock: None,
            frozen_applied: false,
            steps,
        })
    }
}
pub fn update(
    mut state: ResMut<State>,
    cinematic: Res<crate::cinematic_camera::Cinematic>,
    world: Res<crate::world_scene::Scene>,
    pipelines: Res<crate::PipelinesReady>,
    time: Res<Time>,
    mut materials: ResMut<Assets<crate::world_composition::Presentation>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !pipelines.0 || cinematic.paused() {
        return;
    }
    if state.generation != cinematic.generation() {
        state.intensity = 0.;
        state.generation = cinematic.generation();
        state.last_clock = None;
        state.steps = 0;
    }
    if !cinematic.realtime() && state.frozen_applied {
        return;
    }
    if cinematic.realtime()
        && cinematic.benchmark()
        && state.last_clock == Some(cinematic.clock_steps())
    {
        return;
    }
    let result = (|| -> Result<_, Box<dyn std::error::Error>> {
        if cinematic.realtime() {
            let camera = Evaluator {
                animation: cinematic.animation(),
            }
            .camera(cinematic.tick())?;
            state.intensity = step(
                &world.asset,
                state.intensity,
                &camera,
                if cinematic.benchmark() {
                    1. / 60.
                } else {
                    time.delta_secs()
                },
            )?;
            state.steps += 1;
        }
        let parameters = world
            .asset
            .config
            .sun_glare
            .as_ref()
            .ok_or("Missing lens parameters")?;
        Ok(sun_glare::screen_rgb(parameters, state.intensity, 0.)?.unwrap_or([0; 3]))
    })();
    let rgb = match result {
        Ok(rgb) => rgb,
        Err(error) => {
            error!("Sun veil failed: {error}");
            exit.write(AppExit::Error(std::num::NonZeroU8::MIN));
            return;
        }
    };
    let Some(camera) = world.validation_camera.as_ref() else {
        return;
    };
    for (_, material) in materials.iter_mut() {
        material.glare = Vec4::new(
            f32::from(rgb[0]) / 255.,
            f32::from(rgb[1]) / 255.,
            f32::from(rgb[2]) / 255.,
            0.,
        );
        material.glare_viewport = camera.viewport.as_vec4();
    }
    state.last_clock = Some(cinematic.clock_steps());
    state.frozen_applied = true;
    info!(
        "cinematic_glare_step={} director={} tick={} intensity_bits={} rgb={:?} history={}",
        state.steps,
        cinematic.director_index(),
        cinematic.tick(),
        state.intensity.to_bits(),
        rgb,
        if cinematic.realtime() {
            "actual_runtime_steps"
        } else {
            "canonical_60hz_inspection"
        }
    );
}
