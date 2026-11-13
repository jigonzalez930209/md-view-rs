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
    /// Restored session and recent files.
    pub session: SessionSettings,
    /// Media loading policy.
    pub media: MediaSettings,
}

impl Settings for Prefs {
    const APP_ID: &'static str = "mdview";
    const SCHEMA_VERSION: u32 = 1;
}

/// Media loading policy.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct MediaSettings {
    /// Fetch remote images referenced by documents.
    pub remote_images: bool,
    /// Allow URLs that resolve to private network addresses.
    pub private_networks: bool,
    /// Play animated SVG content.
    pub animate_svg: bool,
    /// Show static frames instead of motion.
    pub reduce_motion: bool,
}

impl Default for MediaSettings {
    fn default() -> Self {
        Self {
            remote_images: true,
            private_networks: false,
            animate_svg: true,
            reduce_motion: false,
        }
    }
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
    /// Synchronize editor and preview scrolling in split view.
    pub sync_scroll: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            wrap: false,
            line_numbers: true,
            sync_scroll: true,
        }
    }
}

/// The last session and recent-file list.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct SessionSettings {
    /// Absolute paths of open file tabs, in tab order.
    pub open_files: Vec<String>,
    /// The path of the active tab, when it has one.
    pub active_file: Option<String>,
    /// The open workspace folder, when any.
    pub workspace: Option<String>,
    /// Whether the sidebar is visible.
    pub sidebar_open: bool,
    /// Sidebar width in logical points.
    pub sidebar_width: f32,
    /// Most recent file paths, newest first.
    pub recent_files: Vec<String>,
    /// Most recent workspace folders, newest first.
    pub recent_workspaces: Vec<String>,
    /// Per-file vertical scroll offsets for the editor.
    pub tab_scroll: std::collections::BTreeMap<String, f32>,
    /// Per-file vertical scroll offsets for the reading view.
    pub tab_view_scroll: std::collections::BTreeMap<String, f32>,
    /// Per-file presentation mode (`edit`, `view`, or `split`).
    pub tab_modes: std::collections::BTreeMap<String, String>,
    /// Per-file split ratio; the fraction used by the preview pane.
    pub tab_split_ratio: std::collections::BTreeMap<String, f32>,
    /// Whether the workspace tree shows dotfiles.
    pub show_hidden: bool,
    /// Whether the workspace tree respects `.gitignore`.
    pub respect_gitignore: bool,
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            open_files: Vec::new(),
            active_file: None,
            workspace: None,
            sidebar_open: true,
            sidebar_width: 260.0,
            recent_files: Vec::new(),
            recent_workspaces: Vec::new(),
            tab_scroll: std::collections::BTreeMap::new(),
            tab_view_scroll: std::collections::BTreeMap::new(),
            tab_modes: std::collections::BTreeMap::new(),
            tab_split_ratio: std::collections::BTreeMap::new(),
            show_hidden: false,
            respect_gitignore: true,
        }
    }
}

impl SessionSettings {
    /// Record a recently opened path, keeping at most ten entries.
    pub fn remember_recent(&mut self, path: &str) {
        remember(&mut self.recent_files, path);
    }

    /// Record a recently opened workspace folder.
    pub fn remember_recent_workspace(&mut self, path: &str) {
        remember(&mut self.recent_workspaces, path);
    }
}

fn remember(list: &mut Vec<String>, value: &str) {
    list.retain(|entry| entry != value);
    list.insert(0, value.to_owned());
    list.truncate(10);
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
        assert!(prefs.editor.sync_scroll);
        assert!(prefs.media.remote_images);
        assert!(!prefs.media.private_networks);
        assert!(prefs.session.sidebar_open);
        assert!(prefs.session.open_files.is_empty());
        assert_eq!(prefs.session.sidebar_width, 260.0);
    }

    #[test]
    fn twenty_tab_session_round_trips() {
        let mut prefs = Prefs::default();
        prefs.session.workspace = Some("/tmp/project".to_owned());
        for index in 0..20 {
            let path = format!("/tmp/project/note-{index}.md");
            prefs.session.open_files.push(path.clone());
            prefs.session.tab_scroll.insert(path, index as f32 * 10.0);
        }
        prefs.session.active_file = Some("/tmp/project/note-19.md".to_owned());
        prefs.session.sidebar_width = 321.0;
        let text = toml::to_string(&prefs).expect("serialize prefs");
        let read: Prefs = toml::from_str(&text).expect("deserialize prefs");
        assert_eq!(read.session.open_files.len(), 20);
        assert_eq!(read.session.tab_scroll.len(), 20);
        assert_eq!(read.session.tab_scroll["/tmp/project/note-7.md"], 70.0);
        assert_eq!(read.session.active_file.as_deref(), Some("/tmp/project/note-19.md"));
        assert_eq!(read.session.workspace.as_deref(), Some("/tmp/project"));
        assert_eq!(read.session.sidebar_width, 321.0);
    }

    #[test]
    fn settings_round_trip() {
        let mut prefs = Prefs::default();
        prefs.set_theme_mode(ThemeMode::Dark);
        prefs.set_language_preference(LanguagePreference::Spanish);
        prefs.session.remember_recent("/tmp/a.md");
        prefs.session.remember_recent("/tmp/b.md");
        prefs.session.remember_recent("/tmp/a.md");
        assert_eq!(prefs.session.recent_files, ["/tmp/a.md", "/tmp/b.md"]);
        prefs.session.remember_recent_workspace("/tmp/project");
        prefs.session.tab_scroll.insert("/tmp/a.md".to_owned(), 42.0);
        prefs.session.tab_view_scroll.insert("/tmp/a.md".to_owned(), 84.0);
        prefs
            .session
            .tab_modes
            .insert("/tmp/a.md".to_owned(), "split".to_owned());
        let text = toml::to_string(&prefs).expect("serialize prefs");
        let read: Prefs = toml::from_str(&text).expect("deserialize prefs");
        assert_eq!(read, prefs);
    }
}
