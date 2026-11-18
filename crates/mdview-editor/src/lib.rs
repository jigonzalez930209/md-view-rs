//! A virtualized code editor for MD View.
//!
//! The editor owns the document, selection, view settings, search state, and syntax cache. It paints
//! only the visible rows and handles keyboard, mouse, and IME input. The Markdown preview and the
//! application shell consume read-only status from this crate.

mod actions;
mod brackets;
mod controls;
mod find;
mod goto;
mod input;
mod layout;
mod md_format;
mod md_lists;
mod md_math;
mod md_syntax;
mod md_table;
mod motion;
mod paint;
mod render;
mod scroll;
mod style;
mod wrap;

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use mdview_model::{Document, Encoding, Eol, Indentation};
use mdview_syntax::Highlighter;

pub(crate) use layout::{cell_width, display_col, display_width};

pub use find::{FindState, SearchOptions};
pub use goto::GotoState;
pub use md_format::{MarkdownAction, MarkdownState};
pub use md_math::MathTarget;
pub use style::{BOTTOM_CLEARANCE, EditorStyle, WhitespaceMode};

/// The visible editor status for the shell's status bar.
#[derive(Clone, Debug)]
pub struct EditorStatus {
    /// Cursor line, zero-based.
    pub line: usize,
    /// Cursor column, zero-based.
    pub column: usize,
    /// Number of selected characters.
    pub selection_chars: usize,
    /// Detected language name.
    pub language: String,
    /// File encoding.
    pub encoding: Encoding,
    /// Line-ending style.
    pub eol: Eol,
    /// Whether large-file mode is active.
    pub large_mode: bool,
    /// Whether soft wrap is enabled.
    pub wrap: bool,
    /// Whether line numbers are enabled.
    pub line_numbers: bool,
    /// Detected indentation convention.
    pub indentation: Indentation,
}

/// The code editor widget and its document.
pub struct Editor {
    pub(crate) document: Document,
    pub(crate) cursor: usize,
    pub(crate) anchor: usize,
    pub(crate) wrap: bool,
    pub(crate) line_numbers: bool,
    pub(crate) style: EditorStyle,
    pub(crate) light: bool,
    pub(crate) large_mode: bool,
    pub(crate) highlighter: Arc<Highlighter>,
    pub(crate) highlight_cache: mdview_syntax::HighlightCache,
    pub(crate) highlight_revision: Option<u64>,
    pub(crate) language: String,
    pub(crate) indentation: Indentation,
    pub(crate) display_widths: Vec<usize>,
    pub(crate) layout_revision: Option<u64>,
    pub(crate) bracket_match: Option<(usize, usize)>,
    pub(crate) bracket_cursor: Option<usize>,
    pub(crate) bracket_revision: Option<u64>,
    pub(crate) find: FindState,
    pub(crate) goto: GotoState,
    pub(crate) scroll_to_cursor: bool,
    pub(crate) cursor_blink_start: Instant,
    pub(crate) last_click: Option<(Instant, egui::Pos2, u8)>,
    pub(crate) announced_selection: Option<(usize, usize)>,
    pub(crate) identity: u64,
    pub(crate) scroll_y: f32,
    pub(crate) restore_scroll: Option<f32>,
    pub(crate) pending_line: Option<(usize, f32)>,
    pub(crate) pending_end: bool,
    pub(crate) last_max_offset: f32,
    pub(crate) last_bottom_clearance: f32,
    pub(crate) last_last_row_top: f32,
    pub(crate) last_scroll_at: Instant,
    pub(crate) last_row_height: f32,
    pub(crate) last_columns: usize,
    pub(crate) user_scrolled: bool,
    pub(crate) programmatic_scroll: bool,
    pub(crate) viewport_rect: egui::Rect,
    pub(crate) last_frame_time: std::time::Duration,
    pub(crate) highlight_enabled: bool,
    pub(crate) split_mode: bool,
    pub(crate) focus_requested: bool,
    pub(crate) ignore_input: bool,
    pub(crate) overshoot_paints: u64,
    /// Display column kept by Up and Down across shorter lines.
    pub(crate) preferred_column: Option<usize>,
    pub(crate) scroll_x: f32,
    /// Reveal the cursor in the middle of the viewport if it is off screen.
    pub(crate) center_cursor: bool,
    pub(crate) language_key: Option<(u64, Option<std::path::PathBuf>)>,
    /// Measured character advance keyed by font size and pixels per point.
    pub(crate) char_advance_cache: Option<((f32, f32), f32)>,
    /// Soft-wrap row breaks per line, for `wrap_key` (revision, columns).
    pub(crate) wrap_breaks: Vec<Vec<usize>>,
    pub(crate) wrap_key: Option<(u64, usize)>,
    /// Whether the text area had keyboard focus in the last frame.
    pub(crate) focused: bool,
    /// Full layout rebuilds, for tests that the incremental path is used.
    pub(crate) layout_rebuilds: u64,
    /// A formula double-clicked since the application last asked.
    pub(crate) math_request: Option<MathTarget>,
    /// The character under the pointer in the last frame.
    pub(crate) hovered_char: Option<usize>,
    /// A range tinted for the formula editor's source-to-preview hover.
    pub(crate) marked: Option<md_math::MarkedRange>,
}

