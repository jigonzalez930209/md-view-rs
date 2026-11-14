//! The go-to-line dialog.

use egui::{Button, Context, Frame, Id, Modal, RichText, Stroke};

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
        let visuals = ctx.style_of(ctx.theme()).visuals.clone();
        let mut close = false;
        let mut submit = false;
        let count = self.document.line_count();
        let dialog = Modal::new(Id::unique("mdview_goto"))
            .frame(
                Frame::new()
                    .fill(visuals.window_fill)
                    .stroke(Stroke::new(1.0, visuals.window_stroke.color))
                    .corner_radius(10)
                    .inner_margin(egui::Margin::same(16))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(60),
                    }),
            )
            .show(ctx, |ui| {
                ui.set_width(300.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Go to line")
                            .size(16.0)
                            .strong()
                            .color(ui.visuals().text_color()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(Button::new(RichText::new("×").size(14.0)).frame(false))
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let field = crate::controls::text_field(ui, &mut self.goto.text, &format!("1–{count}"), 220.0);
                    if self.goto.focus {
                        field.request_focus();
                        self.goto.focus = false;
                    }
                    if field.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        submit = true;
                    }
                    ui.vertical(|ui| {
                        let mut line = self.goto.text.trim().parse::<usize>().unwrap_or(1).clamp(1, count);
                        if crate::controls::step_button(ui, true) {
                            line = (line + 1).min(count);
                            self.goto.text = line.to_string();
                        }
                        if crate::controls::step_button(ui, false) {
                            line = line.saturating_sub(1).max(1);
                            self.goto.text = line.to_string();
                        }
                    });
                });
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let go = Button::new(RichText::new("Go").color(ui.visuals().selection.stroke.color))
                        .fill(ui.visuals().selection.bg_fill)
                        .stroke(Stroke::NONE)
                        .corner_radius(6)
                        .min_size(egui::vec2(84.0, 30.0));
                    if ui.add(go).clicked() {
                        submit = true;
                    }
                });
            });
        if dialog.backdrop_response.clicked() || ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            close = true;
        }
        if close {
            self.goto.reset();
            return;
        }
        if submit && let Ok(line) = self.goto.text.trim().parse::<usize>() {
            self.goto_line(line);
            self.goto.reset();
        }
    }
}
