//! Find and replace.

use egui::Key;
use regex::RegexBuilder;

use crate::Editor;

/// Search flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SearchOptions {
    /// Treat the query as a regular expression.
    pub regex: bool,
    /// Match case.
    pub case_sensitive: bool,
    /// Match whole words.
    pub whole_word: bool,
}

/// Find/replace UI state.
#[derive(Clone, Debug, Default)]
pub struct FindState {
    /// Whether the bar is visible.
    pub open: bool,
    /// Whether the replacement row is visible.
    pub replace: bool,
    /// Search query.
    pub query: String,
    /// Replacement text.
    pub replacement: String,
    /// Search flags.
    pub options: SearchOptions,
    /// Character ranges of matches.
    pub matches: Vec<(usize, usize)>,
    /// Request focus for the query field.
    pub focus_query: bool,
    /// Request focus for the replacement field.
    pub focus_replace: bool,
}

impl FindState {
    /// Hide the bar and clear transient state.
    pub fn reset(&mut self) {
        self.open = false;
        self.replace = false;
        self.query.clear();
        self.replacement.clear();
        self.matches.clear();
        self.focus_query = false;
        self.focus_replace = false;
    }
}

impl Editor {
    /// Draw the floating find/replace overlay inside the editor pane.
    pub(crate) fn show_find_overlay(&mut self, ui: &mut egui::Ui, anchor: egui::Rect) {
        if !self.find.open {
            return;
        }
        let mut close = false;
        let mut changed = false;
        let anchor = if anchor == egui::Rect::NOTHING {
            ui.max_rect()
        } else {
            anchor
        };
        let width = (anchor.width() - 16.0).clamp(300.0, 460.0);
        let position = egui::pos2(
            (anchor.right() - width - 8.0).max(anchor.left() + 8.0),
            anchor.top() + 6.0,
        );
        egui::Area::new(egui::Id::unique(("mdview_find_overlay", self.identity)))
            .order(egui::Order::Foreground)
            .fixed_pos(position)
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(ui.visuals().window_fill)
                    .stroke(egui::Stroke::new(1.0, ui.visuals().window_stroke.color))
                    .corner_radius(7)
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 5],
                        blur: 14,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(42),
                    })
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_width(width - 18.0);
                        ui.horizontal(|ui| {
                            // Every control has a fixed width so the row is exactly the panel
                            // width: a wider row pushed the panel left (areas stay on screen),
                            // and the controls moved while typing as the result count changed.
                            ui.spacing_mut().item_spacing.x = 4.0;
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new(if self.find.replace { "v" } else { ">" }))
                                        .frame(false),
                                )
                                .on_hover_text("Toggle replace")
                                .clicked()
                            {
                                self.find.replace = !self.find.replace;
                            }
                            let query = crate::controls::text_field(
                                ui,
                                &mut self.find.query,
                                "Find",
                                (width - 18.0 - FIND_ROW_FIXED).max(100.0),
                            );
                            if self.find.focus_query {
                                query.request_focus();
                                self.find.focus_query = false;
                            }
                            if query.changed() {
                                changed = true;
                            }
                            let f3 = query.has_focus() && ui.input(|input| input.key_pressed(Key::F3));
                            if crate::controls::submitted(ui, &query) || f3 {
                                if ui.input(|input| input.modifiers.shift) {
                                    self.find_previous();
                                } else {
                                    self.find_next();
                                }
                                // Keep typing in the find field: the next Enter finds the next
                                // match instead of inserting a line break in the document.
                                query.request_focus();
                            }
                            if toggle_chip(ui, "Aa", self.find.options.case_sensitive)
                                .on_hover_text("Match case")
                                .clicked()
                            {
                                self.find.options.case_sensitive = !self.find.options.case_sensitive;
                                changed = true;
                            }
                            if toggle_chip(ui, "ab", self.find.options.whole_word)
                                .on_hover_text("Whole word")
                                .clicked()
                            {
                                self.find.options.whole_word = !self.find.options.whole_word;
                                changed = true;
                            }
                            if toggle_chip(ui, ".*", self.find.options.regex)
                                .on_hover_text("Use regular expression")
                                .clicked()
                            {
                                self.find.options.regex = !self.find.options.regex;
                                changed = true;
                            }
                            let (current, total) = self.match_position();
                            ui.add_sized(
                                [RESULT_LABEL_WIDTH, 21.0],
                                egui::Label::new(
                                    egui::RichText::new(if total == 0 {
                                        "No results".to_owned()
                                    } else if total == 1 {
                                        "1 result".to_owned()
                                    } else if current == 0 {
                                        format!("{total} results")
                                    } else {
                                        format!("{current} of {total}")
                                    })
                                    .small()
                                    .weak(),
                                )
                                .truncate(),
                            );
                            if small_icon_button(ui, "<", "Previous match").clicked() {
                                self.find_previous();
                            }
                            if small_icon_button(ui, ">", "Next match").clicked() {
                                self.find_next();
                            }
                            if small_icon_button(ui, "×", "Close").clicked() {
                                close = true;
                            }
                        });
                        if self.find.replace {
                            ui.add_space(3.0);
                            ui.horizontal(|ui| {
                                let replacement =
                                    crate::controls::text_field(ui, &mut self.find.replacement, "Replace with", 230.0);
                                if self.find.focus_replace {
                                    replacement.request_focus();
                                    self.find.focus_replace = false;
                                }
                                if replacement.changed() {
                                    changed = true;
                                }
                                if crate::controls::submitted(ui, &replacement) {
                                    // Enter replaces the current match and moves to the next.
                                    self.replace_current();
                                    replacement.request_focus();
                                }
                                if small_text_button(ui, "Replace").clicked() {
                                    self.replace_current();
                                }
                                if small_text_button(ui, "All").clicked() {
                                    self.replace_all();
                                }
                            });
                        }
                    });
            });
        if ui.input(|input| input.key_pressed(Key::Escape)) {
            close = true;
        }
        if changed {
            self.recompute_matches();
        }
        if close {
            self.find.open = false;
            self.focus_requested = true;
        }
    }

    fn match_position(&self) -> (usize, usize) {
        let total = self.find.matches.len();
        if total == 0 {
            return (0, 0);
        }
        let selection = self.selection();
        let current = self
            .find
            .matches
            .iter()
            .position(|(start, end)| *start == selection.0 && *end == selection.1)
            .map_or(0, |index| index + 1);
        (current, total)
    }

    /// The compiled search, or `None` for an empty query or an invalid pattern.
    fn search_regex(&self) -> Option<regex::Regex> {
        if self.find.query.is_empty() {
            return None;
        }
        let pattern = if self.find.options.regex {
            self.find.query.clone()
        } else {
            regex::escape(&self.find.query)
        };
        let pattern = if self.find.options.whole_word {
            format!(r"\b(?:{pattern})\b")
        } else {
            pattern
        };
        RegexBuilder::new(&pattern)
            .case_insensitive(!self.find.options.case_sensitive)
            // `^` and `$` match at line starts and ends, as in other editors.
            .multi_line(true)
            .size_limit(10 * 1024 * 1024)
            .build()
            .ok()
    }

    /// Recompute matches from the current query and document.
    pub(crate) fn recompute_matches(&mut self) {
        self.find.matches.clear();
        let Some(regex) = self.search_regex() else {
            return;
        };
        let text = self.document.snapshot();
        self.find.matches = regex
            .find_iter(&text)
            // Empty matches (`a*`, `^`) cannot be selected or stepped through.
            .filter(|found| !found.is_empty())
            .map(|found| {
                (
                    self.document.byte_to_char(found.start()),
                    self.document.byte_to_char(found.end()),
                )
            })
            .collect();
    }

    /// The replacement for the match at `[start, end)`: `$1`/`${name}` expand in regex mode.
    fn replacement_for(&self, regex: &regex::Regex, text: &str, start: usize) -> String {
        if !self.find.options.regex {
            return self.find.replacement.clone();
        }
        let byte = self.document.char_to_byte(start);
        regex.captures_at(text, byte).map_or_else(
            || self.find.replacement.clone(),
            |captures| {
                let mut expanded = String::new();
                captures.expand(&self.find.replacement, &mut expanded);
                expanded
            },
        )
    }

    /// Select the next match after the cursor.
    pub(crate) fn find_next(&mut self) {
        self.recompute_matches();
        if self.find.matches.is_empty() {
            return;
        }
        let found = self
            .find
            .matches
            .iter()
            .copied()
            .find(|(start, _)| *start >= self.cursor)
            .or_else(|| self.find.matches.first().copied());
        if let Some((start, end)) = found {
            self.select(start, end);
        }
    }

    /// Select the previous match before the cursor.
    pub(crate) fn find_previous(&mut self) {
        self.recompute_matches();
        if self.find.matches.is_empty() {
            return;
        }
        let found = self
            .find
            .matches
            .iter()
            .copied()
            .rev()
            .find(|(start, _)| *start < self.cursor)
            .or_else(|| self.find.matches.last().copied());
        if let Some((start, end)) = found {
            self.select(start, end);
        }
    }

    fn replace_current(&mut self) {
        self.recompute_matches();
        let Some((start, end)) = self
            .find
            .matches
            .iter()
            .copied()
            .find(|(start, end)| *start == self.anchor.min(self.cursor) && *end == self.anchor.max(self.cursor))
        else {
            self.find_next();
            return;
        };
        let replacement = match self.search_regex() {
            Some(regex) => self.replacement_for(&regex, &self.document.snapshot(), start),
            None => self.find.replacement.clone(),
        };
        self.document.replace_range(start, end, &replacement);
        self.recompute_matches();
        self.cursor = start + replacement.chars().count();
        self.anchor = self.cursor;
        // Select the next match so repeated Replace walks through the document.
        if !self.find.matches.is_empty() {
            self.find_next();
        }
    }

    pub(crate) fn replace_all(&mut self) {
        self.recompute_matches();
        if self.find.matches.is_empty() {
            return;
        }
        let Some(regex) = self.search_regex() else {
            return;
        };
        let text = self.document.snapshot();
        let edits: Vec<(usize, usize, String)> = self
            .find
            .matches
            .iter()
            .map(|(start, end)| (*start, *end, self.replacement_for(&regex, &text, *start)))
            .collect();
        self.document.replace_ranges(&edits);
        self.recompute_matches();
    }
}

