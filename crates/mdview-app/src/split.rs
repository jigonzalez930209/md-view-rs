//! Split editor/preview layout with synchronized scrolling.

use eframe::egui;

use crate::app::MdViewApp;

impl MdViewApp {
    /// Paint the active tab as an editor and preview split.
    pub(crate) fn show_split(&mut self, ui: &mut egui::Ui, index: usize) {
        let theme = mdview_theme::Theme::for_mode(self.theme);
        let light = self.theme == mdview_theme::ThemeMode::Light;
        let identity = self
            .tabs
            .tabs()
            .get(index)
            .map(|tab| tab.editor.identity())
            .unwrap_or_default();
        let ratio = self
            .tabs
            .tabs()
            .get(index)
            .map(|tab| tab.split_ratio.clamp(0.2, 0.8))
            .unwrap_or(0.5);
        let available_width = ui.available_width();
        let narrow = available_width < 760.0;
        let mut preview_output = None;

        if narrow {
            let available = ui.available_height().max(200.0);
            let panel = egui::Panel::bottom(("split_preview_vertical", identity))
                .resizable(true)
                .default_size(available * ratio)
                .min_size(120.0)
                .show(ui, |ui| {
                    if let Some(tab) = self.tabs.tab_mut(index) {
                        tab.ensure_preview();
                        preview_output = Some(tab.preview.show(ui, &tab.preview_source, &theme, light));
                    }
                });
            let height = panel.response.rect.height();
            if height > 0.0
                && let Some(tab) = self.tabs.tab_mut(index)
            {
                tab.split_ratio = (height / available).clamp(0.2, 0.8);
            }
            if let Some(tab) = self.tabs.tab_mut(index) {
                tab.editor.show(ui);
            }
        } else {
            let available = available_width.max(400.0);
            let panel = egui::Panel::right(("split_preview_horizontal", identity))
                .resizable(true)
                .default_size(available * ratio)
                .min_size(200.0)
                .show(ui, |ui| {
                    if let Some(tab) = self.tabs.tab_mut(index) {
                        tab.ensure_preview();
                        preview_output = Some(tab.preview.show(ui, &tab.preview_source, &theme, light));
                    }
                });
            let width = panel.response.rect.width();
            if width > 0.0
                && let Some(tab) = self.tabs.tab_mut(index)
            {
                tab.split_ratio = (width / available).clamp(0.2, 0.8);
            }
            if let Some(tab) = self.tabs.tab_mut(index) {
                tab.editor.show(ui);
            }
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
        let winner = choose_source(editor_scrolled, preview_scrolled, hovered);
        match winner {
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
}

/// Which pane drove the scroll in a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pane {
    Editor,
    Preview,
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
