use serde::Serialize;
use windows::{
    UI::ViewManagement::{UIColorType, UISettings},
    Win32::{Globalization::GetUserDefaultUILanguage, System::Registry::*},
    core::w,
};
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    pub language: String,
    pub dark: bool,
    pub accent: String,
    pub accent_ink: String,
    pub taskbar_light: bool,
    pub taskbar_small: bool,
}
impl Appearance {
    pub fn read(colors: &UISettings) -> Self {
        let color = colors.GetColorValue(UIColorType::Accent).ok();
        let accent = color
            .map(|c| format!("#{:02x}{:02x}{:02x}", c.R, c.G, c.B))
            .unwrap_or("#0067c0".into());
        let accent_ink = color
            .map(|c| {
                if 299 * c.R as u32 + 587 * c.G as u32 + 114 * c.B as u32 > 150000 {
                    "#000000"
                } else {
                    "#ffffff"
                }
            })
            .unwrap_or("#ffffff")
            .into();
        let language = unsafe {
            if GetUserDefaultUILanguage() & 0x3ff == 4 {
                "zh"
            } else {
                "en"
            }
        }
        .into();
        Self {
            language,
            dark: registry_light(w!("AppsUseLightTheme")) == 0,
            taskbar_light: registry_light(w!("SystemUsesLightTheme")) != 0,
            taskbar_small: read_taskbar_small(),
            accent,
            accent_ink,
        }
    }
}
pub fn registry_light(name: windows::core::PCWSTR) -> u32 {
    let mut value = 1u32;
    unsafe {
        let _ = RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            name,
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut 4),
        );
    }
    value
}
pub fn read_taskbar_small() -> bool {
    let read = |name| {
        let mut value = 0u32;
        unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced"),
                name,
                RRF_RT_REG_DWORD,
                None,
                Some((&mut value as *mut u32).cast()),
                Some(&mut 4),
            )
            .is_ok()
            .then_some(value)
        }
    };
    // shortcut: TaskbarSize is undocumented; revisit if Windows changes the tray modes.
    // IconSizePreference sizes app buttons, not notification-area glyphs.
    crate::taskbar_small(read(w!("TaskbarSize")), read(w!("TaskbarSi")))
}
