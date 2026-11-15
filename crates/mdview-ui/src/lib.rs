//! UI composition for MD View.
//!
//! The shell owns presentation only. Commands are returned to the application layer, which
//! performs file I/O, settings updates, and lifecycle decisions.

mod about;
mod close_prompt;
mod controls;
mod header;
mod keyboard_menu;
mod loading;
mod material_icons;
mod menu;
mod metrics;
mod mode;
mod palette;
mod prompt;
mod search;
mod settings;
mod status;
mod welcome;

pub use about::show_about;
pub use close_prompt::{CloseChoice, show_close_prompt};
pub use controls::{
    CONTROL_HEIGHT, compact_menu, menu_item, numeric_stepper, primary_button, secondary_button, text_field,
};
pub use header::{TitleBarState, show_title_bar};
pub use keyboard_menu::{KeyboardMenuState, MenuEntry, MenuGroup, show_keyboard_menu};
pub use loading::show_loading;
pub use material_icons::MaterialIcons;
pub use menu::{MenuState, show_menu};
pub use metrics::{ACTION_BUTTON, ICON_LARGE, ICON_MEDIUM, ICON_SMALL, TAB_HEIGHT, TITLE_HEIGHT, TREE_ROW_HEIGHT};
pub use mode::ViewMode;
pub use palette::{PaletteItem, show_palette};
pub use prompt::{ConfirmPrompt, NamePrompt, PromptAction, show_confirm_prompt, show_name_prompt};
pub use search::{SearchAction, SearchHitView, SearchScope, SearchView, show_search};
pub use settings::{SettingsView, show_settings};
pub use status::show_status;
pub use welcome::show_welcome;

use mdview_i18n::LanguagePreference;
use mdview_model::Encoding;
use mdview_theme::ThemeId;

/// A command requested by the shell.
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    /// Create an empty document.
    New,
    /// Show the native open dialog.
    Open,
    /// Show the native folder dialog.
    OpenFolder,
    /// Close the workspace folder.
    CloseFolder,
    /// Create a file in the workspace root.
    NewFile,
    /// Open a recent file by index.
    OpenRecent(usize),
    /// Open a path directly.
    OpenPath(std::path::PathBuf),
    /// Open a recent workspace by index.
    OpenRecentWorkspace(usize),
    /// Clear the recent-file list.
    ClearRecent,
    /// Close the active tab.
    CloseTab,
    /// Close every clean tab except the active one.
    CloseOtherTabs,
    /// Close every clean tab.
    CloseSavedTabs,
    /// Move the active tab one position left.
    MoveTabLeft,
    /// Move the active tab one position right.
    MoveTabRight,
    /// Activate the next tab.
    NextTab,
    /// Activate the previous tab.
    PreviousTab,
    /// Show or hide the workspace sidebar.
    ToggleSidebar,
    /// Save the current document.
    Save,
    /// Show the native Save As dialog.
    SaveAs,
    /// Rename the active file.
    RenameFile,
    /// Delete the active file.
    DeleteFile,
    /// Request application exit.
    Quit,
    /// Open the About window.
    About,
    /// Open the find bar.
    Find,
    /// Open the find/replace bar.
    Replace,
    /// Open the go-to-line dialog.
    GotoLine,
    /// Toggle soft wrap.
    ToggleWrap,
    /// Toggle line numbers.
    ToggleLineNumbers,
    /// Switch the document presentation mode.
    SetMode(ViewMode),
    /// Toggle split scroll synchronization.
    ToggleSyncScroll,
    /// Show or hide dotfiles in the explorer.
    ToggleShowHidden,
    /// Respect .gitignore while listing the workspace.
    ToggleRespectGitignore,
    /// Toggle remote image loading.
    ToggleRemoteMedia,
    /// Toggle private-network image loading.
    TogglePrivateNetworks,
    /// Toggle SVG animation playback.
    ToggleAnimations,
    /// Toggle reduced motion.
    ToggleReduceMotion,
    /// Select the Markdown reading width preset.
    SetReadingMargins(mdview_theme::ReadingMargins),
    /// Set the per-area Markdown reading text sizes.
    SetReadingTypography(mdview_theme::ReadingTypography),
    /// Apply a complete editor style.
    SetEditorStyle(mdview_editor::EditorStyle),
    /// Reload the current file with another encoding.
    ReloadEncoding(Encoding),
    /// Switch the theme variant.
    SetThemeId(ThemeId),
    /// Follow the system light/dark preference within the active family.
    SetFollowSystem(bool),
    /// Switch the UI language.
    SetLanguage(LanguagePreference),
    /// Set the text scale.
    SetZoom(f32),
    /// Activate a tab by index.
    ActivateTab(usize),
    /// Open the settings window.
    Settings,
    /// Open the command palette.
    CommandPalette,
    /// Open the quick-open file palette.
    QuickOpen,
    /// Check the configured update manifest.
    CheckUpdates,
    /// Open the diagnostics log folder.
    OpenLogs,
    /// Minimize the window.
    MinimizeWindow,
    /// Toggle the maximized state.
    ToggleMaximizeWindow,
    /// Request application exit through the normal close path.
    CloseWindow,
    /// Navigate to the previous document or anchor.
    Back,
    /// Navigate to the next document or anchor.
    Forward,
    /// Open the search popover in document scope.
    Search,
    /// Open the search popover in workspace scope.
    SearchWorkspace,
}
