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
                log::debug!("lightbox open alt={}", lightbox.alt);
                self.lightbox = Some(crate::lightbox::LightboxState::new(lightbox.request, lightbox.alt));
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
        if tab.editor.is_at_end() && tab.editor.scrolling_recently(std::time::Duration::from_millis(350)) {
            log::debug!(
                "editor scroll end mode=split wrap={} offset={:.1} max={:.1} clearance={:.1}",
                tab.editor.status().wrap,
                tab.editor.scroll_offset(),
                tab.editor.max_scroll_offset(),
                tab.editor.bottom_clearance()
            );
        }
        let source = choose_source(editor_scrolled, preview_scrolled, hovered);
        let editor_position = tab.editor.first_visible_line_fraction();
        let preview_position = tab.preview.source_map().line_at_offset(tab.preview.scroll_offset());
        let action = plan_sync(
            source,
            tab.editor.is_at_end(),
            tab.preview.is_at_end(),
            editor_position,
            preview_position,
        );
        let moved = match action {
            SyncAction::PreviewEnd => {
                log::debug!("split sync editor->preview end");
                tab.preview.scroll_to_end()
            }
            SyncAction::PreviewLine(line, fraction) => {
                log::debug!(
                    "split sync editor->preview line={line} fraction={fraction:.3} editor_y={:.1} preview_y={:.1}",
                    tab.editor.scroll_offset(),
                    tab.preview.scroll_offset()
                );
                tab.preview.scroll_to_line(line, fraction)
            }
            SyncAction::EditorEnd => {
                let metrics = tab.preview.scroll_metrics();
                log::debug!(
                    "preview scroll end offset={:.1} max={:.1} last_bottom={:.1} content={:.1}",
                    metrics.offset,
                    metrics.max_offset,
                    metrics.last_block_bottom,
                    metrics.content_height
                );
                log::debug!("split sync preview->editor end");
                tab.editor.scroll_to_end()
            }
            SyncAction::EditorLine(line, fraction) => {
                log::debug!(
                    "split sync preview->editor line={line} fraction={fraction:.3} editor_y={:.1} preview_y={:.1}",
                    tab.editor.scroll_offset(),
                    tab.preview.scroll_offset()
                );
                tab.editor.scroll_to_line(line, fraction)
            }
            SyncAction::Hold => false,
        };
        if moved {
            ctx.request_repaint();
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
        tab.preview.set_reading_style(
            self.prefs.get().reading.margins(),
            self.prefs.get().reading.typography(),
        );
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

/// The next programmatic scroll to apply in a split frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum SyncAction {
    /// Scroll the preview to its maximum offset.
    PreviewEnd,
    /// Scroll the preview to a source line and intra-line fraction.
    PreviewLine(usize, f32),
    /// Scroll the editor to its maximum offset.
    EditorEnd,
    /// Scroll the editor to a source line and intra-line fraction.
    EditorLine(usize, f32),
    /// Neither pane needs a programmatic scroll.
    Hold,
}

/// Decide the next sync action. A pane already at its end never re-sends the
/// same target, which is what caused repeated programmatic paints and flicker
/// when the user kept scrolling the editor after both panes reached the end.
pub(crate) fn plan_sync(
    source: Option<Pane>,
    editor_at_end: bool,
    preview_at_end: bool,
    editor_position: (usize, f32),
    preview_position: Option<(usize, f32)>,
) -> SyncAction {
    match source {
        Some(Pane::Editor) if editor_at_end => {
            if preview_at_end {
                SyncAction::Hold
            } else {
                SyncAction::PreviewEnd
            }
        }
        Some(Pane::Editor) => SyncAction::PreviewLine(editor_position.0, editor_position.1),
        Some(Pane::Preview) if preview_at_end => {
            if editor_at_end {
                SyncAction::Hold
            } else {
                SyncAction::EditorEnd
            }
        }
        Some(Pane::Preview) => preview_position
            .map(|(line, fraction)| SyncAction::EditorLine(line, fraction))
            .unwrap_or(SyncAction::Hold),
        None => SyncAction::Hold,
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

    #[test]
    fn end_targets_do_not_repeat_when_both_panes_are_at_the_end() {
        assert_eq!(
            plan_sync(Some(Pane::Editor), true, true, (10, 1.0), Some((10, 1.0))),
            SyncAction::Hold
        );
        assert_eq!(
            plan_sync(Some(Pane::Preview), true, true, (10, 1.0), Some((10, 1.0))),
            SyncAction::Hold
        );
    }

    #[test]
    fn an_end_in_one_pane_targets_the_other_pane() {
        assert_eq!(
            plan_sync(Some(Pane::Editor), true, false, (10, 1.0), Some((9, 0.5))),
            SyncAction::PreviewEnd
        );
        assert_eq!(
            plan_sync(Some(Pane::Preview), false, true, (8, 0.0), Some((20, 1.0))),
            SyncAction::EditorEnd
        );
    }

    #[test]
    fn regular_scrolls_map_lines_in_both_directions() {
        assert_eq!(
            plan_sync(Some(Pane::Editor), false, false, (7, 0.25), Some((3, 0.5))),
            SyncAction::PreviewLine(7, 0.25)
        );
        assert_eq!(
            plan_sync(Some(Pane::Preview), false, false, (7, 0.25), Some((3, 0.5))),
            SyncAction::EditorLine(3, 0.5)
        );
        assert_eq!(
            plan_sync(Some(Pane::Preview), false, false, (7, 0.25), None),
            SyncAction::Hold
        );
    }

    #[test]
    fn the_plan_never_rewrites_an_end_that_is_already_reached() {
        for source in [None, Some(Pane::Editor), Some(Pane::Preview)] {
            for editor_at_end in [false, true] {
                for preview_at_end in [false, true] {
                    for positions in [(Some((3, 0.2)), Some((8, 0.6))), (None, None)] {
                        let action = plan_sync(
                            source,
                            editor_at_end,
                            preview_at_end,
                            positions.0.unwrap_or((0, 0.0)),
                            positions.1,
                        );
                        if action == SyncAction::PreviewEnd {
                            assert!(!preview_at_end, "{source:?} {editor_at_end} {preview_at_end}");
                        }
                        if action == SyncAction::EditorEnd {
                            assert!(!editor_at_end, "{source:?} {editor_at_end} {preview_at_end}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn no_source_holds_even_at_the_end() {
        assert_eq!(
            plan_sync(None, true, true, (10, 1.0), Some((10, 1.0))),
            SyncAction::Hold
        );
    }
}
