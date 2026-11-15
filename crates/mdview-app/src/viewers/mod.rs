//! Read-only viewer plugins.
//!
//! Markdown and text stay in the editor. Formats that need a dedicated presentation (images
//! today; PDF, video, or diagrams later) register a [`ViewerPlugin`] here. A plugin creates one
//! [`ViewerInstance`] per tab, and the application paints it through [`ViewerContext`].

use std::path::Path;
use std::sync::OnceLock;

use eframe::egui;
use mdview_render::MediaLightbox;

use crate::media_runtime::MediaRuntime;

mod image;

/// Per-frame services a viewer may use.
pub struct ViewerContext<'a> {
    /// Shared background media loader and texture cache.
    pub media: &'a mut MediaRuntime,
    /// Whether the active theme is dark.
    pub dark: bool,
    /// Whether the shared lightbox is already open.
    pub lightbox_open: bool,
}

/// What a viewer wants the application to do after painting.
#[derive(Default)]
pub struct ViewerOutput {
    /// Open the shared image lightbox.
    pub lightbox: Option<MediaLightbox>,
}

/// One open document owned by a viewer plugin.
pub trait ViewerInstance {
    /// Human-readable viewer name for diagnostics and the status bar.
    fn name(&self) -> &'static str;
    /// Paint the document and report application-level actions.
    fn show(&mut self, ui: &mut egui::Ui, context: &mut ViewerContext<'_>) -> ViewerOutput;
}

/// A factory for one family of read-only documents.
pub trait ViewerPlugin: Send + Sync {
    /// Stable identifier used in logs and documentation.
    fn id(&self) -> &'static str;
    /// Human-readable name.
    fn name(&self) -> &'static str;
    /// Extensions handled by this plugin, without the dot and in lowercase.
    fn extensions(&self) -> &'static [&'static str];
    /// Whether this plugin handles a path. The default checks the extension list.
    fn matches(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .is_some_and(|extension| self.extensions().contains(&extension.as_str()))
    }
    /// Create the per-tab viewer.
    fn create(&self, path: &Path) -> Result<Box<dyn ViewerInstance>, String>;
}

/// Registered viewer factories in priority order.
pub struct ViewerRegistry {
    plugins: Vec<Box<dyn ViewerPlugin>>,
}

impl ViewerRegistry {
    /// Build the built-in registry.
    pub fn new() -> Self {
        Self {
            plugins: vec![Box::new(image::ImagePlugin)],
        }
    }

    /// The plugin that handles a path.
    pub fn plugin_for(&self, path: &Path) -> Option<&dyn ViewerPlugin> {
        self.plugins.iter().map(Box::as_ref).find(|plugin| plugin.matches(path))
    }

    /// Whether a path opens in a viewer instead of the text editor.
    pub fn supports(&self, path: &Path) -> bool {
        self.plugin_for(path).is_some()
    }

    /// The plugin name for a path.
    pub fn name_for(&self, path: &Path) -> Option<&'static str> {
        self.plugin_for(path).map(ViewerPlugin::name)
    }
}

impl Default for ViewerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::app::MdViewApp {
    /// Paint the active read-only viewer tab.
    pub(crate) fn show_viewer_document(&mut self, ui: &mut egui::Ui) {
        let dark = self.theme == mdview_theme::ThemeMode::Dark;
        let output = {
            let Some(viewer) = self.tabs.active_mut().viewer.as_mut() else {
                return;
            };
            viewer.show(
                ui,
                &mut ViewerContext {
                    media: &mut self.media,
                    dark,
                    lightbox_open: self.lightbox.is_some(),
                },
            )
        };
        if let Some(lightbox) = output.lightbox {
            log::debug!("lightbox open alt={}", lightbox.alt);
            self.lightbox = Some(crate::lightbox::LightboxState::new(lightbox.request, lightbox.alt));
        }
    }
}

fn registry() -> &'static ViewerRegistry {
    static REGISTRY: OnceLock<ViewerRegistry> = OnceLock::new();
    REGISTRY.get_or_init(ViewerRegistry::new)
}

/// Whether the built-in registry opens a path in a viewer.
pub fn supports(path: &Path) -> bool {
    registry().supports(path)
}

/// The viewer name for a path.
pub fn name_for(path: &Path) -> Option<&'static str> {
    registry().name_for(path)
}

/// Create a viewer instance for a path, logging a plugin failure.
pub fn create(path: &Path) -> Option<Box<dyn ViewerInstance>> {
    let plugin = registry().plugin_for(path)?;
    match plugin.create(path) {
        Ok(viewer) => {
            log::info!(
                "viewer plugin={} id={} path={}",
                plugin.name(),
                plugin.id(),
                path.display()
            );
            Some(viewer)
        }
        Err(error) => {
            log::error!(
                "viewer plugin={} failed path={} error={error}",
                plugin.id(),
                path.display()
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_paths_select_the_image_plugin() {
        assert!(supports(Path::new("photo.png")));
        assert!(supports(Path::new("mark.SVG")));
        assert!(!supports(Path::new("README.md")));
        assert_eq!(name_for(Path::new("photo.jpg")), Some("Image"));
    }
}
