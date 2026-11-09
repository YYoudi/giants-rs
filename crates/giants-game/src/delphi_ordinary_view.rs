//! Acquired ordinary mode-zero, local class-3 camera phase. Authored overrides,
//! alternate owners, vehicles, smoothing and shake must be excluded by the
//! caller. Viewport/depth and world providers remain explicit boundaries.
use crate::{camera_lens, camera_projection, delphi_camera::OrdinaryProfiles, mecc_camera_update};

#[derive(Clone, Copy)]
pub struct Controls {
    /// Global frame time, separate from actor-local slowed time.
    pub seconds: f32,
    pub transition_input: bool,
    pub scaled_input: bool,
    pub input_scale: f32,
    pub projection: camera_projection::Parameters,
}

pub struct Input {
    pub number: u64,
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub look_height: f32,
    pub scale: f32,
    pub skip_floor: bool,
    pub control_flags: u32,
    pub controls: Controls,
}

#[derive(Clone, Debug)]
pub struct Output {
    pub pose: mecc_camera_update::Output,
    pub projection: camera_projection::Output,
}

#[derive(Default)]
pub struct State {
    number: u64,
    lens: camera_lens::State,
    pose: mecc_camera_update::State,
    projection: Option<camera_projection::Output>,
    output: Option<Output>,
    failure: Option<String>,
}
impl State {
    pub fn transition(&self) -> f32 {
        self.lens.transition()
    }
    pub fn projection(&self) -> Option<&camera_projection::Output> {
        self.projection.as_ref()
    }
    pub fn output(&self) -> Option<&Output> {
        self.output.as_ref()
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    /// Consume the next camera invocation exactly once. Lens and projection
    /// finish before pose work; their effects survive a later refusal. A failed
    /// phase is quarantined rather than replaying completed state changes.
    pub fn advance(
        &mut self,
        input: Input,
        profiles: OrdinaryProfiles,
        world: &mut impl mecc_camera_update::World,
    ) -> Result<Output, String> {
        if self.failure.is_some() || self.number.checked_add(1) != Some(input.number) {
            return Err("Delphi ordinary view requires its next nonfailed invocation".into());
        }
        self.number = input.number;
        let result = self.run(input, profiles, world);
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
        i: Input,
        profiles: OrdinaryProfiles,
        world: &mut impl mecc_camera_update::World,
    ) -> Result<Output, String> {
        let c = i.controls;
        let profile = profiles.select(c.scaled_input);
        let projection = camera_projection::advance(
            &mut self.lens,
            camera_lens::Input {
                profile_fov: profile.vertical_fov,
                override_fov: None,
                seconds: c.seconds,
                inhibited: false,
                transition_input: c.transition_input,
                scaled_input: c.scaled_input,
                mode: 0,
                profile_index: 0,
                selected_kind: Some(3),
                input_scale: c.input_scale,
            },
            c.projection,
        )?;
        self.projection = Some(projection);
        let pose = self.pose.advance_delphi_third_person(
            mecc_camera_update::Input {
                number: i.number,
                position: i.position,
                yaw: i.yaw,
                pitch: i.pitch,
                look_height: i.look_height,
                scale: i.scale,
                skip_floor: i.skip_floor,
                seconds: c.seconds,
                profile,
            },
            i.control_flags,
            world,
        )?;
        Ok(Output { pose, projection })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profiles() -> OrdinaryProfiles {
        use crate::executable_metadata::{Layout, synthetic_image};
        let mut bytes = synthetic_image(Layout::DebugReference);
        let at = 4096 + 0x15a2e0 - 1_413_120;
        for (row, values) in [[20_f32, 5., 60., 0., 0.], [5., 2., 50., 0., 0.]]
            .into_iter()
            .enumerate()
        {
            for (i, value) in values.into_iter().enumerate() {
                let offset = at + row * 24 + i * 4;
                bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        OrdinaryProfiles::decode(&bytes).unwrap()
    }
    fn input(number: u64) -> Input {
        Input {
            number,
            position: [0., 0., 30.],
            yaw: 0.,
            pitch: 0.,
            look_height: 6.,
            scale: 1.,
            skip_floor: true,
            control_flags: 0,
            controls: Controls {
                seconds: 0.25,
                transition_input: true,
                scaled_input: true,
                input_scale: 1.,
                projection: camera_projection::Parameters {
                    aspect: None,
                    viewport: [1920., 1080.],
                    near: 1.,
                    far: 10_000.,
                    limit_far: true,
                    depth_reference: [0.; 3],
                    depth_axis: [1., 0., 0.],
                },
            },
        }
    }
    #[derive(Default)]
    struct FailingWorld(u32);
    impl mecc_camera_update::World for FailingWorld {
        fn height(&mut self, _: [f32; 2]) -> Result<f32, String> {
            panic!("floor skipped")
        }
        fn clip(&mut self, _: &mut [f32; 3], _: [f32; 3]) -> Result<(), String> {
            self.0 += 1;
            Err("declared wall-provider failure".into())
        }
    }
    #[test]
    fn late_pose_failure_retains_selected_lens_projection_and_prevents_replay() {
        let mut state = State::default();
        let mut world = FailingWorld::default();
        assert!(state.advance(input(2), profiles(), &mut world).is_err());
        assert_eq!(state.transition(), 0.);
        assert!(state.failure().is_none());
        assert!(state.advance(input(1), profiles(), &mut world).is_err());
        assert_eq!(world.0, 1);
        assert_eq!(state.transition(), 0.25);
        // Uses row one (50 degrees), then blends toward 25 before pose work.
        assert_eq!(state.projection().unwrap().vertical_fov, 43.75);
        assert!(state.output().is_none());
        assert!(state.failure().is_some());
        for number in [1, 2] {
            assert!(
                state
                    .advance(input(number), profiles(), &mut world)
                    .is_err()
            );
        }
        assert_eq!(state.transition(), 0.25);
        assert_eq!(world.0, 1);
    }
    #[test]
    fn projection_refusal_retains_lens_without_entering_pose() {
        let mut state = State::default();
        let mut world = FailingWorld::default();
        let mut input = input(1);
        input.controls.projection.depth_reference[0] = 12_500.;
        assert!(state.advance(input, profiles(), &mut world).is_err());
        assert_eq!(state.transition(), 0.25);
        assert!(state.projection().is_none());
        assert!(state.failure().is_some());
        assert_eq!(world.0, 0);
    }
}
