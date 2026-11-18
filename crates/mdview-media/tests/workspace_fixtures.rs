//! The sample files in `examples/v1-workspace/media` open through the real loaders, so the
//! manual-test workspace never ships a file the viewers cannot show.

#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

use mdview_media::{
    MediaKind, MediaRequest, PdfDocument, SUPPORTED_IMAGE_EXTENSIONS, VIDEO_EXTENSIONS, ffmpeg_available,
    image_kind_for_path, probe_video,
};

fn media_dir(folder: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/v1-workspace/media")
        .join(folder)
}

fn files(folder: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(media_dir(folder))
        .expect("fixture folder")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    files
}

fn load(path: &Path) -> mdview_media::LoadedMedia {
    let mut request = MediaRequest::new(image_kind_for_path(path), path.to_string_lossy());
    request.allow_remote = false;
    request
        .load_with_animation()
        .map_err(|error| format!("{}: {error}", path.display()))
        .expect("the sample loads")
}

#[test]
fn every_still_image_decodes_at_its_size() {
    let images = files("images");
    let mut extensions: Vec<String> = Vec::new();
    for path in &images {
        let loaded = load(path);
        // Icons hold their largest entry, 64 px wide; every other sample is the full card.
        let expected = if path.extension().is_some_and(|e| e == "ico") {
            (64, 40)
        } else {
            (240, 150)
        };
        assert_eq!(
            (loaded.raster.width, loaded.raster.height),
            expected,
            "{}",
            path.display()
        );
        assert!(loaded.raster.is_consistent());
        assert!(loaded.animated.is_none(), "{} is still", path.display());
        extensions.push(path.extension().and_then(|e| e.to_str()).unwrap_or("").to_owned());
    }
    // One sample per raster extension, except aliases the loader shares with another sample.
    for extension in SUPPORTED_IMAGE_EXTENSIONS {
        let alias = matches!(*extension, "apng" | "jpeg" | "tif" | "svg" | "cur" | "pnm" | "farbfeld");
        assert!(
            alias || extensions.iter().any(|e| e == extension),
            "no sample.{extension}"
        );
    }
}

#[test]
fn animated_samples_play_and_static_svg_does_not() {
    for path in files("animated") {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_owned();
        let loaded = load(&path);
        if name == "static.svg" {
            assert!(loaded.animated.is_none());
            continue;
        }
        let duration = loaded.animation_duration.unwrap_or(0.0);
        assert!(duration > 0.0, "{name} animates");
        assert_eq!(
            loaded.animation_unsupported,
            name == "partial.svg",
            "{name} support flag"
        );
        let source = loaded.animated.expect("animated source");
        let request = MediaRequest::new(MediaKind::Image, path.to_string_lossy());
        let later = source.render_frame(duration * 0.37, &request).expect("later frame");
        assert_ne!(later.rgba, loaded.raster.rgba, "{name} changes over time");
    }
}

#[test]
fn the_pdf_has_mixed_page_sizes_and_renders() {
    let bytes = std::fs::read(media_dir("documents").join("guide.pdf")).expect("pdf");
    let document = PdfDocument::open(bytes).expect("open");
    assert_eq!(document.page_count(), 6);
    assert_eq!(document.page_size(0), Some((595.0, 842.0)));
    assert_eq!(document.page_size(2), Some((612.0, 792.0)));
    assert_eq!(document.page_size(5), Some((842.0, 595.0)));
    let page = document.render_page(1, 0.5).expect("render");
    assert!(
        page.rgba.chunks(4).any(|pixel| pixel[0] < 64),
        "the table has dark text"
    );
}

#[test]
fn videos_use_known_extensions_and_probe_when_ffmpeg_is_installed() {
    let videos = files("video");
    for path in &videos {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        assert!(VIDEO_EXTENSIONS.contains(&extension), "{}", path.display());
    }
    if !ffmpeg_available() {
        return;
    }
    for path in &videos {
        let info = probe_video(path)
            .map_err(|error| format!("{}: {error}", path.display()))
            .expect("the sample probes");
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let expected = if name == "portrait.mov" { (270, 480) } else { (480, 270) };
        assert_eq!((info.width, info.height), expected, "{name}");
        assert_eq!(info.has_audio, name.starts_with("clip"), "{name} audio");
    }
}
