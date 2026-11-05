//! Generated CPU texture from the acquired normal cold geometry constructor.
//! Source words are preserved; GPU alpha/blending and material residency remain
//! separate. This does not manufacture a renderer binding or world registration.
use crate::audio_parameters::Random;

pub const WIDTH: usize = 128;
pub const HEIGHT: usize = 16;
#[derive(Debug)]
pub struct Texture {
    /// Row-major source words. Only bits 16..23 are written by this generator;
    /// their eventual GPU interpretation is not inferred here.
    pub pixels: Vec<u32>,
}
/// Consume the shared supplied RNG exactly once per column. The source inclusive
/// integer interval is distinct from the floating interval used by audio.
pub fn prepare(random: &mut Random) -> Texture {
    let mut pixels = vec![0; WIDTH * HEIGHT];
    for column in 0..WIDTH {
        let offset = 64 - ((random.next_15() * 97) >> 15) as i32;
        for row in 0..HEIGHT {
            // Source float32 reciprocal, then Double53 products and truncation.
            let ramp = (row as f64 * f64::from(1.0f32 / 15.) * -255.).trunc() as i32;
            let channel = 255 - (offset - ramp).clamp(0, 255);
            pixels[row * WIDTH + column] = (channel as u32) << 16;
        }
    }
    Texture { pixels }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_words_preserve_layout_and_caller_rng_phase() {
        for seed in [0, u32::MAX] {
            let mut random = Random(seed);
            for _ in 0..7 {
                random.next_15();
            }
            let mut expected = random;
            for _ in 0..WIDTH {
                expected.next_15();
            }
            let texture = prepare(&mut random);
            assert_eq!(random.0, expected.0);
            assert_eq!(texture.pixels.len(), WIDTH * HEIGHT);
            assert!(texture.pixels.iter().all(|word| word & 0xff00ffff == 0));
            for column in 0..WIDTH {
                for row in 1..HEIGHT {
                    let above = texture.pixels[(row - 1) * WIDTH + column] >> 16;
                    let below = texture.pixels[row * WIDTH + column] >> 16;
                    assert!(below <= above && above - below <= 17);
                }
            }
        }
    }
}
