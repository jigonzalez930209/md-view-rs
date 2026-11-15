//! Small, shared size tokens for the workbench chrome.
//!
//! Keeping icon and control sizes in one place prevents the title bar, tabs, explorer,
//! and dialogs from drifting apart as the UI evolves.

/// Compact glyphs used inside dense menus and secondary controls.
pub const ICON_SMALL: f32 = 14.0;
/// Standard glyphs used by title-bar actions, tabs, and tree rows.
pub const ICON_MEDIUM: f32 = 16.0;
/// Prominent glyphs used by the product mark and primary actions.
pub const ICON_LARGE: f32 = 20.0;

/// Height of the document tab strip.
pub const TAB_HEIGHT: f32 = 34.0;
/// Height of the title bar.
pub const TITLE_HEIGHT: f32 = 34.0;
/// Height of a workspace tree row.
pub const TREE_ROW_HEIGHT: f32 = 21.0;
/// Standard square action button.
pub const ACTION_BUTTON: f32 = 28.0;

/// Square size for a title-bar window button cell.
pub const WINDOW_BUTTON_WIDTH: f32 = 44.0;
/// Height of a title-bar window button cell.
pub const WINDOW_BUTTON_HEIGHT: f32 = 32.0;
