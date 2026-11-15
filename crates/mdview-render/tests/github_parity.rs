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

/// The same 240 x 80 test image the GitHub reference page loads as `logo.png`.
fn logo(directory: &std::path::Path) {
    let image = image::RgbaImage::from_fn(240, 80, |x, _| {
        if x < 120 {
            image::Rgba([9, 105, 218, 255])
        } else {
            image::Rgba([130, 80, 223, 255])
        }
    });
    image.save(directory.join("logo.png")).expect("png");
}

#[test]
fn github_parity_snapshot() {
    let directory = tempfile::tempdir().expect("tempdir");
    logo(directory.path());
    let base = directory.path().to_path_buf();
    let mode = ThemeMode::Light;
    let theme = Theme::for_mode(mode);
    let mut preview = Preview::default();
    preview.set_width(680.0);
    preview.set_reading_style(
        ReadingMargins::new(15.0),
        ReadingTypography {
            body: 16.0,
            headings: 32.0,
            code: 13.6,
            tables: 16.0,
            math: 23.0,
        },
    );
    let mut media = Media::default();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(760.0, 3_000.0))
        .with_context_setup(move |ctx| apply(ctx, mode))
        .build_ui(move |ui| {
            preview.show_with_media(ui, DOCUMENT, &theme, true, Some(base.clone()), &mut media);
        });
    harness.run();
    harness.snapshot("github-parity");
}

/// Render any Markdown file at GitHub's scale for an ad-hoc comparison with GitHub:
///
/// ```sh
/// MDVIEW_PARITY_DOC=README.md MDVIEW_PARITY_OUT=/tmp/readme.png \
///     cargo test -p mdview-render --test github_parity render_external -- --ignored
/// ```
#[test]
#[ignore = "manual GitHub comparison of an arbitrary document"]
fn render_external() {
    let Ok(path) = std::env::var("MDVIEW_PARITY_DOC") else {
        return;
    };
    let out = std::env::var("MDVIEW_PARITY_OUT").expect("MDVIEW_PARITY_OUT");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join(path);
    let source = std::fs::read_to_string(&path).expect("document");
    let base = path.parent().map(std::path::Path::to_path_buf);
    let height: f32 = std::env::var("MDVIEW_PARITY_HEIGHT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(4_000.0);
    let mode = ThemeMode::Light;
    let theme = Theme::for_mode(mode);
    let mut preview = Preview::default();
    preview.set_width(680.0);
    preview.set_reading_style(
        ReadingMargins::new(15.0),
        ReadingTypography {
            body: 16.0,
            headings: 32.0,
            code: 13.6,
            tables: 16.0,
            math: 23.0,
        },
    );
    let mut media = Media::default();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(760.0, height))
        .with_context_setup(move |ctx| apply(ctx, mode))
        .build_ui(move |ui| {
            preview.show_with_media(ui, &source, &theme, true, base.clone(), &mut media);
        });
    harness.run();
    harness.render().expect("render").save(out).expect("save");
}
