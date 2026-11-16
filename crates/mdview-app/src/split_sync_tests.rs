//! Frame-level split synchronization tests with real editor and preview widgets.

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use egui_kittest::{Harness, kittest::Queryable};
use mdview_editor::Editor;
use mdview_model::{Document, Encoding, Eol};
use mdview_render::Preview;
use mdview_theme::{Theme, ThemeId};

use super::*;
use crate::split::{choose_source, follow_leader};

const EDITOR_X: f32 = 200.0;
const PREVIEW_X: f32 = 650.0;

fn markdown() -> String {
    let mut text = String::new();
    for section in 0..40 {
        text.push_str(&format!("## Section {section}\n\n"));
        let words = "word ".repeat(10 + (section * 7) % 60);
        text.push_str(&format!("Paragraph {section} {words}\n\n"));
        if section % 3 == 0 {
            text.push_str("```rust\nfn main() {\n    println!(\"hi\");\n}\n```\n\n");
        }
        if section % 4 == 1 {
            text.push_str("- one\n- two\n- three\n\n");
        }
    }
    text.push_str("Zzfinalmarker\n");
    text
}

/// Per-frame stored offsets after the sync step: `(editor, editor max, preview, preview max)`.
///
/// egui applies wheel input at the end of a scroll area's frame, so the editor value is
/// what it paints next frame. The preview value is what it painted this frame; its new
/// target is painted next frame, together with the editor. Follower values therefore
/// trail the leader by exactly one record.
type Frames = Rc<RefCell<Vec<(f32, f32, f32, f32)>>>;

/// The editor's distance from the offset the map assigns to the preview's offset.
type LockError = Rc<RefCell<f32>>;

fn split_harness() -> (Harness<'static>, Frames, LockError) {
    split_harness_with(false)
}

fn split_harness_with(wrap: bool) -> (Harness<'static>, Frames, LockError) {
    let source = markdown();
    let document = Document::from_loaded(
        PathBuf::from("split.md"),
        source.clone(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    );
    let mut editor = Editor::new(document);
    editor.set_identity(7);
    editor.set_wrap(wrap);
    editor.set_split_mode(true);
    let mut preview = Preview::new();
    let mut leader = None;
    let mut leader_at_end = false;
    let theme = Theme::for_id(ThemeId::GithubLight);
    let frames: Frames = Rc::default();
    let record = frames.clone();
    let lock_error: LockError = Rc::default();
    let record_error = lock_error.clone();
    let harness = Harness::builder()
        .with_size(egui::vec2(900.0, 420.0))
        .build_ui(move |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let size = egui::vec2(ui.available_width() * 0.5, ui.available_height());
                ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Min), |ui| {
                    editor.show(ui);
                });
                ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Min), |ui| {
                    preview.show(ui, &source, &theme, true);
                });
            });
            let hover = ui.ctx().input(|input| input.pointer.hover_pos());
            let hovered = match hover {
                Some(position) if editor.viewport_rect().contains(position) => Some(Pane::Editor),
                Some(position) if preview.viewport_rect().contains(position) => Some(Pane::Preview),
                _ => None,
            };
            let source_pane = choose_source(editor.take_user_scroll(), preview.take_user_scroll(), hovered);
            let previous_leader = leader;
            leader = follow_leader(source_pane, leader, false);
            if previous_leader != leader || source_pane.is_some() {
                leader_at_end = match leader {
                    Some(Pane::Editor) => editor.is_at_end(),
                    Some(Pane::Preview) => preview.is_at_end(),
                    None => false,
                };
            }
            if let Some(pane) = leader
                && !keep_leader_at_end(&mut editor, &mut preview, pane, leader_at_end, source_pane, false)
            {
                leader_at_end = match pane {
                    Pane::Editor => editor.is_at_end(),
                    Pane::Preview => preview.is_at_end(),
                };
                sync_frame(ui.ctx(), &mut editor, &mut preview, source_pane, pane);
            }
            PREVIEW_VIEWPORT.with(|cell| cell.set(preview.viewport_rect()));
            EDITOR_VIEWPORT.with(|cell| cell.set(editor.viewport_rect()));
            let map = ScrollMap::from_layout(
                &editor.line_tops(),
                preview.source_map(),
                editor.max_scroll_offset(),
                preview.max_scroll_offset(),
            );
            *record_error.borrow_mut() =
                (map.editor_for_preview(preview.scroll_offset()) - editor.scroll_offset()).abs();
            // No frame may paint a pane past its end: that frame is the visible flicker.
            assert_eq!(editor.overshoot_paints(), 0, "the editor painted past its end");
            assert_eq!(preview.overshoot_paints(), 0, "the preview painted past its end");
            record.borrow_mut().push((
                editor.scroll_offset(),
                editor.max_scroll_offset(),
                preview.scroll_offset(),
                preview.max_scroll_offset(),
            ));
        });
    // The same pass budget as the app (see `configure_passes`).
    configure_passes(&harness.ctx);
    (harness, frames, lock_error)
}