/// Width of the match counter ("12 of 345"), fixed so the row never changes width.
const RESULT_LABEL_WIDTH: f32 = 64.0;
/// Width of everything in the find row except the query field: the replace toggle, three
/// option chips, the counter, three buttons, and the gaps between them.
const FIND_ROW_FIXED: f32 = 20.0 + 3.0 * 23.0 + RESULT_LABEL_WIDTH + 3.0 * 24.0 + 9.0 * 4.0;

fn toggle_chip(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(23.0, 21.0), egui::Sense::click());
    let fill = if active {
        ui.visuals().selection.bg_fill
    } else if response.hovered() {
        ui.visuals().widgets.hovered.weak_bg_fill
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(4), fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::monospace(10.5),
        if active {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().text_color()
        },
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn small_icon_button(ui: &mut egui::Ui, label: &str, tooltip: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(4),
            ui.visuals().widgets.hovered.weak_bg_fill,
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(14.0),
        ui.visuals().text_color(),
    );
    response
        .on_hover_text(tooltip)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn small_text_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(egui::Button::new(egui::RichText::new(label).size(11.5)).min_size(egui::vec2(54.0, 24.0)))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use mdview_model::{Document, Encoding, Eol};

    use crate::Editor;

    fn editor(text: &str) -> Editor {
        Editor::new(Document::from_loaded(
            PathBuf::from("fixture.txt"),
            text.to_owned(),
            Eol::Lf,
            Encoding::Utf8,
            false,
        ))
    }

    #[test]
    fn finds_literal_matches_and_replaces_all() {
        let mut editor = editor("one two one\n");
        editor.find.open = true;
        editor.find.query = "one".to_owned();
        editor.recompute_matches();
        assert_eq!(editor.find.matches.len(), 2);
        editor.replace_all();
        assert_eq!(editor.document().snapshot(), " two \n");
    }

    #[test]
    fn finds_regex_and_case_insensitive() {
        let mut editor = editor("Item 12 and item 34\n");
        editor.find.open = true;
        editor.find.query = r"item \d+".to_owned();
        editor.find.options.regex = true;
        editor.find.options.case_sensitive = false;
        editor.recompute_matches();
        assert_eq!(editor.find.matches.len(), 2);
        editor.find_next();
        assert_eq!(editor.document().slice(editor.anchor, editor.cursor), "Item 12");
    }
}

#[cfg(test)]
mod transaction_tests {
    use std::path::PathBuf;

    use mdview_model::{Document, Encoding, Eol};

    use crate::Editor;

    #[test]
    fn replace_all_is_one_undo_transaction() {
        let mut editor = Editor::new(Document::from_loaded(
            PathBuf::from("fixture.txt"),
            "one two one\n".to_owned(),
            Eol::Lf,
            Encoding::Utf8,
            false,
        ));
        editor.find.open = true;
        editor.find.query = "one".to_owned();
        editor.recompute_matches();
        editor.replace_all();
        assert_eq!(editor.document().snapshot(), " two \n");
        assert!(editor.document_mut().undo());
        assert_eq!(editor.document().snapshot(), "one two one\n");
    }
}
