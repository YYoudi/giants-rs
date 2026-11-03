//! Source tracer ribbon before clipping/projection, using supplied camera-space points.
//! No owner clock, camera transform, device blend state or GPU submission is selected here.
#[derive(Clone, Debug)]
pub struct Ribbon {
    pub positions: [[f32; 3]; 4],
    pub rgb: u32,
    pub alpha: u32,
}
pub const TRIANGLES: [[usize; 3]; 4] = [[0, 1, 2], [1, 3, 2], [0, 2, 1], [1, 2, 3]];

pub fn camera_quad(
    previous: [f32; 3],
    current: [f32; 3],
    color: [f32; 3],
    fade: f32,
    opacity: f32,
    half_width: f32,
) -> Result<Option<Ribbon>, &'static str> {
    if previous.iter().chain(&current).any(|v| !v.is_finite())
        || color
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=255.).contains(&(f64::from(*v) * 255.).trunc()))
        || [fade, opacity]
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
        || !half_width.is_finite()
        || half_width < 0.
    {
        return Err("ribbon input outside finite source profile");
    }
    let dy = (f64::from(current[1]) - f64::from(previous[1])) as f32;
    let dx_extended = f64::from(previous[0]) - f64::from(current[0]);
    let dx = dx_extended as f32;
    let squared = dx_extended * f64::from(dx) + f64::from(dy) * f64::from(dy);
    if squared == 0. {
        return Ok(None);
    }
    // Source stores the inverse and x offset while retaining their unrounded
    // counterparts on the x87 stack for one side of each endpoint.
    let (x, y) = if squared < f64::from(f32::from_bits(0x3dcccccd)) {
        (0., f64::from(half_width))
    } else {
        let inverse = 1. / squared.sqrt();
        (
            inverse * f64::from(dy) * f64::from(half_width),
            f64::from(inverse as f32) * f64::from(dx) * f64::from(half_width),
        )
    };
    let positions = [
        [
            (f64::from(current[0]) + x) as f32,
            (f64::from(current[1]) + y) as f32,
            current[2],
        ],
        [
            (f64::from(current[0]) - f64::from(x as f32)) as f32,
            (f64::from(current[1]) - y) as f32,
            current[2],
        ],
        [
            (f64::from(previous[0]) + f64::from(x as f32)) as f32,
            (f64::from(previous[1]) + y) as f32,
            previous[2],
        ],
        [
            (f64::from(previous[0]) - f64::from(x as f32)) as f32,
            (f64::from(previous[1]) - y) as f32,
            previous[2],
        ],
    ];
    let channel = |v: f32| (f64::from(v) * 255.) as u32;
    let rgb = (channel(color[0]) << 16) | (channel(color[1]) << 8) | channel(color[2]);
    let alpha = (((1. - f64::from(fade)) * f64::from(opacity) * 255.) as u32) << 24;
    Ok(Some(Ribbon {
        positions,
        rgb,
        alpha,
    }))
}

/// The source ribbon uses clip X/Y and homogeneous W from the composed camera.
/// These are the three values consumed before its perspective division.
pub fn tracer_position(composed: [f32; 16], position: [f32; 3]) -> Result<[f32; 3], &'static str> {
    if composed.iter().chain(&position).any(|v| !v.is_finite()) {
        return Err("nonfinite tracer camera input");
    }
    let result = [0usize, 1, 3].map(|column| {
        ((f64::from(composed[column]) * f64::from(position[0])
            + f64::from(composed[8 + column]) * f64::from(position[2]))
            + f64::from(composed[4 + column]) * f64::from(position[1])
            + f64::from(composed[12 + column])) as f32
    });
    if result.iter().any(|v| !v.is_finite()) {
        return Err("tracer camera overflow");
    }
    Ok(result)
}