thread_local! {
    /// The preview viewport from the last pass, in points, for pixel measurements.
    static PREVIEW_VIEWPORT: std::cell::Cell<egui::Rect> = const { std::cell::Cell::new(egui::Rect::NOTHING) };
    /// The editor viewport from the last pass, in points, for pixel measurements.
    static EDITOR_VIEWPORT: std::cell::Cell<egui::Rect> = const { std::cell::Cell::new(egui::Rect::NOTHING) };
}

/// The pixels of a pane's viewport, without its scrollbar strips.
fn pane_pixels(image: &image::RgbaImage, viewport: egui::Rect, pixels_per_point: f32) -> Vec<u8> {
    let scale = |value: f32| (value * pixels_per_point).round().max(0.0) as u32;
    let (width, height) = image.dimensions();
    let (left, right) = (
        scale(viewport.left()),
        scale(viewport.right()).min(width).saturating_sub(16),
    );
    let (top, bottom) = (
        scale(viewport.top()),
        scale(viewport.bottom()).min(height).saturating_sub(16),
    );
    let mut pixels = Vec::new();
    for y in top..bottom {
        for x in left..right {
            pixels.extend_from_slice(&image.get_pixel(x, y).0);
        }
    }
    pixels
}

fn wheel(harness: &mut Harness<'_>, x: f32, delta_y: f32, frames: usize) {
    harness.hover_at(egui::pos2(x, 200.0));
    for _ in 0..frames {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, delta_y),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::NONE,
        });
        harness.step();
    }
    for _ in 0..6 {
        harness.step();
    }
}

/// Assert one pane follows the other without ever moving against the wheel.
fn assert_monotonic(frames: &[(f32, f32, f32, f32)], down: bool) {
    for pair in frames.windows(2) {
        let (before, after) = (pair[0], pair[1]);
        for (from, to, max, name) in [
            (before.0, after.0, after.1, "editor"),
            (before.2, after.2, after.3, "preview"),
        ] {
            assert!(to <= max + 0.5, "{name} offset {to} is past its max {max}");
            let backwards = if down { to < from - 0.5 } else { to > from + 0.5 };
            assert!(
                !backwards,
                "{name} moved against the wheel: {from} -> {to} ({frames:?})"
            );
        }
    }
}

#[test]
fn editor_wheel_to_the_end_keeps_the_preview_locked_without_flicker() {
    let (mut harness, frames, _) = split_harness();
    harness.run();
    frames.borrow_mut().clear();
    // Far more wheel input than the document needs, so the editor keeps receiving
    // wheel events at its end: the follower must hold perfectly still there.
    wheel(&mut harness, EDITOR_X, -60.0, 400);
    let frames = frames.borrow();
    assert_monotonic(&frames, true);
    let last = frames.last().copied().unwrap_or_default();
    assert!(last.1 > 0.0 && last.3 > 0.0, "both panes must scroll: {last:?}");
    assert!(
        (last.0 - last.1).abs() <= 0.005,
        "editor did not reach its end: {last:?}"
    );
    assert!(
        (last.2 - last.3).abs() <= 0.005,
        "preview did not reach its end: {last:?}"
    );
    let held = frames
        .iter()
        .rev()
        .take_while(|frame| (frame.0 - last.0).abs() <= 0.5)
        .count();
    // Skip the first hold record: the preview's end target is painted in the next frame.
    let still = frames
        .iter()
        .rev()
        .take(held.saturating_sub(1))
        .all(|frame| (frame.2 - last.2).abs() <= 0.01);
    assert!(held > 10 && still, "preview moved while the editor held its end");
}

#[test]
fn preview_wheel_to_the_end_and_back_keeps_the_editor_locked() {
    let (mut harness, frames, _) = split_harness();
    harness.run();
    frames.borrow_mut().clear();
    wheel(&mut harness, PREVIEW_X, -60.0, 400);
    {
        let frames = frames.borrow();
        assert_monotonic(&frames, true);
        let last = frames.last().copied().unwrap_or_default();
        assert!(
            (last.0 - last.1).abs() <= 0.005,
            "editor did not reach its end: {last:?}"
        );
        assert!(
            (last.2 - last.3).abs() <= 0.005,
            "preview did not reach its end: {last:?}"
        );
    }
    frames.borrow_mut().clear();
    wheel(&mut harness, PREVIEW_X, 60.0, 400);
    let frames = frames.borrow();
    assert_monotonic(&frames, false);
    let last = frames.last().copied().unwrap_or_default();
    assert!(
        last.0 <= 0.5 && last.2 <= 0.5,
        "both panes must return to the top: {last:?}"
    );
}

