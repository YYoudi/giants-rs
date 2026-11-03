//! Measured unsigned height-to-normal image profile. Row selection belongs to
//! the consuming pixel descriptor, not the TGA origin metadata.
use giants_formats::tga::{Image, Plane8, RgbImage};

/// Explicit native conversion policy, not an inferred backend or live FPU mode.
#[derive(Clone, Copy, Debug)]
pub enum ByteRounding {
    Truncate,
    NearestEven,
}
impl ByteRounding {
    fn encode(self, value: f64) -> u8 {
        let value = match self {
            Self::Truncate => value.trunc(),
            Self::NearestEven => value.round_ties_even(),
        };
        value.clamp(0., 255.) as u8
    }
}

/// Terrain's unsigned normal route first preserves varying alpha. Constant
/// alpha instead falls through the original RGB-height cases to the final
/// screen-combination rule. Uniform single-channel sources therefore use zero
/// RGB height; their constant alpha is not a special exemption.
pub fn terrain_normal_map(
    image: &Image,
    reverse_rows: bool,
    rounding: ByteRounding,
) -> Result<(RgbImage, bool), &'static str> {
    terrain_normal_map_level(image, reverse_rows, rounding, image.dimensions())
}

/// Resample source channels before selecting height and constructing normals.
/// Reduced levels are restricted to square power-of-two inputs and outputs.
pub fn terrain_normal_map_level(
    image: &Image,
    reverse_rows: bool,
    rounding: ByteRounding,
    dimensions: [u32; 2],
) -> Result<(RgbImage, bool), &'static str> {
    let [width, height] = image.dimensions();
    let [target_width, target_height] = dimensions;
    let resized = dimensions != [width, height];
    if resized
        && (width != height
            || !width.is_power_of_two()
            || target_width != target_height
            || !target_width.is_power_of_two()
            || target_width > width)
    {
        return Err("terrain bump resize outside measured profile");
    }
    let count = (width as usize)
        .checked_mul(height as usize)
        .filter(|n| *n > 0 && *n <= 16_777_216)
        .ok_or("terrain bump pixel budget")?;
    let (mut rgb, mut alpha) = match image {
        Image::TrueColor(source) => {
            if source.pixels.len() != count * 3
                || source.alpha.as_ref().is_some_and(|a| a.len() != count)
            {
                return Err("terrain bump source shape mismatch");
            }
            (
                source.pixels.clone(),
                source.alpha.clone().unwrap_or_else(|| vec![0; count]),
            )
        }
        Image::Plane8(source) => {
            if source.samples.len() != count {
                return Err("terrain bump plane shape mismatch");
            }
            (vec![0; count * 3], source.samples.clone())
        }
    };
    if resized {
        let step = width / target_width;
        let scale = 1. / (step * step) as f32;
        let mut reduced_rgb = Vec::with_capacity((target_width * target_height * 3) as usize);
        let mut reduced_alpha = Vec::with_capacity((target_width * target_height) as usize);
        for y in 0..target_height {
            for x in 0..target_width {
                let mut sum = [0f32; 4];
                for dx in 0..step {
                    for dy in 0..step {
                        let row = y * step + dy;
                        let row = if reverse_rows { height - 1 - row } else { row };
                        let index = (row * width + x * step + dx) as usize;
                        for (channel, value) in sum[..3].iter_mut().enumerate() {
                            *value += f32::from(rgb[index * 3 + channel]);
                        }
                        sum[3] += f32::from(alpha[index]);
                    }
                }
                reduced_rgb.extend(
                    sum[..3]
                        .iter()
                        .map(|v| rounding.encode(f64::from(*v * scale))),
                );
                reduced_alpha.push(rounding.encode(f64::from(sum[3] * scale)));
            }
        }
        rgb = reduced_rgb;
        alpha = reduced_alpha;
    }
    let from_rgb = alpha.iter().all(|v| *v == alpha[0]);
    let mut samples = alpha;
    if from_rgb {
        let reciprocal = f64::from(1. / 255f32);
        for (height, pixel) in samples.iter_mut().zip(rgb.as_chunks::<3>().0) {
            let [r, g, b] = pixel.map(f64::from);
            let product = ((1. - r * reciprocal) * (1. - g * reciprocal)) * (1. - b * reciprocal);
            *height = rounding.encode((1. - product) * 255.);
        }
    }
    let plane = Plane8 {
        width: target_width,
        height: target_height,
        top_origin: false,
        samples,
    };
    normal_map_with_rounding(&plane, reverse_rows && !resized, rounding)
        .map(|normal| (normal, from_rgb))
}

/// Native color-normal profile with height scale 1/64, forward differences,
/// clamped neighbors, mathematical normalization and byte encoding.
pub fn normal_map(image: &Plane8, reverse_rows: bool) -> Result<RgbImage, &'static str> {
    normal_map_with_rounding(image, reverse_rows, ByteRounding::Truncate)
}

pub fn normal_map_with_rounding(
    image: &Plane8,
    reverse_rows: bool,
    rounding: ByteRounding,
) -> Result<RgbImage, &'static str> {
    let width = image.width as usize;
    let height = image.height as usize;
    let count = width
        .checked_mul(height)
        .ok_or("bump image dimensions overflow")?;
    if width == 0 || height == 0 || count > 16_777_216 || image.samples.len() != count {
        return Err("bump image sample count or dimensions outside profile");
    }
    let sample = |x: usize, y: usize| {
        let row = if reverse_rows { height - 1 - y } else { y };
        image.samples[row * width + x]
    };
    let mut pixels = Vec::with_capacity(count * 3);
    let mut alpha = Vec::with_capacity(count);
    for y in 0..height {
        for x in 0..width {
            let base = sample(x, y);
            let dx = (f64::from(sample((x + 1).min(width - 1), y)) - f64::from(base)) / 64.;
            let dy = (f64::from(sample(x, (y + 1).min(height - 1))) - f64::from(base)) / 64.;
            let length = (dx * dx + 1. + dy * dy).sqrt();
            let inverse = 1. / length;
            let normal = [
                (-dx * inverse) as f32,
                inverse as f32,
                (-dy * inverse) as f32,
            ];
            pixels.extend(normal.map(|v| rounding.encode((f64::from(v) + 1.) * 127.5)));
            alpha.push(base);
        }
    }
    Ok(RgbImage {
        width: image.width,
        height: image.height,
        pixels,
        alpha: Some(alpha),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flat_height_encodes_the_middle_axis_and_retains_alpha() {
        let image = Plane8 {
            width: 1,
            height: 1,
            top_origin: false,
            samples: vec![193],
        };
        let normal = normal_map(&image, true).unwrap();
        assert_eq!(normal.pixels, [127, 255, 127]);
        assert_eq!(normal.alpha.unwrap(), [193]);
    }
    #[test]
    fn forward_neighbor_clamps_at_the_edge_and_shape_errors_stay_explicit() {
        let mut image = Plane8 {
            width: 2,
            height: 1,
            top_origin: false,
            samples: vec![0, 64],
        };
        assert_eq!(
            normal_map(&image, false).unwrap().pixels,
            [37, 217, 127, 127, 255, 127]
        );
        image.samples.pop();
        assert!(normal_map(&image, false).is_err());
    }
}
