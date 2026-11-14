//! Split editor/preview layout with synchronized scrolling.

use eframe::egui;

use crate::app::MdViewApp;

impl MdViewApp {
    /// Paint the active tab as an editor and preview split.
    pub(crate) fn show_split(&mut self, ui: &mut egui::Ui, index: usize) {
        let theme = mdview_theme::Theme::for_id(self.effective_theme);
        let light = self.theme == mdview_theme::ThemeMode::Light;
        let mut ratio = self
            .tabs
            .tabs()
            .get(index)
            .map(|tab| tab.split_ratio.clamp(0.2, 0.8))
            .unwrap_or(0.5);
        let width = ui.available_width().max(320.0);
        let height = ui.available_height().max(200.0);
        let mut preview_output = None;

        if width < 620.0 {
            let handle_height = 6.0;
            let preview_height = (height * ratio).clamp(120.0, height - 120.0);
            let editor_height = (height - preview_height - handle_height).max(80.0);
            ui.allocate_ui_with_layout(
                egui::vec2(width, editor_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    if let Some(tab) = self.tabs.tab_mut(index) {
                        tab.editor.show(ui);
                    }
                },
            );
            let handle = ui.allocate_response(egui::vec2(width, handle_height), egui::Sense::drag());
            paint_divider(ui, &handle, true);
            if handle.dragged() {
                ratio = (ratio - handle.drag_delta().y / height).clamp(0.2, 0.8);
                if let Some(tab) = self.tabs.tab_mut(index) {
                    tab.split_ratio = ratio;
                }
            }
            ui.allocate_ui_with_layout(
                egui::vec2(width, preview_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    preview_output = self.preview_pane(ui, index, &theme, light);
                },
            );
        } else {
            let handle_width = 6.0;
            let preview_width = (width * ratio).clamp(200.0, width - 200.0);
            let editor_width = (width - preview_width - handle_width).max(160.0);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.allocate_ui_with_layout(
                    egui::vec2(editor_width, height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        if let Some(tab) = self.tabs.tab_mut(index) {
                            tab.editor.show(ui);
                        }
                    },
                );
                let handle = ui.allocate_response(egui::vec2(handle_width, height), egui::Sense::drag());
                paint_divider(ui, &handle, false);
                if handle.dragged() {
                    ratio = (ratio - handle.drag_delta().x / width).clamp(0.2, 0.8);
                    if let Some(tab) = self.tabs.tab_mut(index) {
                        tab.split_ratio = ratio;
                    }
                }
                ui.allocate_ui_with_layout(
                    egui::vec2(preview_width, height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        preview_output = self.preview_pane(ui, index, &theme, light);
                    },
                );
            });
        }

        self.sync_split_scroll(ui.ctx(), index, {
            let hover = ui.ctx().input(|input| input.pointer.hover_pos());
            let (editor_rect, preview_rect) = self
                .tabs
                .tabs()
                .get(index)
                .map(|tab| (tab.editor.viewport_rect(), tab.preview.viewport_rect()))
                .unwrap_or((egui::Rect::NOTHING, egui::Rect::NOTHING));
            match hover {
                Some(position) if editor_rect.contains(position) => Some(Pane::Editor),
                Some(position) if preview_rect.contains(position) => Some(Pane::Preview),
                _ => None,
            }
        });

        if let Some(output) = preview_output {
            if let Some(lightbox) = output.lightbox.clone() {
                self.lightbox = Some(crate::lightbox::LightboxState {
                    request: lightbox.request,
                    alt: lightbox.alt,
                    scale: 1.0,
                });
            }
            self.handle_preview_output(output, ui.ctx());
        }
    }

    fn sync_split_scroll(&mut self, ctx: &egui::Context, index: usize, hovered: Option<Pane>) {
        if !self.prefs.get().editor.sync_scroll {
            return;
        }
        let Some(tab) = self.tabs.tab_mut(index) else {
            return;
        };
        let editor_scrolled = tab.editor.take_user_scroll();
        let preview_scrolled = tab.preview.take_user_scroll();
        match choose_source(editor_scrolled, preview_scrolled, hovered) {
            Some(Pane::Editor) => {
                let line = tab.editor.first_visible_line();
                tab.preview.scroll_to_line(line, 0.0);
                ctx.request_repaint();
            }
            Some(Pane::Preview) => {
                if let Some((line, fraction)) = tab.preview.source_map().line_at_offset(tab.preview.scroll_offset()) {
                    tab.editor.scroll_to_line(line, fraction);
                    ctx.request_repaint();
                }
            }
            None => {}
        }
    }

    fn preview_pane(
        &mut self,
        ui: &mut egui::Ui,
        index: usize,
        theme: &mdview_theme::Theme,
        light: bool,
    ) -> Option<mdview_render::PreviewOutput> {
        let base = self
            .tabs
            .tabs()
            .get(index)
            .and_then(|tab| tab.editor.document().path())
            .and_then(std::path::Path::parent)
            .map(std::path::Path::to_path_buf);
        let tab = self.tabs.tab_mut(index)?;
        tab.ensure_preview();
        tab.preview
            .set_compact_margins(self.prefs.get().appearance.compact_margins);
        Some(
            tab.preview
                .show_with_media(ui, &tab.preview_source, theme, light, base, &mut self.media),
        )
    }
}

fn paint_divider(ui: &egui::Ui, response: &egui::Response, horizontal: bool) {
    let color = ui.visuals().widgets.inactive.bg_stroke.color;
    let rect = response.rect;
    if horizontal {
        let y = rect.center().y;
        ui.painter().line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, color),
        );
    } else {
        let x = rect.center().x;
        ui.painter().line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, color),
        );
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(if horizontal {
            egui::CursorIcon::ResizeVertical
        } else {
            egui::CursorIcon::ResizeHorizontal
        });
    }
}

/// Pick one scroll source per frame. Both scrolled: the hovered pane wins, then the editor.
pub(crate) fn choose_source(editor_scrolled: bool, preview_scrolled: bool, hovered: Option<Pane>) -> Option<Pane> {
    match (editor_scrolled, preview_scrolled) {
        (true, true) => Some(hovered.unwrap_or(Pane::Editor)),
        (true, false) => Some(Pane::Editor),
        (false, true) => Some(Pane::Preview),
        (false, false) => None,
    }
}

/// Which pane drove the scroll in a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pane {
    Editor,
    Preview,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_scrolling_pane_wins() {
        assert_eq!(choose_source(true, false, None), Some(Pane::Editor));
        assert_eq!(choose_source(false, true, None), Some(Pane::Preview));
        assert_eq!(choose_source(false, false, None), None);
    }

    #[test]
    fn the_hovered_pane_wins_when_both_scroll() {
        assert_eq!(choose_source(true, true, Some(Pane::Preview)), Some(Pane::Preview));
        assert_eq!(choose_source(true, true, Some(Pane::Editor)), Some(Pane::Editor));
        assert_eq!(choose_source(true, true, None), Some(Pane::Editor));
    }

    #[test]
    fn a_thousand_scroll_pairs_do_not_oscillate() {
        // Each simulated frame has at most one user-initiated scroll; applying the other
        // direction is programmatic and therefore cannot create a new source in the next frame.
        let mut last = None;
        for frame in 0..1_000 {
            let editor = frame % 2 == 0;
            let preview = !editor;
            let source = choose_source(editor, preview, None);
            let expected = if editor { Pane::Editor } else { Pane::Preview };
            assert_eq!(source, Some(expected));
            assert_ne!(source, last);
            last = source;
        }
    }
}
