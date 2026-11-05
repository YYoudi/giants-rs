//! Stateful ordinary camera producer. The caller owns actor/profile selection,
//! its world phase and the current floor/mesh providers. Alternate character,
//! carried/vehicle, smoothed-angle and shake branches remain separate.
use crate::{first_person::Camera, mecc_camera::Pose};

#[derive(Clone, Copy, Debug)]
pub struct Profile {
    pub distance: f32,
    pub height: f32,
    pub side: f32,
    pub along: f32,
    pub vertical_fov: f32,
}
impl Profile {
    pub fn normal(first_person: bool) -> Self {
        Self {
            distance: if first_person { 2. } else { 16. },
            height: if first_person { 6. } else { 8. },
            side: 0.,
            along: 0.,
            vertical_fov: 62.,
        }
    }
}
#[derive(Clone, Copy)]
pub struct Input {
    pub number: u64,
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    /// Actual header look-height, kept separate from scale until the wall ray.
    pub look_height: f32,
    pub scale: f32,
    pub skip_floor: bool,
    pub seconds: f32,
    pub profile: Profile,
}
pub trait World {
    fn height(&mut self, xy: [f32; 2]) -> Result<f32, String>;
    /// Perform the actual ordered camera mesh query, shortening eye in place.
    fn clip(&mut self, eye: &mut [f32; 3], target: [f32; 3]) -> Result<(), String>;
}
#[derive(Clone, Debug)]
pub struct Output {
    pub number: u64,
    pub camera: Camera,
    pub pose: Pose,
    pub view: [f32; 16],
    pub viewport_origin: [f32; 3],
    pub viewport_forward: [f32; 3],
    pub floor_queries: u32,
    pub chase: [f32; 4],
}
#[derive(Default)]
pub struct State {
    number: u64,
    initialized: bool,
    chase: [f32; 4],
    output: Option<Output>,
    failure: Option<String>,
}
impl State {
    pub fn output(&self) -> Option<&Output> {
        self.output.as_ref()
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub fn advance(&mut self, input: Input, world: &mut impl World) -> Result<Output, String> {
        let p = input.profile;
        if self.failure.is_some()
            || self.number.checked_add(1) != Some(input.number)
            || !input
                .position
                .into_iter()
                .chain([
                    input.yaw,
                    input.pitch,
                    input.look_height,
                    input.scale,
                    input.seconds,
                    p.distance,
                    p.height,
                    p.side,
                    p.along,
                    p.vertical_fov,
                ])
                .all(f32::is_finite)
            || input.yaw.abs() > 1e6
            || input.pitch.abs() > 1e6
            || input.seconds < 0.
            || input.scale <= 0.
            || p.distance < 0.
            || !(0. ..180.).contains(&p.vertical_fov)
            || p.vertical_fov == 0.
        {
            return Err("camera requires the next finite ordinary source frame".into());
        }
        self.number = input.number;
        let result = self.run(input, world);
        match result {
            Ok(output) => {
                self.output = Some(output.clone());
                Ok(output)
            }
            Err(error) => {
                self.failure = Some(error.clone());
                Err(error)
            }
        }
    }
    fn run(&mut self, input: Input, world: &mut impl World) -> Result<Output, String> {
        let p = input.profile;
        let mut distance = p.distance;
        let mut pitch = input.pitch;
        if !self.initialized {
            self.chase = [p.distance, p.height, p.side, p.along];
            self.initialized = true;
        }
        let mut floor_queries = 0;
        let (mut eye, forward, up) = loop {
            let (sy, cy) = crate::object_transform::sincos(input.yaw);
            let (sp, cp) = crate::object_transform::sincos(pitch);
            let forward = [(cp * sy) as f32, (-(cp * cy)) as f32, sp as f32];
            let up = [(-(sp * sy)) as f32, (sp * cy) as f32, cp as f32];
            let mut eye = [
                ((sy * f64::from(p.side) + cy * f64::from(p.along) + f64::from(input.position[0]))
                    - f64::from(forward[0]) * f64::from(distance)) as f32,
                ((sy * f64::from(p.along) - cy * f64::from(p.side) + f64::from(input.position[1]))
                    - f64::from(forward[1]) * f64::from(distance)) as f32,
                (f64::from(input.position[2]) - (sp * f64::from(distance) - f64::from(p.height)))
                    as f32,
            ];
            if input.skip_floor {
                break (eye, forward, up);
            }
            floor_queries += 1;
            let height = world.height([eye[0], eye[1]])?;
            if !height.is_finite() {
                return Err("camera floor provider returned a nonfinite height".into());
            }
            let floor = f64::from(height).max(0.) + 4.;
            if floor - f64::from(eye[2]) <= 10. {
                if f64::from(eye[2]) < floor {
                    eye[2] = floor as f32;
                }
                break (eye, forward, up);
            }
            pitch = (f64::from(pitch) - 1.) as f32;
            distance = (f64::from(distance) * f64::from(0.95_f32)) as f32;
            // The last failed iteration retains its computed eye/basis while
            // publishing the already adjusted pitch and chase distance.
            if floor_queries == 50 {
                break (eye, forward, up);
            }
        };
        for (current, target) in self
            .chase
            .iter_mut()
            .zip([distance, p.height, p.side, p.along])
        {
            if *current != target {
                approach(current, target, input.seconds);
            }
        }
        let target = [
            input.position[0],
            input.position[1],
            (f64::from(input.position[2]) + f64::from(input.look_height) * f64::from(input.scale))
                as f32,
        ];
        world.clip(&mut eye, target)?;
        if !eye.into_iter().all(f32::is_finite) {
            return Err("camera mesh provider returned a nonfinite eye".into());
        }
        let view = view(eye, forward, up)?;
        let viewport_origin = std::array::from_fn(|i| {
            (((f64::from(view[i * 4 + 3]) - f64::from(view[i * 4 + 2]) * f64::from(view[14]))
                - f64::from(view[13]) * f64::from(view[i * 4 + 1]))
                - f64::from(view[12]) * f64::from(view[i * 4])) as f32
        });
        Ok(Output {
            number: input.number,
            camera: Camera {
                position: eye,
                angles: [input.yaw, pitch, 0.],
            },
            pose: Pose {
                eye,
                forward,
                up,
                vertical_fov: p.vertical_fov,
            },
            view,
            viewport_origin,
            viewport_forward: [view[2], view[6], view[10]],
            floor_queries,
            chase: self.chase,
        })
    }
}
fn approach(current: &mut f32, target: f32, seconds: f32) {
    let proportional = ((f64::from(*current) - f64::from(target)).abs() / f64::from(0.1_f32)
        * f64::from(seconds)) as f32;
    let amount = (f64::from(seconds) * 10.).max(f64::from(proportional));
    if *current > target {
        let next = f64::from(*current) - amount;
        *current = if next < f64::from(target) {
            target
        } else {
            next as f32
        };
        if f64::from(*current) - f64::from(target) > 50. {
            *current = (f64::from(target) + 50.) as f32;
        }
    } else if *current < target {
        let next = f64::from(*current) + amount;
        *current = if next > f64::from(target) {
            target
        } else {
            next as f32
        };
        if f64::from(target) - f64::from(*current) > 50. {
            *current = (f64::from(target) - 50.) as f32;
        }
    }
}
/// Source view construction, including normalization and stored-vector rounding.
pub fn view(eye: [f32; 3], forward: [f32; 3], up: [f32; 3]) -> Result<[f32; 16], String> {
    if !eye.into_iter().chain(forward).chain(up).all(f32::is_finite) {
        return Err("nonfinite camera basis".into());
    }
    let [x, y, z] = forward.map(f64::from);
    let reciprocal = 1. / ((z * z + y * y) + x * x).sqrt();
    let [x, y, z] = forward.map(|v| f64::from(v) * reciprocal);
    let [u, v, w] = up.map(f64::from);
    let a = z * v - y * w;
    let b = x * w - z * u;
    let c = f64::from((y * u - x * v) as f32);
    let q = f64::from((y * c - z * b) as f32);
    let r = f64::from((z * a - x * c) as f32);
    let s = f64::from((x * b - y * a) as f32);
    let side_reciprocal = 1. / ((c * c + b * b) + a * a).sqrt();
    // The first side component is stored with the wide reciprocal; the other
    // two reload its binary32 spill. Forward remains wide for translation.
    let side_a = (side_reciprocal * a) as f32;
    let side_b = f64::from(side_reciprocal as f32) * b;
    let side_c = f64::from(side_reciprocal as f32) * c;
    let up_reciprocal = 1. / ((s * s + r * r) + q * q).sqrt();
    let uv = [
        (q * up_reciprocal) as f32,
        (r * up_reciprocal) as f32,
        (s * up_reciprocal) as f32,
    ];
    let [ex, ey, ez] = eye.map(f64::from);
    let matrix = [
        -side_a,
        uv[0],
        x as f32,
        0.,
        -side_b as f32,
        uv[1],
        y as f32,
        0.,
        -side_c as f32,
        uv[2],
        z as f32,
        0.,
        ((side_b * ey + side_c * ez) + f64::from(side_a) * ex) as f32,
        -((f64::from(uv[2]) * ez + f64::from(uv[1]) * ey) + f64::from(uv[0]) * ex) as f32,
        -((z * ez + y * ey) + x * ex) as f32,
        1.,
    ];
    if !matrix.into_iter().all(f32::is_finite) {
        return Err("camera basis is degenerate or overflowing".into());
    }
    Ok(matrix)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Provider {
        height: f32,
        floor_calls: u32,
        clip_calls: u32,
        fail_floor: bool,
        fail_clip: bool,
    }
    impl World for Provider {
        fn height(&mut self, _: [f32; 2]) -> Result<f32, String> {
            self.floor_calls += 1;
            if self.fail_floor {
                Err("floor unavailable".into())
            } else {
                Ok(self.height)
            }
        }
        fn clip(&mut self, _: &mut [f32; 3], _: [f32; 3]) -> Result<(), String> {
            self.clip_calls += 1;
            if self.fail_clip {
                Err("mesh unavailable".into())
            } else {
                Ok(())
            }
        }
    }
    fn input(number: u64) -> Input {
        Input {
            number,
            position: [0.; 3],
            yaw: 0.,
            pitch: 0.,
            look_height: 6.,
            scale: 1.,
            skip_floor: false,
            seconds: 1. / 60.,
            profile: Profile::normal(false),
        }
    }
    #[test]
    fn invalid_or_stale_input_never_consumes_a_provider_frame() {
        let mut state = State::default();
        let mut provider = Provider::default();
        assert!(state.advance(input(0), &mut provider).is_err());
        let mut invalid = input(1);
        invalid.scale = f32::NAN;
        assert!(state.advance(invalid, &mut provider).is_err());
        assert_eq!((provider.floor_calls, provider.clip_calls), (0, 0));
        state.advance(input(1), &mut provider).unwrap();
        assert!(state.advance(input(1), &mut provider).is_err());
        assert_eq!((provider.floor_calls, provider.clip_calls), (1, 1));
        assert!(state.failure().is_none());
        state.advance(input(2), &mut provider).unwrap();
    }
    #[test]
    fn provider_failure_latches_without_republishing_or_replaying_partial_state() {
        for fail_floor in [true, false] {
            let mut state = State::default();
            let mut provider = Provider::default();
            let previous = state.advance(input(1), &mut provider).unwrap();
            provider.fail_floor = fail_floor;
            provider.fail_clip = !fail_floor;
            assert!(state.advance(input(2), &mut provider).is_err());
            let calls = (provider.floor_calls, provider.clip_calls);
            assert!(state.failure().is_some());
            assert_eq!(state.output().unwrap().number, 1);
            assert_eq!(state.output().unwrap().view, previous.view);
            provider.fail_floor = false;
            provider.fail_clip = false;
            assert!(state.advance(input(2), &mut provider).is_err());
            assert!(state.advance(input(3), &mut provider).is_err());
            assert_eq!((provider.floor_calls, provider.clip_calls), calls);
        }
    }
    #[test]
    fn floor_iteration_cap_and_skip_still_execute_the_mesh_query() {
        let mut state = State::default();
        let mut provider = Provider {
            height: 1200.,
            ..Default::default()
        };
        let out = state.advance(input(1), &mut provider).unwrap();
        assert_eq!(out.floor_queries, 50);
        assert_eq!(provider.clip_calls, 1);
        let mut next = input(2);
        next.skip_floor = true;
        let out = state.advance(next, &mut provider).unwrap();
        assert_eq!(out.floor_queries, 0);
        assert_eq!((provider.floor_calls, provider.clip_calls), (50, 2));
    }
    #[test]
    fn unusable_view_bases_are_not_published_as_nan_matrices() {
        assert!(view([0.; 3], [0.; 3], [0., 0., 1.]).is_err());
        assert!(view([0.; 3], [0., 0., 1.], [0., 0., 1.]).is_err());
        assert!(view([f32::INFINITY, 0., 0.], [0., 1., 0.], [0., 0., 1.]).is_err());
    }
}
