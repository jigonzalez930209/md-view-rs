//! The editor pane: the Markdown formatting bar (for Markdown files) above the editor.

use eframe::egui;
use mdview_editor::Editor;
use mdview_ui::MaterialIcons;

use crate::math_flow::MathOpen;

/// A shortcut as the platform writes it: `⌘B` on macOS, `Ctrl+B` elsewhere.
pub(crate) fn chord_label(chord: &str) -> String {
    if cfg!(target_os = "macos") {
        chord.replace("Cmd+", "⌘").replace("Shift+", "⇧")
    } else {
        chord.replace("Cmd+", "Ctrl+")
    }
}

/// The formatting bar's labels in the current language, resolved before borrowing the tab.
pub(crate) fn format_labels(
    translations: &mdview_i18n::Translations,
    lang: craft_i18n::Lang,
) -> impl Fn(&str) -> String + use<> {
    let labels: std::collections::HashMap<&'static str, String> = mdview_ui::FORMAT_BAR_LABELS
        .iter()
        .chain(mdview_ui::GALLERY.iter().map(|equation| &equation.name))
        .map(|label| (*label, translations.t(lang, label)))
        .collect();
    move |source: &str| labels.get(source).cloned().unwrap_or_else(|| source.to_owned())
}

/// Show the formatting bar when the document is Markdown, apply its action, and show the
/// editor below it.
///
/// Returns a request to open the formula editor (from the bar or a double-click on a
/// formula) together with the identity of the editor that receives the formula.
pub(crate) fn show_editor_pane(
    ui: &mut egui::Ui,
    editor: &mut Editor,
    icons: &mut MaterialIcons,
    t: &dyn Fn(&str) -> String,
    input_blocked: bool,
) -> Option<(u64, MathOpen)> {
    let mut open = None;
    if editor.is_markdown() {
        let state = editor.markdown_state();
        let action = mdview_ui::show_format_bar(ui, icons, state, t, &chord_label);
        if let Some(action) = action
            && !input_blocked
        {
            log::debug!("format action={action:?}");
            open = crate::math_flow::equation_action(editor, action);
        }
    }
    editor.show(ui);
    let open = open.or_else(|| editor.take_math_request().map(MathOpen::Edit));
    open.filter(|_| !input_blocked).map(|open| (editor.identity(), open))
}
