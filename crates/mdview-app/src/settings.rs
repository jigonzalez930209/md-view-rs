//! Persisted application preferences.

use craft_settings::Settings;
use mdview_i18n::LanguagePreference;
use mdview_theme::ThemeId;
#[cfg(test)]
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
    /// Markdown reading style.
    pub reading: ReadingSettings,
    /// Media loading policy.
    pub media: MediaSettings,
    /// Update channel settings.
    pub updates: UpdateSettings,
}

impl Settings for Prefs {
    const APP_ID: &'static str = "mdview";
    const SCHEMA_VERSION: u32 = 1;
}

/// Update channel settings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(default)]
pub struct UpdateSettings {
    /// Manifest URL. Empty disables the manual check with a clear message.
    pub manifest_url: String,
    /// Allow checking for updates.
    pub enabled: bool,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            manifest_url: String::new(),
            enabled: true,
        }
    }
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
    /// Stable theme identifier (`github-light`, `dracula`, ...).
    pub theme: String,
    /// Follow the system light/dark preference within the active family.
    pub follow_system: bool,
    /// Text scale from 0.8 to 2.0.
    pub zoom: f32,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "github-light".to_owned(),
            follow_system: false,
            zoom: 1.0,
        }
    }
}

/// Markdown reading width and per-area text sizes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ReadingSettings {
    /// Left and right reading margin as a percentage of the pane, 5–40.
    pub margin_percent: f32,
    /// Body text size in points.
    pub body_size: f32,
    /// Base heading size in points.
    pub heading_size: f32,
    /// Code size in points.
    pub code_size: f32,
    /// Table size in points.
    pub table_size: f32,
    /// Equation size in points.
    pub math_size: f32,
}

impl Default for ReadingSettings {
    fn default() -> Self {
        Self {
            margin_percent: mdview_theme::ReadingMargins::DEFAULT_PERCENT,
            body_size: 14.0,
            heading_size: 28.0,
            code_size: 13.0,
            table_size: 13.0,
            math_size: 20.0,
        }
    }
}

impl ReadingSettings {
    /// The selected reading margins, clamped before use.
    pub fn margins(&self) -> mdview_theme::ReadingMargins {
        mdview_theme::ReadingMargins::new(self.margin_percent)
    }

    /// The selected text sizes, clamped before use.
    pub fn typography(&self) -> mdview_theme::ReadingTypography {
        mdview_theme::ReadingTypography {
            body: self.body_size,
            headings: self.heading_size,
            code: self.code_size,
            tables: self.table_size,
            math: self.math_size,
        }
        .sanitized()
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
    /// Editor monospace font size in points.
    pub font_size: f32,
    /// Editor line-height multiplier.
    pub line_height: f32,
    /// Allow the last line to reach the top of the viewport.
    pub scroll_beyond_last_line: bool,
    /// Draw indentation guides.
    pub indent_guides: bool,
    /// `none`, `selection`, or `all`.
    pub render_whitespace: String,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            wrap: false,
            line_numbers: true,
            sync_scroll: true,
            font_size: 13.0,
            line_height: 1.45,
            scroll_beyond_last_line: true,
            indent_guides: true,
            render_whitespace: "selection".to_owned(),
        }
    }
}

impl EditorSettings {
    /// The editor style used by every tab.
    pub fn style(&self) -> mdview_editor::EditorStyle {
        mdview_editor::EditorStyle {
            font_size: self.font_size,
            line_height: self.line_height,
            scroll_beyond_last_line: self.scroll_beyond_last_line,
            indent_guides: self.indent_guides,
            whitespace: mdview_editor::WhitespaceMode::from_setting(&self.render_whitespace),
        }
        .sanitized()
    }

    /// Store a style returned by the settings dialog.
    pub fn set_style(&mut self, style: mdview_editor::EditorStyle) {
        let style = style.sanitized();
        self.font_size = style.font_size;
        self.line_height = style.line_height;
        self.scroll_beyond_last_line = style.scroll_beyond_last_line;
        self.indent_guides = style.indent_guides;
        self.render_whitespace = style.whitespace.as_setting().to_owned();
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
    /// The saved theme identifier.
    pub fn theme_id(&self) -> ThemeId {
        ThemeId::from_setting(&self.appearance.theme)
    }

    /// Whether the active family follows the system mode.
    pub fn follow_system(&self) -> bool {
        self.appearance.follow_system
    }

    /// The saved theme mode.
    #[cfg(test)]
    pub fn theme_mode(&self) -> ThemeMode {
        self.theme_id().mode()
    }

    /// The saved language preference.
    pub fn language_preference(&self) -> LanguagePreference {
        LanguagePreference::from_setting(&self.language.language)
    }

    /// Store the theme identifier.
    pub fn set_theme_id(&mut self, id: ThemeId) {
        self.appearance.theme = id.as_setting().to_owned();
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
        assert_eq!(prefs.theme_id(), ThemeId::GithubLight);
        assert_eq!(prefs.theme_mode(), mdview_theme::ThemeMode::Light);
        assert!(!prefs.follow_system());
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
        prefs.set_theme_id(ThemeId::Dracula);
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
        assert_eq!(read.theme_id(), ThemeId::Dracula);
    }
}
