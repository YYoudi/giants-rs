//! Monochrome glyph bounds and mutable font-atlas packing, before GPU upload.
//! Rasterized 128x64 GDI masks are supplied by a platform font service.
pub const BITMAP_BYTES: usize = 128 * 64 / 8;
pub const ATLAS_WIDTH: usize = 256;
pub const ATLAS_HEIGHT: usize = 128;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Glyph {
    pub above_baseline: i8,
    pub height: u8,
    pub width: u8,
    pub x: u8,
    pub y: u8,
}
impl Glyph {
    pub fn bytes(self) -> [u8; 5] {
        [
            self.above_baseline as u8,
            self.height,
            self.width,
            self.x,
            self.y,
        ]
    }
}

pub struct Atlas {
    pub glyphs: [Glyph; 256],
    /// Source 16-bit white/transparent texels, not an exported game texture.
    pub pixels: Vec<u16>,
    pub row_y: u32,
    pub next_x: u32,
    pub row_height: u32,
    pub max_above_baseline: i32,
    pub max_below_baseline: i32,
    /// Native packing can wrap onto old texels. Preserve that behavior and
    /// expose it rather than silently clearing cached glyph declarations.
    pub wraps: u32,
}
impl Default for Atlas {
    fn default() -> Self {
        Self {
            glyphs: [Glyph::default(); 256],
            pixels: vec![0; ATLAS_WIDTH * ATLAS_HEIGHT],
            row_y: 0,
            next_x: 0,
            row_height: 0,
            max_above_baseline: 0,
            max_below_baseline: 0,
            wraps: 0,
        }
    }
}
impl Atlas {
    pub fn insert(&mut self, code: u8, mask: &[u8; BITMAP_BYTES]) -> Result<Glyph, &'static str> {
        if self.glyphs[code as usize].width != 0 {
            return Ok(self.glyphs[code as usize]);
        }
        let ink = |x: usize, y: usize| mask[y * 16 + x / 8] & (0x80 >> (x % 8)) != 0;
        let (mut left, mut right, mut top, mut bottom) = (127, 0, 63, 0);
        for y in 0..64 {
            for x in 0..128 {
                if ink(x, y) {
                    left = left.min(x);
                    right = right.max(x);
                    top = top.min(y);
                    bottom = bottom.max(y);
                }
            }
        }
        if right < left {
            if code != b' ' {
                return Err("font rasterizer produced no ink for a non-space glyph");
            }
            (left, right, top, bottom) = (0, 4, 32, 32);
        }
        let width = (right - left + 1) as u32;
        let height = (bottom - top + 1) as u32;
        self.max_below_baseline = self.max_below_baseline.max(bottom as i32 - 32);
        self.max_above_baseline = self.max_above_baseline.max(32 - top as i32);
        if 256i32 - (self.next_x as i32) < width as i32 {
            self.row_y += self.row_height;
            self.next_x = 0;
            self.row_height = 0;
        }
        if self.row_height < height {
            if 128i32 - (self.row_y as i32) < height as i32 {
                self.row_y = 0;
                self.wraps += 1;
            }
            self.row_height = height;
        }
        let glyph = Glyph {
            above_baseline: (32 - top as i32) as i8,
            height: height as u8,
            width: width as u8,
            x: self.next_x as u8,
            y: self.row_y as u8,
        };
        for y in top..=bottom {
            for x in left..=right {
                if ink(x, y) {
                    let index =
                        (self.row_y as usize + y - top) * ATLAS_WIDTH + self.next_x as usize + x
                            - left;
                    *self
                        .pixels
                        .get_mut(index)
                        .ok_or("font atlas write outside supported bounds")? = 0xffff;
                }
            }
        }
        self.glyphs[code as usize] = glyph;
        self.next_x += width + 1;
        Ok(glyph)
    }
    pub fn metric_bytes(&self) -> Vec<u8> {
        self.glyphs.iter().flat_map(|g| g.bytes()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mask(left: usize, top: usize, width: usize, height: usize) -> [u8; BITMAP_BYTES] {
        let mut bits = [0; BITMAP_BYTES];
        for y in top..top + height {
            for x in left..left + width {
                bits[y * 16 + x / 8] |= 0x80 >> (x % 8);
            }
        }
        bits
    }
    #[test]
    fn source_bearing_signed_rows_and_cached_glyphs_keep_the_original_mask() {
        let mut atlas = Atlas::default();
        let glyph = atlas.insert(b'_', &mask(2, 35, 9, 2)).unwrap();
        assert_eq!(glyph.bytes(), [253, 2, 9, 0, 0]);
        assert_eq!(atlas.insert(b'_', &mask(0, 0, 1, 1)).unwrap(), glyph);
        assert_eq!(atlas.next_x, 10);
        assert_eq!(atlas.max_below_baseline, 4);
        assert_eq!(atlas.pixels.iter().filter(|p| **p != 0).count(), 18);
    }
    #[test]
    fn space_is_transparent_and_missing_ink_does_not_become_a_fake_visible_glyph() {
        let mut atlas = Atlas::default();
        assert!(atlas.insert(b'X', &[0; BITMAP_BYTES]).is_err());
        assert_eq!(atlas.next_x, 0);
        assert_eq!(
            atlas.insert(b' ', &[0; BITMAP_BYTES]).unwrap().bytes(),
            [0, 1, 5, 0, 0]
        );
        assert!(atlas.pixels.iter().all(|p| *p == 0));
    }
    #[test]
    fn row_growth_and_atlas_wrap_keep_previously_declared_cache_entries() {
        let mut atlas = Atlas::default();
        let full = mask(0, 0, 128, 64);
        atlas.insert(b'A', &full).unwrap();
        atlas.insert(b'B', &full).unwrap();
        let third = atlas.insert(b'C', &full).unwrap();
        assert_eq!((third.x, third.y, atlas.wraps), (0, 0, 1));
        assert_eq!(atlas.glyphs[b'A' as usize].width, 128);
    }
}