#[test]
fn relayout_relocks_the_editor_to_a_preview_leader() {
    // Soft wrap makes the editor re-flow too, so its offset alone no longer matches.
    let (mut harness, frames, lock_error) = split_harness_with(true);
    harness.run();
    wheel(&mut harness, PREVIEW_X, -60.0, 60);
    let before = frames.borrow().last().copied().unwrap_or_default();
    assert!(
        before.2 > 0.0 && *lock_error.borrow() <= 0.005,
        "not locked before re-layout: {before:?}"
    );
    // Zoom and split resizes both re-flow the panes in points; a resize is the
    // re-layout the test harness can drive deterministically.
    for width in [1300.0, 980.0, 1600.0] {
        harness.set_size(egui::vec2(width, 420.0));
        for _ in 0..10 {
            harness.step();
        }
        let error = *lock_error.borrow();
        let last = frames.borrow().last().copied().unwrap_or_default();
        assert!(
            error <= 0.005,
            "editor drifted {error} points at width {width}: {last:?}"
        );
    }
}

/// Where the preview actually painted its final paragraph, read from the widget tree.
fn final_marker_top(harness: &Harness<'_>) -> Option<f32> {
    harness
        .query_by_label_contains("Zzfinalmarker")
        .map(|node| node.rect().top())
}

#[test]
fn the_preview_never_paints_past_its_end_while_the_editor_leads() {
    let (mut harness, _, _) = split_harness();
    harness.run();
    harness.hover_at(egui::pos2(EDITOR_X, 200.0));
    let mut tops = Vec::new();
    for _ in 0..400 {
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -60.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: egui::Modifiers::NONE,
        });
        harness.step();
        if let Some(top) = final_marker_top(&harness) {
            tops.push(top);
        }
    }
    for _ in 0..6 {
        harness.step();
        tops.extend(final_marker_top(&harness));
    }
    assert!(tops.len() > 20, "the final paragraph was never painted");
    let resting = tops.last().copied().unwrap_or_default();
    // An out-of-range offset paints the content above its resting place for a frame
    // and then snaps back down: that is the end-of-document flicker.
    for pair in tops.windows(2) {
        assert!(pair[1] <= pair[0] + 0.5, "the preview jumped back down: {pair:?}");
    }
    let highest = tops.iter().copied().fold(f32::MAX, f32::min);
    assert!(
        highest >= resting - 0.5,
        "painted {highest} above its resting {resting}"
    );
}

/// Apply an app zoom the way the real window experiences it: the window keeps its size
/// in pixels, so the area in points shrinks or grows by the zoom factor.
fn zoom(harness: &mut Harness<'_>, factor: f32) {
    harness.ctx.set_zoom_factor(factor);
    harness.set_size(egui::vec2(900.0 / factor, 420.0 / factor));
    for _ in 0..12 {
        harness.step();
    }
}

#[test]
fn zoom_keeps_both_panes_locked_for_either_leader() {
    for leader_x in [EDITOR_X, PREVIEW_X] {
        let (mut harness, frames, lock_error) = split_harness_with(true);
        harness.run();
        wheel(&mut harness, leader_x, -60.0, 50);
        for factor in [1.3, 1.6, 0.8, 1.1, 1.0] {
            let before = frames.borrow().last().copied().unwrap_or_default();
            zoom(&mut harness, factor);
            let after = frames.borrow().last().copied().unwrap_or_default();
            let error = *lock_error.borrow();
            assert!(after != before, "zoom {factor} did not re-flow the panes: {after:?}");
            assert!(
                error <= 0.005,
                "panes drifted {error} points at zoom {factor} (leader x={leader_x}): {after:?}"
            );
        }
        // Scroll to the end at a zoomed size: no flicker, both ends reached together.
        zoom(&mut harness, 1.3);
        frames.borrow_mut().clear();
        wheel(&mut harness, leader_x * 0.9 / 1.3 + 20.0, -60.0, 400);
        let frames = frames.borrow();
        assert_monotonic(&frames, true);
        let last = frames.last().copied().unwrap_or_default();
        assert!(
            (last.0 - last.1).abs() <= 0.005,
            "editor not at its end at zoom 1.3: {last:?}"
        );
        assert!(
            (last.2 - last.3).abs() <= 0.005,
            "preview not at its end at zoom 1.3: {last:?}"
        );
    }
}

