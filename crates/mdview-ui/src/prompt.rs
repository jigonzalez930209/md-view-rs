//! Styled modal prompts used by the workspace and settings flows.
//!
//! Native dialogs handle file picking, but short in-app decisions use these modals so the
//! workbench keeps one visual language: sunken text fields, aligned footer buttons, an explicit
//! primary action, and keyboard support.

use egui::{Button, Context, Frame, Id, Key, Modal, RichText, Stroke, Ui};

use crate::{primary_button, secondary_button, text_field};

/// What the user chose in a prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptAction {
    /// Confirm the primary action.
    Submit,
    /// Dismiss without changing anything.
    Cancel,
}

/// A single-line name prompt, used for creating and renaming entries.
pub struct NamePrompt<'a> {
    /// Dialog title.
    pub title: String,
    /// Short explanation under the title.
    pub description: String,
    /// Field label.
    pub label: String,
    /// Editable value.
    pub value: &'a mut String,
    /// Primary button label.
    pub confirm: String,
    /// Cancel button label.
    pub cancel: String,
    /// Optional validation message.
    pub error: Option<String>,
}

/// Show the name prompt and report the user's choice.
pub fn show_name_prompt(ctx: &Context, prompt: &mut NamePrompt<'_>) -> Option<PromptAction> {
    let mut action = None;
    let dialog = Modal::new(Id::unique("mdview_name_prompt"))
        .frame(dialog_frame(ui_frame(ctx)))
        .show(ctx, |ui| {
            ui.set_width(380.0);
            dialog_header(ui, &prompt.title, &mut action);
            if !prompt.description.is_empty() {
                ui.label(RichText::new(&prompt.description).small().weak());
                ui.add_space(10.0);
            }
            ui.label(
                RichText::new(&prompt.label)
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.add_space(3.0);
            let response = text_field(ui, prompt.value, "", 344.0);
            if ctx.memory(|memory| memory.focused()).is_none() {
                response.request_focus();
            }
            if let Some(error) = &prompt.error {
                ui.add_space(5.0);
                ui.label(RichText::new(error).small().color(ui.visuals().error_fg_color));
            }
            ui.add_space(14.0);
            footer(ui, &prompt.confirm, &prompt.cancel, false, &mut action);
            if ui.input(|input| input.key_pressed(Key::Enter)) {
                action = Some(PromptAction::Submit);
            }
        });
    if dialog.backdrop_response.clicked() || ctx.input(|input| input.key_pressed(Key::Escape)) {
        action = Some(PromptAction::Cancel);
    }
    action
}

/// A compact confirmation dialog.
pub struct ConfirmPrompt {
    /// Dialog title.
    pub title: String,
    /// Question or description.
    pub message: String,
    /// Primary button label.
    pub confirm: String,
    /// Cancel button label.
    pub cancel: String,
    /// Paint the primary action as destructive.
    pub danger: bool,
}

/// Show a confirmation dialog and report the user's choice.
pub fn show_confirm_prompt(ctx: &Context, prompt: &ConfirmPrompt) -> Option<PromptAction> {
    let mut action = None;
    let dialog = Modal::new(Id::unique("mdview_confirm_prompt"))
        .frame(dialog_frame(ui_frame(ctx)))
        .show(ctx, |ui| {
            ui.set_width(340.0);
            dialog_header(ui, &prompt.title, &mut action);
            ui.label(&prompt.message);
            ui.add_space(14.0);
            footer(ui, &prompt.confirm, &prompt.cancel, prompt.danger, &mut action);
        });
    if dialog.backdrop_response.clicked() || ctx.input(|input| input.key_pressed(Key::Escape)) {
        action = Some(PromptAction::Cancel);
    }
    action
}

pub(crate) fn dialog_frame(visuals: egui::Visuals) -> Frame {
    Frame::new()
        .fill(visuals.window_fill)
        .stroke(Stroke::new(1.0, visuals.window_stroke.color))
        .corner_radius(10)
        .inner_margin(egui::Margin::same(18))
        .shadow(egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: egui::Color32::from_black_alpha(60),
        })
}

pub(crate) fn ui_frame(ctx: &Context) -> egui::Visuals {
    ctx.style_of(ctx.theme()).visuals.clone()
}

fn dialog_header(ui: &mut Ui, title: &str, action: &mut Option<PromptAction>) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .size(17.0)
                .strong()
                .color(ui.visuals().text_color()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(Button::new(RichText::new("×").size(14.0)).frame(false))
                .on_hover_text("Close")
                .clicked()
            {
                *action = Some(PromptAction::Cancel);
            }
        });
    });
    ui.add_space(8.0);
}

fn footer(ui: &mut Ui, confirm: &str, cancel: &str, danger: bool, action: &mut Option<PromptAction>) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        if primary_button(ui, confirm, danger).clicked() {
            *action = Some(PromptAction::Submit);
        }
        ui.add_space(8.0);
        if secondary_button(ui, cancel).clicked() {
            *action = Some(PromptAction::Cancel);
        }
    });
}
