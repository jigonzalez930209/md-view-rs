//! Visual snapshots for the Markdown preview.
//!
//! Regenerate deliberately with `UPDATE_SNAPSHOTS=1 cargo test -p mdview-render`.
//! The images are local references for review; they are not uploaded anywhere.

use egui_kittest::Harness;
use mdview_render::Preview;
use mdview_theme::{ReadingMargins, ReadingTypography, Theme, ThemeMode, apply};

const DOCUMENT: &str = r#"# Release notes

A paragraph with **strong**, *emphasis*, ~~strikethrough~~, `inline code`, and a
[link](https://example.com). A second line wraps to show reading width.

> [!NOTE]
> Alerts render with a title and a quiet surface.

## Lists

- first item
- second item with a nested list
  - nested
- [x] completed task
- [ ] Roadmap checkboxes updated in `docs/roadmap/` and the parity matrix in `docs/api/parity.md`
- [ ] long checklist item with inline `xtalforge` code and enough text to wrap naturally

1. ordered one
2. ordered two

## Code

```rust
fn main() {
    let answer = 42;
    println!("answer = {answer}");
}
```

## Table

| Left | Center | Right |
|:-----|:------:|------:|
| one  | two    | three |
| four | five   | six   |

## Math

Inline $a^2 + b^2 = c^2$ and display math:

$$
\int_0^1 x^2 dx = \frac{1}{3}
$$

---

Footnotes work too.[^note]

[^note]: A small note with `code`.
"#;

fn snapshot(name: &str, mode: ThemeMode) {
    snapshot_with(name, mode, ReadingMargins::new(15.0), ReadingTypography::default());
}

fn snapshot_with(name: &str, mode: ThemeMode, margins: ReadingMargins, typography: ReadingTypography) {
    let theme = Theme::for_mode(mode);
    let mut preview = Preview::default();
    preview.set_width(680.0);
    preview.set_reading_style(margins, typography);
    let harness_width = if margins.percent() >= 15.0 { 760.0 } else { 1100.0 };
    let mut harness = Harness::builder()
        .with_size(egui::vec2(harness_width, 900.0))
        .with_context_setup(move |ctx| {
            apply(ctx, mode);
        })
        .build_ui(move |ui| {
            preview.show(ui, DOCUMENT, &theme, mode == ThemeMode::Light);
        });
    harness.run();
    harness.snapshot(name);
}

#[test]
fn preview_light_snapshot() {
    snapshot("preview-light", ThemeMode::Light);
}

#[test]
fn preview_dark_snapshot() {
    snapshot("preview-dark", ThemeMode::Dark);
}

#[test]
fn preview_compact_light_snapshot() {
    snapshot_with(
        "preview-compact-light",
        ThemeMode::Light,
        ReadingMargins::new(10.0),
        ReadingTypography::default(),
    );
}

#[test]
fn preview_custom_typography_snapshot() {
    snapshot_with(
        "preview-custom-typography",
        ThemeMode::Light,
        ReadingMargins::new(5.0),
        ReadingTypography {
            body: 16.0,
            headings: 34.0,
            code: 15.0,
            tables: 14.0,
            math: 24.0,
        },
    );
}
