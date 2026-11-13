//! Theme gallery references for the reading view.

use egui_kittest::Harness;
use mdview_render::Preview;
use mdview_theme::{Theme, ThemeId, apply_id};

const DOCUMENT: &str = r#"# Theme reference

A paragraph with **strong**, *emphasis*, `code`, and a [link](https://example.com).

> A quiet quote with an accent border.

| Name | Value |
|------|------:|
| one  | 1     |
| two  | 2     |

```rust
fn main() {
    println!("theme");
}
```

- first
- second

The end.
"#;

fn snapshot(name: &str, id: ThemeId) {
    let theme = Theme::for_id(id);
    let mut preview = Preview::default();
    preview.set_width(680.0);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(760.0, 700.0))
        .with_context_setup(move |ctx| apply_id(ctx, id))
        .build_ui(move |ui| {
            preview.show(ui, DOCUMENT, &theme, !theme.is_dark());
        });
    harness.run();
    harness.snapshot(name);
}

#[test]
fn github_light_reference() {
    snapshot("theme-github-light", ThemeId::GithubLight);
}

#[test]
fn dracula_reference() {
    snapshot("theme-dracula", ThemeId::Dracula);
}

#[test]
fn one_dark_light_reference() {
    snapshot("theme-one-dark-light", ThemeId::OneDarkProLight);
}
