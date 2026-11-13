//! Go-to-line dialog.

use egui::{Context, TextEdit};

use crate::Editor;

/// Go-to-line state.
#[derive(Clone, Debug, Default)]
pub struct GotoState {
    /// Whether the dialog is visible.
    pub open: bool,
    /// The entered line number.
    pub text: String,
    /// Request focus on the input.
    pub focus: bool,
}

impl GotoState {
    /// Close and clear the dialog.
    pub fn reset(&mut self) {
        self.open = false;
        self.text.clear();
        self.focus = false;
    }
}

impl Editor {
    /// Draw the go-to-line dialog.
    pub(crate) fn show_goto(&mut self, ctx: &Context) {
        if !self.goto.open {
            return;
        }
        let mut open = self.goto.open;
        egui::Window::new("Go to line")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                let response = ui.add(
                    TextEdit::singleline(&mut self.goto.text)
                        .hint_text(format!("1–{}", self.document.line_count()))
                        .desired_width(180.0),
                );
                if self.goto.focus {
                    response.request_focus();
                    self.goto.focus = false;
                }
                let submitted = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                if (ui.button("Go").clicked() || submitted)
                    && let Ok(line) = self.goto.text.trim().parse::<usize>()
                {
                    self.goto_line(line);
                    self.goto.reset();
                    self.goto.open = false;
                }
            });
        if !open {
            self.goto.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use mdview_model::{Document, Encoding, Eol};

    use crate::Editor;

    #[test]
    fn goto_line_clamps_and_moves_the_cursor() {
        let document = Document::from_loaded(
            PathBuf::from("fixture.txt"),
            "one\ntwo\nthree\n".to_owned(),
            Eol::Lf,
            Encoding::Utf8,
            false,
        );
        let mut editor = Editor::new(document);
        editor.goto_line(2);
        assert_eq!(editor.document().line_col(editor.cursor).0, 1);
        editor.goto_line(99);
        assert_eq!(editor.document().line_col(editor.cursor).0, 3);
    }
}
