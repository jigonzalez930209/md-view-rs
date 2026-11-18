//! The formula editor dialog and the equation gallery.
//!
//! The dialog pairs a source editor (the same editor widget as documents) with a live
//! preview. Hovering a part of the source highlights the part of the formula it produces,
//! so a reader who does not know TeX can see which command draws which symbol.

mod catalog;
mod gallery;
mod preview;

use std::path::PathBuf;

use egui::{Color32, Context, Id, Key, Modal, Modifiers, RichText, Sense, Ui, Vec2};
use mdview_editor::Editor;
use mdview_model::{Document, Encoding, Eol};

pub use catalog::{GALLERY, GalleryEquation, PALETTE, PLACEHOLDER, PaletteCategory, next_placeholder};
pub(crate) use gallery::{GalleryPick, show_gallery};
use preview::PreviewCache;

/// Every fixed label the dialog and gallery translate; gallery equation and palette
/// category names are translated too.
pub const MATH_EDITOR_LABELS: &[&str] = &[
    "Equation editor",
    "Edit equation",
    "New equation",
    "Edit before inserting",
    "Inline",
    "Display",
    "Insert",
    "Update",
    "Cancel",
    "Source",
    "Preview",
    "Hover the source to find its part in the formula",
    "Tab moves to the next placeholder",
];

/// Width of the dialog, in points.
const DIALOG_WIDTH: f32 = 860.0;
/// Height of the source editor and the preview, in points.
const BODY_HEIGHT: f32 = 230.0;
/// Live preview size, in points per TeX em.
const PREVIEW_EM: f32 = 30.0;
/// Type size of every palette template, in points per TeX em. One size for all, so the
/// buttons show the templates at their relative sizes in a formula.
const PALETTE_EM: f32 = 13.0;
/// Smallest palette button, in points; wider or taller templates get a larger button.
const PALETTE_BUTTON: Vec2 = Vec2::new(40.0, 48.0);
/// Space inside a palette button around its template, in points.
const PALETTE_INSET: Vec2 = Vec2::new(12.0, 8.0);
/// Space between palette buttons, in points.
const PALETTE_GAP: Vec2 = Vec2::new(4.0, 4.0);
/// Height of the palette: two rows of buttons, so every category fits the same box (the
/// matrices, the tallest templates, fit in one taller row).
const PALETTE_HEIGHT: f32 = PALETTE_BUTTON.y * 2.0 + PALETTE_GAP.y;

/// What the user decided in the formula editor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MathEditorOutcome {
    /// Insert or update the formula.
    Submit {
        /// TeX source, without delimiters.
        tex: String,
        /// Whether the formula is a display block.
        display: bool,
    },
    /// Close without changing the document.
    Cancel,
}

/// The open formula editor.
pub struct MathEditorState {
    editor: Editor,
    display: bool,
    editing: bool,
    category: usize,
    preview: PreviewCache,
    /// The last hover lookup, keyed by source and byte offset; lookups lay the formula out
    /// several times, so they are not repeated while the pointer rests on one character.
    hover: Option<((String, usize), Option<mdview_media::MathHighlight>)>,
}

impl MathEditorState {
    /// Open the editor on `tex`. `editing` labels the primary button Update instead of Insert.
    pub fn new(tex: &str, display: bool, editing: bool) -> Self {
        let document = Document::from_loaded(
            PathBuf::from("formula.tex"),
            tex.to_owned(),
            Eol::Lf,
            Encoding::Utf8,
            false,
        );
        let mut editor = Editor::new(document);
        editor.set_wrap(true);
        editor.set_identity(u64::MAX - 7);
        match next_placeholder(tex, 0) {
            Some(range) => {
                let start = editor.document().byte_to_char(range.start);
                let end = editor.document().byte_to_char(range.end);
                editor.select(start, end);
            }
            None => {
                let end = editor.document().len_chars();
                editor.move_cursor_to(end);
            }
        }
        editor.request_focus();
        Self {
            editor,
            display,
            editing,
            category: 0,
            preview: PreviewCache::default(),
            hover: None,
        }
    }

    /// The TeX source being edited.
    pub fn tex(&self) -> String {
        self.editor.document().snapshot()
    }

    /// Whether the formula will be written as a display block.
    pub fn display(&self) -> bool {
        self.display
    }

    /// Insert a palette template at the cursor and select its first placeholder.
    pub fn insert_template(&mut self, template: &str) {
        let start = self.editor.insert_snippet(template);
        let start_byte = self.editor.document().char_to_byte(start);
        let tex = self.tex();
        if let Some(range) =
            next_placeholder(&tex, start_byte).filter(|range| range.start < start_byte + template.len())
        {
            let document = self.editor.document();
            let (start, end) = (document.byte_to_char(range.start), document.byte_to_char(range.end));
            self.editor.select(start, end);
        }
    }

