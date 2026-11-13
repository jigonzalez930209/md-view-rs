//! Persisted application preferences.

use craft_settings::Settings;
use mdview_i18n::LanguagePreference;
use mdview_theme::ThemeMode;
use serde::{Deserialize, Serialize};

/// The application settings stored in `config.toml`.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Prefs {
    /// Appearance settings.
    pub appearance: Appearance,
    /// Language settings.
    pub language: LanguageSettings,
    /// Window settings.
    pub window: WindowSettings,
    /// Editor settings.
    pub editor: EditorSettings,
}

impl Settings for Prefs {
    const APP_ID: &'static str = "mdview";
    const SCHEMA_VERSION: u32 = 1;
}

/// Theme and density settings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Appearance {
    /// `light` or `dark`.
    pub theme: String,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "light".to_owned(),
        }
    }
}

/// UI language settings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct LanguageSettings {
    /// `auto`, `en`, or `es`.
    pub language: String,
}

impl Default for LanguageSettings {
    fn default() -> Self {
        Self {
            language: "auto".to_owned(),
        }
    }
}

/// Window geometry.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct WindowSettings {
    /// Width in logical points.
    pub width: f32,
    /// Height in logical points.
    pub height: f32,
}

/// Editor view settings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct EditorSettings {
    /// Soft-wrap long lines.
    pub wrap: bool,
    /// Show the line-number gutter.
    pub line_numbers: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            wrap: false,
            line_numbers: true,
        }
    }
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            width: 980.0,
            height: 720.0,
        }
    }
}

impl Prefs {
    /// The saved theme mode.
    pub fn theme_mode(&self) -> ThemeMode {
        ThemeMode::from_setting(&self.appearance.theme)
    }

    /// The saved language preference.
    pub fn language_preference(&self) -> LanguagePreference {
        LanguagePreference::from_setting(&self.language.language)
    }

    /// Store the theme mode.
    pub fn set_theme_mode(&mut self, mode: ThemeMode) {
        self.appearance.theme = mode.as_setting().to_owned();
    }

    /// Store the language preference.
    pub fn set_language_preference(&mut self, preference: LanguagePreference) {
        self.language.language = preference.as_setting().to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_stable() {
        let prefs = Prefs::default();
        assert_eq!(prefs.theme_mode(), ThemeMode::Light);
        assert_eq!(prefs.language_preference(), LanguagePreference::Auto);
        assert!(prefs.window.width > 0.0);
        assert!(!prefs.editor.wrap);
        assert!(prefs.editor.line_numbers);
    }

    #[test]
    fn settings_round_trip() {
        let mut prefs = Prefs::default();
        prefs.set_theme_mode(ThemeMode::Dark);
        prefs.set_language_preference(LanguagePreference::Spanish);
        let text = toml::to_string(&prefs).expect("serialize prefs");
        let read: Prefs = toml::from_str(&text).expect("deserialize prefs");
        assert_eq!(read, prefs);
    }
}