#[test]
fn zooming_at_the_end_keeps_both_panes_at_the_end_without_overshoot() {
    for leader_x in [EDITOR_X, PREVIEW_X] {
        let (mut harness, frames, lock_error) = split_harness_with(false);
        harness.run();
        wheel(&mut harness, leader_x, -60.0, 400);
        // The harness closure asserts that no frame paints a pane past its end.
        for factor in [1.3, 1.0, 0.8, 1.6, 0.9, 1.0] {
            zoom(&mut harness, factor);
            let last = frames.borrow().last().copied().unwrap_or_default();
            assert!(
                (last.0 - last.1).abs() <= 0.005,
                "editor left its end at zoom {factor}: {last:?}"
            );
            assert!(
                (last.2 - last.3).abs() <= 0.005,
                "preview left its end at zoom {factor}: {last:?}"
            );
            assert!(
                *lock_error.borrow() <= 0.005,
                "panes drifted at zoom {factor}: {last:?}"
            );
        }
    }
}

/// Blank pixel rows between the preview's last painted content and its viewport bottom.
///
/// At the end of the document the final paragraph sits just above the bottom, so this
/// is constant for a given zoom. A frame shown at the wrong offset (past the end, or
/// short of it after a re-layout) changes it for that one frame: a visible jump.
fn preview_blank_bottom(image: &image::RgbaImage, pixels_per_point: f32) -> u32 {
    let viewport = PREVIEW_VIEWPORT.with(std::cell::Cell::get);
    let scale = |value: f32| (value * pixels_per_point).round().max(0.0) as u32;
    let (width, height) = image.dimensions();
    // Skip the scrollbar on the right and the frame edge at the bottom.
    let (left, right) = (
        scale(viewport.left()) + 8,
        scale(viewport.right()).min(width).saturating_sub(16),
    );
    let (top, bottom) = (
        scale(viewport.top()),
        scale(viewport.bottom()).min(height).saturating_sub(4),
    );
    let background = *image.get_pixel(right.saturating_sub(4), (top + bottom) / 2);
    let ink = |pixel: &image::Rgba<u8>| {
        pixel
            .0
            .iter()
            .zip(background.0)
            .take(3)
            .any(|(a, b)| a.abs_diff(b) > 40)
    };
    (top..bottom)
        .rev()
        .take_while(|&y| (left..right).all(|x| !ink(image.get_pixel(x, y))))
        .count() as u32
}

#[test]
fn rendered_frames_never_jump_while_zooming_at_the_end() {
    for (wrap, leader_x) in [
        (false, EDITOR_X),
        (false, PREVIEW_X),
        (true, EDITOR_X),
        (true, PREVIEW_X),
    ] {
        let (mut harness, _, _) = split_harness_with(wrap);
        harness.run();
        wheel(&mut harness, leader_x, -60.0, 400);
        for factor in [1.3, 1.0, 0.8, 1.6, 0.9, 1.0] {
            harness.ctx.set_zoom_factor(factor);
            harness.set_size(egui::vec2(900.0 / factor, 420.0 / factor));
            let mut strips = Vec::new();
            let mut panes = Vec::new();
            for frame in 0..8 {
                harness.step();
                let image = harness
                    .render()
                    .unwrap_or_else(|error| panic!("render failed: {error}"));
                if let Ok(dir) = std::env::var("DUMP_FRAMES") {
                    let _ = image.save(format!("{dir}/x{leader_x}-z{factor}-f{frame}.png"));
                }
                let ppp = harness.ctx.pixels_per_point();
                strips.push(preview_blank_bottom(&image, ppp));
                panes.push((
                    pane_pixels(&image, EDITOR_VIEWPORT.with(std::cell::Cell::get), ppp),
                    pane_pixels(&image, PREVIEW_VIEWPORT.with(std::cell::Cell::get), ppp),
                ));
            }
            // The panes come to rest: no re-lock keeps requesting repaints (kittest fails
            // `run` when frames never stop repainting).
            harness.run();
            // Both panes look exactly like the settled frame on every shown frame.
            if let Some(settled) = panes.last() {
                for (frame, pane) in panes.iter().enumerate() {
                    assert!(
                        pane.0 == settled.0,
                        "zoom {factor} (wrap={wrap} leader x={leader_x}): editor frame {frame} differs"
                    );
                    assert!(
                        pane.1 == settled.1,
                        "zoom {factor} (wrap={wrap} leader x={leader_x}): preview frame {frame} differs"
                    );
                }
            }
            // Every shown frame, starting with the first one after the zoom, must already
            // be at the settled end position.
            let settled = strips.last().copied().unwrap_or_default();
            assert!(
                strips.iter().all(|strip| strip.abs_diff(settled) <= 1),
                "zoom {factor} (wrap={wrap} leader x={leader_x}) showed a jump: blank rows per frame {strips:?}"
            );
        }
    }
}
