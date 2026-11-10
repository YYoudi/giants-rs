//! Initialized PC metadata for the acquired family/view selector. Header layout
//! recognition does not authenticate the binary or produce camera startup state.
use super::{Catalog, Family, Selection};
use crate::camera_mode::Profile;

#[derive(Clone, Copy, Debug)]
pub struct Record {
    pub camera: crate::mecc_camera_update::Profile,
    /// Signed owner look angle, separate from model scale and body orientation.
    pub pitch: f32,
}
pub struct ExecutableCatalog {
    records: [[Record; 3]; 9],
}
impl ExecutableCatalog {
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        Self::decode(&crate::executable_metadata::read(path)?)
    }
    /// Read coefficients only. Validation belongs to the consuming kernel:
    /// projection reset does not consume pitch, distance or body scale.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        use crate::executable_metadata::{Image, Layout};
        let image = Image::decode(bytes)?;
        if image.layout != Layout::DebugReference {
            return Err("camera coefficient metadata layout not acquired".into());
        }
        let data = image.data.data(0x15a208, 9 * 3 * 24)?;
        let records = std::array::from_fn(|family| {
            std::array::from_fn(|view| {
                let at = (family * 3 + view) * 24;
                let value = |offset| {
                    f32::from_le_bytes(data[at + offset..at + offset + 4].try_into().unwrap())
                };
                Record {
                    camera: crate::mecc_camera_update::Profile {
                        distance: value(0),
                        height: value(4),
                        vertical_fov: value(8),
                        side: value(12),
                        along: value(16),
                    },
                    pitch: value(20),
                }
            })
        });
        Ok(Self { records })
    }
}
impl ExecutableCatalog {
    /// Includes signed coefficients; this is data, not complete camera support
    /// for every family/view, nor startup or projection acknowledgement.
    pub fn record(&self, selection: Selection) -> Result<Record, String> {
        let family = match selection.family() {
            Family::Model2 => 0,
            Family::Model2Variant => 1,
            Family::Model1 => 2,
            Family::Model3 => 3,
            Family::Model1076 => 4,
            Family::Model7 => 5,
            Family::Model6 => 6,
            Family::Model4 => 7,
            Family::Default => 8,
        };
        self.records[family]
            .get(usize::from(selection.view()))
            .copied()
            .ok_or_else(|| "camera coefficient view outside acquired table".into())
    }
}
impl Catalog for ExecutableCatalog {
    fn profile(&self, selection: Selection) -> Result<Profile, String> {
        let record = self.record(selection)?;
        Ok(Profile {
            vertical_fov: record.camera.vertical_fov,
            actor_scale: record.pitch,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executable_metadata::{Layout, synthetic_image};

    fn synthetic() -> Vec<u8> {
        let mut bytes = synthetic_image(Layout::DebugReference);
        let start = 4096 + 0x15a208 - 1_413_120;
        for row in 0..27 {
            for (offset, value) in [
                (0, row as f32 - 12.),
                (4, row as f32 + 3.),
                (8, 40. + row as f32),
                (12, row as f32 - 5.),
                (16, row as f32 + 7.),
                (20, 1. + row as f32 / 32.),
            ] {
                bytes[start + row * 24 + offset..start + row * 24 + offset + 4]
                    .copy_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }
    #[test]
    fn relocated_catalog_distinguishes_every_family_and_view_and_refuses_other_layouts() {
        let catalog = ExecutableCatalog::decode(&synthetic()).unwrap();
        let families = [
            Family::Model2,
            Family::Model2Variant,
            Family::Model1,
            Family::Model3,
            Family::Model1076,
            Family::Model7,
            Family::Model6,
            Family::Model4,
            Family::Default,
        ];
        for (index, family) in families.into_iter().enumerate() {
            for view in 0..3 {
                let profile = Selection { family, view }.resolve(&catalog).unwrap();
                let row = index * 3 + usize::from(view);
                assert_eq!(profile.vertical_fov, 40. + row as f32);
                assert_eq!(profile.actor_scale, 1. + row as f32 / 32.);
                let record = catalog.record(Selection { family, view }).unwrap();
                assert_eq!(record.camera.distance, row as f32 - 12.);
                assert_eq!(record.camera.height, row as f32 + 3.);
                assert_eq!(record.camera.side, row as f32 - 5.);
                assert_eq!(record.camera.along, row as f32 + 7.);
                assert_eq!(record.pitch, profile.actor_scale);
            }
        }
        assert!(ExecutableCatalog::decode(&synthetic_image(Layout::Retail)).is_err());
        assert!(ExecutableCatalog::decode(&synthetic_image(Layout::Original)).is_err());
        assert!(ExecutableCatalog::decode(&synthetic()[..8192]).is_err());
    }

    struct Reset<'a> {
        catalog: &'a ExecutableCatalog,
        submitted: Option<f32>,
    }
    impl crate::camera_mode::Services<()> for Reset<'_> {
        fn profile(&mut self, _: u32, view: u32) -> Result<Profile, String> {
            Selection {
                family: Family::Model3,
                view: u8::try_from(view).unwrap(),
            }
            .resolve(self.catalog)
        }
        fn scale(&mut self, _: (), _: f32) -> Result<(), String> {
            panic!("projection reset must not consume actor scale")
        }
        fn projection(&mut self, fov: f32) -> Result<(), String> {
            self.submitted = Some(fov);
            Ok(())
        }
    }
    #[test]
    fn projection_reset_validates_its_consumed_lens_after_clearing_transition() {
        let mut bytes = synthetic();
        let at = 4096 + 0x15a208 - 1_413_120 + 9 * 24;
        bytes[at + 20..at + 24].copy_from_slice(&f32::NAN.to_le_bytes());
        let catalog = ExecutableCatalog::decode(&bytes).unwrap();
        let mut services = Reset {
            catalog: &catalog,
            submitted: None,
        };
        let mut mode = crate::camera_mode::State::new(0, 0, 0.5).unwrap();
        mode.reset_projection(&mut services).unwrap();
        assert_eq!(services.submitted, Some(49.));
        assert_eq!(mode.transition(), 0.);
        bytes[at + 8..at + 12].copy_from_slice(&0f32.to_le_bytes());
        let catalog = ExecutableCatalog::decode(&bytes).unwrap();
        let mut services = Reset {
            catalog: &catalog,
            submitted: None,
        };
        let mut mode = crate::camera_mode::State::new(0, 0, 0.5).unwrap();
        assert!(mode.reset_projection(&mut services).is_err());
        assert_eq!(mode.transition(), 0.);
        assert!(mode.failure().is_some());
        assert!(services.submitted.is_none());
    }

    #[test]
    #[ignore = "Requires GIANTS_DELPHI_ROOT and GIANTS_DELPHI_EXECUTABLE; actual owned class-3 selection and external coefficients, supplied mode/owner, projection capture only"]
    fn source_camera_coefficients_feed_owned_delphi_projection_reset() {
        use crate::{animation_channels::Blending, resident_states::Session};
        let root = std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_ROOT").unwrap());
        let executable =
            std::path::PathBuf::from(std::env::var("GIANTS_DELPHI_EXECUTABLE").unwrap());
        let resources =
            crate::scene_resources::SceneResources::load(&root, "w_Rstory1.bin", "English")
                .unwrap();
        let globals =
            giants_formats::global_data::parse(&std::fs::read(root.join("Bin/GData.gbn")).unwrap())
                .unwrap();
        let weapons = giants_formats::weapons::parse_with_profile(
            &std::fs::read(root.join("Bin/weapondefs.bin")).unwrap(),
            giants_formats::weapons::Profile::Debug,
        )
        .unwrap();
        let factory = crate::delphi_special::Factory::load(&resources, &globals, &weapons).unwrap();
        let mut session = Session::new(1833);
        let id = factory
            .construct_player_fields(
                &mut session,
                crate::delphi_player::ColdFrame {
                    position: [0.; 3],
                    current: [0.; 3],
                    previous: [0.; 3],
                    angles: [0.; 3],
                },
                Blending::Enabled,
            )
            .unwrap()
            .owner;
        let catalog = ExecutableCatalog::read(&executable).unwrap();
        let ordinary = crate::delphi_camera::OrdinaryProfiles::read(&executable).unwrap();
        let owners = crate::camera_owner::State::new(Some(id), 7);
        struct OwnedReset<'a> {
            session: &'a Session,
            owners: &'a crate::camera_owner::State<crate::resident_states::Id>,
            controlled: crate::resident_states::Id,
            catalog: &'a ExecutableCatalog,
            scaled: bool,
            submitted: Option<f32>,
        }
        impl crate::camera_mode::Services<()> for OwnedReset<'_> {
            fn profile(&mut self, _: u32, view: u32) -> Result<Profile, String> {
                super::super::select(
                    super::super::Input {
                        owner: self.owners.owner_from_session(self.session)?,
                        controlled: Some(self.controlled),
                        view,
                        scaled_input: self.scaled,
                    },
                    self.session,
                )?
                .resolve(self.catalog)
            }
            fn scale(&mut self, _: (), _: f32) -> Result<(), String> {
                panic!("reset does not scale")
            }
            fn projection(&mut self, fov: f32) -> Result<(), String> {
                self.submitted = Some(fov);
                Ok(())
            }
        }
        for scaled in [false, true] {
            let mut mode = crate::camera_mode::State::new(0, 0, 0.25).unwrap();
            let mut services = OwnedReset {
                session: &session,
                owners: &owners,
                controlled: id,
                catalog: &catalog,
                scaled,
                submitted: None,
            };
            mode.reset_projection(&mut services).unwrap();
            assert_eq!(
                services.submitted,
                Some(ordinary.select(scaled).vertical_fov)
            );
            assert_eq!(mode.transition(), 0.);
        }
        session.clear();
        let mut mode = crate::camera_mode::State::new(0, 0, 0.25).unwrap();
        let mut services = OwnedReset {
            session: &session,
            owners: &owners,
            controlled: id,
            catalog: &catalog,
            scaled: false,
            submitted: None,
        };
        assert!(mode.reset_projection(&mut services).is_err());
        assert_eq!(mode.transition(), 0.25);
        assert!(services.submitted.is_none());
        assert!(mode.failure().is_some());
    }
}
