//! Measured source text metrics and glyph rectangles before vertex submission.
//! Cached glyph masks, translation bytes and full-screen dimensions are inputs.
use crate::{arithmetic::Precision, font_atlas::Atlas};

pub struct Context<'a> {
    pub atlas: &'a Atlas,
    pub extent: [u32; 2],
    pub reference_height: f32,
    pub precision: Precision,
}
#[derive(Clone, Debug)]
pub struct Draw {
    pub code: u8,
    pub color: u32,
    /// Pixel edges: left, right, top, bottom. No half-texel/viewport conversion.
    pub rect: [f32; 4],
    pub atlas_rect: [u8; 4],
}
pub struct Layout {
    pub draws: Vec<Draw>,
    pub after_words: [u32; 13],
}
impl Context<'_> {
    fn round(&self, value: f64) -> f64 {
        self.precision.round(value)
    }
    fn store(&self, value: f64) -> f32 {
        self.round(value) as f32
    }
    fn scale(&self, size: f32) -> Result<f32, &'static str> {
        if self.extent.contains(&0)
            || self.extent.iter().any(|n| *n > 16384)
            || !self.reference_height.is_finite()
            || self.reference_height <= 0.
            || !size.is_finite()
            || size < 0.
        {
            return Err("font metrics outside explicit finite screen/font profile");
        }
        Ok(self.store(
            self.round(f64::from(size) / f64::from(self.reference_height))
                * f64::from(self.extent[1]),
        ))
    }
    fn glyph(&self, code: u8) -> Result<crate::font_atlas::Glyph, &'static str> {
        let glyph = self.atlas.glyphs[code as usize];
        if glyph.width == 0 {
            return Err("font glyph metric not constructed");
        }
        Ok(glyph)
    }
    pub fn advance(&self, size: f32, code: u8) -> Result<f32, &'static str> {
        let scale = self.scale(size)?;
        let width = f64::from(self.glyph(code)?.width);
        Ok(self.store(
            self.round(self.round(width * f64::from(scale)) + f64::from(scale))
                / f64::from(self.extent[0]),
        ))
    }
    /// Native first-line measure, excluding its final inter-character gap.
    /// Bytes after a linefeed are handled by the next line's own measurement.
    pub fn first_line_width(&self, size: f32, text: &[u8]) -> Result<f32, &'static str> {
        Ok(self.first_line_width_extended(size, text)? as f32)
    }
    fn first_line_width_extended(&self, size: f32, text: &[u8]) -> Result<f64, &'static str> {
        let scale = self.scale(size)?;
        let mut total = 0.;
        for &code in text.iter().take_while(|c| **c != 0 && **c != b'\n') {
            if code < 32 {
                return Err("font metric for control byte not established");
            }
            if total != 0. {
                total = self.round(total + f64::from(scale));
            }
            total = self
                .round(self.round(f64::from(self.glyph(code)?.width) * f64::from(scale)) + total);
        }
        Ok(self.round(total / f64::from(self.extent[0])))
    }
    pub fn text_height(&self, size: f32, text: &[u8]) -> Result<f32, &'static str> {
        self.scale(size)?;
        let lines = 1 + text
            .iter()
            .take_while(|c| **c != 0)
            .filter(|c| **c == b'\n')
            .count();
        Ok(self.store(lines as f64 * f64::from(size)))
    }
    pub fn layout(&self, words: [u32; 13], text: &[u8]) -> Result<Layout, &'static str> {
        if text.len() > 16384 {
            return Err("font layout byte budget");
        }
        let mut info = words.map(f32::from_bits);
        if !info[..12].iter().all(|n| n.is_finite()) {
            return Err("font layout nonfinite input");
        }
        let size = info[3];
        let scale = self.scale(size)?;
        let mut draws = Vec::new();
        let width = self.first_line_width(size, text)?;
        let height = self.text_height(size, text)?;
        info[4] =
            self.store(f64::from(info[4]) - self.round(f64::from(width) * f64::from(info[0])));
        info[0] = 0.;
        info[5] =
            self.store(f64::from(info[5]) - self.round(f64::from(height) * f64::from(info[1])));
        info[1] = 0.;
        for (index, &code) in text.iter().take_while(|c| **c != 0).enumerate() {
            if code == b'\n' {
                info[4] = f32::from_bits(words[4]);
                info[0] = f32::from_bits(words[0]);
                info[5] = self.store(f64::from(info[5]) + f64::from(size));
                // Unlike the first line, this call keeps its x87 result through
                // the alignment multiply before the final f32 store.
                let width = self.first_line_width_extended(size, &text[index + 1..])?;
                if width > 0. {
                    info[4] =
                        self.store(f64::from(info[4]) - self.round(width * f64::from(info[0])));
                    info[0] = 0.;
                    info[5] = self.store(
                        f64::from(info[5]) - self.round(f64::from(height) * f64::from(info[1])),
                    );
                    info[1] = 0.;
                }
            } else if code >= 32 {
                let glyph = self.glyph(code)?;
                let advance = self.advance(size, code)?;
                info[4] = self.store(
                    f64::from(info[4]) - self.round(f64::from(advance) * f64::from(info[0])),
                );
                info[0] = 0.;
                info[5] = self
                    .store(f64::from(info[5]) - self.round(f64::from(size) * f64::from(info[1])));
                info[1] = 0.;
                let inverse_alpha = self.store(
                    1. - self.round(
                        f64::from(info[8].clamp(0., 1.)) * f64::from(info[11].clamp(0., 1.)),
                    ),
                );
                let alpha = self.round(self.round(1. - f64::from(inverse_alpha)) * 255.) as u32;
                let x = self.store(f64::from(self.extent[0]) * f64::from(info[4]));
                let baseline = self.store(
                    self.round(
                        self.round(f64::from(size) / f64::from(self.reference_height))
                            * f64::from(self.atlas.max_above_baseline),
                    ) + f64::from(info[5]),
                );
                let y = self.store(f64::from(self.extent[1]) * f64::from(baseline));
                let rect = [
                    x,
                    self.store(
                        self.round(f64::from(glyph.width) * f64::from(scale)) + f64::from(x),
                    ),
                    self.store(
                        f64::from(y)
                            - self.round(f64::from(glyph.above_baseline) * f64::from(scale)),
                    ),
                    self.store(
                        self.round(
                            f64::from(i32::from(glyph.height) - i32::from(glyph.above_baseline))
                                * f64::from(scale),
                        ) + f64::from(y),
                    ),
                ];
                draws.push(Draw {
                    code,
                    color: words[12] | (alpha << 24),
                    rect,
                    atlas_rect: [glyph.x, glyph.y, glyph.width, glyph.height],
                });
                info[8] = self.store(f64::from(info[8]) + f64::from(info[9]));
                info[4] = self.store(f64::from(advance) + f64::from(info[4]));
            }
        }
        let mut after_words = info.map(f32::to_bits);
        after_words[12] = words[12];
        Ok(Layout { draws, after_words })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn atlas() -> Atlas {
        let mut a = Atlas::default();
        a.glyphs[b'A' as usize] = crate::font_atlas::Glyph {
            above_baseline: 15,
            height: 15,
            width: 13,
            x: 0,
            y: 0,
        };
        a.max_above_baseline = 15;
        a
    }
    #[test]
    fn centered_lines_are_measured_independently_and_absent_glyphs_are_not_substituted() {
        let a = atlas();
        let c = Context {
            atlas: &a,
            extent: [1920, 1080],
            reference_height: 24.,
            precision: Precision::Double53,
        };
        assert_eq!(
            c.first_line_width(0.04, b"AA\nA").unwrap(),
            c.first_line_width(0.04, b"AA").unwrap()
        );
        assert_eq!(c.text_height(0.04, b"AA\nA").unwrap(), 0.08);
        assert!(c.first_line_width(0.04, b"missing").is_err());
    }
    #[test]
    fn transparent_source_phase_still_produces_geometry_with_zero_alpha() {
        let a = atlas();
        let c = Context {
            atlas: &a,
            extent: [1920, 1080],
            reference_height: 24.,
            precision: Precision::Double53,
        };
        let mut words = [0; 13];
        words[3] = 0.04f32.to_bits();
        words[11] = 1f32.to_bits();
        words[12] = 0xffff00;
        let layout = c.layout(words, b"A").unwrap();
        assert_eq!(layout.draws[0].color, 0xffff00);
        words[8] = 1f32.to_bits();
        assert_eq!(c.layout(words, b"A").unwrap().draws[0].color, 0xffffff00);
    }
}
