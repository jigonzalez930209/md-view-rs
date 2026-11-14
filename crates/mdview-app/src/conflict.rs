//! Dirty-file conflict handling.
//!
//! When the watcher detects that a dirty document changed on disk, the editor stops guessing and
//! offers three explicit choices: reload the disk version, keep the local buffer, or compare both
//! versions before deciding.

use eframe::egui;
use mdview_i18n::Translations;
use similar::{ChangeTag, TextDiff};

/// A choice made in the conflict bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictAction {
    /// Replace the buffer with the disk contents.
    Reload,
    /// Keep the buffer and overwrite the disk contents on the next save.
    KeepMine,
    /// Open the comparison window.
    Compare,
}

/// The disk state for one open document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskState {
    /// Nothing changed since the last load or save.
    Unchanged,
    /// The backing file is missing.
    Missing,
    /// The file changed and the buffer is clean: reload it.
    Reload,
    /// The file changed while the buffer has unsaved edits.
    Conflict,
}

/// Classify a document's disk state from existence, modification time, and dirty state.
pub fn classify_disk(exists: bool, changed: bool, dirty: bool) -> DiskState {
    if !exists {
        DiskState::Missing
    } else if !changed {
        DiskState::Unchanged
    } else if dirty {
        DiskState::Conflict
    } else {
        DiskState::Reload
    }
}

/// A choice made when the backing file is missing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingAction {
    /// Save the buffer under a new path.
    SaveAs,
    /// Keep editing the buffer without recreating the file yet.
    KeepEditing,
}

/// Draw the missing-file banner.
pub fn show_missing_bar(
    ui: &mut egui::Ui,
    document_name: &str,
    translations: &Translations,
    lang: craft_i18n::Lang,
) -> Option<MissingAction> {
    let t = |source: &str| translations.t(lang, source);
    let mut action = None;
    egui::Frame::new()
        .fill(ui.visuals().warn_fg_color.gamma_multiply(0.12))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} {document_name}", t("The file no longer exists on disk.")))
                        .color(ui.visuals().warn_fg_color),
                );
                if ui.button(t("Save As…")).clicked() {
                    action = Some(MissingAction::SaveAs);
                }
                if ui.button(t("Keep editing")).clicked() {
                    action = Some(MissingAction::KeepEditing);
                }
            });
        });
    action
}

/// Draw the in-editor conflict banner.
pub fn show_conflict_bar(
    ui: &mut egui::Ui,
    document_name: &str,
    translations: &Translations,
    lang: craft_i18n::Lang,
) -> Option<ConflictAction> {
    let t = |source: &str| translations.t(lang, source);
    let mut action = None;
    egui::Frame::new()
        .fill(ui.visuals().warn_fg_color.gamma_multiply(0.12))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} {document_name}",
                        t("This file changed on disk while you had unsaved changes.")
                    ))
                    .color(ui.visuals().warn_fg_color),
                );
                if ui.button(t("Reload from disk")).clicked() {
                    action = Some(ConflictAction::Reload);
                }
                if ui.button(t("Keep my changes")).clicked() {
                    action = Some(ConflictAction::KeepMine);
                }
                if ui.button(t("Compare")).clicked() {
                    action = Some(ConflictAction::Compare);
                }
            });
        });
    action
}

/// Draw the comparison window. `open` is cleared when the window closes.
pub fn show_diff_window(ctx: &egui::Context, document_name: &str, mine: &str, disk: &str, open: &mut bool) {
    egui::Window::new(format!("Compare — {document_name}"))
        .open(open)
        .default_size(egui::vec2(860.0, 560.0))
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Mine").strong().color(ui.visuals().text_color()));
                ui.separator();
                ui.label(egui::RichText::new("Disk").strong().color(ui.visuals().text_color()));
            });
            ui.separator();
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                let diff = TextDiff::from_lines(mine, disk);
                for (shown, change) in diff.iter_all_changes().enumerate() {
                    if shown >= 4_000 {
                        ui.label("… diff truncated …");
                        break;
                    }
                    let (prefix, color) = match change.tag() {
                        ChangeTag::Delete => ("- ", ui.visuals().error_fg_color),
                        ChangeTag::Insert => ("+ ", ui.visuals().hyperlink_color),
                        ChangeTag::Equal => ("  ", ui.visuals().weak_text_color()),
                    };
                    let text = change.to_string();
                    ui.monospace(egui::RichText::new(format!("{prefix}{}", text.trim_end_matches('\n'))).color(color));
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use mdview_i18n::{LanguagePreference, Translations};

    use super::*;

    #[test]
    fn conflict_banner_snapshot() {
        let translations = Translations::new();
        let lang = translations.lang(LanguagePreference::English);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(780.0, 70.0))
            .build_ui(move |ui| {
                show_conflict_bar(ui, "notes.md", &translations, lang);
            });
        harness.run();
        harness.snapshot("conflict-banner");
    }

    #[test]
    fn conflict_diff_snapshot() {
        let mut open = true;
        let mut harness = Harness::builder()
            .with_size(egui::vec2(820.0, 480.0))
            .build_ui(move |ui| {
                show_diff_window(
                    ui.ctx(),
                    "notes.md",
                    "alpha\nlocal change\nomega\n",
                    "alpha\nexternal change\nomega\n",
                    &mut open,
                );
            });
        harness.run();
        harness.snapshot("conflict-diff");
    }

    #[test]
    fn classifies_clean_dirty_and_missing_files() {
        assert_eq!(classify_disk(true, false, false), DiskState::Unchanged);
        assert_eq!(classify_disk(true, false, true), DiskState::Unchanged);
        assert_eq!(classify_disk(true, true, false), DiskState::Reload);
        assert_eq!(classify_disk(true, true, true), DiskState::Conflict);
        assert_eq!(classify_disk(false, true, false), DiskState::Missing);
        assert_eq!(classify_disk(false, true, true), DiskState::Missing);
    }
}
