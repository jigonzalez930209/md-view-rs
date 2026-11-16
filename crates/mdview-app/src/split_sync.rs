//! Continuous scroll mapping between the split editor and preview.
//!
//! The map is a monotonic piecewise-linear function built from matching
//! `(editor offset, preview offset)` pairs: the top of each preview block and the
//! top of its first source line, plus the bottom of the block and the row after its
//! last source line. Both ends are pinned, `(0, 0)` and `(editor max, preview max)`,
//! and anchors that fall past either pane's maximum are dropped. The result is that
//!
//! - every target is inside the follower's scrollable range, so egui never paints an
//!   out-of-range offset for a frame and snaps back (the end-of-document flicker);
//! - both panes reach their ends on the same frame instead of one pane jumping when
//!   the other one hits its end;
//! - the map is invertible, so either pane can lead without drift.

use mdview_editor::Editor;
use mdview_render::{Preview, SourceMap};

use crate::split::Pane;

/// Anchors closer than this in either axis are merged into the previous anchor.
const MIN_STEP: f32 = 0.5;

/// A monotonic editor <-> preview offset map.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ScrollMap {
    /// `(editor, preview)` points, strictly increasing on both axes.
    points: Vec<(f32, f32)>,
}

impl ScrollMap {
    /// Build a map from candidate anchors and both panes' maximum offsets.
    pub(crate) fn new(anchors: impl IntoIterator<Item = (f32, f32)>, editor_max: f32, preview_max: f32) -> Self {
        let editor_max = finite_or_zero(editor_max).max(0.0);
        let preview_max = finite_or_zero(preview_max).max(0.0);
        let mut points = vec![(0.0, 0.0)];
        for (editor, preview) in anchors {
            let Some(&(last_editor, last_preview)) = points.last() else {
                continue;
            };
            let inside = editor < editor_max - MIN_STEP && preview < preview_max - MIN_STEP;
            let increasing = editor > last_editor + MIN_STEP && preview > last_preview + MIN_STEP;
            if editor.is_finite() && preview.is_finite() && inside && increasing {
                points.push((editor, preview));
            }
        }
        points.push((editor_max, preview_max));
        Self { points }
    }

    /// Build the map from the editor's line tops and the preview's block layout.
    ///
    /// `line_tops[n]` is the content-space top of zero-based line `n`; the extra final
    /// entry is the bottom of the last line.
    pub(crate) fn from_layout(line_tops: &[f32], source_map: &SourceMap, editor_max: f32, preview_max: f32) -> Self {
        let line_top = |line: usize| line_tops.get(line).or(line_tops.last()).copied().unwrap_or(0.0);
        let anchors = source_map.blocks.iter().flat_map(|block| {
            // Block lines are one-based and inclusive.
            let start = line_top(block.start_line.saturating_sub(1));
            let end = line_top(block.end_line);
            [(start, block.top), (end, block.top + block.height())]
        });
        Self::new(anchors, editor_max, preview_max)
    }

    /// The preview offset that matches an editor offset.
    pub(crate) fn preview_for_editor(&self, editor: f32) -> f32 {
        interpolate(&self.points, editor, |point| point.0, |point| point.1)
    }

    /// The editor offset that matches a preview offset.
    pub(crate) fn editor_for_preview(&self, preview: f32) -> f32 {
        interpolate(&self.points, preview, |point| point.1, |point| point.0)
    }
}

/// Allow one extra discarded pass per frame.
///
/// A zoom can need two corrections in one frame: the preview re-anchoring to its new
/// layout, then the follower re-locking to it. Each discards the pass so the frame is
/// shown corrected; egui's default of two passes only fits one. Frames that request no
/// discard still run a single pass.
pub(crate) fn configure_passes(ctx: &egui::Context) {
    ctx.options_mut(|options| {
        options.max_passes = std::num::NonZeroUsize::new(3).unwrap_or(options.max_passes);
    });
}

/// Run one frame of split synchronization and schedule the follower's repaint.
///
/// When the user scrolled the leader this frame (`source` is set), egui applies that
/// input at the end of the frame, so the leader and the follower both show their new
/// offsets next frame. Without user input the leader is already showing its offset
/// (a zoom, resize, or late re-layout moved only the mapping), so the pass is
/// discarded and repainted: both panes are then shown aligned in this same frame.
pub(crate) fn sync_frame(
    ctx: &egui::Context,
    editor: &mut Editor,
    preview: &mut Preview,
    source: Option<Pane>,
    leader: Pane,
) -> bool {
    let moved = sync_panes(editor, preview, leader);
    if moved {
        if source.is_none() {
            ctx.request_discard("split follower re-locked after a re-layout");
        }
        ctx.request_repaint();
    }
    moved
}

