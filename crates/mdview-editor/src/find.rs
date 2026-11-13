//! Find and replace.

use egui::{Key, TextEdit, Ui};
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
    /// Draw the find/replace bar above the editor.
    pub(crate) fn show_find_bar(&mut self, ui: &mut Ui) {
        if !self.find.open {
            return;
        }
        let mut changed = false;
        ui.horizontal(|ui| {
            let query = ui.add(
                TextEdit::singleline(&mut self.find.query)
                    .hint_text("Find")
                    .desired_width(220.0),
            );
            if self.find.focus_query {
                query.request_focus();
                self.find.focus_query = false;
            }
            if query.changed() {
                changed = true;
            }
            if query.has_focus() && ui.input(|input| input.key_pressed(Key::Enter) || input.key_pressed(Key::F3)) {
                if ui.input(|input| input.modifiers.shift) {
                    self.find_previous();
                } else {
                    self.find_next();
                }
            }

            if ui
                .add(egui::Checkbox::new(&mut self.find.options.case_sensitive, "Aa"))
                .changed()
            {
                changed = true;
            }
            if ui
                .add(egui::Checkbox::new(&mut self.find.options.whole_word, "Word"))
                .changed()
            {
                changed = true;
            }
            if ui
                .add(egui::Checkbox::new(&mut self.find.options.regex, ".*"))
                .changed()
            {
                changed = true;
            }

            if ui.button("Previous").clicked() {
                self.find_previous();
            }
            if ui.button("Next").clicked() {
                self.find_next();
            }
            let count = self.find.matches.len();
            ui.label(format!("{count} matches"));
            if ui.button("Close").clicked() {
                self.find.open = false;
            }
        });

        if self.find.replace {
            ui.horizontal(|ui| {
                let replacement = ui.add(
                    TextEdit::singleline(&mut self.find.replacement)
                        .hint_text("Replace with")
                        .desired_width(260.0),
                );
                if self.find.focus_replace {
                    replacement.request_focus();
                    self.find.focus_replace = false;
                }
                if replacement.changed() {
                    changed = true;
                }
                if ui.button("Replace").clicked() {
                    self.replace_current();
                }
                if ui.button("Replace All").clicked() {
                    self.replace_all();
                }
            });
        }

        if changed {
            self.recompute_matches();
        }
        if ui.input(|input| input.key_pressed(Key::Escape)) {
            self.find.open = false;
        }
    }

    /// Recompute matches from the current query and document.
    pub(crate) fn recompute_matches(&mut self) {
        self.find.matches.clear();
        if self.find.query.is_empty() {
            return;
        }
        let text = self.document.snapshot();
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
        let Ok(regex) = RegexBuilder::new(&pattern)
            .case_insensitive(!self.find.options.case_sensitive)
            .size_limit(10 * 1024 * 1024)
            .build()
        else {
            return;
        };
        self.find.matches = regex
            .find_iter(&text)
            .map(|found| {
                (
                    self.document.byte_to_char(found.start()),
                    self.document.byte_to_char(found.end()),
                )
            })
            .collect();
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
        self.document.replace_range(start, end, &self.find.replacement);
        self.recompute_matches();
        self.cursor = start + self.find.replacement.chars().count();
        self.anchor = self.cursor;
    }

    fn replace_all(&mut self) {
        self.recompute_matches();
        if self.find.matches.is_empty() {
            return;
        }
        self.document.replace_ranges(&self.find.matches, &self.find.replacement);
        self.recompute_matches();
    }
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
