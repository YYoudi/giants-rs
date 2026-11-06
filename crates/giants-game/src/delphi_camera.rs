//! Delphi's pre-heading authored-view override. The surrounding primary,
//! target production, ordinary camera update and renderer remain separate.
use crate::procedural_aim::Matrix;

#[derive(Clone, Debug, Default)]
pub struct State {
    pub enabled: bool,
    pub descriptor: Option<i32>,
    pub matrix_selected: bool,
    /// Retained across reset. Absence is never replaced by an identity pose.
    pub retained_view: Option<Matrix>,
    pub vertical_fov: Option<f32>,
    pub failure: Option<String>,
}
impl State {
    /// Only a selected active view may be consumed as a camera override.
    pub fn active_view(&self) -> Result<Option<Matrix>, &'static str> {
        if self.failure.is_some() {
            return Err("Delphi camera producer is quarantined");
        }
        if !self.enabled || !self.matrix_selected {
            return Ok(None);
        }
        self.retained_view
            .map(Some)
            .ok_or("selected Delphi view absent")
    }
}

pub struct Environment {
    pub local_owner: bool,
    /// Supplied source mode; only -1 permits this callback's view changes.
    pub mode: i32,
    pub buttons: u32,
    pub aspect: f32,
    /// An acquired actor identity boundary, not a target-selection producer.
    pub target: Option<crate::resident_states::Id>,
}

/// Borrowed world camera state for the controlled owner's entered phase.
/// Local ownership and buttons come from that same movement invocation.
pub struct Context<'a> {
    pub state: &'a mut State,
    pub mode: i32,
    pub aspect: f32,
    pub target: Option<crate::resident_states::Id>,
}

/// Base class-3 third-person profile from user-supplied initialized PC data.
/// No native code is executed. Dynamic profile selection and lens effects are
/// separate; the acquired layout is not authentication or cross-build parity.
pub fn ordinary_profile(
    path: &std::path::Path,
) -> Result<crate::mecc_camera_update::Profile, String> {
    decode_ordinary_profile(&crate::executable_metadata::read(path)?)
}
pub fn decode_ordinary_profile(bytes: &[u8]) -> Result<crate::mecc_camera_update::Profile, String> {
    use crate::executable_metadata::{Image, Layout, word};
    let image = Image::decode(bytes)?;
    if image.layout != Layout::DebugReference {
        return Err("Delphi camera profile metadata layout not acquired".into());
    }
    let data = image.data.data(0x15a2e0, 20)?;
    let values = (0..5)
        .map(|i| word(data, i * 4).map(f32::from_bits))
        .collect::<Result<Vec<_>, _>>()?;
    if !values.iter().all(|v| v.is_finite())
        || values[0] < 0.
        || !(0. ..180.).contains(&values[2])
        || values[2] == 0.
    {
        return Err("Delphi camera profile is not finite and usable".into());
    }
    Ok(crate::mecc_camera_update::Profile {
        distance: values[0],
        height: values[1],
        vertical_fov: values[2],
        side: values[3],
        along: values[4],
    })
}

#[cfg(test)]
mod profile_tests {
    use super::*;
    #[test]
    fn external_profile_reads_relocated_data_and_refuses_unacquired_or_invalid_inputs() {
        use crate::executable_metadata::{Layout, synthetic_image};
        let mut bytes = synthetic_image(Layout::DebugReference);
        let at = 4096 + 0x15a2e0 - 1_413_120;
        for (i, v) in [27_f32, 9., 73., -2., 3.].into_iter().enumerate() {
            bytes[at + i * 4..at + i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        let profile = decode_ordinary_profile(&bytes).unwrap();
        assert_eq!(
            [
                profile.distance,
                profile.height,
                profile.vertical_fov,
                profile.side,
                profile.along
            ],
            [27., 9., 73., -2., 3.]
        );
        for invalid in [f32::NAN, 0., 180.] {
            bytes[at + 8..at + 12].copy_from_slice(&invalid.to_le_bytes());
            assert!(decode_ordinary_profile(&bytes).is_err());
        }
        for layout in [Layout::Original, Layout::Retail] {
            assert!(decode_ordinary_profile(&synthetic_image(layout)).is_err());
        }
        assert!(decode_ordinary_profile(&bytes[..at]).is_err());
    }
}
