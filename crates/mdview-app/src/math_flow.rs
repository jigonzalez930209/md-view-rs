//! The formula editor session: opening it from the formatting bar, the editor, or the
//! preview, and writing the result back into the document it came from.

use eframe::egui;
use mdview_editor::{Editor, MathTarget};
use mdview_render::MathActivation;
use mdview_ui::{FormatBarAction, MathEditorOutcome, MathEditorState};

use crate::app::MdViewApp;

/// Why the formula editor opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MathOpen {
    /// A new formula, optionally starting from a template.
    New(Option<&'static str>),
    /// An existing formula in the document.
    Edit(MathTarget),
}

/// An open formula editor and the document it writes to.
pub(crate) struct MathSession {
    state: MathEditorState,
    /// Identity of the editor (tab) that receives the formula.
    editor: u64,
    target: Option<MathTarget>,
}

/// Apply a formatting-bar equation action to `editor`; return a request to open the dialog.
pub(crate) fn equation_action(editor: &mut Editor, action: FormatBarAction) -> Option<MathOpen> {
    match action {
        FormatBarAction::Format(action) => {
            editor.apply_markdown(action);
            None
        }
        FormatBarAction::InsertEquation(tex) => {
            editor.insert_math(tex, true);
            None
        }
        FormatBarAction::EditEquation(Some(template)) => Some(MathOpen::New(Some(template))),
        // The editor button edits the formula at the cursor, or starts a new one.
        FormatBarAction::EditEquation(None) => Some(
            editor
                .math_at(editor.cursor())
                .map_or(MathOpen::New(None), MathOpen::Edit),
        ),
    }
}

impl MdViewApp {
    /// Whether the formula editor owns the keyboard.
    pub(crate) fn math_editor_open(&self) -> bool {
        self.math_editor.is_some()
    }

    /// Open the formula editor for the editor with identity `editor`.
    pub(crate) fn open_math_editor(&mut self, editor: u64, open: MathOpen) {
        log::debug!("math editor open {open:?}");
        let (state, target) = match open {
            MathOpen::New(template) => (MathEditorState::new(template.unwrap_or(""), true, false), None),
            MathOpen::Edit(target) => (MathEditorState::new(&target.tex, target.display, true), Some(target)),
        };
        self.math_editor = Some(MathSession { state, editor, target });
    }

    /// Open the formula the user double-clicked or chose to edit in the active preview.
    pub(crate) fn open_math_from_preview(&mut self, activation: &MathActivation) {
        let editor = self.tabs.editor();
        let text = editor.document().snapshot();
        let wanted = activation.tex.trim();
        let formulas = mdview_markdown::math_sources(&text);
        let in_block = |formula: &&mdview_markdown::MathSource| {
            activation
                .block
                .as_ref()
                .is_none_or(|block| formula.range.start >= block.start && formula.range.end <= block.end)
        };
        // Prefer the formula inside the reported block; fall back to the first match in
        // case the preview painted an older revision.
        let found = formulas
            .iter()
            .filter(in_block)
            .find(|formula| formula.tex.trim() == wanted)
            .or_else(|| formulas.iter().find(|formula| formula.tex.trim() == wanted));
        let Some(formula) = found else {
            log::warn!("math editor: preview formula not found in source");
            return;
        };
        let start = editor.document().byte_to_char(formula.range.start);
        if let Some(target) = editor.math_at(start) {
            // The lightbox may have opened on the first click of the double-click.
            self.lightbox = None;
            let identity = editor.identity();
            self.open_math_editor(identity, MathOpen::Edit(target));
        }
    }

    /// Show the formula editor and apply its outcome.
    pub(crate) fn show_math_editor_flow(&mut self, ctx: &egui::Context) {
        let Some(session) = self.math_editor.as_mut() else {
            return;
        };
        let accent = mdview_theme::Theme::for_id(self.effective_theme).accent;
        let (translations, lang) = (&self.translations, self.lang);
        let t = |source: &str| translations.t(lang, source);
        let Some(outcome) = mdview_ui::show_math_editor(ctx, &mut session.state, accent, &t) else {
            return;
        };
        let Some(session) = self.math_editor.take() else {
            return;
        };
        let MathEditorOutcome::Submit { tex, display } = outcome else {
            self.focus_math_target(session.editor);
            return;
        };
        let Some(index) = self
            .tabs
            .tabs()
            .iter()
            .position(|tab| tab.editor.identity() == session.editor)
        else {
            log::warn!("math editor: the target tab was closed");
            return;
        };
        let Some(tab) = self.tabs.tab_mut(index) else {
            return;
        };
        let editor = &mut tab.editor;
        // Replace the original formula only if it is still where it was; an external
        // reload may have moved it, and then the formula is inserted at the cursor.
        let current = session.target.as_ref().and_then(|target| editor.math_at(target.start));
        match (session.target, current) {
            (Some(target), Some(current)) if current == target => editor.replace_math(&target, &tex, display),
            _ => editor.insert_math(&tex, display),
        }
        ctx.request_repaint();
    }

    fn focus_math_target(&mut self, identity: u64) {
        if let Some(index) = self
            .tabs
            .tabs()
            .iter()
            .position(|tab| tab.editor.identity() == identity)
            && let Some(tab) = self.tabs.tab_mut(index)
        {
            tab.editor.request_focus();
        }
    }
}

#[cfg(test)]
mod tests {
    use mdview_editor::Editor;
    use mdview_i18n::{LanguagePreference, Translations};
    use mdview_model::{Document, Encoding, Eol};
    use mdview_ui::FormatBarAction;

    use super::{MathOpen, equation_action};

    #[test]
    fn every_equation_label_is_translated() {
        let translations = Translations::new();
        let spanish = translations.lang(LanguagePreference::Spanish);
        let labels = mdview_ui::FORMAT_BAR_LABELS
            .iter()
            .chain(mdview_ui::MATH_EDITOR_LABELS)
            .copied()
            .chain(mdview_ui::GALLERY.iter().map(|equation| equation.name))
            .chain(mdview_ui::PALETTE.iter().map(|category| category.name))
            .chain(["Edit equation", "Equation Editor…"]);
        for label in labels {
            // "Matrices" is spelled the same in Spanish; every other label must change.
            let translated = translations.t(spanish, label);
            assert!(translated != label || label == "Matrices", "untranslated: {label}");
        }
    }

    #[test]
    fn the_editor_button_edits_the_formula_at_the_cursor() {
        let document = Document::from_loaded(
            "notes.md".into(),
            "a $x^2$ b".to_owned(),
            Eol::Lf,
            Encoding::Utf8,
            false,
        );
        let mut editor = Editor::new(document);
        editor.move_cursor_to(4);
        let open = equation_action(&mut editor, FormatBarAction::EditEquation(None));
        assert!(matches!(open, Some(MathOpen::Edit(target)) if target.tex == "x^2"));
        editor.move_cursor_to(0);
        let open = equation_action(&mut editor, FormatBarAction::EditEquation(None));
        assert_eq!(open, Some(MathOpen::New(None)));
        assert_eq!(equation_action(&mut editor, FormatBarAction::InsertEquation("y")), None);
        assert!(editor.document().snapshot().contains("$$\ny\n$$"));
    }
}