impl Editor {
    /// Build an editor around a document.
    pub fn new(document: Document) -> Self {
        let large_mode = document.len_bytes() > 10 * 1024 * 1024 || document.line_count() > 200_000;
        let indentation = document.detect_indentation();
        let mut editor = Self {
            document,
            cursor: 0,
            anchor: 0,
            wrap: false,
            line_numbers: true,
            style: EditorStyle::default(),
            light: true,
            large_mode,
            highlighter: Highlighter::shared(),
            highlight_cache: mdview_syntax::HighlightCache::default(),
            highlight_revision: None,
            language: "Plain Text".to_owned(),
            indentation,
            display_widths: Vec::new(),
            layout_revision: None,
            bracket_match: None,
            bracket_cursor: None,
            bracket_revision: None,
            find: FindState::default(),
            goto: GotoState::default(),
            scroll_to_cursor: false,
            cursor_blink_start: Instant::now(),
            last_click: None,
            announced_selection: None,
            identity: 0,
            scroll_y: 0.0,
            restore_scroll: None,
            pending_line: None,
            pending_end: false,
            last_max_offset: 0.0,
            last_bottom_clearance: BOTTOM_CLEARANCE,
            last_last_row_top: 0.0,
            last_scroll_at: Instant::now(),
            last_row_height: 17.0,
            last_columns: 0,
            user_scrolled: false,
            programmatic_scroll: false,
            viewport_rect: egui::Rect::NOTHING,
            last_frame_time: std::time::Duration::ZERO,
            highlight_enabled: true,
            split_mode: false,
            focus_requested: false,
            ignore_input: false,
            overshoot_paints: 0,
            preferred_column: None,
            scroll_x: 0.0,
            center_cursor: false,
            language_key: None,
            char_advance_cache: None,
            wrap_breaks: Vec::new(),
            wrap_key: None,
            focused: false,
            layout_rebuilds: 0,
            math_request: None,
            hovered_char: None,
            marked: None,
        };
        editor.highlight_enabled = editor.highlight_budget_allows();
        editor.ensure_layout();
        editor.ensure_language();
        editor
    }

    /// The document, for saving and status.
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Mutable document metadata access for the application layer.
    ///
    /// Text edits must go through the editor so selection and syntax state stay coherent.
    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    /// The cursor as a character index.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Move the cursor and collapse the selection.
    pub fn move_cursor_to(&mut self, index: usize) {
        self.cursor = index.min(self.document.len_chars());
        self.anchor = self.cursor;
        self.preferred_column = None;
        self.scroll_to_cursor = true;
        self.center_cursor = true;
    }

    /// Replace the document and reset view state.
    pub fn replace_document(&mut self, document: Document) {
        self.document = document;
        self.cursor = 0;
        self.anchor = 0;
        self.large_mode = self.document.len_bytes() > 10 * 1024 * 1024 || self.document.line_count() > 200_000;
        self.indentation = self.document.detect_indentation();
        if self.large_mode {
            self.wrap = false;
        }
        self.find.reset();
        self.goto.reset();
        self.preferred_column = None;
        self.highlight_cache.reset();
        self.language_key = None;
        self.highlight_revision = None;
        self.layout_revision = None;
        self.bracket_match = None;
        self.bracket_cursor = None;
        self.bracket_revision = None;
        self.highlight_enabled = self.highlight_budget_allows();
        self.ensure_layout();
        self.ensure_language();
    }

    /// Replace the document while keeping the cursor on the same line and column.
    ///
    /// External reloads use this so a file change does not jump the user to the start.
    pub fn replace_document_preserving_position(&mut self, document: Document) {
        let (line, column) = self.document.line_col(self.cursor);
        self.replace_document(document);
        let index = self.document.char_index(line, column);
        self.cursor = index;
        self.anchor = index;
        self.scroll_to_cursor = false;
    }

    /// Update the theme mode; highlighting is refreshed on the next frame.
    pub fn set_light(&mut self, light: bool) {
        if self.light != light {
            self.light = light;
            self.highlight_cache.reset();
            self.highlight_revision = None;
        }
    }

    /// Enable or disable soft wrap.
    pub fn set_wrap(&mut self, wrap: bool) {
        self.wrap = wrap;
        self.layout_revision = None;
    }

    /// Toggle soft wrap.
    pub fn toggle_wrap(&mut self) {
        self.wrap = !self.wrap;
        self.layout_revision = None;
    }

    /// Enable or disable line numbers.
    pub fn set_line_numbers(&mut self, line_numbers: bool) {
        self.line_numbers = line_numbers;
    }

    /// Replace the visual and scrolling style.
    pub fn set_style(&mut self, style: EditorStyle) {
        let style = style.sanitized();
        if self.style != style {
            self.style = style;
            self.layout_revision = None;
        }
    }

