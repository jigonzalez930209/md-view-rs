//! The grid view of delimited files (CSV, TSV, …): a spreadsheet view of the document text.
//!
//! The grid edits the tab's own document. Each grid action becomes minimal text edits from
//! `mdview-table`, applied through the editor as one undo step, so saving, undo, external
//! changes, and the unsaved-changes prompt behave exactly as for text.

use std::path::Path;

use eframe::egui;
use mdview_editor::Editor;
use mdview_table::{Dialect, Table, TextEdit};
use mdview_ui::{GridAction, GridState};

/// Grid state and the table parsed for one document revision.
#[derive(Default)]
pub(crate) struct GridView {
    pub(crate) state: GridState,
    parsed: Option<(u64, Table)>,
}

impl GridView {
    /// Whether a document opens in the grid: a delimited extension, within the grid size limit.
    pub(crate) fn applies(path: Option<&Path>, bytes: usize) -> bool {
        path.is_some_and(mdview_table::is_table_path) && bytes <= mdview_table::MAX_GRID_BYTES
    }

    /// Parse the editor's text again if it changed since the last parse.
    fn refresh(&mut self, editor: &Editor) {
        let document = editor.document();
        let revision = document.revision();
        if self.parsed.as_ref().is_none_or(|(parsed, _)| *parsed != revision) {
            let text = document.snapshot();
            let dialect = Dialect::sniff(&text, document.path());
            self.parsed = Some((revision, Table::parse(&text, dialect)));
        }
    }

    /// Paint the grid for the editor's document and apply what the user changed.
    ///
    /// Actions are applied one at a time, each against a table parsed from the text the
    /// previous one produced, so byte ranges never go stale.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui, editor: &mut Editor, t: &dyn Fn(&str) -> String) {
        self.refresh(editor);
        let Some((_, table)) = &self.parsed else {
            return;
        };
        let actions = mdview_ui::show_grid(ui, table, &mut self.state, t);
        for action in actions {
            log::debug!("grid action {action:?}");
            self.refresh(editor);
            if let Some((_, table)) = &self.parsed {
                apply(editor, table, action);
            }
        }
    }
}

impl crate::app::MdViewApp {
    /// Paint the grid of tab `index` with translated menu labels.
    pub(crate) fn show_grid_pane(&mut self, ui: &mut egui::Ui, index: usize) {
        let (translations, lang) = (&self.translations, self.lang);
        let t = |source: &str| translations.t(lang, source);
        let Some(tab) = self.tabs.tab_mut(index) else {
            return;
        };
        if let Some(grid) = tab.grid.as_mut() {
            grid.show(ui, &mut tab.editor, &t);
        }
    }

    /// Whether the active tab's grid has keyboard focus (F2 then edits a cell).
    pub(crate) fn grid_focused(&self) -> bool {
        let tab = self.tabs.active();
        tab.mode != mdview_ui::ViewMode::Edit && tab.grid.as_ref().is_some_and(|grid| grid.state.has_focus())
    }
}

/// Apply one grid action to the editor's document.
fn apply(editor: &mut Editor, table: &Table, action: GridAction) {
    let text = editor.document().snapshot();
    let edits: Vec<TextEdit> = match action {
        GridAction::SetBlock { row, column, values } => table.set_block(&text, row, column, &values),
        GridAction::InsertRows { at, count } => table.insert_rows(at, count),
        GridAction::DeleteRows { start, end } => table.delete_rows(start..end),
        GridAction::InsertColumns { at, count } => table.insert_columns(&text, at, count),
        GridAction::DeleteColumns { start, end } => table.delete_columns(&text, start..end),
        GridAction::Undo => {
            editor.undo();
            return;
        }
        GridAction::Redo => {
            editor.redo();
            return;
        }
    };
    let document = editor.document();
    let edits: Vec<(usize, usize, String)> = edits
        .into_iter()
        .map(|edit| {
            (
                document.byte_to_char(edit.range.start),
                document.byte_to_char(edit.range.end),
                edit.text,
            )
        })
        .collect();
    editor.apply_edits(&edits);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use mdview_model::{Document, Encoding, Eol};

    use super::*;

    fn editor(text: &str) -> Editor {
        Editor::new(Document::from_loaded(
            PathBuf::from("data.csv"),
            text.to_owned(),
            Eol::Lf,
            Encoding::Utf8,
            false,
        ))
    }

    fn parsed(view: &GridView) -> Table {
        view.parsed
            .as_ref()
            .map(|(_, table)| table.clone())
            .unwrap_or_else(|| Table::parse("", Dialect::default()))
    }

    #[test]
    fn grid_edits_change_the_document_as_one_undo_step() {
        let mut editor = editor("name,city\nAda,London\nAlan,Wilmslow\n");
        let mut view = GridView::default();
        view.refresh(&editor);
        let table = parsed(&view);
        apply(
            &mut editor,
            &table,
            GridAction::SetBlock {
                row: 1,
                column: 1,
                values: vec![vec!["Paris, FR".to_owned()]],
            },
        );
        assert_eq!(
            editor.document().snapshot(),
            "name,city\nAda,\"Paris, FR\"\nAlan,Wilmslow\n"
        );
        assert!(editor.document().is_dirty());
        view.refresh(&editor);
        let table = parsed(&view);
        assert_eq!(table.value(1, 1), Some("Paris, FR"), "parsed again after the edit");
        apply(&mut editor, &table, GridAction::DeleteColumns { start: 0, end: 1 });
        assert_eq!(editor.document().snapshot(), "city\n\"Paris, FR\"\nWilmslow\n");
        apply(&mut editor, &table, GridAction::Undo);
        assert_eq!(
            editor.document().snapshot(),
            "name,city\nAda,\"Paris, FR\"\nAlan,Wilmslow\n"
        );
    }

    #[test]
    fn multibyte_text_maps_bytes_to_characters() {
        let mut editor = editor("ñ,ü\nä,ö\n");
        let mut view = GridView::default();
        view.refresh(&editor);
        apply(&mut editor, &parsed(&view), GridAction::InsertRows { at: 1, count: 1 });
        assert_eq!(editor.document().snapshot(), "ñ,ü\n,\nä,ö\n");
    }

    #[test]
    fn every_grid_label_is_translated() {
        let translations = mdview_i18n::Translations::new();
        let spanish = translations.lang(mdview_i18n::LanguagePreference::Spanish);
        for label in mdview_ui::GRID_LABELS {
            assert_ne!(translations.t(spanish, label), *label, "untranslated: {label}");
        }
    }

    #[test]
    fn only_delimited_files_within_the_limit_open_as_grids() {
        assert!(GridView::applies(Some(Path::new("a.csv")), 10));
        assert!(GridView::applies(Some(Path::new("a.tsv")), 10));
        assert!(!GridView::applies(Some(Path::new("a.md")), 10));
        assert!(!GridView::applies(
            Some(Path::new("a.csv")),
            mdview_table::MAX_GRID_BYTES + 1
        ));
    }
}