/// Re-pin a leader that was at its end before a re-layout.
///
/// Zoom, a split resize, a wrap change, or late media can move the end of the
/// scroll range. The leader's stored offset is then short of the new maximum, and
/// mapping that stale offset to the follower would take both panes away from the
/// end together. Returns `true` when the leader received a new end request and the
/// caller should wait one frame before syncing the follower.
pub(crate) fn keep_leader_at_end(
    editor: &mut Editor,
    preview: &mut Preview,
    leader: Pane,
    was_at_end: bool,
    source: Option<Pane>,
    edited: bool,
) -> bool {
    if !was_at_end || source.is_some() || edited {
        return false;
    }
    let at_end = match leader {
        Pane::Editor => editor.is_at_end(),
        Pane::Preview => preview.is_at_end(),
    };
    if at_end {
        return false;
    }
    match leader {
        Pane::Editor => {
            editor.scroll_to_end();
        }
        Pane::Preview => {
            preview.scroll_to_end();
        }
    }
    true
}

/// Move the follower pane to the leader's mapped offset.
///
/// Returns `true` when the follower received a new target and needs a repaint.
pub(crate) fn sync_panes(editor: &mut Editor, preview: &mut Preview, leader: Pane) -> bool {
    if preview.source_map().blocks.is_empty() {
        return false;
    }
    let map = ScrollMap::from_layout(
        &editor.line_tops(),
        preview.source_map(),
        editor.max_scroll_offset(),
        preview.max_scroll_offset(),
    );
    match leader {
        Pane::Editor => {
            let target = map.preview_for_editor(editor.scroll_offset());
            let moved = preview.sync_scroll_to(target);
            // Only an exact end target counts as an end write; the last sub-point steps of a
            // smoothed wheel approach are ordinary follow moves.
            if moved && target >= preview.max_scroll_offset() - 0.01 {
                log::debug!("split sync editor->preview end");
            } else if moved {
                log::trace!(
                    "split sync editor->preview editor_y={:.1} preview_y={target:.1}",
                    editor.scroll_offset()
                );
            }
            moved
        }
        Pane::Preview => {
            let target = map.editor_for_preview(preview.scroll_offset());
            let moved = editor.sync_scroll_to(target);
            if moved && preview.scroll_offset() >= preview.max_scroll_offset() - 0.01 {
                let metrics = preview.scroll_metrics();
                log::debug!(
                    "preview scroll end offset={:.1} max={:.1} last_bottom={:.1} content={:.1}",
                    metrics.offset,
                    metrics.max_offset,
                    metrics.last_block_bottom,
                    metrics.content_height
                );
                log::debug!("split sync preview->editor end");
            } else if moved {
                log::trace!(
                    "split sync preview->editor preview_y={:.1} editor_y={target:.1}",
                    preview.scroll_offset()
                );
            }
            moved
        }
    }
}

