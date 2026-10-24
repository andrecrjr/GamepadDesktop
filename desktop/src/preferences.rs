use std::path::PathBuf;

use crate::{control::ControlSettings, i18n::Locale};

/// Global interface scale, kept bounded so controls remain reachable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u16)]
pub enum UiScale {
    Standard = 100,
    #[default]
    Comfortable = 125,
    Large = 150,
    ExtraLarge = 175,
}

impl UiScale {
    pub const ALL: [Self; 4] = [
        Self::Standard,
        Self::Comfortable,
        Self::Large,
        Self::ExtraLarge,
    ];

    pub fn factor(self) -> f32 {
        self as u16 as f32 / 100.0
    }

    pub fn step(self, increase: bool) -> Self {
        let index = Self::ALL
            .iter()
            .position(|scale| *scale == self)
            .unwrap_or(1);
        Self::ALL[if increase {
            (index + 1).min(3)
        } else {
            index.saturating_sub(1)
        }]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DesktopPreferences {
    pub locale: Locale,
    pub ui_scale: UiScale,
    pub controls: ControlSettings,
    pub start_on_login: bool,
    pub onboarding_completed: bool,
}

impl Default for DesktopPreferences {
    fn default() -> Self {
        Self {
            locale: Locale::detect(),
            ui_scale: UiScale::default(),
            controls: ControlSettings::default(),
            start_on_login: false,
            onboarding_completed: false,
        }
    }
}

impl DesktopPreferences {
    pub fn load() -> Self {
        preferences_path()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .unwrap_or_default()
            .normalized()
    }

    pub fn save(self) -> Result<(), String> {
        let path = preferences_path().ok_or("Configuration folder is unavailable")?;
        std::fs::create_dir_all(path.parent().expect("preferences directory"))
            .map_err(|error| error.to_string())?;
        let pending = path.with_extension("pending");
        std::fs::write(
            &pending,
            serde_json::to_vec_pretty(&self.normalized()).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        std::fs::rename(pending, path).map_err(|error| error.to_string())
    }

    fn normalized(mut self) -> Self {
        self.controls = self.controls.normalized();
        self
    }
}

fn preferences_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let directory = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(target_os = "windows"))]
    let directory = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));

    directory.map(|path| path.join("gamepad-navigator").join("preferences.json"))
}
