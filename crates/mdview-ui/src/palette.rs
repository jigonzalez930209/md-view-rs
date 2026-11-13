//! Command palette and quick open.

use egui::{Context, Key, TextEdit, Window};

use crate::Command;

/// One palette row.
#[derive(Clone, Debug, PartialEq)]
pub struct PaletteItem {
    /// Display label.
    pub label: String,
    /// Secondary text.
    pub detail: String,
    /// Command executed on activation.
    pub command: Command,
}

/// Show the palette and return the chosen command.
pub fn show_palette(
    ctx: &Context,
    open: &mut bool,
    query: &mut String,
    title: &str,
    items: &[PaletteItem],
) -> Option<Command> {
    if !*open {
        return None;
    }
    let mut command = None;
    let mut window_open = *open;
    let mut keep_open = *open;
    Window::new(title)
        .open(&mut window_open)
        .resizable(false)
        .default_width(560.0)
        .collapsible(false)
        .show(ctx, |ui| {
            let response = ui.add(
                TextEdit::singleline(query)
                    .hint_text("Type to filter…")
                    .desired_width(f32::INFINITY),
            );
            response.request_focus();
            if ui.input(|input| input.key_pressed(Key::Escape)) {
                keep_open = false;
            }
            let needle = query.to_ascii_lowercase();
            let matches = items
                .iter()
                .filter(|item| {
                    needle.is_empty()
                        || item.label.to_ascii_lowercase().contains(&needle)
                        || item.detail.to_ascii_lowercase().contains(&needle)
                })
                .take(12)
                .collect::<Vec<_>>();
            ui.separator();
            if matches.is_empty() {
                ui.label(egui::RichText::new("No matches").weak());
            }
            for (index, item) in matches.iter().enumerate() {
                let text = if item.detail.is_empty() {
                    item.label.clone()
                } else {
                    format!("{}  ·  {}", item.label, item.detail)
                };
                let response = ui.selectable_label(index == 0, text);
                if response.clicked() {
                    command = Some(item.command);
                    keep_open = false;
                }
            }
            if ui.input(|input| input.key_pressed(Key::Enter))
                && let Some(item) = matches.first()
            {
                command = Some(item.command);
                keep_open = false;
            }
        });
    *open = window_open && keep_open;
    command
}
