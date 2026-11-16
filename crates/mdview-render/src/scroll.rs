//! Preview scroll offsets, programmatic scroll requests, and end detection.

use crate::{Preview, PreviewScrollMetrics};

/// Offsets closer than this (in points) count as already synchronized: a twentieth of
/// a pixel at 1x, invisible, but above egui's pixel-grid rounding so no target loops.
const SYNC_TOLERANCE: f32 = 0.05;

/// Whether a pass painted past the scrollable end by accident.
///
/// Trackpad rubber-banding paints past the end on purpose and keeps the stored offset
/// unclamped; only a pass whose stored offset egui clamped back into range overshot.
pub(crate) fn overshot(painted: f32, state: &egui::scroll_area::State, max_offset: f32) -> bool {
    painted > max_offset + 0.5 && state.unclamped_offset().y <= max_offset + 0.5
}

impl Preview {
    /// The last painted scroll offset in points.
    pub fn scroll_offset(&self) -> f32 {
        self.scroll
    }

    /// The largest valid scroll offset from the last paint.
    pub fn max_scroll_offset(&self) -> f32 {
        self.max_offset
    }

    /// Diagnostics for E2E assertions and the status bar.
    pub fn scroll_metrics(&self) -> PreviewScrollMetrics {
        PreviewScrollMetrics {
            offset: self.scroll,
            max_offset: self.max_offset,
            viewport_height: self.viewport.height(),
            content_height: self.content_height,
            last_block_bottom: self
                .source_map
                .blocks
                .last()
                .map(|block| block.top + block.height)
                .unwrap_or(0.0),
        }
    }

    /// Request a scroll offset on the next paint.
    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.pending_scroll = Some(offset.max(0.0));
    }

    /// Scroll to a content-space offset for split synchronization.
    ///
    /// The target is clamped to the scrollable range from the last paint, because an
    /// out-of-range request is painted unclamped for one frame before it snaps back.
    /// Returns `false` when the preview is already at the clamped target.
    pub fn sync_scroll_to(&mut self, offset: f32) -> bool {
        let target = offset.clamp(0.0, self.max_offset.max(0.0));
        // egui stores an explicit offset exactly, so a tight tolerance settles in one
        // frame and leaves no visible gap between the panes.
        if !target.is_finite() || (target - self.scroll).abs() <= SYNC_TOLERANCE {
            return false;
        }
        self.pending_line = None;
        self.pending_scroll = Some(target);
        true
    }

    /// How many frames were shown painted past the scrollable end.
    ///
    /// A pass that paints past the end (after a zoom, a resize, or an out-of-range
    /// request) is discarded and repainted in range before it is shown; only a pass
    /// that could not be discarded counts. Such a frame is visible as flicker.
    pub fn overshoot_paints(&self) -> u64 {
        self.overshoot_paints
    }

    /// Scroll to the maximum offset on the next paint.
    ///
    /// Returns `false` when the preview is already at the end. The request still
    /// completes then (see [`Self::take_end_request`]), without a scroll rewrite.
    pub fn scroll_to_end(&mut self) -> bool {
        self.end_requested = true;
        if self.is_at_end() {
            return false;
        }
        self.pending_scroll = Some(if self.max_offset > 0.0 {
            self.max_offset
        } else {
            f32::MAX
        });
        true
    }

    /// Consume a completed end request from `End` or split synchronization.
    pub fn take_end_request(&mut self) -> bool {
        if self.end_requested && self.is_at_end() {
            self.end_requested = false;
            true
        } else {
            false
        }
    }

    /// Whether the preview is at the end of its scrollable range.
    pub fn is_at_end(&self) -> bool {
        self.max_offset > 0.0 && self.scroll >= self.max_offset - 2.0
    }

    /// Whether the user scrolled this frame, consuming the flag.
    pub fn take_user_scroll(&mut self) -> bool {
        std::mem::take(&mut self.user_scrolled)
    }

    /// The preview viewport rectangle from the last paint.
    pub fn viewport_rect(&self) -> egui::Rect {
        self.viewport
    }

    /// Scroll to a zero-based source line, preserving an intra-line fraction.
    ///
    /// Returns `false` when the preview is already at that offset.
    pub fn scroll_to_line(&mut self, line: usize, fraction: f32) -> bool {
        if let Some(offset) = self.source_map.offset_for_line(line, fraction) {
            let offset = if self.max_offset > 0.0 {
                offset.min(self.max_offset)
            } else {
                offset
            };
            if (offset - self.scroll).abs() <= 1.0 {
                return false;
            }
            self.pending_scroll = Some(offset);
        } else {
            self.pending_line = Some((line, fraction));
        }
        true
    }

    /// Scroll to a heading or footnote anchor.
    pub fn scroll_to_anchor(&mut self, anchor: &str) -> bool {
        self.pending_anchor = Some(anchor.to_owned());
        if let Some(offset) = self.anchors.get(anchor).copied() {
            // A heading near the end cannot reach the top; stop at the end instead of
            // painting one frame past it.
            let offset = if self.max_offset > 0.0 {
                offset.min(self.max_offset)
            } else {
                offset
            };
            self.pending_scroll = Some(offset);
            self.pending_anchor = None;
            true
        } else {
            false
        }
    }
}
