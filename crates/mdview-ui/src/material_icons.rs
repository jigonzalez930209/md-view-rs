//! The VS Code Material Icon Theme specification, embedded.
//!
//! The association tables and SVG files come from
//! `material-extensions/vscode-material-icon-theme` (MIT). Resolution follows the extension's
//! manifest: exact file name, then extension; folder name; expanded folder variants; root folder.
//! Icons are rasterized once and cached as egui textures.

use std::collections::HashMap;

use egui::{Context, TextureHandle, TextureOptions};
use rust_embed::RustEmbed;
use serde::Deserialize;

#[derive(RustEmbed)]
#[folder = "../../assets/material-icons"]
struct Assets;

#[derive(RustEmbed)]
#[folder = "../../assets/ui-icons"]
struct UiAssets;

#[derive(Clone, Debug, Deserialize)]
struct Definition {
    #[serde(rename = "iconPath")]
    icon_path: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
struct Overrides {
    #[serde(rename = "fileExtensions")]
    file_extensions: HashMap<String, String>,
    #[serde(rename = "fileNames")]
    file_names: HashMap<String, String>,
    #[serde(rename = "folderNames")]
    folder_names: HashMap<String, String>,
    #[serde(rename = "folderNamesExpanded")]
    folder_names_expanded: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct Manifest {
    #[serde(rename = "iconDefinitions")]
    definitions: HashMap<String, Definition>,
    #[serde(rename = "fileExtensions")]
    file_extensions: HashMap<String, String>,
    #[serde(rename = "fileNames")]
    file_names: HashMap<String, String>,
    #[serde(rename = "folderNames")]
    folder_names: HashMap<String, String>,
    #[serde(rename = "folderNamesExpanded")]
    folder_names_expanded: HashMap<String, String>,
    file: String,
    folder: String,
    #[serde(rename = "folderExpanded")]
    folder_expanded: String,
    #[serde(rename = "rootFolder")]
    root_folder: String,
    #[serde(rename = "rootFolderExpanded")]
    root_folder_expanded: String,
    light: Overrides,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            definitions: HashMap::new(),
            file_extensions: HashMap::new(),
            file_names: HashMap::new(),
            folder_names: HashMap::new(),
            folder_names_expanded: HashMap::new(),
            file: "file".to_owned(),
            folder: "folder".to_owned(),
            folder_expanded: "folder-open".to_owned(),
            root_folder: "folder-root".to_owned(),
            root_folder_expanded: "folder-root-open".to_owned(),
            light: Overrides::default(),
        }
    }
}

/// Material icon resolver and texture cache.
pub struct MaterialIcons {
    manifest: Manifest,
    textures: HashMap<(String, bool, u32), TextureHandle>,
    ui_textures: HashMap<String, TextureHandle>,
}

impl Default for MaterialIcons {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialIcons {
    /// Load the embedded manifest.
    pub fn new() -> Self {
        let manifest = Assets::get("material-icons.json")
            .and_then(|file| serde_json::from_slice(&file.data).ok())
            .unwrap_or_default();
        Self {
            manifest,
            textures: HashMap::new(),
            ui_textures: HashMap::new(),
        }
    }

    /// Resolve the icon id for a file path according to the Material Icon Theme rules.
    pub fn file_icon(&self, path: &std::path::Path, light: bool) -> String {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if let Some(id) = self.file_name_icon(&name, light) {
            return id;
        }
        // Compound extensions such as `schema.json` and `ts.map` are manifest keys.
        let parts = name.split('.').collect::<Vec<_>>();
        for start in 0..parts.len() {
            let suffix = parts[start..].join(".");
            if let Some(id) = self.file_extension_icon(&suffix, light) {
                return id;
            }
        }

        if let Some(extension) = path.extension().and_then(|value| value.to_str())
            && let Some(id) = self.file_extension_icon(&extension.to_ascii_lowercase(), light)
        {
            return id;
        }
        self.manifest.file.clone()
    }

    /// Resolve the icon id for a folder name.
    pub fn folder_icon(&self, name: &str, expanded: bool, root: bool, light: bool) -> String {
        if root {
            return if expanded {
                self.manifest.root_folder_expanded.clone()
            } else {
                self.manifest.root_folder.clone()
            };
        }
        let key = name.to_ascii_lowercase();
        let (base, overrides) = (&self.manifest.folder_names, &self.manifest.folder_names_expanded);
        let found = if light {
            let light = if expanded {
                self.manifest.light.folder_names_expanded.get(&key)
            } else {
                self.manifest.light.folder_names.get(&key)
            };
            light.or_else(|| if expanded { overrides.get(&key) } else { base.get(&key) })
        } else if expanded {
            overrides.get(&key)
        } else {
            base.get(&key)
        };
        if let Some(id) = found {
            return id.clone();
        }
        if expanded {
            self.manifest.folder_expanded.clone()
        } else {
            self.manifest.folder.clone()
        }
    }

    fn file_name_icon(&self, name: &str, light: bool) -> Option<String> {
        if light && let Some(id) = self.manifest.light.file_names.get(name) {
            return Some(id.clone());
        }
        self.manifest.file_names.get(name).cloned()
    }

