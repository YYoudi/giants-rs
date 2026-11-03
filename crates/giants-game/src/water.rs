//! Measured above-water sun band. Camera and sun are explicit caller inputs.
use giants_formats::world::WaterParameters;

pub struct Vertex {
    pub position: [f32; 3],
    pub color: u32,
    pub uv: [f32; 2],
}
pub struct Band {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}
pub struct TextureAsset {
    pub levels: Vec<giants_formats::pixels::MipLevel>,
    pub water: WaterParameters,
}

/// Five native producer offsets. Drawing uses these offsets before advancing.
/// The observed benchmark supplies 1/60 per producer call and a zero angle clock.
/// This is not a global game simulation clock or an autonomous sun model.
#[derive(Clone, Copy, Debug, Default)]
pub struct Motion {
    pub phases: [[f32; 2]; 5],
}
impl Motion {
    pub fn advance(&mut self, dt: f32, speed: f32, angle_clock: f32) -> Result<(), &'static str> {
        if !dt.is_finite()
            || !(0. ..=1.).contains(&dt)
            || !speed.is_finite()
            || !(0. ..=32.).contains(&speed)
            || !angle_clock.is_finite()
            || !(0. ..=128.).contains(&angle_clock)
            || self
                .phases
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || *v > 32.)
        {
            return Err("unsupported water motion input");
        }
        for (pair, factor) in self.phases.iter_mut().zip([0f32, 0.025, 0.05, 0.075, 0.1]) {
            let mut angle = f64::from(factor) * f64::from(31.415928f32) + f64::from(angle_clock);
            while angle > f64::from(std::f32::consts::TAU) {
                angle -= f64::from(std::f32::consts::TAU);
            }
            for (phase, direction) in pair.iter_mut().zip([angle.sin(), angle.cos()]) {
                let next = (direction * f64::from(0.03f32) * f64::from(dt) * f64::from(speed)
                    + f64::from(*phase)) as f32;
                let mut wrapped = f64::from(next);
                while wrapped > 1. {
                    wrapped -= 1.;
                }
                *phase = wrapped as f32;
            }
        }
        Ok(())
    }
}
pub fn load_texture(
    root: &std::path::Path,
    world: &str,
) -> Result<TextureAsset, Box<dyn std::error::Error>> {
    let source = crate::world_source::resolve(root, world)?;
    let config =
        giants_formats::world::parse_world(&std::fs::read(root.join("Bin").join(source.catalog))?)?;
    let data = std::fs::read(root.join("Bin/xx_efx.gzp"))?;
    let archive = giants_formats::gzp::Archive::parse(&data)?;
    let image = giants_formats::tga::decode(&archive.read("EFX_water_test_BW.tga")?)?;
    if (image.width, image.height) != (256, 256) {
        return Err("Unsupported water source dimensions".into());
    }
    Ok(TextureAsset {
        levels: giants_formats::pixels::terrain_mipmaps(&image.pixels, 256, 256, [1., 0., 0.])?,
        water: config.water,
    })
}
pub fn texture_control(asset: &TextureAsset) -> Vec<u8> {
    let mut out = b"GWTX0001".to_vec();
    out.extend((asset.levels.len() as u32).to_le_bytes());
    for level in &asset.levels {
        out.extend(level.width.to_le_bytes());
        out.extend(level.height.to_le_bytes());
        for word in &level.words {
            out.extend(word.to_le_bytes());
        }
    }
    out
}

