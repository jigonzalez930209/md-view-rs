//! The eframe application shell.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use craft_dialogs::{Answer, Dialogs, FileDialog, Message, MessageButtons, MessageLevel};
use craft_keymap::Chord;
use craft_settings::{Location, SaveStatus, SettingsStore};
use eframe::egui;
use mdview_editor::Editor;
use mdview_i18n::{LanguagePreference, Translations};
use mdview_model::{Document, Encoding};
use mdview_theme::ThemeMode;
use mdview_ui::{CloseChoice, Command};

use crate::cli::CliAction;
use crate::file_ops;
use crate::settings::Prefs;
use crate::single_instance::Primary;

/// The continuation attached to a native dialog request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DialogThen {
    Open,
    SaveAs,
    SaveAsClose,
    Ignored,
}

/// The MD View application.
pub struct MdViewApp {
    pub(crate) prefs: SettingsStore<Prefs>,
    translations: Translations,
    lang: craft_i18n::Lang,
    pub(crate) theme: ThemeMode,
    language: LanguagePreference,
    pub(crate) editor: Editor,
    dialogs: Dialogs<DialogThen>,
    about_open: bool,
    close_prompt_open: bool,
    allow_close: bool,
    startup_error: Option<String>,
    window_title: String,
    primary: Option<Primary>,
    pub(crate) pending_load: Option<crate::load_job::LoadJob>,
}

