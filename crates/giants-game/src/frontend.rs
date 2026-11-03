//! First observed profile/main-menu slice. Persistence and other screens are open.
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    PlayerIds,
    NewPlayer,
    Main,
    LoadWorlds,
    LoadMissions,
    MissionLoading,
    DeletePlayer,
    ExitConfirmation,
    Options,
    Sound,
    Graphics,
    Resolution,
    Controls(usize),
}

#[derive(Clone)]
pub struct Entry {
    pub text: String,
    pub help: String,
    /// One measured 1920x1080 native text layout: left, baseline, nominal height.
    /// These samples do not implement the source menu mesh's projection.
    pub layout: [f32; 3],
}

pub struct ControlPage {
    pub title: String,
    pub entries: Vec<Entry>,
    pub bindings: Vec<(String, [f32; 3])>,
    pub slider_y: f32,
    pub invert_labels: [String; 2],
}
pub struct Frontend {
    pub control_pages: Vec<ControlPage>,
    pub mapping_help: String,
    pub title: String,
    pub new_id: String,
    pub new_title: String,
    pub new_help: String,
    pub new_id_help: String,
    pub profile: Entry,
    pub main: Vec<Entry>,
    pub exit_title: String,
    pub exit_choices: Vec<Entry>,
    pub delete_title: String,
    pub delete_choices: Vec<Entry>,
    pub options_title: String,
    pub options: Vec<Entry>,
    pub resolution_text: [String; 9],
    pub graphics_title: String,
    pub graphics: Vec<Entry>,
    pub graphics_values: [String; 6],
    pub sound_title: String,
    pub sound: Vec<Entry>,
    pub unavailable: String,
    pub sound_unavailable: Vec<Entry>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Activation {
    Transition(Screen),
    Quit,
    Unimplemented(usize),
}

impl Frontend {
    pub fn load(root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let table = giants_formats::gtext::Catalog::parse(&super::read_asset(
            root.join("Bin/GTextEnglish.bin"),
        )?)?;
        let text = |key: &str| -> Result<String, Box<dyn std::error::Error>> {
            let bytes = table
                .get(key.as_bytes())
                .ok_or_else(|| format!("Missing frontend text key: {key}"))?;
            if !bytes.is_ascii() {
                return Err(
                    format!("Frontend inspection currently supports ASCII English: {key}").into(),
                );
            }
            Ok(String::from_utf8(bytes.to_vec())?)
        };
        // S01 native draw inputs, independently matched to GText keys. Other poses open.
        let specs = [
            ("MainStart", "MH_StartSingle", 0.3679602, 0.1210093),
            ("MainJoin", "MH_JoinMulti", 0.405_894, 0.1721776),
            ("MainHost", "MH_HostMulti", 0.4024023, 0.2245475),
            ("Options", "MH_GameOptions", 0.4667488, 0.2791011),
            ("MainLoad", "MH_LoadSingle", 0.3914278, 0.3300646),
            ("MainChangeID", "MH_ChangeID", 0.4188631, 0.3806581),
            ("MainExitGame", "MH_ExitGame", 0.4542787, 0.4311327),
        ];
        let main = specs
            .into_iter()
            .map(|(key, help, x, y)| {
                Ok(Entry {
                    text: text(key)?,
                    help: text(help)?,
                    layout: [x, y, 0.0425653],
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
        // S04 verified native text-draw inputs, matched independently to GText.
        let options = [
            ("@G_Video_Options", "MH_GraphicsOptions", 0.23581605),
            ("@G_Audio_Options", "MH_SoundOptions", 0.29031357),
            ("@G_Keyboard_Mecc", "MH_MeccKeys", 0.3446977),
            ("@G_Keyboard_Reaper", "MH_ReaperKeys", 0.39878222),
            ("@G_Keyboard_Kabuto", "MH_KabutoKeys", 0.45262313),
            ("@G_Quit_To_Menu", "MH_GraphicsPrevMenu", 0.87922394),
        ]
        .into_iter()
        .map(|(key, help, y)| {
            Ok(Entry {
                text: text(key)?,
                help: text(help)?,
                layout: [0.3500367, y, 0.0426218],
            })
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
        let delete_choices = [("@E_Yes", 0.23348993), ("@E_No", 0.29031357)]
            .into_iter()
            .map(|(key, y)| {
                Ok(Entry {
                    text: text(key)?,
                    help: String::new(),
                    layout: [0.35, y, 0.0426219],
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
        let sound = [
            ("@A_Sound_Vol", "MH_AudioSFXVol", 0.13720639, 0.43970147),
            ("@A_Music_Vol", "MH_AudioMusicVol", 0.13720639, 0.4965251),
            ("@A_reset", "MH_AudioReset", 0.35087875, 0.8245615),
            ("@A_Return_To_Menu", "MH_AudioBack", 0.35087875, 0.87922394),
        ]
        .into_iter()
        .map(|(key, help, x, y)| {
            Ok(Entry {
                text: text(key)?,
                help: text(help)?,
                layout: [x, y, 0.0426218],
            })
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
        let graphics = [
            ("@V_model_detail", "MH_GraphicsLOD", 0.12064203, 0.23215395),
            ("@V_wave", "MH_GraphicsWaveEffects", 0.12064203, 0.28711513),
            (
                "@V_sea_effects",
                "MH_GraphicsWaterFX",
                0.12064203,
                0.34512427,
            ),
            (
                "@V_shadow_detail",
                "MH_GraphicsGeneralShadows",
                0.12064203,
                0.40194792,
            ),
            (
                "@V_mip_mapping",
                "MH_GraphicsMipmapping",
                0.12064203,
                0.4563321,
            ),
            ("@V_sun_flare", "MH_GraphicsSunFX", 0.12064203, 0.51097834),
            (
                "@V_object_bump",
                "MH_GraphicsObjBumps",
                0.12064203,
                0.5672396,
            ),
            (
                "@V_landscape_bump",
                "MH_GraphicsLandBumps",
                0.12064203,
                0.62188584,
            ),
            (
                "@V_environment_map",
                "MH_GraphicsEnvmapping",
                0.12064203,
                0.6787095,
            ),
            ("@V_clipping", "MH_GraphicsClipPlane", 0.12064203, 0.7340321),
            ("@V_reset2", "MH_GraphicsDefaults", 0.43159282, 0.8056084),
            (
                "@V_change_res",
                "MH_GraphicsResolutionMenu",
                0.41510984,
                0.8851387,
            ),
            (
                "@V_return_to_menu",
                "MH_GraphicsPrevMenu",
                0.39563078,
                0.9514694,
            ),
        ]
        .into_iter()
        .map(|(key, help, x, y)| {
            Ok(Entry {
                text: text(key)?,
                help: text(help)?,
                layout: [x, y, 0.0426218],
            })
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
        let legacy_control_titles = ["$MC_kbd_00", "$RP_kbd_00", "$KB_kbd_00"]
            .iter()
            .all(|key| table.get(key.as_bytes()).is_some());
        let independent_sensitivity_label =
            legacy_control_titles && table.get(b"$RP_kbd_mouse_speed").is_none();
        if independent_sensitivity_label {
            eprintln!(
                "frontend_control_labels legacy_titles=true sensitivity_label=independent_ui native_debug_layout_verified=false"
            );
        }
        let mut control_pages = Vec::new();
        for (unit, specs) in [
            crate::control_layout::MECC,
            crate::control_layout::REAPER,
            crate::control_layout::KABUTO,
        ]
        .into_iter()
        .enumerate()
        {
            let prefix = ["MC", "RP", "KB"][unit];
            let mut entries = Vec::new();
            let mut bindings = Vec::new();
            for &(key, binding, [x, y, size, bx, by]) in specs {
                entries.push(Entry {
                    text: text(key)?,
                    help: text("MH_NonMappingMode")?,
                    layout: [x, y, size],
                });
                bindings.push((binding.into(), [bx, by, size]));
            }
            let footers = [
                [[0.40318194, 0.9008504], [0.41919756, 0.9537304]],
                [[0.39584622, 0.9206389], [0.41811186, 0.96580666]],
                [[0.39610735, 0.87308645], [0.41759172, 0.9439456]],
            ][unit];
            for (suffix, [x, y]) in ["reset", "return"].into_iter().zip(footers) {
                entries.push(Entry {
                    text: text(&format!("${prefix}_kbd_{suffix}"))?,
                    help: if suffix == "return" {
                        text("MH_GraphicsPrevMenu")?
                    } else {
                        "Restore the initial character bindings".into()
                    },
                    layout: [x, y, 0.0333333],
                });
            }
            let mouse_y = [
                [0.8366906, 0.84837353],
                [0.85129267, 0.8740252],
                [0.80398375, 0.80398375],
            ][unit];
            let mouse_entries = [
                ("$RP_kbd_mouse_speed", "MH_MSpeed", 0.099957354, mouse_y[0]),
                ("$RP_kbd_invert_on", "MH_InvertMouse", 0.426, mouse_y[1]),
            ]
            .into_iter()
            .map(|(key, help, x, y)| {
                Ok(Entry {
                    text: if key == "$RP_kbd_mouse_speed" && independent_sensitivity_label {
                        "Mouse sensitivity".into()
                    } else {
                        text(key)?
                    },
                    help: if help == "MH_MSpeed" && independent_sensitivity_label {
                        "Adjust mouse sensitivity".into()
                    } else {
                        text(help)?
                    },
                    layout: [x, y, 0.0333333],
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
            for (offset, entry) in mouse_entries.into_iter().enumerate() {
                entries.insert(specs.len() + offset, entry);
            }
            control_pages.push(ControlPage {
                title: {
                    let patched = format!("${prefix}_kbd_00_P");
                    let legacy = format!("${prefix}_kbd_00");
                    text(if table.get(patched.as_bytes()).is_some() {
                        &patched
                    } else if legacy_control_titles {
                        &legacy
                    } else {
                        &patched
                    })?
                },
                entries,
                bindings,
                slider_y: [0.9144, 0.928, 0.882][unit],
                invert_labels: [text("$RP_kbd_invert_on")?, text("$RP_kbd_invert_off")?],
            });
        }
        Ok(Self {
            control_pages,
            mapping_help: text("MH_MappingMode")?,
            title: text("$IDselect")?,
            new_id: text("$IDnew")?,
            new_title: text("IDNewTitle")?,
            new_help: text("MH_CreateNewID")?,
            new_id_help: text("MH_NewID")?,
            // Observed native default only. No registry/profile persistence is emulated.
            profile: Entry {
                text: "Player".into(),
                help: text("MH_SelectID")?,
                layout: [0.4722118, 0.1235324, 0.0425653],
            },
            main,
            exit_title: text("@E_Exit_To_Desktop")?,
            exit_choices: [
                ("@E_Yes", [0.48333257, 0.23348992, 0.042621866]),
                ("@E_No", [0.48803326, 0.29031357, 0.042621642]),
            ]
            .into_iter()
            .map(|(key, layout)| {
                Ok(Entry {
                    text: text(key)?,
                    help: String::new(),
                    layout,
                })
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?,
            delete_title: text("@E_Delete_ID")?,
            delete_choices,
            options_title: text("@G_Title")?,
            options,
            resolution_text: [
                text("@V_Title_res")?,
                text("@V_currentsettings")?,
                text("@V_newsettings")?,
                text("@V_select_driver")?,
                text("@V_select_res")?,
                text("@V_apply_change")?,
                text("@V_reset")?,
                text("@V_Return_To_Video")?,
                "Back to graphics options menu".into(),
            ],
            graphics_title: text("@V_title")?,
            graphics,
            graphics_values: [
                text("Off")?,
                text("On")?,
                text("Low")?,
                text("Medium")?,
                text("High")?,
                "Med".into(),
            ],
            sound_title: text("@A_Title")?,
            sound,
            unavailable: text("UnAvail")?,
            sound_unavailable: [("@A_eax", 0.5548412), ("@A_occlusion", 0.6116648)]
                .into_iter()
                .map(|(key, y)| {
                    Ok(Entry {
                        text: text(key)?,
                        help: String::new(),
                        layout: [0.13720639, y, 0.0426218],
                    })
                })
                .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?,
        })
    }

    pub fn entries(&self, screen: Screen) -> &[Entry] {
        match screen {
            Screen::PlayerIds => std::slice::from_ref(&self.profile),
            Screen::NewPlayer
            | Screen::Resolution
            | Screen::LoadWorlds
            | Screen::LoadMissions
            | Screen::MissionLoading => &[],
            Screen::Main => &self.main,
            Screen::DeletePlayer => &self.delete_choices,
            Screen::ExitConfirmation => &self.exit_choices,
            Screen::Options => &self.options,
            Screen::Sound => &self.sound,
            Screen::Graphics => &self.graphics,
            Screen::Controls(unit) => &self.control_pages[unit].entries,
        }
    }

    pub fn activate(screen: Screen, index: usize) -> Activation {
        match (screen, index) {
            (Screen::PlayerIds, 0) => Activation::Transition(Screen::Main),
            (Screen::Main, 6) => Activation::Transition(Screen::ExitConfirmation),
            (Screen::ExitConfirmation, 0) => Activation::Quit,
            (Screen::ExitConfirmation, 1) => Activation::Transition(Screen::Main),
            (Screen::Main, 3) => Activation::Transition(Screen::Options),
            (Screen::Options, index @ 2..=4) => Activation::Transition(Screen::Controls(index - 2)),
            (Screen::Options, 0) => Activation::Transition(Screen::Graphics),
            (Screen::Options, 1) => Activation::Transition(Screen::Sound),
            (Screen::Sound, 3) => Activation::Transition(Screen::Options),
            (Screen::Options, 5) => Activation::Transition(Screen::Main),
            (Screen::Main, 5) => Activation::Transition(Screen::PlayerIds),
            _ => Activation::Unimplemented(index),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_exit_requires_explicit_confirmation() {
        assert_eq!(
            Frontend::activate(Screen::Main, 6),
            Activation::Transition(Screen::ExitConfirmation)
        );
        assert_eq!(
            Frontend::activate(Screen::ExitConfirmation, 1),
            Activation::Transition(Screen::Main)
        );
        assert_eq!(
            Frontend::activate(Screen::ExitConfirmation, 0),
            Activation::Quit
        );
        assert_eq!(
            Frontend::activate(Screen::ExitConfirmation, 2),
            Activation::Unimplemented(2)
        );
    }
    #[test]
    fn only_observed_profile_transitions_are_supported() {
        assert_eq!(
            Frontend::activate(Screen::PlayerIds, 0),
            Activation::Transition(Screen::Main)
        );
        assert_eq!(
            Frontend::activate(Screen::Main, 5),
            Activation::Transition(Screen::PlayerIds)
        );
        for index in [0, 1, 2, 4, usize::MAX] {
            assert_eq!(
                Frontend::activate(Screen::Main, index),
                Activation::Unimplemented(index)
            );
        }
    }
}