/// Reproduce the demonstrated 0x431230 strip, including x87 float-store boundaries.
/// This does not determine the sun, camera, simulation clock or animated texture.
pub fn band(
    eye: [f32; 2],
    sun: [f32; 2],
    sea_scale: f32,
    water: WaterParameters,
) -> Result<Band, &'static str> {
    if !eye
        .into_iter()
        .chain(sun)
        .chain([sea_scale])
        .chain(water.values)
        .all(f32::is_finite)
        || sea_scale <= 0.
        || water.values[0] <= 0.
        || water.values[1] <= 0.
        || water.values[2] < 0.
        || water.values[3] < 0.
    {
        return Err("unsupported water band input");
    }
    let [sx, sy] = sun.map(f64::from);
    let length2 = sx * sx + sy * sy;
    if length2 < 0.0001 {
        return Err("degenerate water sun direction");
    }
    let inv = 1. / length2.sqrt();
    let sx = f64::from(((sx * inv) as f32) * 12500.);
    let sy = f64::from(((sy * inv) as f32) * 12500.);
    let [ex, ey] = eye.map(f64::from);
    let dx = sx - ex;
    let dy = sy - ey;
    let length = (dx * dx + dy * dy).sqrt() as f32;
    if !length.is_finite() || length < 0.0001 {
        return Err("degenerate water camera direction");
    }
    let inv = 1. / f64::from(length);
    let nx = f64::from((inv * dx) as f32);
    let ny = f64::from(inv as f32) * dy;
    let distance = 2. * (nx * ex + ny * ey) + f64::from(length);
    let ex = f64::from((ex - nx * distance) as f32);
    let ey = f64::from((ey - ny * distance) as f32);
    let px = f64::from((-ny) as f32);
    let half = f64::from(water.values[1] * 0.5);
    let uv_scale = f64::from(1. / sea_scale);
    let mut rows = [0f32; 13];
    rows[12] = 1.;
    for (i, row) in rows.iter_mut().enumerate().take(12).skip(1) {
        *row = (((i as f64 - 1.) - 5.) * f64::from(0.1f32) * 0.25 + 0.5) as f32;
    }
    let [r, g, b] = water.colors[0];
    let rgb = (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
    let colors: [u32; 33] = std::array::from_fn(|column| {
        let angle =
            (((column as f64 - 16.) * f64::from(water.values[2]) * f64::from(0.09817477f32))
                as f32)
                .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
        let alpha = (f64::from(angle).cos() * f64::from(water.values[3]))
            .max(0.)
            .powf(f64::from(water.values[0]))
            .clamp(0., 1.);
        (((alpha * 255.) as u32) << 24) | rgb
    });
    let mut vertices = Vec::with_capacity(429);
    for fraction in rows.into_iter().rev().map(f64::from) {
        let cx = f64::from((fraction * sx + (1. - fraction) * ex) as f32);
        let cy = fraction * sy + (1. - fraction) * ey;
        for (column, color) in colors.into_iter().enumerate() {
            let x = ((cx - px * half) + px * column as f64 * 0.0625 * half) as f32;
            let y = (cy - nx * half) + column as f64 * 0.0625 * nx * half;
            vertices.push(Vertex {
                position: [x, y as f32, 0.],
                color,
                uv: [(f64::from(x) * uv_scale) as f32, (y * uv_scale) as f32],
            });
        }
    }
    let mut indices = Vec::with_capacity(2304);
    for row in 0..12 {
        for column in 0..32 {
            let a = row * 33 + column;
            indices.extend([a, a + 1, a + 33, a + 1, a + 34, a + 33]);
        }
    }
    if !vertices
        .iter()
        .all(|v| v.position.into_iter().chain(v.uv).all(f32::is_finite))
    {
        return Err("water band overflow");
    }
    Ok(Band { vertices, indices })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn motion_rejects_invalid_input_without_changing_state() {
        let mut motion = Motion {
            phases: [[0.75, -0.5]; 5],
        };
        for (dt, speed, clock) in [
            (f32::NAN, 1., 0.),
            (-1., 1., 0.),
            (2., 1., 0.),
            (0.1, -1., 0.),
            (0.1, 1., f32::INFINITY),
        ] {
            let before = motion.phases;
            assert!(motion.advance(dt, speed, clock).is_err());
            assert_eq!(motion.phases, before);
        }
        // Negative offsets are never wrapped by the native producer and can
        // pass -32 in a long-running viewer. They must not stop animation.
        motion.phases = [[-40.; 2]; 5];
        assert!(motion.advance(1. / 60., 1.5, 0.).is_ok());
    }
    #[test]
    fn rejects_missing_direction_nonfinite_and_overflow() {
        let water = WaterParameters::default();
        assert!(band([0., 0.], [0., 0.], 250., water).is_err());
        assert!(band([0., 0.], [1., 0.], 0., water).is_err());
        assert!(band([0., f32::NAN], [1., 0.], 250., water).is_err());
        assert!(band([f32::MAX, f32::MAX], [1., 0.], 250., water).is_err());
        assert!(band([0., 0.], [1., 0.], f32::from_bits(1), water).is_err());
        let mut invalid = water;
        invalid.values[3] = -0.1;
        assert!(band([0., 0.], [1., 0.], 250., invalid).is_err());
    }
    #[test]
    fn source_amplitude_above_one_saturates_after_power() {
        let water = WaterParameters {
            values: [1.6, 4000., 1.32, 1.128, 1.5],
            ..Default::default()
        };
        let band = band([0., 0.], [1., 0.], 250., water).unwrap();
        assert_eq!(band.vertices[16].color >> 24, 255);
        assert!(band.vertices[0].color >> 24 < 255);
        assert_eq!(band.vertices[16].color & 0xffffff, 0x80);
    }
}
