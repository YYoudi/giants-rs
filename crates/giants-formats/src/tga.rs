//! Bounded uncompressed TGA profiles. RGB is top-down; single-channel samples
//! retain serialized order, matching the measured native texture-data copy.
use crate::{Error, slice};
/// Source-bounded TGA2 metadata: no developer blocks or subordinate tables.
/// Some retained exporters declare494 for a physical495-byte extension. That
/// legacy profile is admitted only with straight alpha and exact boundaries.
fn footer(data: &[u8], pixel_end: usize, alpha: bool) -> Result<(), Error> {
    if pixel_end == data.len() {
        return Ok(());
    }
    let start = data
        .len()
        .checked_sub(26)
        .filter(|s| *s >= pixel_end)
        .ok_or(Error("truncated TGA footer"))?;
    let trailer = &data[start..];
    if &trailer[8..] != b"TRUEVISION-XFILE.\0" || trailer[4..8] != [0; 4] {
        return Err(Error("unsupported TGA footer"));
    }
    let extension = u32::from_le_bytes(
        trailer[..4]
            .try_into()
            .map_err(|_| Error("invalid TGA extension offset"))?,
    ) as usize;
    if extension == 0 {
        return if start == pixel_end {
            Ok(())
        } else {
            Err(Error("unreferenced TGA trailing bytes"))
        };
    }
    if extension != pixel_end || start.checked_sub(extension) != Some(495) {
        return Err(Error("unsupported TGA extension span"));
    }
    let metadata = &data[extension..start];
    let declared = u16::from_le_bytes([metadata[0], metadata[1]]);
    let attributes = metadata[494];
    if !(declared == 495 || declared == 494 && alpha && attributes == 3)
        || metadata[482..494].iter().any(|v| *v != 0)
        || attributes != if alpha { 3 } else { 0 }
    {
        return Err(Error("unsupported TGA extension metadata"));
    }
    Ok(())
}
pub struct RgbImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub alpha: Option<Vec<u8>>,
}
/// Single-channel file data. No RGB, alpha, height or GPU interpretation is
/// inferred. The origin bit is metadata; `samples` always keeps file row order.
pub struct Plane8 {
    pub width: u32,
    pub height: u32,
    pub top_origin: bool,
    pub samples: Vec<u8>,
}
pub enum Image {
    TrueColor(RgbImage),
    Plane8(Plane8),
}
impl Image {
    pub fn profile(&self) -> &'static str {
        match self {
            Self::TrueColor(_) => "true_color",
            Self::Plane8(_) => "plane8",
        }
    }
    pub fn dimensions(&self) -> [u32; 2] {
        match self {
            Self::TrueColor(image) => [image.width, image.height],
            Self::Plane8(image) => [image.width, image.height],
        }
    }
}
/// Typed source data only. Consumer-specific upload/interpretation is separate.
pub fn decode_typed(data: &[u8]) -> Result<Image, Error> {
    match slice(data, 0, 18)?[2] {
        2 => decode(data).map(Image::TrueColor),
        3 => decode_plane8(data).map(Image::Plane8),
        _ => Err(Error("unsupported typed TGA image kind")),
    }
}
pub fn decode_plane8(data: &[u8]) -> Result<Plane8, Error> {
    let header = slice(data, 0, 18)?;
    if header[0] != 0
        || header[1] != 0
        || header[2] != 3
        || header[16] != 8
        || !matches!(header[17] & !0x20, 0 | 8)
        || header[3..12].iter().any(|byte| *byte != 0)
    {
        return Err(Error("unsupported single-channel TGA profile"));
    }
    let width = u16::from_le_bytes([header[12], header[13]]) as usize;
    let height = u16::from_le_bytes([header[14], header[15]]) as usize;
    if width == 0 || height == 0 || width * height > 16_777_216 {
        return Err(Error("single-channel TGA pixel budget"));
    }
    let samples = slice(data, 18, width * height)?;
    footer(data, 18 + samples.len(), false)?;
    Ok(Plane8 {
        width: width as u32,
        height: height as u32,
        top_origin: header[17] & 0x20 != 0,
        samples: samples.to_vec(),
    })
}
pub fn decode(data: &[u8]) -> Result<RgbImage, Error> {
    let header = slice(data, 0, 18)?;
    if header[0] != 0
        || header[1] != 0
        || header[2] != 2
        || !matches!(header[16], 24 | 32)
        || header[17] & !0x20 != if header[16] == 32 { 8 } else { 0 }
    {
        return Err(Error("unsupported TGA profile"));
    }
    if header[3..12].iter().any(|byte| *byte != 0) {
        return Err(Error("unsupported TGA origin or color map"));
    }
    let width = u16::from_le_bytes([header[12], header[13]]) as usize;
    let height = u16::from_le_bytes([header[14], header[15]]) as usize;
    if width == 0 || height == 0 || width * height > 16_777_216 {
        return Err(Error("TGA pixel budget"));
    }
    let channels = usize::from(header[16] / 8);
    let size = width * height * channels;
    let source = slice(data, 18, size)?;
    footer(data, 18 + size, channels == 4)?;
    let mut pixels = vec![0; width * height * 3];
    let mut alpha = (channels == 4).then(|| vec![0; width * height]);
    for y in 0..height {
        let source_y = if header[17] & 0x20 == 0 {
            height - 1 - y
        } else {
            y
        };
        for x in 0..width {
            let from = (source_y * width + x) * channels;
            let to = (y * width + x) * 3;
            pixels[to..to + 3].copy_from_slice(&[source[from + 2], source[from + 1], source[from]]);
            if let Some(alpha) = &mut alpha {
                alpha[y * width + x] = source[from + 3];
            }
        }
    }
    Ok(RgbImage {
        width: width as u32,
        height: height as u32,
        pixels,
        alpha,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extension_metadata_preserves_pixels_and_rejects_unbounded_references() {
        let mut data = vec![0u8; 18];
        data[2] = 2;
        data[12] = 1;
        data[14] = 1;
        data[16] = 32;
        data[17] = 8;
        data.extend([11, 22, 33, 44]);
        let end = data.len();
        let mut extension = vec![0; 495];
        extension[..2].copy_from_slice(&495u16.to_le_bytes());
        extension[494] = 3;
        data.extend(extension);
        data.extend((end as u32).to_le_bytes());
        data.extend(0u32.to_le_bytes());
        data.extend(b"TRUEVISION-XFILE.\0");
        for size in [494u16, 495] {
            data[end..end + 2].copy_from_slice(&size.to_le_bytes());
            let image = decode(&data).unwrap();
            assert_eq!(image.pixels, [33, 22, 11]);
            assert_eq!(image.alpha.unwrap(), [44]);
        }
        for (at, value) in [
            (end, 0),
            (end + 482, 1),
            (end + 494, 4),
            (end + 495, 1),
            (end + 499, 1),
        ] {
            let mut bad = data.clone();
            bad[at] = value;
            assert!(decode(&bad).is_err());
        }
        for length in end + 1..data.len() {
            assert!(decode(&data[..length]).is_err());
        }
    }
    #[test]
    fn single_channel_origin_is_metadata_not_an_implicit_flip() {
        let mut data = vec![0; 18];
        data[2] = 3;
        data[12] = 2;
        data[14] = 2;
        data[16] = 8;
        data.extend([11, 73, 129, 254]);
        for descriptor in [0, 8, 0x20, 0x28] {
            data[17] = descriptor;
            let image = decode_plane8(&data).unwrap();
            assert_eq!(image.samples, [11, 73, 129, 254]);
            assert_eq!(image.top_origin, descriptor & 0x20 != 0);
            assert!(matches!(decode_typed(&data).unwrap(), Image::Plane8(_)));
            assert!(decode(&data).is_err());
        }
    }
    #[test]
    fn single_channel_truncations_and_other_payloads_fail_explicitly() {
        let mut data = vec![0; 18];
        data[2] = 3;
        data[12] = 2;
        data[14] = 2;
        data[16] = 8;
        data.extend([1, 2, 3, 4]);
        for end in 0..data.len() {
            assert!(decode_plane8(&data[..end]).is_err());
        }
        data.push(0);
        assert!(decode_plane8(&data).is_err());
        data.pop();
        data[2] = 2;
        assert!(decode_plane8(&data).is_err());
        data[2] = 3;
        data[1] = 1;
        assert!(decode_plane8(&data).is_err());
        data[1] = 0;
        data[17] = 0x10;
        assert!(decode_plane8(&data).is_err());
    }
    #[test]
    fn bgra_alpha_tracks_vertical_orientation_and_truncation() {
        let mut data = vec![0; 18];
        data[2] = 2;
        data[12] = 1;
        data[14] = 2;
        data[16] = 32;
        data[17] = 8;
        data.extend([255, 0, 0, 31, 0, 0, 255, 192]);
        let image = decode(&data).unwrap();
        assert_eq!(image.pixels, [255, 0, 0, 0, 0, 255]);
        assert_eq!(image.alpha.as_deref(), Some(&[192, 31][..]));
        data[17] = 0x28;
        assert_eq!(
            decode(&data).unwrap().alpha.as_deref(),
            Some(&[31, 192][..])
        );
        for end in 0..data.len() {
            assert!(decode(&data[..end]).is_err());
        }
    }
    #[test]
    fn bgr_bottom_origin_and_truncations() {
        let mut data = vec![0; 18];
        data[2] = 2;
        data[12] = 1;
        data[14] = 2;
        data[16] = 24;
        data.extend([255, 0, 0, 0, 0, 255]);
        assert_eq!(decode(&data).unwrap().pixels, [255, 0, 0, 0, 0, 255]);
        for end in 0..data.len() {
            assert!(decode(&data[..end]).is_err());
        }
        data[17] = 0x10;
        assert!(decode(&data).is_err());
    }
}
