//! 120 Hz frame-budget regression test for the editor and preview.
//!
//! This measures CPU time spent inside the two widgets, not wall time waiting for vsync. Run in
//! release mode with:
//!
//! ```sh
//! cargo test -p mdview-render --release --test frame_budget -- --nocapture
//! ```
//!
//! A 120 Hz frame has 8.33 ms of budget. Debug builds use a relaxed assertion so the normal test
//! suite remains usable; `cargo xtask perf` runs the release gate.

#![allow(clippy::expect_used)]

use std::{cell::RefCell, rc::Rc, time::Duration};

use egui_kittest::Harness;
use mdview_editor::Editor;
use mdview_model::{Document, Encoding, Eol};
use mdview_render::Preview;
use mdview_theme::Theme;

const FRAMES: usize = 120;

fn large_markdown() -> String {
    let mut source = String::from("# Frame budget\n\n");
    for index in 0..500 {
        source.push_str(&format!(
            "## Section {index}\n\nParagraph {index} with **strong**, *emphasis*, `code`, and a [link](#section-{index}).\n\n| Key | Value |\n|---|---|\n| a | b |\n| c | d |\n\n```rust\nfn section_{index}() {{ let value = {index}; }}\n```\n\n"
        ));
    }
    source
}

fn percentile(samples: &[Duration], percent: f64) -> Duration {
    let mut sorted = samples.to_vec();
    sorted.sort();
    let index = ((sorted.len() - 1) as f64 * percent).round() as usize;
    sorted[index]
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

#[test]
fn editor_and_preview_stay_within_120hz_budget() {
    let source = large_markdown();
    let document = Document::from_loaded(
        std::path::PathBuf::from("frame-budget.md"),
        source.clone(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    );
    let editor = Rc::new(RefCell::new(Editor::new(document)));
    let preview = Rc::new(RefCell::new(Preview::default()));
    preview.borrow_mut().set_width(560.0);
    let theme = Theme::light();
    let (editor_handle, preview_handle) = (editor.clone(), preview.clone());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 800.0))
        .build_ui(move |ui| {
            egui::Panel::right("budget_preview").default_size(560.0).show(ui, |ui| {
                preview_handle.borrow_mut().show(ui, &source, &theme, true);
            });
            editor_handle.borrow_mut().show(ui);
        });

    for _ in 0..10 {
        harness.run();
    }
    let mut editor_times = Vec::with_capacity(FRAMES);
    let mut preview_times = Vec::with_capacity(FRAMES);
    let mut combined = Vec::with_capacity(FRAMES);
    for frame in 0..FRAMES {
        let line = frame * 38;
        editor.borrow_mut().scroll_to_line(line, 0.0);
        preview.borrow_mut().scroll_to_line(line, 0.0);
        harness.run();
        let editor_time = editor.borrow().last_frame_time();
        let preview_time = preview.borrow().last_frame_time();
        editor_times.push(editor_time);
        preview_times.push(preview_time);
        combined.push(editor_time + preview_time);
    }
    let editor_p50 = percentile(&editor_times, 0.50);
    let editor_p95 = percentile(&editor_times, 0.95);
    let preview_p50 = percentile(&preview_times, 0.50);
    let preview_p95 = percentile(&preview_times, 0.95);
    let combined_p50 = percentile(&combined, 0.50);
    let combined_p95 = percentile(&combined, 0.95);
    let max = *combined.iter().max().expect("samples");
    println!(
        "120Hz frame budget ({} frames): editor p50={:.3}ms p95={:.3}ms; preview p50={:.3}ms p95={:.3}ms; combined p50={:.3}ms p95={:.3}ms max={:.3}ms",
        FRAMES,
        millis(editor_p50),
        millis(editor_p95),
        millis(preview_p50),
        millis(preview_p95),
        millis(combined_p50),
        millis(combined_p95),
        millis(max),
    );
    let budget = if cfg!(debug_assertions) { 100.0 } else { 8.33 };
    assert!(
        millis(combined_p95) <= budget,
        "combined p95 {:.3}ms exceeds {:.2}ms",
        millis(combined_p95),
        budget
    );
}