    /// Select the placeholder after the cursor, wrapping to the first. Returns false when
    /// the source has none.
    pub fn select_next_placeholder(&mut self) -> bool {
        let tex = self.tex();
        let (_, end) = self.editor.selection_range();
        let from = self.editor.document().char_to_byte(end);
        let Some(range) = next_placeholder(&tex, from) else {
            return false;
        };
        let document = self.editor.document();
        let (start, end) = (document.byte_to_char(range.start), document.byte_to_char(range.end));
        self.editor.select(start, end);
        self.editor.request_focus();
        true
    }
}

/// Show the formula editor. `highlight` is the theme accent used for the hovered part.
pub fn show_math_editor(
    ctx: &Context,
    state: &mut MathEditorState,
    highlight: Color32,
    t: &dyn Fn(&str) -> String,
) -> Option<MathEditorOutcome> {
    let mut outcome = None;
    let dark = ctx.global_style().visuals.dark_mode;
    state.editor.set_light(!dark);
    // Claim the editor's keys before it reads them: Tab jumps between placeholders
    // instead of indenting, and Cmd+Enter confirms.
    let editor_focused = state.editor.has_focus();
    if editor_focused
        && state.tex().contains(PLACEHOLDER)
        && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Tab))
    {
        state.select_next_placeholder();
    }
    if ctx.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Enter)) {
        outcome = Some(submit(state));
    }
    let modal = Modal::new(Id::unique("mdview-math-editor"))
        .frame(crate::prompt::dialog_frame(crate::prompt::ui_frame(ctx)))
        .show(ctx, |ui| {
            ui.set_width(DIALOG_WIDTH);
            header(ui, state, t, &mut outcome);
            ui.add_space(8.0);
            palette(ui, state, dark, t);
            ui.add_space(10.0);
            body(ui, state, dark, highlight, t);
            ui.add_space(12.0);
            footer(ui, state, t, &mut outcome);
        });
    if modal.should_close() && outcome.is_none() {
        outcome = Some(MathEditorOutcome::Cancel);
    }
    outcome
}

fn submit(state: &MathEditorState) -> MathEditorOutcome {
    MathEditorOutcome::Submit {
        tex: state.tex().trim().to_owned(),
        display: state.display,
    }
}

fn header(ui: &mut Ui, state: &MathEditorState, t: &dyn Fn(&str) -> String, outcome: &mut Option<MathEditorOutcome>) {
    ui.horizontal(|ui| {
        let title = if state.editing {
            t("Edit equation")
        } else {
            t("Equation editor")
        };
        ui.label(RichText::new(title).size(17.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close = ui.add(egui::Button::new(RichText::new("×").size(14.0)).frame(false));
            if close.on_hover_text(t("Cancel")).clicked() {
                *outcome = Some(MathEditorOutcome::Cancel);
            }
        });
    });
}

fn palette(ui: &mut Ui, state: &mut MathEditorState, dark: bool, t: &dyn Fn(&str) -> String) {
    ui.horizontal_wrapped(|ui| {
        for (index, category) in PALETTE.iter().enumerate() {
            if ui.selectable_label(state.category == index, t(category.name)).clicked() {
                state.category = index;
            }
        }
    });
    ui.add_space(4.0);
    let Some(category) = PALETTE.get(state.category) else {
        return;
    };
    let mut picked = None;
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(6)
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // A fixed height (two rows of buttons) keeps the dialog the same size for
            // every category; longer categories scroll inside it.
            ui.set_height(PALETTE_HEIGHT);
            egui::ScrollArea::vertical()
                .id_salt("mdview-math-palette")
                .max_height(PALETTE_HEIGHT)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = PALETTE_GAP;
                        for template in category.templates {
                            if template_button(ui, template, dark).clicked() {
                                picked = Some(*template);
                            }
                        }
                    });
                });
        });
    if let Some(template) = picked {
        state.insert_template(template);
    }
}

/// A palette button drawn with the formula it inserts, sized to the template.
fn template_button(ui: &mut Ui, template: &str, dark: bool) -> egui::Response {
    // No size limit: every template renders at the same type size.
    let formula = preview::template(ui.ctx(), template, dark, Vec2::splat(f32::INFINITY), PALETTE_EM).ok();
    let size = formula.as_ref().map_or(PALETTE_BUTTON, |formula| {
        (formula.size() + PALETTE_INSET).max(PALETTE_BUTTON)
    });
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, template));
    let visuals = ui.visuals();
    let fill = if response.is_pointer_button_down_on() {
        visuals.widgets.active.weak_bg_fill
    } else if response.hovered() || response.has_focus() {
        visuals.widgets.hovered.weak_bg_fill
    } else {
        visuals.widgets.inactive.weak_bg_fill
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(1.5, visuals.selection.stroke.color),
            egui::StrokeKind::Inside,
        );
    }
    if let Some(formula) = formula {
        preview::paint(ui, &formula, rect, egui::Align::Center, Color32::WHITE);
    }
    response.on_hover_text(template)
}

