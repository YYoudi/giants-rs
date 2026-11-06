//! Geometry-header tail after the acquired animation/effect phase. Mode selection,
//! animation changes, reference publication and world dispatch remain separate.
use crate::{kabuto_submission::State, procedural_aim::Matrix};

#[derive(Clone, Copy)]
pub struct Camera {
    pub position: [f32; 3],
    pub direction: [f32; 3],
}
pub struct Body {
    pub matrix: Matrix,
    /// Current model attachment input, not a baked model-space constant.
    pub attachment: [f32; 3],
    pub scale: f32,
    pub target_bounds: Option<([f32; 3], [f32; 3])>,
}

/// `selected_camera` exists only for the selected local owner. Values are PC
/// source-space inputs. This does not choose the mode or run the preceding phase.
pub fn prepare(
    mut previous: State,
    mode: u8,
    selected_camera: Option<Camera>,
    body: &Body,
) -> Result<State, &'static str> {
    previous.enabled = false;
    if mode == 0 || (mode == 1 && selected_camera.is_none()) {
        return Ok(previous);
    }
    if let Some(camera) = selected_camera {
        if !camera
            .position
            .into_iter()
            .chain(camera.direction)
            .chain([previous.weight])
            .all(f32::is_finite)
        {
            return Err("non-finite Kabuto camera projection");
        }
        previous.position = std::array::from_fn(|i| {
            (f64::from(camera.direction[i]) * 20. + f64::from(camera.position[i])) as f32
        });
        previous.direction = camera.direction;
        let width = (0.5 - f64::from(previous.weight) * f64::from(0.4f32)) as f32;
        previous.scale = [width, width, 50.];
        previous.enabled = true;
        previous.flags = 0x60;
        previous.state_word = 2;
    } else if mode >= 2 {
        if !body
            .matrix
            .values
            .into_iter()
            .chain(body.attachment)
            .chain([body.scale, previous.weight])
            .all(f32::is_finite)
            || body.matrix.flags & 0x20 != 0
            || body
                .target_bounds
                .is_some_and(|(low, high)| !low.into_iter().chain(high).all(f32::is_finite))
        {
            return Err("Kabuto body projection outside finite affine profile");
        }
        previous.position = body.matrix.transform_point(body.attachment);
        previous.direction = [
            body.matrix.values[1],
            body.matrix.values[5],
            body.matrix.values[9],
        ]
        .map(|v| -v);
        let width = ((3. - f64::from(previous.weight)) * f64::from(body.scale)) as f32;
        previous.scale = [width, width, (f64::from(body.scale) * 200.) as f32];
        if let Some((low, high)) = body.target_bounds {
            previous.direction = std::array::from_fn(|i| {
                let center = (f64::from(low[i]) + f64::from(high[i])) * 0.5;
                let center = if i == 2 {
                    f64::from(center as f32)
                } else {
                    center
                };
                (center - f64::from(previous.position[i])) as f32
            });
            let [x, y, z] = previous.direction.map(f64::from);
            let squared = ((x * x + y * y) + z * z) as f32;
            previous.scale[2] = f64::from(squared).sqrt() as f32;
            previous.enabled = true;
        }
        previous.flags = 0;
        previous.state_word = 1;
    }
    if !previous
        .position
        .into_iter()
        .chain(previous.direction)
        .chain(previous.scale)
        .all(f32::is_finite)
    {
        return Err("Kabuto projection overflow");
    }
    Ok(previous)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn previous() -> State {
        State {
            enabled: true,
            position: [7.; 3],
            direction: [8.; 3],
            scale: [9.; 3],
            weight: 1.,
            flags: 123,
            state_word: 456,
        }
    }
    fn body() -> Body {
        let mut matrix = Matrix::identity();
        matrix.flags = 16;
        for (i, value) in [10., 20., 30.].into_iter().enumerate() {
            matrix.values[i * 4 + 3] = value;
        }
        Body {
            matrix,
            attachment: [1., 2., 3.],
            scale: 2.,
            target_bounds: None,
        }
    }
    #[test]
    fn inactive_paths_only_disable_without_reading_unused_inputs() {
        let mut invalid = body();
        invalid.scale = f32::NAN;
        let old = State {
            position: [f32::NAN; 3],
            ..previous()
        };
        for mode in [0, 1] {
            let out = prepare(old, mode, None, &invalid).unwrap();
            assert!(!out.enabled);
            assert_eq!(
                out.position.map(f32::to_bits),
                old.position.map(f32::to_bits)
            );
            assert_eq!(out.scale, old.scale);
            assert_eq!(out.flags, old.flags);
            assert_eq!(out.state_word, old.state_word);
        }
    }
    #[test]
    fn selected_camera_overrides_body_and_target_for_every_nonzero_mode() {
        let camera = Camera {
            position: [1., 2., 3.],
            direction: [0., 1., 0.],
        };
        let mut invalid = body();
        invalid.scale = f32::NAN;
        for mode in [1, 2, 3, 255] {
            let out = prepare(
                State {
                    weight: 0.,
                    ..previous()
                },
                mode,
                Some(camera),
                &invalid,
            )
            .unwrap();
            assert!(out.enabled);
            assert_eq!(out.position, [1., 22., 3.]);
            assert_eq!(out.direction, camera.direction);
            assert_eq!(out.scale, [0.5, 0.5, 50.]);
            assert_eq!((out.flags, out.state_word), (0x60, 2));
        }
    }
    #[test]
    fn body_without_target_updates_transform_but_stays_disabled() {
        let out = prepare(previous(), 2, None, &body()).unwrap();
        assert!(!out.enabled);
        assert_eq!(out.position, [11., 22., 33.]);
        assert_eq!(out.direction, [0., -1., 0.]);
        assert_eq!(out.scale, [4., 4., 400.]);
        assert_eq!((out.flags, out.state_word), (0, 1));
    }
    #[test]
    fn target_distance_flows_into_consumable_submission_and_refuses_overflow() {
        let mut input = body();
        input.target_bounds = Some(([11., 25., 37.], [11., 25., 37.]));
        let out = prepare(previous(), 3, None, &input).unwrap();
        assert!(out.enabled);
        assert_eq!(out.direction, [0., 3., 4.]);
        assert_eq!(out.scale, [4., 4., 5.]);
        let geometry = crate::kabuto_geometry::prepare(3, 2, 1.).unwrap();
        let mut random = crate::audio_parameters::Random(5);
        let packet = crate::kabuto_submission::prepare(out, &geometry, &mut random)
            .unwrap()
            .unwrap();
        let mut calls = 0;
        packet
            .emit::<()>(|_| {
                calls += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(calls, 7);
        input.target_bounds = Some(([f32::MAX; 3], [f32::MAX; 3]));
        assert!(prepare(previous(), 3, None, &input).is_err());
    }
}
