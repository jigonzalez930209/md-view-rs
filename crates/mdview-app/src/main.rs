//! MD View entry point.

mod app;
mod cli;
mod commands;
mod conflict;
mod document_ops;
mod file_ops;
mod install;
mod lightbox;
mod links;
mod load_job;
mod logging;
mod media_runtime;
mod palette_flow;
mod perf;
mod reading;
mod settings;
mod sidebar;
mod single_instance;
mod split;
mod tab_bar;
mod tabs;
mod ui_shell;
mod update_ui;
mod updater;
mod workspace_flow;
mod workspace_ops;

use std::sync::{Arc, Mutex};

use cli::CliAction;
use eframe::egui;
use single_instance::Acquired;

fn main() -> eframe::Result {
    logging::init();
    log::info!(
        "startup version={} args={:?} log={:?}",
        env!("CARGO_PKG_VERSION"),
        std::env::args().skip(1).collect::<Vec<_>>(),
        logging::active_log_path()
    );

    let action = match cli::parse(std::env::args().skip(1)) {
        Ok(action) => action,
        Err(error) => {
            eprintln!("error: {error}\n\n{}", cli::HELP);
            std::process::exit(2);
        }
    };

    match &action {
        CliAction::Help => {
            print!("{}", cli::HELP);
            return Ok(());
        }
        CliAction::Version => {
            println!("mdview {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        CliAction::Run { .. } => {}
    }

    let wake_slot: Arc<Mutex<Option<egui::Context>>> = Arc::new(Mutex::new(None));
    let primary = if let CliAction::Run { paths } = &action {
        let wake_slot = wake_slot.clone();
        let wake = move || {
            if let Ok(slot) = wake_slot.lock()
                && let Some(ctx) = slot.as_ref()
            {
                ctx.request_repaint();
            }
        };
        match single_instance::acquire(paths, wake) {
            Ok(Acquired::Primary(primary)) => Some(primary),
            Ok(Acquired::Forwarded) => return Ok(()),
            Err(error) => {
                log::warn!("single-instance coordination unavailable: {error}");
                None
            }
        }
    } else {
        None
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 720.0])
            .with_min_inner_size([640.0, 420.0])
            .with_decorations(false)
            .with_title("MD View"),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "MD View",
        options,
        Box::new(move |cc| Ok(Box::new(app::MdViewApp::new(cc, action, primary, wake_slot)))),
    )
}
