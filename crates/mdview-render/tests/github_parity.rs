#![allow(clippy::expect_used)]
//! GitHub parity snapshot: every common Markdown element at GitHub's 16 px scale.
//!
//! `scripts/github-parity/measure.py` compares `github-parity.png` with GitHub's own
//! render of `fixtures/github-parity.md`; see `docs/github-parity.md`.

use std::collections::HashMap;

use egui_kittest::Harness;
use mdview_media::{MediaRequest, Raster};
use mdview_render::{MediaProvider, MediaState, Preview};
use mdview_theme::{ReadingMargins, ReadingTypography, Theme, ThemeMode, apply};

const DOCUMENT: &str = include_str!("fixtures/github-parity.md");

#[derive(Default)]
struct Media(HashMap<String, MediaState>);

impl MediaProvider for Media {
    fn resolve(&mut self, ctx: &egui::Context, request: &MediaRequest) -> MediaState {
        let key = request.cache_key();
        self.0
            .entry(key.clone())
            .or_insert_with(|| match request.load() {
                Ok(Raster { width, height, rgba }) => {
                    let image = egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
                    MediaState::Ready(ctx.load_texture(key, image, Default::default()))
                }
                Err(error) => MediaState::Error(error.to_string()),
            })
            .clone()
    }
}

#[test]
fn github_parity_snapshot() {
    let mode = ThemeMode::Light;
    let theme = Theme::for_mode(mode);
    let mut preview = Preview::default();
    preview.set_width(680.0);
    preview.set_reading_style(
        ReadingMargins::new(15.0),
        ReadingTypography {
            body: 16.0,
            headings: 32.0,
            code: 14.0,
            tables: 16.0,
            math: 23.0,
        },
    );
    let mut media = Media::default();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(760.0, 2_600.0))
        .with_context_setup(move |ctx| apply(ctx, mode))
        .build_ui(move |ui| {
            preview.show_with_media(ui, DOCUMENT, &theme, true, None, &mut media);
        });
    harness.run();
    harness.snapshot("github-parity");
}
