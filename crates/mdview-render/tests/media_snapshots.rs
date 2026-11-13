#![allow(clippy::expect_used, clippy::unwrap_used)]
//! Visual snapshots for local images, SVG, math, and Mermaid.

use std::collections::HashMap;

use egui_kittest::Harness;
use mdview_media::{MediaRequest, Raster};
use mdview_render::{MediaProvider, MediaState, Preview};
use mdview_theme::{Theme, ThemeMode, apply};

struct TestMedia {
    cache: HashMap<String, MediaState>,
}

impl TestMedia {
    fn new() -> Self {
        Self { cache: HashMap::new() }
    }
}

impl MediaProvider for TestMedia {
    fn resolve(&mut self, ctx: &egui::Context, request: &MediaRequest) -> MediaState {
        let key = request.cache_key();
        if let Some(state) = self.cache.get(&key) {
            return state.clone();
        }
        let state = match request.load() {
            Ok(Raster { width, height, rgba }) => {
                let image = egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
                MediaState::Ready(ctx.load_texture(key.clone(), image, Default::default()))
            }
            Err(error) => MediaState::Error(error.to_string()),
        };
        self.cache.insert(key, state.clone());
        state
    }
}

const DOCUMENT: &str = r#"# Rich content

An inline formula $E = mc^2$ sits in this paragraph.

$$
\int_0^1 x^2 dx = \frac{1}{3}
$$

```mermaid
flowchart LR
    A[Markdown] --> B[Parser]
    B --> C[Preview]
```

![A generated test image](image.png)

![A vector mark](mark.svg)

![A missing image](missing.png)
"#;

fn fixture_dir() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("tempdir");
    let image = image::RgbaImage::from_fn(120, 80, |x, y| image::Rgba([(x * 2) as u8, (y * 3) as u8, 160, 255]));
    image.save(directory.path().join("image.png")).expect("png");
    std::fs::write(
        directory.path().join("mark.svg"),
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="60">
            <rect width="120" height="60" rx="10" fill="#0969da"/>
            <circle cx="30" cy="30" r="16" fill="#ffffff"/>
            <rect x="55" y="22" width="50" height="16" fill="#ffffff"/>
        </svg>"##,
    )
    .expect("svg");
    directory
}

fn snapshot(name: &str, mode: ThemeMode) {
    let directory = fixture_dir();
    let base = directory.path().to_path_buf();
    let theme = Theme::for_mode(mode);
    let mut preview = Preview::default();
    preview.set_width(680.0);
    let mut media = TestMedia::new();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(760.0, 980.0))
        .with_context_setup(move |ctx| apply(ctx, mode))
        .build_ui(move |ui| {
            preview.show_with_media(
                ui,
                DOCUMENT,
                &theme,
                mode == ThemeMode::Light,
                Some(base.clone()),
                &mut media,
            );
        });
    harness.run();
    harness.snapshot(name);
}

#[test]
fn rich_content_light_snapshot() {
    snapshot("media-light", ThemeMode::Light);
}

#[test]
fn rich_content_dark_snapshot() {
    snapshot("media-dark", ThemeMode::Dark);
}

#[test]
fn media_request_rejects_missing_local_files() {
    let request = MediaRequest::new(mdview_media::MediaKind::Image, "missing.png");
    assert!(request.load().is_err());
}
