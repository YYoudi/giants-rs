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
    /// Native position/basis calculation before floor, chase and mesh work.
    /// Useful as a scoped inspection; it is not a completed ordinary camera.
    pub fn unclipped_pose(self, position: [f32; 3], yaw: f32, pitch: f32) -> Result<Pose, String> {
        if !position
            .into_iter()
            .chain([
                yaw,
                pitch,
                self.distance,
                self.height,
                self.side,
                self.along,
                self.vertical_fov,
            ])
            .all(f32::is_finite)
            || yaw.abs() > 1e6
            || pitch.abs() > 1e6
            || self.vertical_fov <= 0.
            || self.vertical_fov >= 180.
        {
            return Err("Unclipped camera requires finite profile and pose inputs".into());
        }
        let (sy, cy) = crate::object_transform::sincos(yaw);
        let (sp, cp) = crate::object_transform::sincos(pitch);
        let forward = [(cp * sy) as f32, (-(cp * cy)) as f32, sp as f32];
        let up = [(-(sp * sy)) as f32, (sp * cy) as f32, cp as f32];
        let eye = [
            ((sy * f64::from(self.side) + cy * f64::from(self.along) + f64::from(position[0]))
                - f64::from(forward[0]) * f64::from(self.distance)) as f32,
            ((sy * f64::from(self.along) - cy * f64::from(self.side) + f64::from(position[1]))
                - f64::from(forward[1]) * f64::from(self.distance)) as f32,
            (f64::from(position[2]) - (sp * f64::from(self.distance) - f64::from(self.height)))
                as f32,
        ];
        if !eye.into_iter().all(f32::is_finite) {
            return Err("Unclipped camera result exceeds finite profile".into());
        }
        Ok(Pose {
            eye,
            forward,
            up,
            vertical_fov: self.vertical_fov,
        })
    }
    /// Acquired class-1 controlled-actor adjustment, before chase initialization
    /// and the ordinary camera loop. This does not select an owner or view.
    pub fn for_kabuto(self, gain: f32) -> Result<Self, String> {
        if !gain.is_finite() || gain <= 0. {
            return Err("Kabuto camera gain outside acquired positive profile".into());
        }
        let g = f64::from(gain);
        Ok(Self {
            distance: (f64::from(self.distance) * g) as f32,
            height: (f64::from(self.height) * g - (1. - g) * 5.) as f32,
            side: (f64::from(self.side) * g) as f32,
            along: (f64::from(self.along) * g) as f32,
            vertical_fov: self.vertical_fov,
        })
    }
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
    /// Direct-angle ordinary camera with the acquired class-1 adjustment.
    /// Caller mode, actual owner/controlled identity, clock, pitch, initial
    /// history and world-query ownership remain required inputs.
    pub fn advance_kabuto(
        &mut self,
        mut input: Input,
        gain: f32,
        world: &mut impl World,
    ) -> Result<Output, String> {
        input.profile = input.profile.for_kabuto(gain)?;
        self.advance_profile(input, world, None)
    }
    pub fn output(&self) -> Option<&Output> {
        self.output.as_ref()
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub fn advance(&mut self, input: Input, world: &mut impl World) -> Result<Output, String> {
        self.advance_profile(input, world, None)
    }
    /// Ordinary third-person class-3 distance policy (source camera mode zero).
    /// The caller still supplies acquired
    /// profile/actor/world inputs; this does not select or schedule a player.
    pub fn advance_delphi_third_person(
        &mut self,
        input: Input,
        flags: u32,
        world: &mut impl World,
    ) -> Result<Output, String> {
        if !input.skip_floor && input.position[2] < 0. {
            return Err("Delphi underwater camera floor policy remains unacquired".into());
        }
        self.advance_profile(input, world, Some(flags))
    }
    fn advance_profile(
        &mut self,
        input: Input,
        world: &mut impl World,
        delphi_flags: Option<u32>,
    ) -> Result<Output, String> {
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
        let result = self.run(input, world, delphi_flags);
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
    fn run(
        &mut self,
        input: Input,
        world: &mut impl World,
        delphi_flags: Option<u32>,
    ) -> Result<Output, String> {
        let p = input.profile;
        let mut distance = p.distance;
        let mut pitch = input.pitch;
        if !self.initialized {
            self.chase = [p.distance, p.height, p.side, p.along];
            self.initialized = true;
        }
        let mut floor_queries = 0;
        let (mut eye, forward, up) = loop {
            let pose =
                Profile { distance, ..p }.unclipped_pose(input.position, input.yaw, pitch)?;
            let (mut eye, forward, up) = (pose.eye, pose.forward, pose.up);
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
        for (index, (current, target)) in self
            .chase
            .iter_mut()
            .zip([distance, p.height, p.side, p.along])
            .enumerate()
        {
            if let Some(flags) = delphi_flags.filter(|_| index == 0) {
                let adjusted =
                    crate::delphi_camera_distance::advance(target, *current, flags, input.seconds)?;
                *current = adjusted.chase;
            } else if *current != target {
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
    approach_with_time(current, target, seconds, 0.1_f32);
}
pub(crate) fn approach_with_time(current: &mut f32, target: f32, seconds: f32, time: f32) {
    let proportional = ((f64::from(*current) - f64::from(target)).abs() / f64::from(time)
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
    #[test]
    fn delphi_distance_policy_preserves_pose_order_and_common_failure_guards() {
        let mut delphi = State::default();
        let mut ordinary = State::default();
        let mut provider = Provider::default();
        let mut frame = input(1);
        frame.skip_floor = true;
        frame.position[2] = 100.;
        frame.seconds = 0.1;
        let baseline = ordinary.advance(frame, &mut provider).unwrap();
        let acquired = delphi
            .advance_delphi_third_person(frame, 0x100, &mut provider)
            .unwrap();
        // Native geometry is calculated before the distance chase callback.
        assert_eq!(acquired.pose.eye, baseline.pose.eye);
        assert_eq!(acquired.view, baseline.view);
        assert!(acquired.chase[0] > baseline.chase[0]);
        assert_eq!(acquired.chase[1..], baseline.chase[1..]);

        frame.number = 2;
        frame.skip_floor = false;
        frame.position[2] = -1.;
        let calls = (provider.floor_calls, provider.clip_calls);
        assert!(
            delphi
                .advance_delphi_third_person(frame, 0, &mut provider)
                .is_err()
        );
        assert_eq!((provider.floor_calls, provider.clip_calls), calls);
        assert!(delphi.failure().is_none());
        frame.position[2] = 100.;
        provider.fail_clip = true;
        assert!(
            delphi
                .advance_delphi_third_person(frame, 0, &mut provider)
                .is_err()
        );
        assert_eq!(delphi.output().unwrap().view, acquired.view);
        let calls = (provider.floor_calls, provider.clip_calls);
        provider.fail_clip = false;
        assert!(
            delphi
                .advance_delphi_third_person(frame, 0, &mut provider)
                .is_err()
        );
        assert_eq!((provider.floor_calls, provider.clip_calls), calls);
    }
}
