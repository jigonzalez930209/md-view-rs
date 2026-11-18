# Viewer plugins

MD View keeps Markdown and text in the editor. Files that need a dedicated presentation use a
read-only viewer plugin. The image, PDF, and video viewers are the built-in plugins; the same API is the
extension point for video, notebooks, 3D models, or diagrams.

**Compatibility:** for `1.0`, this is an internal compile-time API inside `mdview-app`, not a
public Rust ABI or a stable plugin contract. External plugins are Post-1.0. The traits may change
while the registry is prepared; a public plugin API would require its own versioning and signing
policy.

## Contracts

The application API lives in `crates/mdview-app/src/viewers/mod.rs`:

```rust
pub trait ViewerPlugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn matches(&self, path: &Path) -> bool { /* extension check */ }
    fn create(&self, path: &Path) -> Result<Box<dyn ViewerInstance>, String>;
}

pub trait ViewerInstance {
    fn name(&self) -> &'static str;
    fn show(&mut self, ui: &mut egui::Ui, context: &mut ViewerContext<'_>) -> ViewerOutput;
}
```

- The registry is compile-time and priority-ordered. `ViewerRegistry::new()` registers the
  built-in image plugin; new plugins are added there.
- `DocumentTab` owns one `ViewerInstance` per tab, so page, zoom, or playback state survives
  repaints and tab switches.
- Viewer tabs open in `ViewMode::View`, reject Edit/Split, and never write the file back through
  the text save path.
- `ViewerContext` exposes the shared `MediaRuntime` and the active dark/light mode.
- `ViewerOutput` currently reports a lightbox request; additional actions can be added without
  touching the editor.

## Built-in image plugin

`crates/mdview-app/src/viewers/image.rs` handles PNG, JPEG, GIF, WebP, BMP, TIFF, and SVG. It
fits the image to the pane, keeps transparency, plays supported SVG animation, reports the
format, and opens the shared lightbox on click.

## Adding a format

1. Add a module under `crates/mdview-app/src/viewers/`.
2. Implement `ViewerPlugin` and a per-tab `ViewerInstance`.
3. Register the factory in `ViewerRegistry::new()`.
4. Add the extension to the plugin's `extensions()` list.
5. Add a unit test for path matching and an E2E step that opens a fixture file.
6. Document limits, security rules, and fallback behavior here.

Example skeleton:

```rust
pub(super) struct PdfPlugin;

impl ViewerPlugin for PdfPlugin {
    fn id(&self) -> &'static str { "pdf" }
    fn name(&self) -> &'static str { "PDF" }
    fn extensions(&self) -> &'static [&'static str] { &["pdf"] }
    fn create(&self, path: &Path) -> Result<Box<dyn ViewerInstance>, String> {
        Ok(Box::new(PdfViewer::open(path)?))
    }
}
```

## Rules

- Viewers must be read-only. Editing remains the editor's responsibility.
- Never block the UI thread: use `MediaRuntime` workers or a plugin-owned worker.
- Bound every decoded resource (page count, source bytes, texture size, timeouts).
- Never execute scripts, external resources, or embedded HTML.
- Report loading, empty, and error states with an actionable message.
- Keep plugin IDs and names stable; they are used in diagnostics and the status bar.
- Treat the API as internal until the public plugin contract is explicitly frozen after `1.0`.
