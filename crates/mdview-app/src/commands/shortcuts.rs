//! Keyboard shortcut dispatch.

use craft_keymap::Chord;
use eframe::egui;
use mdview_ui::Command;

use crate::app::MdViewApp;

impl MdViewApp {
    fn consume_shortcut(&self, ctx: &egui::Context, text: &str) -> bool {
        let Ok(chord) = Chord::parse(text) else {
            log::warn!("invalid shortcut in application code: {text}");
            return false;
        };
        ctx.input_mut(|input| input.consume_shortcut(&chord.to_egui_shortcut()))
    }

    pub(super) fn ensure_edit_mode(&mut self, ctx: &egui::Context) {
        if self.tabs.active().mode == mdview_ui::ViewMode::View {
            self.handle_command(ctx, Command::SetMode(mdview_ui::ViewMode::Edit));
        }
    }

    pub(crate) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.dialogs.is_open() || self.close_prompt_open {
            return;
        }
        if self.tabs.active().conflict {
            if self.consume_shortcut(ctx, "Alt+R") {
                let index = self.tabs.active_index();
                self.apply_conflict(index, crate::conflict::ConflictAction::Reload, ctx);
                return;
            }
            if self.consume_shortcut(ctx, "Alt+K") {
                let index = self.tabs.active_index();
                self.apply_conflict(index, crate::conflict::ConflictAction::KeepMine, ctx);
                return;
            }
        }
        if self.consume_shortcut(ctx, "Cmd+N") {
            self.new_document(ctx);
        } else if self.consume_shortcut(ctx, "Cmd+Shift+O") {
            self.ask_open_folder();
        } else if self.consume_shortcut(ctx, "Cmd+O") {
            self.ask_open();
        } else if self.consume_shortcut(ctx, "Cmd+Shift+S") {
            self.ask_save_as(false);
        } else if self.consume_shortcut(ctx, "Cmd+S") {
            self.request_save(false);
        } else if self.consume_shortcut(ctx, "Cmd+W") {
            self.close_tab(self.tabs.active_index(), ctx);
        } else if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F2)) {
            self.handle_command(ctx, Command::RenameFile);
        } else if self.consume_shortcut(ctx, "Alt+Left") {
            self.handle_command(ctx, Command::Back);
        } else if self.consume_shortcut(ctx, "Alt+Right") {
            self.handle_command(ctx, Command::Forward);
        } else if self.consume_shortcut(ctx, "Cmd+Shift+F") {
            self.handle_command(ctx, Command::SearchWorkspace);
        } else if self.consume_shortcut(ctx, "Cmd+F") {
            self.handle_command(ctx, Command::Find);
        } else if self.consume_shortcut(ctx, "Cmd+H") {
            self.handle_command(ctx, Command::Replace);
        } else if self.consume_shortcut(ctx, "Cmd+G") {
            self.handle_command(ctx, Command::GotoLine);
        } else if self.consume_shortcut(ctx, "Cmd+Shift+E") {
            self.handle_command(ctx, Command::SetMode(mdview_ui::ViewMode::Split));
        } else if self.consume_shortcut(ctx, "Cmd+E") {
            let mode = self.tabs.active().mode;
            let next = if mode == mdview_ui::ViewMode::Edit {
                mdview_ui::ViewMode::View
            } else {
                mdview_ui::ViewMode::Edit
            };
            self.handle_command(ctx, Command::SetMode(next));
        } else if self.consume_shortcut(ctx, "Cmd+Shift+P") {
            self.handle_command(ctx, Command::CommandPalette);
        } else if self.consume_shortcut(ctx, "Cmd+P") {
            self.handle_command(ctx, Command::QuickOpen);
        } else if self.consume_shortcut(ctx, "Cmd+,") {
            self.handle_command(ctx, Command::Settings);
        } else if self.consume_shortcut(ctx, "Cmd+=") {
            let zoom = ctx.zoom_factor() + 0.1;
            self.handle_command(ctx, Command::SetZoom(zoom));
        } else if self.consume_shortcut(ctx, "Cmd+-") {
            let zoom = ctx.zoom_factor() - 0.1;
            self.handle_command(ctx, Command::SetZoom(zoom));
        } else if self.consume_shortcut(ctx, "Cmd+0") {
            self.handle_command(ctx, Command::SetZoom(1.0));
        } else if self.consume_shortcut(ctx, "Cmd+B") {
            self.sidebar.open = !self.sidebar.open;
            self.sync_session();
        } else if self.consume_shortcut(ctx, "Ctrl+Shift+Tab") {
            self.handle_command(ctx, Command::PreviousTab);
        } else if self.consume_shortcut(ctx, "Ctrl+Tab") {
            self.handle_command(ctx, Command::NextTab);
        } else if self.consume_shortcut(ctx, "Cmd+Q") {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