fn body(ui: &mut Ui, state: &mut MathEditorState, dark: bool, highlight: Color32, t: &dyn Fn(&str) -> String) {
    let gap = 12.0;
    let half = (ui.available_width() - gap) / 2.0;
    let mut hovered = None;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.vertical(|ui| {
            ui.set_width(half);
            caption(ui, &t("Source"));
            ui.allocate_ui_with_layout(
                egui::vec2(half, BODY_HEIGHT),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_min_size(egui::vec2(half, BODY_HEIGHT));
                    state.editor.show(ui);
                },
            );
            hovered = state.editor.hovered_char();
        });
        ui.vertical(|ui| {
            ui.set_width(half);
            caption(ui, &t("Preview"));
            show_preview(ui, state, hovered, dark, highlight, half);
        });
    });
    ui.add_space(4.0);
    let hint = format!(
        "{} · {}",
        t("Hover the source to find its part in the formula"),
        t("Tab moves to the next placeholder")
    );
    ui.label(RichText::new(hint).small().color(ui.visuals().weak_text_color()));
}

fn caption(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).small().color(ui.visuals().weak_text_color()));
    ui.add_space(2.0);
}

/// Paint the live preview, highlighting the formula part under the hovered source.
fn show_preview(
    ui: &mut Ui,
    state: &mut MathEditorState,
    hovered: Option<usize>,
    dark: bool,
    highlight: Color32,
    width: f32,
) {
    let tex = state.tex();
    let marked = hovered.and_then(|index| {
        let key = (tex.clone(), state.editor.document().char_to_byte(index));
        if let Some((cached, marked)) = &state.hover
            && *cached == key
        {
            return marked.clone();
        }
        let marked = mdview_media::highlight_math_at(&tex, key.1, &preview::tex_color(highlight));
        state.hover = Some((key, marked.clone()));
        marked
    });
    let range = marked.as_ref().map(|marked| {
        let document = state.editor.document();
        (
            document.byte_to_char(marked.span.start),
            document.byte_to_char(marked.span.end),
        )
    });
    state.editor.set_marked_range(range, highlight);
    let source = marked.map_or(tex.clone(), |marked| marked.tex);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, BODY_HEIGHT), Sense::hover());
    let visuals = ui.visuals();
    ui.painter().rect_filled(rect, 6.0, visuals.extreme_bg_color);
    ui.painter().rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
    let inner = rect.shrink(14.0);
    if tex.trim().is_empty() {
        return;
    }
    match state.preview.get(ui.ctx(), &source, dark, inner.size(), PREVIEW_EM) {
        Ok(formula) => {
            if source == tex {
                state.preview.remember_good(&formula);
            }
            preview::paint(ui, &formula, inner, egui::Align::Center, Color32::WHITE);
        }
        Err(error) => {
            if let Some(formula) = state.preview.last_good() {
                let dimmed = Color32::WHITE.gamma_multiply(0.35);
                preview::paint(ui, formula, inner, egui::Align::Center, dimmed);
            }
            let error_color = ui.visuals().error_fg_color;
            ui.painter().text(
                rect.left_bottom() + egui::vec2(10.0, -10.0),
                egui::Align2::LEFT_BOTTOM,
                error,
                egui::FontId::proportional(12.0),
                error_color,
            );
        }
    }
}

fn footer(
    ui: &mut Ui,
    state: &mut MathEditorState,
    t: &dyn Fn(&str) -> String,
    outcome: &mut Option<MathEditorOutcome>,
) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.display, false, t("Inline"));
        ui.selectable_value(&mut state.display, true, t("Display"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let confirm = if state.editing { t("Update") } else { t("Insert") };
            let shortcut = if cfg!(target_os = "macos") {
                "⌘↩"
            } else {
                "Ctrl+Enter"
            };
            let primary = crate::primary_button(ui, &confirm, false).on_hover_text(shortcut);
            if primary.clicked() && !state.tex().trim().is_empty() {
                *outcome = Some(submit(state));
            }
            ui.add_space(8.0);
            if crate::secondary_button(ui, &t("Cancel")).clicked() {
                *outcome = Some(MathEditorOutcome::Cancel);
            }
        });
    });
}

#[cfg(test)]
mod tests;
