//! Explicit optional comparison input; never required by the standalone viewer.
use bevy::prelude::*;
use std::io::Read;
#[derive(Clone)]
pub struct ValidationCamera {
    pub view: [f32; 16],
    pub projection: [f32; 16],
    pub viewport: UVec4,
}
fn multiply(a: [f64; 16], b: [f64; 16]) -> [f64; 16] {
    std::array::from_fn(|i| (0..4).map(|k| a[i / 4 * 4 + k] * b[k * 4 + i % 4]).sum())
}
fn bevy_to_source() -> Mat4 {
    Mat4::from_cols(Vec4::X, Vec4::Z, -Vec4::Y, Vec4::W)
}
impl ValidationCamera {
    pub fn read(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut data = Vec::with_capacity(153);
        std::fs::File::open(path)?
            .take(153)
            .read_to_end(&mut data)?;
        Self::parse(&data)
    }
    fn parse(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if data.len() != 152 || &data[..8] != b"GCAM0001" {
            return Err("Invalid validation camera profile".into());
        }
        let scalars: Vec<f32> = data[8..136]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|a| f32::from_le_bytes(*a))
            .collect();
        let view: [f32; 16] = scalars[..16].try_into()?;
        let projection: [f32; 16] = scalars[16..].try_into()?;
        let fields: Vec<u32> = data[136..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|a| u32::from_le_bytes(*a))
            .collect();
        let viewport = UVec4::new(fields[0], fields[1], fields[2], fields[3]);
        let view_det = Mat4::from_cols_array(&view).determinant();
        let projection_det = Mat4::from_cols_array(&projection).determinant();
        if !scalars.iter().all(|v| v.is_finite())
            || viewport.z == 0
            || viewport.w == 0
            || viewport.x.checked_add(viewport.z).is_none_or(|n| n > 1920)
            || viewport.y.checked_add(viewport.w).is_none_or(|n| n > 1080)
            || !view_det.is_finite()
            || view_det.abs() < 1e-6
            || !projection_det.is_finite()
            || projection_det.abs() < 1e-10
            || view[3] != 0.
            || view[7] != 0.
            || view[11] != 0.
            || view[15] != 1.
        {
            return Err("Invalid validation camera values or viewport".into());
        }
        let result = Self {
            view,
            projection,
            viewport,
        };
        if !result.clip_from_bevy().is_finite()
            || (0..9).any(|c| !result.uv_from_bevy(c, 1.).is_finite())
        {
            return Err("Validation matrix overflow".into());
        }
        Ok(result)
    }
    pub fn clip_from_bevy(&self) -> Mat4 {
        let m = giants_game::camera_matrix::compose(
            self.view,
            Some(self.projection),
            giants_game::arithmetic::Precision::Double53,
        );
        Mat4::from_cols_array(&m) * bevy_to_source()
    }
    pub fn uv_from_bevy(&self, class: u8, scale: f32) -> Mat4 {
        Self::uv_for_view(self.view, class, scale)
    }
    pub fn uv_for_view(view: [f32; 16], class: u8, scale: f32) -> Mat4 {
        let texture = giants_game::terrain_uv::texture_from_eye(view, class, scale)
            .expect("terrain camera preparation requires a finite affine view and class");
        let uv = multiply(view.map(f64::from), texture.map(f64::from)).map(|n| n as f32);
        Mat4::from_cols_array(&uv) * bevy_to_source()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut out = b"GCAM0001".to_vec();
        for _ in 0..2 {
            for value in Mat4::IDENTITY.to_cols_array() {
                out.extend(value.to_le_bytes());
            }
        }
        for value in [0u32, 135, 1920, 810] {
            out.extend(value.to_le_bytes());
        }
        out
    }
    #[test]
    fn bounded_camera_rejects_truncation_nan_and_viewport_overflow() {
        let data = fixture();
        assert!(ValidationCamera::parse(&data).is_ok());
        let mut trailing = data.clone();
        trailing.push(0);
        assert!(ValidationCamera::parse(&trailing).is_err());
        for end in 0..data.len() {
            assert!(ValidationCamera::parse(&data[..end]).is_err());
        }
        let mut bad = data.clone();
        bad[8..12].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(ValidationCamera::parse(&bad).is_err());
        let mut bad = data;
        bad[136..140].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(ValidationCamera::parse(&bad).is_err());
    }
    #[test]
    fn uv_keeps_view_basis_drift_instead_of_normalizing_it() {
        let mut view = Mat4::IDENTITY.to_cols_array();
        view[0] = 0.99;
        let uv = ValidationCamera::uv_for_view(view, 0, 1.) * Vec4::X;
        assert!((uv.x - 0.9801).abs() < 1e-6);
        assert_ne!(uv.x, 1.);
    }
}
