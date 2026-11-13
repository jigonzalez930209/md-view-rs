//! The eframe application shell.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use craft_dialogs::{Dialogs, FileDialog, Message, MessageButtons, MessageLevel};
use craft_settings::{Location, SettingsStore};
use eframe::egui;
use mdview_i18n::{LanguagePreference, Translations};
use mdview_theme::ThemeMode;
use mdview_ui::CloseChoice;
use mdview_workspace::Workspace;

use crate::cli::CliAction;
use crate::document_ops::DiffState;
use crate::media_runtime::{MediaPolicy, MediaRuntime};
use crate::settings::Prefs;
use crate::sidebar::Sidebar;
use crate::single_instance::Primary;
use crate::tabs::TabSet;

/// The continuation attached to a native dialog request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DialogThen {
    Open,
    OpenFolder,
    SaveAs,
    SaveAsClose,
    ReloadConflict(usize),
    KeepMineConflict(usize),
    Ignored,
}

/// The MD View application.
pub struct MdViewApp {
    pub(crate) prefs: SettingsStore<Prefs>,
    pub(crate) translations: Translations,
    pub(crate) lang: craft_i18n::Lang,
    pub(crate) theme: ThemeMode,
    pub(crate) language: LanguagePreference,
    pub(crate) tabs: TabSet,
    pub(crate) dialogs: Dialogs<DialogThen>,
    pub(crate) about_open: bool,
    pub(crate) close_prompt_open: bool,
    pub(crate) allow_close: bool,
    pub(crate) window_title: String,
    pub(crate) primary: Option<Primary>,
    pub(crate) pending_load: Option<crate::load_job::LoadJob>,
    pub(crate) workspace: Option<Workspace>,
    pub(crate) sidebar: Sidebar,
    pub(crate) diff: Option<DiffState>,
    pub(crate) last_file_poll: Instant,
    pub(crate) last_window_poll: Instant,
    pub(crate) media: MediaRuntime,
    pub(crate) lightbox: Option<crate::lightbox::LightboxState>,
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

        let tabs = TabSet::new(
            theme == ThemeMode::Light,
            snapshot.editor.wrap,
            snapshot.editor.line_numbers,
        );
        let sidebar = Sidebar::new(
            snapshot.session.sidebar_width,
            snapshot.session.show_hidden,
            snapshot.session.respect_gitignore,
        );

        let mut app = Self {
            prefs,
            translations,
            lang,
            theme,
            language,
            tabs,
            dialogs,
            about_open: false,
            close_prompt_open: false,
            allow_close: false,
            window_title: String::new(),
            primary,
            pending_load: None,
            workspace: None,
            sidebar,
            diff: None,
            last_file_poll: Instant::now(),
            last_window_poll: Instant::now(),
            media: MediaRuntime::new(&cc.egui_ctx),
            lightbox: None,
        };

        mdview_theme::apply(&cc.egui_ctx, theme);
        let media = snapshot.media.clone();
        app.media.set_policy(MediaPolicy {
            remote_images: media.remote_images,
            private_networks: media.private_networks,
        });
        let width = snapshot.window.width.clamp(640.0, 10_000.0);
        let height = snapshot.window.height.clamp(420.0, 10_000.0);
        cc.egui_ctx
            .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(width, height)));

        match startup {
            CliAction::Run { paths } if !paths.is_empty() => {
                for path in &paths {
                    app.handle_external_path(path, &cc.egui_ctx);
                }
            }
            _ => app.restore_session(cc),
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

    pub(crate) fn ask_open(&mut self) {
        self.dialogs.ask(
            FileDialog::open()
                .title(self.t("Open file"))
                .filter("Markdown", &["md", "markdown"])
                .filter("Text", &["txt"])
                .filter(
                    "Source",
                    &[
                        "rs", "py", "js", "ts", "tsx", "jsx", "json", "toml", "yaml", "yml", "html", "css", "scss",
                        "xml", "sql", "sh", "c", "h", "cpp", "hpp", "cs", "go", "java", "kt", "swift", "php", "rb",
                        "lua",
                    ],
                ),
            DialogThen::Open,
        );
    }

    pub(crate) fn ask_open_folder(&mut self) {
        self.dialogs.ask(
            FileDialog::folder().title(self.t("Open folder")),
            DialogThen::OpenFolder,
        );
    }

    pub(crate) fn ask_save_as(&mut self, close_after: bool) {
        let name = if self.tabs.editor().document().path().is_some() {
            self.tabs.editor().document().display_name()
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

    pub(crate) fn update_title(&mut self, ctx: &egui::Context) {
        let marker = if self.tabs.any_dirty() { " •" } else { "" };
        let name = self.tabs.editor().document().display_name();
        let title = format!("MD View — {name}{marker}");
        if title != self.window_title {
            self.window_title.clone_from(&title);
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }
    }

    pub(crate) fn handle_close_request(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| input.viewport().close_requested()) && !self.allow_close {
            if self.tabs.any_dirty() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.close_prompt_open = true;
            } else {
                self.allow_close = true;
            }
        }
        craft_quit::on_close_request(ctx);
    }

    pub(crate) fn show_close_prompt(&mut self, ctx: &egui::Context) {
        let choice = mdview_ui::show_close_prompt(ctx, &self.translations, self.lang, &mut self.close_prompt_open);
        match choice {
            Some(CloseChoice::Save) => self.continue_close(ctx),
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
