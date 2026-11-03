//! The measured 24-bit RGB -> RGB565 upload conversion, without gamma conversion.
use crate::Error;
pub struct MipLevel {
    pub width: u32,
    pub height: u32,
    pub words: Vec<u16>,
    pub rgba: Vec<u8>,
}
/// Measured square terrain profile: reduce the original RGB source for every
/// level, apply the polynomial gain, store float32 then truncate and pack.
pub fn terrain_mipmaps(
    rgb: &[u8],
    width: u32,
    height: u32,
    curve: [f32; 3],
) -> Result<Vec<MipLevel>, Error> {
    if width != height
        || !(2..=512).contains(&width)
        || !width.is_power_of_two()
        || rgb.len() != width as usize * height as usize * 3
        || !curve.iter().all(|n| n.is_finite())
    {
        return Err(Error("unsupported terrain mip profile"));
    }
    let mut result = Vec::new();
    let mut level = 0;
    while width >> level >= 2 {
        let step = 1usize << level;
        let size = width as usize / step;
        let n = f64::from(level);
        let gain = ((f64::from(curve[2]) * n + f64::from(curve[1])) * n + f64::from(curve[0]))
            .clamp(0., 1.) as f32;
        let factor = gain / (step * step) as f32;
        let mut reduced = Vec::with_capacity(size * size * 3);
        for y in 0..size {
            for x in 0..size {
                let mut sums = [0u32; 3];
                for dy in 0..step {
                    for dx in 0..step {
                        let at = ((y * step + dy) * width as usize + x * step + dx) * 3;
                        for channel in 0..3 {
                            sums[channel] += u32::from(rgb[at + channel]);
                        }
                    }
                }
                reduced.extend(sums.map(|sum| (f64::from(sum) * f64::from(factor)) as f32 as u8));
            }
        }
        let words = pack_rgb565(&reduced)?;
        result.push(MipLevel {
            width: size as u32,
            height: size as u32,
            rgba: rgba8(&words),
            words,
        });
        level += 1;
    }
    Ok(result)
}
pub fn pack_rgb565(rgb: &[u8]) -> Result<Vec<u16>, Error> {
    if !rgb.len().is_multiple_of(3) {
        return Err(Error("RGB input must contain complete pixels"));
    }
    Ok(rgb
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| ((p[0] as u16 >> 3) << 11) | ((p[1] as u16 >> 2) << 5) | (p[2] as u16 >> 3))
        .collect())
}
/// Measured 32-bit source upload to A4R4G4B4; alpha is retained.
pub fn pack_argb4444(rgb: &[u8], alpha: &[u8]) -> Result<Vec<u16>, Error> {
    if rgb.len() != alpha.len() * 3 {
        return Err(Error("RGB/alpha pixel count differs"));
    }
    Ok(rgb
        .as_chunks::<3>()
        .0
        .iter()
        .zip(alpha)
        .map(|(p, a)| {
            (u16::from(*a >> 4) << 12)
                | (u16::from(p[0] >> 4) << 8)
                | (u16::from(p[1] >> 4) << 4)
                | u16::from(p[2] >> 4)
        })
        .collect())
}
pub fn rgba8_argb4444(words: &[u16]) -> Vec<u8> {
    words
        .iter()
        .flat_map(|w| {
            [
                ((w >> 8) & 15) as u8 * 17,
                ((w >> 4) & 15) as u8 * 17,
                (w & 15) as u8 * 17,
                ((w >> 12) & 15) as u8 * 17,
            ]
        })
        .collect()
}
/// Rounded eight-bit normalized expansion for filterable RGBA8 upload.
/// The packed words are exact; the GPU's original internal normalized precision is not assumed.
pub fn rgba8(words: &[u16]) -> Vec<u8> {
    words
        .iter()
        .flat_map(|word| {
            let r = (word >> 11) & 31;
            let g = (word >> 5) & 63;
            let b = word & 31;
            [
                ((r * 255 + 15) / 31) as u8,
                ((g * 255 + 31) / 63) as u8,
                ((b * 255 + 15) / 31) as u8,
                255,
            ]
        })
        .collect()
}
#[cfg(test)]
mod tests {
    #[test]
    fn argb_retains_alpha_and_rejects_mismatched_channels() {
        let words = super::pack_argb4444(&[255, 128, 31, 0, 17, 34], &[63, 255]).unwrap();
        assert_eq!(words, [0x3f81, 0xf012]);
        assert_eq!(
            super::rgba8_argb4444(&words),
            [255, 136, 17, 51, 0, 17, 34, 255]
        );
        assert!(super::pack_argb4444(&[0, 1], &[3]).is_err());
    }
    #[test]
    fn terrain_mips_reduce_original_and_end_at_two() {
        let levels =
            super::terrain_mipmaps(&vec![255; 16 * 16 * 3], 16, 16, [1., -0.1, -0.05]).unwrap();
        assert_eq!(
            levels.iter().map(|m| m.width).collect::<Vec<_>>(),
            [16, 8, 4, 2]
        );
        assert_eq!(
            levels.iter().map(|m| m.words[0]).collect::<Vec<_>>(),
            [65535, 57051, 40147, 14823]
        );
        assert!(super::terrain_mipmaps(&[0; 12], 2, 2, [f32::NAN, 0., 0.]).is_err());
        assert!(super::terrain_mipmaps(&[0; 27], 3, 3, [1., 0., 0.]).is_err());
    }
    #[test]
    fn quantization_and_normalized_endpoints() {
        use super::*;
        assert_eq!(
            pack_rgb565(&[255, 255, 255, 7, 3, 7, 8, 4, 8]).unwrap(),
            [65535, 0, 0x821]
        );
        assert_eq!(
            rgba8(&[0, 65535, 0xf800, 0x7e0, 31]),
            [
                0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255
            ]
        );
        assert!(pack_rgb565(&[1]).is_err());
        // Every possible packed word survives expansion and requantization.
        let words: Vec<u16> = (0..=65535).collect();
        let rgb: Vec<u8> = rgba8(&words)
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| p[..3].iter().copied())
            .collect();
        assert_eq!(pack_rgb565(&rgb).unwrap(), words);
    }
}
