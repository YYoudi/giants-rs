//! Acquired finite perspective matrix and lens caches in PC source space.
//! Viewport/depth inputs and device publication are separate producers. This
//! does not apply Bevy's reversed-depth adaptation or certify rendered fidelity.

#[derive(Clone, Copy, Debug)]
pub struct Parameters {
    /// Absence selects the actual viewport ratio, narrowed before matrix use.
    pub aspect: Option<f32>,
    pub viewport: [f32; 2],
    pub near: f32,
    pub far: f32,
    pub limit_far: bool,
    pub depth_reference: [f32; 3],
    pub depth_axis: [f32; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct Output {
    pub matrix: [f32; 16],
    pub vertical_fov: f32,
    pub aspect: f32,
    pub half_angle_sine: f32,
}

pub fn build(vertical_fov: f32, i: Parameters) -> Result<Output, &'static str> {
    if ![vertical_fov, i.near, i.far]
        .into_iter()
        .chain(i.depth_reference)
        .chain(i.depth_axis)
        .all(f32::is_finite)
        || vertical_fov <= 0.
        || vertical_fov >= 180.
        || i.near <= 0.
        || i.far <= i.near
    {
        return Err("Camera projection outside acquired finite perspective profile");
    }
    let aspect = match i.aspect {
        Some(value) => value,
        None => {
            if !i.viewport.into_iter().all(|v| v.is_finite() && v > 0.) {
                return Err("Camera projection requires a usable viewport");
            }
            (f64::from(i.viewport[0]) / f64::from(i.viewport[1])) as f32
        }
    };
    if !aspect.is_finite() || aspect <= 0. {
        return Err("Camera projection requires a positive finite aspect");
    }
    let a = i.depth_reference.map(f64::from);
    let b = i.depth_axis.map(f64::from);
    let candidate = (12_500. - ((a[0] * b[0] + a[1] * b[1]) + a[2] * b[2])) as f32;
    let far = if i.limit_far && candidate < i.far {
        candidate
    } else {
        i.far
    };
    if !far.is_finite() || far <= i.near {
        return Err("Camera projection depth interval is not usable");
    }
    let angle = f64::from(vertical_fov) * f64::from(0.008726646_f32);
    let sine = angle.sin() as f32;
    let cotangent = angle.cos() / f64::from(sine);
    let depth = f64::from(far) / (f64::from(far) - f64::from(i.near));
    let mut matrix = [0.; 16];
    matrix[0] = (cotangent / f64::from(aspect)) as f32;
    matrix[5] = cotangent as f32;
    matrix[10] = depth as f32;
    matrix[11] = 1.;
    // The source retains the wide ratio after storing matrix[10].
    matrix[14] = (-depth * f64::from(i.near)) as f32;
    if !matrix.into_iter().all(f32::is_finite) || !sine.is_finite() || sine <= 0. {
        return Err("Camera projection result exceeds acquired finite profile");
    }
    Ok(Output {
        matrix,
        vertical_fov,
        aspect,
        half_angle_sine: sine,
    })
}

/// Lens state is updated before projection, as in the acquired caller. A later
/// projection refusal retains that completed transition; invocation/replay and
/// world scheduling belong to the surrounding camera phase.
pub fn advance(
    lens: &mut crate::camera_lens::State,
    input: crate::camera_lens::Input,
    projection: Parameters,
) -> Result<Output, &'static str> {
    let fov = lens.advance(input)?;
    build(fov, projection)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> Parameters {
        Parameters {
            aspect: None,
            viewport: [1920., 1080.],
            near: 1.,
            far: 10_000.,
            limit_far: true,
            depth_reference: [0.; 3],
            depth_axis: [1., 0., 0.],
        }
    }
    #[test]
    fn explicit_aspect_does_not_consume_viewport() {
        let output = build(
            60.,
            Parameters {
                aspect: Some(2.),
                viewport: [f32::NAN; 2],
                ..input()
            },
        )
        .unwrap();
        assert_eq!(output.aspect, 2.);
        assert_eq!(output.matrix[11], 1.);
        assert!(
            build(
                60.,
                Parameters {
                    viewport: [f32::NAN; 2],
                    ..input()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn unusable_limited_depth_refuses_instead_of_clamping_the_source_plane() {
        assert!(
            build(
                60.,
                Parameters {
                    depth_reference: [12_500., 0., 0.],
                    ..input()
                }
            )
            .is_err()
        );
        assert!(
            build(
                60.,
                Parameters {
                    limit_far: false,
                    depth_reference: [12_500., 0., 0.],
                    ..input()
                }
            )
            .is_ok()
        );
    }
    #[test]
    fn completed_lens_transition_survives_later_projection_refusal() {
        let mut lens = crate::camera_lens::State::new(0.5).unwrap();
        let request = crate::camera_lens::Input {
            profile_fov: 60.,
            override_fov: None,
            seconds: 0.25,
            inhibited: false,
            transition_input: true,
            scaled_input: true,
            mode: 0,
            profile_index: 0,
            selected_kind: Some(3),
            input_scale: 1.,
        };
        assert!(
            advance(
                &mut lens,
                request,
                Parameters {
                    depth_reference: [12_500., 0., 0.],
                    ..input()
                }
            )
            .is_err()
        );
        assert_eq!(lens.transition(), 0.75);
    }
}