    fn file_extension_icon(&self, extension: &str, light: bool) -> Option<String> {
        if light && let Some(id) = self.manifest.light.file_extensions.get(extension) {
            return Some(id.clone());
        }
        self.manifest.file_extensions.get(extension).cloned()
    }

    /// The SVG resource for an icon id.
    fn icon_svg(&self, id: &str) -> Option<Vec<u8>> {
        let definition = self.manifest.definitions.get(id)?;
        let path = definition.icon_path.replace("./../", "");
        Assets::get(&path).map(|file| file.data.into_owned())
    }

    /// Load or create the texture for an icon id at a specific display size.
    pub fn texture_sized(&mut self, ctx: &Context, id: &str, light: bool, size_points: f32) -> Option<TextureHandle> {
        // Rasterize at the device pixel ratio so folder glyphs keep their inner detail.
        let pixels = (size_points * ctx.pixels_per_point().max(1.0))
            .round()
            .clamp(16.0, 64.0) as u32;
        let key = (id.to_owned(), light, pixels);
        if let Some(texture) = self.textures.get(&key) {
            return Some(texture.clone());
        }
        let bytes = self.icon_svg(id)?;
        let mut request = mdview_media::MediaRequest::new(mdview_media::MediaKind::Svg, id);
        request.max_width = pixels;
        request.max_height = pixels;
        request.dark = !light;
        let raster = mdview_media::rasterize_svg(&bytes, &request).ok()?;
        let image =
            egui::ColorImage::from_rgba_unmultiplied([raster.width as usize, raster.height as usize], &raster.rgba);
        let texture = ctx.load_texture(
            format!("material-icon-{id}-{light}-{pixels}"),
            image,
            TextureOptions::LINEAR,
        );
        self.textures.insert(key, texture.clone());
        Some(texture)
    }

    /// Load or create the texture for a standard 16-point icon.
    pub fn texture(&mut self, ctx: &Context, id: &str, light: bool) -> Option<TextureHandle> {
        self.texture_sized(ctx, id, light, crate::ICON_MEDIUM)
    }

    /// Draw a monochrome UI action icon tinted with the current text color.
    pub fn draw_ui(&mut self, ui: &mut egui::Ui, name: &str, size: f32) -> bool {
        if let Some(texture) = self.ui_texture(ui.ctx(), name) {
            ui.add(
                egui::Image::new(&texture)
                    .fit_to_exact_size(egui::vec2(size, size))
                    .tint(ui.visuals().text_color()),
            );
            true
        } else {
            false
        }
    }

    /// The tinted-ready texture for a UI action icon.
    pub fn ui_texture(&mut self, ctx: &Context, name: &str) -> Option<TextureHandle> {
        if let Some(texture) = self.ui_textures.get(name) {
            return Some(texture.clone());
        }
        let file = UiAssets::get(&format!("{name}.svg"))?;
        let mut request = mdview_media::MediaRequest::new(mdview_media::MediaKind::Svg, name);
        request.max_width = 32;
        request.max_height = 32;
        let raster = mdview_media::rasterize_svg(&file.data, &request).ok()?;
        let image =
            egui::ColorImage::from_rgba_unmultiplied([raster.width as usize, raster.height as usize], &raster.rgba);
        let texture = ctx.load_texture(format!("ui-icon-{name}"), image, TextureOptions::LINEAR);
        self.ui_textures.insert(name.to_owned(), texture.clone());
        Some(texture)
    }

    /// Draw an icon at the current cursor. Returns false when the icon has no asset.
    pub fn draw(&mut self, ui: &mut egui::Ui, id: &str, light: bool, size: f32) -> bool {
        let Some(texture) = self.texture_sized(ui.ctx(), id, light, size) else {
            return false;
        };
        ui.add(egui::Image::new(&texture).fit_to_exact_size(egui::vec2(size, size)));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn resolves_the_material_specification() {
        let icons = MaterialIcons::new();
        assert_eq!(icons.file_icon(Path::new("main.rs"), true), "rust");
        assert_eq!(icons.file_icon(Path::new("README.md"), true), "readme");
        assert_eq!(icons.file_icon(Path::new(".gitignore"), true), "git");
        assert_eq!(icons.file_icon(Path::new("Cargo.toml"), true), "toml_light");
        assert_eq!(icons.file_icon(Path::new("unknown.zzz"), true), "file");
        assert_eq!(icons.file_icon(Path::new("schema.json"), true), "json_schema");
        assert_eq!(icons.file_icon(Path::new("main.ts.map"), true), "json");
        assert_eq!(icons.folder_icon("src", false, false, true), "folder-src");
        assert_ne!(
            icons.folder_icon("src", false, false, true),
            icons.folder_icon("src", true, false, true)
        );
        assert_eq!(icons.folder_icon("whatever", false, true, true), "folder-root");
    }

    #[test]
    fn every_resolved_icon_has_an_embedded_svg() {
        let icons = MaterialIcons::new();
        for id in [
            icons.file_icon(Path::new("main.rs"), true),
            icons.file_icon(Path::new("README.md"), true),
            icons.file_icon(Path::new(".gitignore"), true),
            icons.folder_icon("src", true, false, true),
            icons.file_icon(Path::new("schema.json"), true),
            icons.folder_icon("unknown", false, true, true),
        ] {
            assert!(icons.icon_svg(&id).is_some(), "{id}");
        }
    }
}