    /// The active editor style.
    pub fn style(&self) -> EditorStyle {
        self.style
    }

    /// Toggle line numbers.
    pub fn toggle_line_numbers(&mut self) {
        self.line_numbers = !self.line_numbers;
    }

    /// Force large-file mode. The application uses the on-disk size before loading.
    pub fn set_large_mode(&mut self, large: bool) {
        self.large_mode = large;
        // Large mode measures lines differently (characters instead of cells).
        self.layout_revision = None;
        self.highlight_enabled = !large && self.highlight_budget_allows();
        if large {
            self.wrap = false;
            self.highlight_cache.reset();
            self.highlight_revision = None;
            self.ensure_language();
        }
    }

    /// The current editor status.
    pub fn status(&self) -> EditorStatus {
        let (line, column) = self.document.line_col(self.cursor);
        let selection_chars = self.cursor.abs_diff(self.anchor);
        EditorStatus {
            line,
            column,
            selection_chars,
            language: self.language.clone(),
            encoding: self.document.encoding(),
            eol: self.document.eol(),
            large_mode: self.large_mode,
            wrap: self.wrap,
            line_numbers: self.line_numbers,
            indentation: self.indentation,
        }
    }

    /// Whether the text area had keyboard focus in the last frame.
    pub fn has_focus(&self) -> bool {
        self.focused
    }

    /// Open the find bar.
    pub fn open_find(&mut self, replace: bool) {
        self.find.open = true;
        self.find.replace = replace;
        self.find.focus_query = true;
        self.focus_requested = false;
    }

    /// Open the go-to-line dialog.
    pub fn open_goto(&mut self) {
        self.goto.open = true;
        self.goto.focus = true;
        self.focus_requested = false;
    }

    /// Select a character range.
    pub fn select(&mut self, start: usize, end: usize) {
        self.anchor = start.min(self.document.len_chars());
        self.cursor = end.min(self.document.len_chars());
        self.preferred_column = None;
        self.scroll_to_cursor = true;
        self.center_cursor = true;
    }

    /// Move the cursor to the start of a one-based line.
    pub fn goto_line(&mut self, line: usize) {
        let index = line.saturating_sub(1).min(self.document.line_count().saturating_sub(1));
        self.cursor = self.document.line_start_char(index);
        self.anchor = self.cursor;
        self.preferred_column = None;
        self.scroll_to_cursor = true;
        self.center_cursor = true;
        self.document.break_undo_group();
    }

    pub(crate) fn selection(&self) -> (usize, usize) {
        if self.anchor <= self.cursor {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }

    fn highlight_budget_allows(&self) -> bool {
        self.document.line_count() <= 2_000 && self.document.len_bytes() <= 1024 * 1024
    }

    pub(crate) fn ensure_language(&mut self) {
        let key = (self.document.revision(), self.document.path().map(Path::to_path_buf));
        if self.language_key.as_ref() == Some(&key) {
            return;
        }
        self.language_key = Some(key);
        let prefix = self.document.slice(0, self.document.len_chars().min(4096));
        self.language = self.highlighter.language_name(self.document.path(), &prefix);
    }

    pub(crate) fn ensure_highlight_range(&mut self, start: usize, end: usize) {
        if !self.highlight_enabled {
            return;
        }
        let revision = self.document.revision();
        if self.highlight_revision != Some(revision) {
            // Invalidate from the first line the edits touched, not from the cursor: Enter
            // in the middle of a line changes the line above the new cursor, and undo can
            // change text anywhere.
            if let Some(line) = self.document.take_changed_line() {
                self.highlight_cache.invalidate_from(line);
            }
            self.highlight_revision = Some(revision);
            self.ensure_language();
        }
        if self.large_mode || self.highlight_cache.covers(start, end) {
            return;
        }
        let text = self.document.snapshot();
        let path = self.document.path().map(|path| path.to_path_buf());
        self.highlighter.ensure_range(
            &text,
            path.as_deref(),
            self.light,
            start,
            end,
            &mut self.highlight_cache,
        );
    }
}

#[cfg(test)]
mod editor_tests;

#[cfg(test)]
mod scale_tests {
    use std::path::PathBuf;

    use mdview_model::{Document, Encoding, Eol};

    use super::Editor;

    #[test]
    fn handles_ten_thousand_lines_without_panicking() {
        let text = (0..10_000)
            .map(|index| format!("fn line_{index}() {{}}\n"))
            .collect::<String>();
        let document = Document::from_loaded(PathBuf::from("large.rs"), text, Eol::Lf, Encoding::Utf8, false);
        let mut editor = Editor::new(document);
        assert_eq!(editor.document().line_count(), 10_001);
        editor.set_wrap(true);
        editor.ensure_layout();
        editor.set_wrap(false);
        assert!(!editor.status().large_mode);
    }
}

#[cfg(test)]
mod typing_tests;

#[cfg(test)]
mod markdown_tests;

#[cfg(test)]
mod feature_tests;

#[cfg(test)]
mod md_math_tests;