impl MdViewApp {
    /// Build the application and apply persisted settings.
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        startup: CliAction,
        primary: Option<Primary>,
        wake_slot: Arc<Mutex<Option<egui::Context>>>,
    ) -> Self {
        if let Ok(mut slot) = wake_slot.lock() {
            *slot = Some(cc.egui_ctx.clone());
        }
        let (prefs, report) = SettingsStore::<Prefs>::open(Location::Auto);
        if report.has_problems() {
            log::warn!("settings: {report:?}");
        }
        let snapshot = prefs.get().clone();
        let theme = snapshot.theme_mode();
        let language = snapshot.language_preference();
        let translations = Translations::new();
        let lang = translations.lang(language);
        let dialogs = {
            let ctx = cc.egui_ctx.clone();
            Dialogs::new(Arc::new(move || ctx.request_repaint()))
        };

        let mut editor = Editor::new(Document::untitled());
        if !editor.status().large_mode {
            editor.set_wrap(snapshot.editor.wrap);
        }
        editor.set_line_numbers(snapshot.editor.line_numbers);
        editor.set_light(theme == ThemeMode::Light);

        let mut app = Self {
            prefs,
            translations,
            lang,
            theme,
            language,
            editor,
            dialogs,
            about_open: false,
            close_prompt_open: false,
            allow_close: false,
            startup_error: None,
            window_title: String::new(),
            primary,
            pending_load: None,
        };

        mdview_theme::apply(&cc.egui_ctx, theme);

        if let CliAction::Run { path: Some(path) } = startup
            && let Err(error) = app.load_document(&path)
        {
            app.startup_error = Some(format!(
                "{} {}",
                app.translations.t(app.lang, "Could not open file."),
                error
            ));
        }

        app.update_title(&cc.egui_ctx);
        app
    }

    pub(crate) fn t(&self, source: &str) -> String {
        self.translations.t(self.lang, source)
    }

    pub(crate) fn show_error(&mut self, description: impl Into<String>) {
        self.dialogs.ask_message(
            Message {
                level: MessageLevel::Error,
                title: self.t("Error").to_owned(),
                description: description.into(),
                buttons: MessageButtons::Ok,
            },
            DialogThen::Ignored,
        );
    }

    fn open_from_dialog(&mut self, path: &Path, ctx: &egui::Context) {
        if self.editor.document().is_dirty() {
            self.show_error(self.t("Save or discard your changes first."));
            return;
        }
        match self.load_document(path) {
            Ok(()) => self.update_title(ctx),
            Err(error) => self.show_error(format!("{} {error}", self.t("Could not open file."))),
        }
    }

    fn ask_open(&mut self) {
        self.dialogs.ask(
            FileDialog::open()
                .title(self.t("Open file"))
                .filter("Markdown", &["md", "markdown"])
                .filter("Text", &["txt"]),
            DialogThen::Open,
        );
    }

    fn ask_save_as(&mut self, close_after: bool) {
        let name = if self.editor.document().path().is_some() {
            self.editor.document().display_name()
        } else {
            "untitled.md".to_owned()
        };
        self.dialogs.ask(
            FileDialog::save()
                .title(self.t("Save file"))
                .filter("Markdown", &["md", "markdown"])
                .filter("Text", &["txt"])
                .file_name(name),
            if close_after {
                DialogThen::SaveAsClose
            } else {
                DialogThen::SaveAs
            },
        );
    }

    fn request_save(&mut self, close_after: bool) {
        let Some(path) = self.editor.document().path().map(PathBuf::from) else {
            self.ask_save_as(close_after);
            return;
        };
        self.save_to(&path, close_after, None);
    }

    fn save_to(&mut self, path: &Path, close_after: bool, ctx: Option<&egui::Context>) {
        match file_ops::save(self.editor.document(), path) {
            Ok(()) => {
                self.editor.document_mut().set_path(path.to_path_buf());
                self.editor.document_mut().mark_saved();
                if let Some(ctx) = ctx {
                    self.update_title(ctx);
                }
                if close_after {
                    self.close_prompt_open = false;
                    self.allow_close = true;
                    if let Some(ctx) = ctx {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
            Err(error) => {
                self.show_error(format!("{} {error}", self.t("Could not save file.")));
            }
        }
    }

    fn new_document(&mut self, ctx: &egui::Context) {
        if self.editor.document().is_dirty() {
            self.show_error(self.t("Save or discard your changes first."));
            return;
        }
        self.editor.replace_document(Document::untitled());
        self.update_title(ctx);
    }

    fn handle_command(&mut self, ctx: &egui::Context, command: Command) {
        match command {
            Command::New => self.new_document(ctx),
            Command::Open => {
                if self.editor.document().is_dirty() {
                    self.show_error(self.t("Save or discard your changes first."));
                } else {
                    self.ask_open();
                }
            }
            Command::Save => self.request_save(false),
            Command::SaveAs => self.ask_save_as(false),
            Command::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Command::About => self.about_open = true,
            Command::Find => self.editor.open_find(false),
            Command::Replace => self.editor.open_find(true),
            Command::GotoLine => self.editor.open_goto(),
            Command::ToggleWrap => {
                self.editor.toggle_wrap();
                let wrap = self.editor.status().wrap;
                self.prefs.update(|prefs| prefs.editor.wrap = wrap);
            }
            Command::ToggleLineNumbers => {
                self.editor.toggle_line_numbers();
                let line_numbers = self.editor.status().line_numbers;
                self.prefs.update(|prefs| prefs.editor.line_numbers = line_numbers);
            }
            Command::ReloadEncoding(encoding) => self.reload_encoding(ctx, encoding),
            Command::SetTheme(mode) => {
                self.theme = mode;
                self.editor.set_light(mode == ThemeMode::Light);
                self.prefs.update(|prefs| prefs.set_theme_mode(mode));
                mdview_theme::apply(ctx, mode);
            }
            Command::SetLanguage(preference) => {
                self.language = preference;
                self.lang = self.translations.lang(preference);
                self.prefs.update(|prefs| prefs.set_language_preference(preference));
            }
        }
    }

    fn reload_encoding(&mut self, ctx: &egui::Context, encoding: Encoding) {
        if self.editor.document().is_dirty() {
            self.show_error(self.t("Save or discard your changes first."));
            return;
        }
        let Some(path) = self.editor.document().path().map(PathBuf::from) else {
            return;
        };
        match file_ops::load_with_encoding(&path, encoding) {
            Ok(document) => {
                self.editor.replace_document(document);
                self.editor.set_light(self.theme == ThemeMode::Light);
                self.update_title(ctx);
            }
            Err(error) => self.show_error(format!("{} {error}", self.t("Could not open file."))),
        }
    }

    fn handle_dialog_answers(&mut self, ctx: &egui::Context) {
        for (then, answer) in self.dialogs.take_answers() {
            match (then, answer) {
                (DialogThen::Open, Answer::Paths(paths)) => {
                    if let Some(path) = paths.first() {
                        self.open_from_dialog(path, ctx);
                    }
                }
                (DialogThen::SaveAs, Answer::Paths(paths)) => {
                    if let Some(path) = paths.first() {
                        self.save_to(path, false, Some(ctx));
                    }
                }
                (DialogThen::SaveAsClose, Answer::Paths(paths)) => {
                    if let Some(path) = paths.first() {
                        self.save_to(path, true, Some(ctx));
                    }
                }
                (_, Answer::Unavailable(reason)) => {
                    self.show_error(reason);
                }
                _ => {}
            }
        }
    }

    fn consume_shortcut(&self, ctx: &egui::Context, text: &str) -> bool {
        let Ok(chord) = Chord::parse(text) else {
            log::warn!("invalid shortcut in application code: {text}");
            return false;
        };
        ctx.input_mut(|input| input.consume_shortcut(&chord.to_egui_shortcut()))
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.dialogs.is_open() || self.close_prompt_open {
            return;
        }
        if self.consume_shortcut(ctx, "Cmd+N") {
            self.new_document(ctx);
        } else if self.consume_shortcut(ctx, "Cmd+O") {
            if !self.editor.document().is_dirty() {
                self.ask_open();
            }
        } else if self.consume_shortcut(ctx, "Cmd+S") {
            self.request_save(false);
        } else if self.consume_shortcut(ctx, "Cmd+Shift+S") {
            self.ask_save_as(false);
        } else if self.consume_shortcut(ctx, "Cmd+Q") {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    pub(crate) fn update_title(&mut self, ctx: &egui::Context) {
        let marker = if self.editor.document().is_dirty() { " •" } else { "" };
        let title = format!("MD View — {}{marker}", self.editor.document().display_name());
        if title != self.window_title {
            self.window_title.clone_from(&title);
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| input.viewport().close_requested()) && !self.allow_close {
            if self.editor.document().is_dirty() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.close_prompt_open = true;
            } else {
                self.allow_close = true;
            }
        }
        craft_quit::on_close_request(ctx);
    }

    fn show_close_prompt(&mut self, ctx: &egui::Context) {
        let choice = mdview_ui::show_close_prompt(ctx, &self.translations, self.lang, &mut self.close_prompt_open);
        match choice {
            Some(CloseChoice::Save) => self.request_save(true),
            Some(CloseChoice::Discard) => {
                self.close_prompt_open = false;
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Some(CloseChoice::Cancel) => self.close_prompt_open = false,
            None => {}
        }
    }
}

impl eframe::App for MdViewApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_load(ctx);
        let forwarded = self.primary.as_ref().map(Primary::take_paths).unwrap_or_default();
        for path in forwarded {
            if path.as_os_str().is_empty() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else if self.editor.document().is_dirty() {
                self.show_error(self.t("Save or discard your changes first."));
            } else if let Err(error) = self.load_document(&path) {
                self.show_error(format!("{} {error}", self.t("Could not open file.")));
            } else {
                self.update_title(ctx);
            }
        }

        self.handle_dialog_answers(ctx);
        if let Some(error) = self.startup_error.take() {
            self.show_error(error);
        }

        let tick = self.prefs.tick(Instant::now());
        if let Some(deadline) = tick.next_deadline {
            ctx.request_repaint_after(deadline.saturating_duration_since(Instant::now()));
        }
        if let SaveStatus::Failed(error) = tick.status {
            log::error!("settings save failed: {error}");
        }

        self.handle_shortcuts(ctx);
        self.handle_close_request(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.update_title(ui.ctx());

        let mut command = None;
        egui::Panel::top("menu_bar").show(ui, |ui| {
            let status = self.editor.status();
            command = mdview_ui::show_menu(
                ui,
                mdview_ui::MenuState {
                    translations: &self.translations,
                    lang: self.lang,
                    theme: self.theme,
                    language: self.language,
                    document_name: &self.editor.document().display_name(),
                    dirty: self.editor.document().is_dirty(),
                    wrap: status.wrap,
                    line_numbers: status.line_numbers,
                },
            );
        });
        if let Some(command) = command {
            self.handle_command(ui.ctx(), command);
        }

        egui::Panel::bottom("status_bar").show(ui, |ui| {
            mdview_ui::show_status(
                ui,
                &self.translations,
                self.lang,
                self.editor.document(),
                &self.editor.status(),
            );
        });

        let mut open_requested = false;
        egui::CentralPanel::default().show(ui, |ui| {
            if self.editor.document().path().is_none() && self.editor.document().is_empty() {
                open_requested = mdview_ui::show_welcome(ui, &self.translations, self.lang);
            } else {
                self.editor.show(ui);
            }
        });
        if open_requested {
            self.ask_open();
        }

        if self.about_open {
            mdview_ui::show_about(ui.ctx(), &self.translations, self.lang, &mut self.about_open);
        }
        if self.close_prompt_open {
            self.show_close_prompt(ui.ctx());
        }
        if let Some(job) = &self.pending_load {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
            let cancel = mdview_ui::show_loading(ui.ctx(), &self.translations, self.lang, job.path(), job.progress());
            if cancel && let Some(job) = self.pending_load.take() {
                job.cancel();
            }
        }

        self.dialogs.show_pending(Some(&*frame));
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        mdview_theme::Theme::for_mode(self.theme)
            .background
            .to_normalized_gamma_f32()
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Err(error) = self.prefs.flush() {
            log::error!("settings flush failed: {error}");
        }
    }
}
