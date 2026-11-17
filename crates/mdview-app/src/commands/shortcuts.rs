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

    /// Whether keys go to a Markdown document's editor, where Cmd+B/I/U/K format text.
    fn markdown_editor_focused(&self) -> bool {
        let tab = self.tabs.active();
        tab.mode != mdview_ui::ViewMode::View
            && !tab.is_viewer()
            && self.tabs.editor().is_markdown()
            && self.tabs.editor().has_focus()
    }

    pub(crate) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.dialogs.is_open() || self.close_prompt_open {
            return;
        }
        let alt_menu = if self.consume_shortcut(ctx, "Alt+F") {
            Some(0)
        } else if self.consume_shortcut(ctx, "Alt+E") {
            Some(1)
        } else if self.consume_shortcut(ctx, "Alt+V") {
            Some(2)
        } else if self.consume_shortcut(ctx, "Alt+G") {
            Some(3)
        } else if self.consume_shortcut(ctx, "Alt+H") {
            Some(4)
        } else if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F10)) {
            Some(0)
        } else {
            None
        };
        if let Some(group) = alt_menu {
            log::debug!("keyboard menu group={group}");
            self.keyboard_menu.open(group);
            // Alt+F/E/V/G/H also produces a text event in some backends. The menu owns
            // the key; the focused editor must not insert the letter.
            consume_text_events(ctx);
            return;
        }
        if self.tabs.active().conflict {
            if self.consume_shortcut(ctx, "Alt+R") {
                let index = self.tabs.active_index();
                self.apply_conflict(index, crate::conflict::ConflictAction::Reload, ctx);
                consume_text_events(ctx);
                return;
            }
            if self.consume_shortcut(ctx, "Alt+K") {
                let index = self.tabs.active_index();
                self.apply_conflict(index, crate::conflict::ConflictAction::KeepMine, ctx);
                consume_text_events(ctx);
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
        } else if self.consume_shortcut(ctx, "Cmd+Shift+M") {
            self.handle_command(ctx, Command::InsertLatexBlock);
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
        } else if !self.markdown_editor_focused() && self.consume_shortcut(ctx, "Cmd+B") {
            // In a focused Markdown editor Cmd+B is Bold; the editor handles it.
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

/// Drop text events produced by an Alt shortcut so the focused editor cannot insert the key.
fn consume_text_events(ctx: &egui::Context) {
    ctx.input_mut(|input| {
        input.events.retain(|event| !matches!(event, egui::Event::Text(_)));
    });
}
