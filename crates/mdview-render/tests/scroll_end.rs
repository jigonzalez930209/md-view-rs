//! Preview scroll-end regression: media and tables must reach the true end.

#![allow(clippy::expect_used)]

use std::{cell::RefCell, rc::Rc};

use egui_kittest::Harness;
use mdview_media::MediaRequest;
use mdview_render::{MediaProvider, MediaState, Preview};
use mdview_theme::Theme;

struct FakeMedia {
    texture: Option<egui::TextureHandle>,
}

impl MediaProvider for FakeMedia {
    fn resolve(&mut self, ctx: &egui::Context, _request: &MediaRequest) -> MediaState {
        if self.texture.is_none() {
            let image = egui::ColorImage::from_rgba_unmultiplied([400, 240], &[90u8, 120, 200, 255].repeat(400 * 240));
            self.texture = Some(ctx.load_texture("fake-media", image, Default::default()));
        }
        MediaState::Ready(self.texture.clone().expect("texture"))
    }
}

fn document() -> String {
    let mut source = String::from("# Media end\n\n");
    for index in 0..10 {
        source.push_str(&format!("![image {index}](image-{index}.png)\n\n"));
        source.push_str(&format!(
            "Paragraph {index} with enough text to wrap across the reading measure and verify height estimates.\n\n"
        ));
    }
    source.push_str("| A | B | C |\n|---|---|---|\n| one | two | very long cell that wraps |\n\n");
    source.push_str("```mermaid\nflowchart LR\nA --> B --> C\n```\n\n");
    source.push_str("## Final heading\n\nThe final paragraph must be fully visible.\n");
    source
}

#[test]
fn media_heavy_preview_reaches_the_last_block() {
    let source = document();
    let theme = Theme::light();
    let preview = Rc::new(RefCell::new(Preview::default()));
    let media = Rc::new(RefCell::new(FakeMedia { texture: None }));
    let preview_handle = preview.clone();
    let media_handle = media.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(760.0, 560.0))
        .build_ui(move |ui| {
            preview_handle.borrow_mut().show_with_media(
                ui,
                &source,
                &theme,
                true,
                None,
                &mut *media_handle.borrow_mut(),
            );
        });
    harness.run();
    preview.borrow_mut().set_scroll_offset(1_000_000.0);
    for _ in 0..12 {
        harness.run();
    }
    let preview = preview.borrow();
    let metrics = preview.scroll_metrics();
    assert!(metrics.max_offset > 0.0, "{metrics:?}");
    assert!(preview.is_at_end(), "{metrics:?}");
    assert!(metrics.last_block_bottom <= metrics.content_height + 0.5, "{metrics:?}");
    assert!(metrics.offset >= metrics.max_offset - 1.0, "{metrics:?}");
}

#[test]
fn sync_targets_past_the_end_are_clamped_and_not_repeated() {
    let source = (0..80)
        .map(|index| format!("Paragraph {index} with some text.\n\n"))
        .collect::<String>();
    let theme = Theme::light();
    let preview = Rc::new(RefCell::new(Preview::default()));
    let handle = preview.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 400.0))
        .build_ui(move |ui| {
            handle.borrow_mut().show(ui, &source, &theme, true);
        });
    harness.run();
    let max = preview.borrow().max_scroll_offset();
    assert!(max > 0.0);
    // An unclamped target would be painted past the end for one frame (flicker).
    assert!(preview.borrow_mut().sync_scroll_to(max + 5_000.0));
    harness.run();
    assert!((preview.borrow().scroll_offset() - max).abs() < 0.5);
    assert!(preview.borrow().is_at_end());
    assert!(!preview.borrow_mut().sync_scroll_to(max + 5_000.0));
    assert!(!preview.borrow_mut().sync_scroll_to(f32::INFINITY));
    // Line targets past the end are clamped too.
    assert!(!preview.borrow_mut().scroll_to_line(158, 1.0));
}
