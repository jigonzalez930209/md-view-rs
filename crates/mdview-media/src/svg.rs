//! Static SVG parsing and rasterization.

use std::sync::{Arc, OnceLock};

use craft_svg::tree::{self, Limits, usvg};
use resvg::tiny_skia::{Pixmap, Transform};

use crate::{MAX_IMAGE_SIDE, MAX_SOURCE_BYTES, MAX_SVG_DEPTH, MediaError, MediaRequest, Raster};

/// Rasterize SVG bytes at the request's display size.
pub fn rasterize_svg(bytes: &[u8], request: &MediaRequest) -> Result<Raster, MediaError> {
    if bytes.is_empty() {
        return Err(MediaError::Empty);
    }
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(MediaError::TooLarge);
    }
    reject_unsafe_svg(bytes)?;
    let limits = Limits {
        bytes: MAX_SOURCE_BYTES,
        depth: MAX_SVG_DEPTH,
        xml_nodes: 500_000,
        embedded_bytes: 8 * 1024 * 1024,
        refuse_entity_markup: true,
        ..Limits::default()
    };
    let options = usvg::Options {
        fontdb: system_fonts(),
        ..usvg::Options::default()
    };
    let tree = tree::parse(bytes, &limits, &options).map_err(|error| MediaError::Svg(error.to_string()))?;
    let size = tree.size();
    let natural_width = size.width().max(1.0);
    let natural_height = size.height().max(1.0);
    let max_width = request.max_width.clamp(1, MAX_IMAGE_SIDE) as f32;
    let max_height = request.max_height.clamp(1, MAX_IMAGE_SIDE) as f32;
    let scale = (max_width / natural_width)
        .min(max_height / natural_height)
        .clamp(0.01, 1.0);
    let width = (natural_width * scale).round().max(1.0) as u32;
    let height = (natural_height * scale).round().max(1.0) as u32;
    if width == 0 || height == 0 || width > MAX_IMAGE_SIDE || height > MAX_IMAGE_SIDE {
        return Err(MediaError::TooLarge);
    }
    let mut pixmap = Pixmap::new(width, height).ok_or(MediaError::TooLarge)?;
    resvg::render(&tree, Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    Ok(Raster {
        width,
        height,
        rgba: unpremultiply(pixmap.data()),
    })
}

fn system_fonts() -> Arc<fontdb::Database> {
    static FONTS: OnceLock<Arc<fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut database = fontdb::Database::new();
            database.load_system_fonts();
            Arc::new(database)
        })
        .clone()
}

fn unpremultiply(data: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(data.len());
    for pixel in data.chunks_exact(4) {
        let alpha = pixel[3];
        if alpha == 0 {
            rgba.extend_from_slice(&[0, 0, 0, 0]);
        } else if alpha == 255 {
            rgba.extend_from_slice(pixel);
        } else {
            let alpha = alpha as u16;
            rgba.push(((pixel[0] as u16 * 255) / alpha).min(255) as u8);
            rgba.push(((pixel[1] as u16 * 255) / alpha).min(255) as u8);
            rgba.push(((pixel[2] as u16 * 255) / alpha).min(255) as u8);
            rgba.push(alpha as u8);
        }
    }
    rgba
}

fn reject_unsafe_svg(bytes: &[u8]) -> Result<(), MediaError> {
    let text = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    for (needle, reason) in [
        ("<script", "script elements are not allowed"),
        ("javascript:", "javascript URLs are not allowed"),
        ("<foreignobject", "foreignObject is not allowed"),
        ("<!entity", "entities are not allowed"),
    ] {
        if text.contains(needle) {
            return Err(MediaError::Svg(reason.to_owned()));
        }
    }
    for prefix in [
        "href=\"http://",
        "href='http://",
        "href=\"https://",
        "href='https://",
        "href=\"file:",
        "href='file:",
        "xlink:href=\"http",
        "xlink:href='http",
    ] {
        if text.contains(prefix) {
            return Err(MediaError::Svg("external SVG resources are not allowed".to_owned()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MediaKind;

    fn request() -> MediaRequest {
        let mut request = MediaRequest::new(MediaKind::Svg, "test.svg");
        request.max_width = 200;
        request.max_height = 200;
        request
    }

    #[test]
    fn renders_a_basic_svg() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="#09a"/></svg>"##;
        let raster = rasterize_svg(svg, &request()).expect("render");
        assert_eq!((raster.width, raster.height), (100, 50));
        assert!(raster.is_consistent());
    }

    #[test]
    fn scripts_and_external_resources_are_refused() {
        let script = br#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>"#;
        assert!(matches!(rasterize_svg(script, &request()), Err(MediaError::Svg(_))));
        let external = br#"<svg xmlns="http://www.w3.org/2000/svg"><image href="https://example.com/a.png"/></svg>"#;
        assert!(matches!(rasterize_svg(external, &request()), Err(MediaError::Svg(_))));
    }

    #[test]
    fn nesting_beyond_the_limit_fails_cleanly() {
        let mut svg = String::from("<svg xmlns=\"http://www.w3.org/2000/svg\">");
        for _ in 0..300 {
            svg.push_str("<g>");
        }
        for _ in 0..300 {
            svg.push_str("</g>");
        }
        svg.push_str("</svg>");
        assert!(matches!(
            rasterize_svg(svg.as_bytes(), &request()),
            Err(MediaError::Svg(_))
        ));
    }
}