/// The follower's distance in points from the offset the map assigns to the leader.
pub(crate) fn lock_error(editor: &Editor, preview: &Preview, leader: Pane) -> Option<f32> {
    if preview.source_map().blocks.is_empty() {
        return None;
    }
    let map = ScrollMap::from_layout(
        &editor.line_tops(),
        preview.source_map(),
        editor.max_scroll_offset(),
        preview.max_scroll_offset(),
    );
    Some(match leader {
        Pane::Editor => (map.preview_for_editor(editor.scroll_offset()) - preview.scroll_offset()).abs(),
        Pane::Preview => (map.editor_for_preview(preview.scroll_offset()) - editor.scroll_offset()).abs(),
    })
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn interpolate(
    points: &[(f32, f32)],
    value: f32,
    input: fn(&(f32, f32)) -> f32,
    output: fn(&(f32, f32)) -> f32,
) -> f32 {
    let (Some(first), Some(last)) = (points.first(), points.last()) else {
        return 0.0;
    };
    let value = finite_or_zero(value);
    if value <= input(first) {
        return output(first);
    }
    if value >= input(last) {
        return output(last);
    }
    // `points` is strictly increasing on both axes, so a binary search is valid.
    let upper = points.partition_point(|point| input(point) < value).max(1);
    let (from, to) = (&points[upper - 1], &points[upper]);
    let span = input(to) - input(from);
    if span <= f32::EPSILON {
        return output(to);
    }
    let t = (value - input(from)) / span;
    output(from) + (output(to) - output(from)) * t
}

#[cfg(test)]
#[path = "split_sync_tests.rs"]
mod frame_tests;

#[cfg(test)]
mod tests {
    use mdview_render::BlockAnchor;

    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 0.01
    }

    #[test]
    fn both_ends_are_pinned_together() {
        let map = ScrollMap::new([(100.0, 300.0), (200.0, 500.0)], 400.0, 900.0);
        assert!(close(map.preview_for_editor(0.0), 0.0));
        assert!(close(map.preview_for_editor(400.0), 900.0));
        assert!(close(map.editor_for_preview(0.0), 0.0));
        assert!(close(map.editor_for_preview(900.0), 400.0));
    }

    #[test]
    fn targets_never_leave_the_follower_range() {
        // The last anchors are past the preview maximum: the old line mapping sent those
        // offsets to egui, which painted them for one frame and then clamped (flicker).
        let map = ScrollMap::new([(100.0, 200.0), (300.0, 950.0), (380.0, 1200.0)], 400.0, 900.0);
        for step in 0..=1000 {
            let editor = step as f32 * 0.5 - 50.0;
            let preview = map.preview_for_editor(editor);
            assert!((0.0..=900.0).contains(&preview), "{editor} -> {preview}");
            let back = map.editor_for_preview(step as f32 - 50.0);
            assert!((0.0..=400.0).contains(&back), "{step} -> {back}");
        }
    }

    #[test]
    fn the_map_is_continuous_and_monotonic() {
        let map = ScrollMap::new(
            [
                (10.0, 40.0),
                (50.0, 60.0),
                (55.0, 300.0),
                (400.0, 310.0),
                (600.0, 800.0),
            ],
            700.0,
            1000.0,
        );
        let mut previous = map.preview_for_editor(0.0);
        for step in 1..=7000 {
            let editor = step as f32 * 0.1;
            let preview = map.preview_for_editor(editor);
            assert!(preview >= previous, "not monotonic at {editor}");
            // The steepest segment is (50,60)->(55,300): 48 points per editor point.
            assert!(preview - previous <= 48.0 * 0.1 + 0.01, "jump at {editor}");
            previous = preview;
        }
    }

    #[test]
    fn mapping_round_trips_in_both_directions() {
        let map = ScrollMap::new([(30.0, 90.0), (120.0, 200.0), (300.0, 640.0)], 500.0, 800.0);
        for step in 0..=500 {
            let editor = step as f32;
            assert!(close(map.editor_for_preview(map.preview_for_editor(editor)), editor));
        }
    }

    #[test]
    fn out_of_order_and_duplicate_anchors_are_dropped() {
        // A footnote rendered at the end of the preview but defined early in the source.
        let map = ScrollMap::new(
            [(100.0, 100.0), (100.0, 150.0), (20.0, 700.0), (200.0, 300.0)],
            400.0,
            800.0,
        );
        assert_eq!(
            map.points,
            vec![(0.0, 0.0), (100.0, 100.0), (200.0, 300.0), (400.0, 800.0)]
        );
    }

    #[test]
    fn panes_that_cannot_scroll_map_to_zero() {
        let map = ScrollMap::new([(10.0, 10.0)], 0.0, 500.0);
        assert!(close(map.preview_for_editor(0.0), 0.0));
        assert!(close(map.editor_for_preview(250.0), 0.0));
        let map = ScrollMap::new([], 300.0, 0.0);
        assert!(close(map.preview_for_editor(150.0), 0.0));
        assert!(close(map.editor_for_preview(0.0), 0.0));
        let map = ScrollMap::new([], f32::NAN, f32::INFINITY);
        assert!(close(map.preview_for_editor(f32::NAN), 0.0));
    }

    #[test]
    fn layout_anchors_follow_block_lines() {
        // Lines are 20 points; three blocks on lines 1-2, 4-6, and 8.
        let tops = (0..=10).map(|line| line as f32 * 20.0).collect::<Vec<_>>();
        let block = |block_id, start_line, end_line, top, height| BlockAnchor {
            block_id,
            start_line,
            end_line,
            top,
            height,
        };
        let source = SourceMap::from_anchors(
            vec![
                block(0, 1, 2, 0.0, 50.0),
                block(1, 4, 6, 70.0, 200.0),
                block(2, 8, 8, 290.0, 30.0),
            ],
            320.0,
        );
        let map = ScrollMap::from_layout(&tops, &source, 1000.0, 1000.0);
        // Line 4 (zero-based 3) starts the second block.
        assert!(close(map.preview_for_editor(60.0), 70.0));
        // Halfway through the second block in the editor is halfway in the preview.
        assert!(close(map.preview_for_editor(90.0), 170.0));
        assert!(close(map.editor_for_preview(290.0), 140.0));
    }
}
