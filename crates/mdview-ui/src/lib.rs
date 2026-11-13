//! UI composition for MD View.
//!
//! The shell owns presentation only. Commands are returned to the application layer, which
//! performs file I/O, settings updates, and lifecycle decisions.

mod about;
mod close_prompt;
mod menu;
mod status;
mod welcome;

pub use about::show_about;
pub use close_prompt::{CloseChoice, show_close_prompt};
pub use menu::show_menu;
pub use status::show_status;
pub use welcome::show_welcome;

use mdview_i18n::LanguagePreference;
use mdview_theme::ThemeMode;

/// A command requested by the shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// Create an empty document.
    New,
    /// Show the native open dialog.
    Open,
    /// Save the current document.
    Save,
    /// Show the native Save As dialog.
    SaveAs,
    /// Request application exit.
    Quit,
    /// Open the About window.
    About,
    /// Switch the base theme.
    SetTheme(ThemeMode),
    /// Switch the UI language.
    SetLanguage(LanguagePreference),
}
