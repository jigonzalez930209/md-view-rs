//! Every raster format in `SUPPORTED_IMAGE_EXTENSIONS` decodes through `load_bytes`.

#![allow(clippy::expect_used)]

use std::io::Cursor;

use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use mdview_media::{MediaError, MediaKind, MediaRequest, load_bytes};

fn sample() -> DynamicImage {
    let mut image = RgbaImage::new(6, 4);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        *pixel = Rgba([(x * 40) as u8, (y * 60) as u8, 128, 255]);
    }
    DynamicImage::ImageRgba8(image)
}

fn encode(format: ImageFormat) -> Vec<u8> {
    let image = match format {
        // These encoders take RGB only, 32-bit float for the HDR formats, or 16-bit Farbfeld.
        ImageFormat::Pnm | ImageFormat::Tga => DynamicImage::ImageRgb8(sample().to_rgb8()),
        ImageFormat::Hdr | ImageFormat::OpenExr => DynamicImage::ImageRgb32F(sample().to_rgb32f()),
        ImageFormat::Farbfeld => DynamicImage::ImageRgba16(sample().to_rgba16()),
        _ => sample(),
    };
    let mut bytes = Vec::new();
    image.write_to(&mut Cursor::new(&mut bytes), format).expect("encode");
    bytes
}

fn request() -> MediaRequest {
    MediaRequest::new(MediaKind::Image, "sample")
}

fn request_for(format: ImageFormat) -> MediaRequest {
    let extension = format.extensions_str().first().copied().unwrap_or("bin");
    MediaRequest::new(MediaKind::Image, format!("sample.{extension}"))
}

#[test]
fn every_encodable_format_decodes() {
    for format in [
        ImageFormat::Png,
        ImageFormat::Jpeg,
        ImageFormat::Gif,
        ImageFormat::WebP,
        ImageFormat::Bmp,
        ImageFormat::Tiff,
        ImageFormat::Ico,
        ImageFormat::Tga,
        ImageFormat::Hdr,
        ImageFormat::OpenExr,
        ImageFormat::Pnm,
        ImageFormat::Qoi,
        ImageFormat::Farbfeld,
    ] {
        let raster =
            load_bytes(&encode(format), &request_for(format)).unwrap_or_else(|error| panic!("{format:?}: {error}"));
        assert_eq!((raster.width, raster.height), (6, 4), "{format:?}");
        assert!(raster.is_consistent(), "{format:?}");
    }
}

#[test]
fn corrupt_jpeg_xl_is_an_error_not_a_panic() {
    let mut bytes = vec![0xFF, 0x0A];
    bytes.extend_from_slice(&[0x55; 64]);
    assert!(matches!(load_bytes(&bytes, &request()), Err(MediaError::Decode(_))));
    let mut container = b"\0\0\0\x0cJXL \r\n\x87\n".to_vec();
    container.extend_from_slice(&[0; 32]);
    assert!(matches!(load_bytes(&container, &request()), Err(MediaError::Decode(_))));
}

#[test]
fn listed_extensions_open_in_the_image_viewer() {
    for name in [
        "scan.jxl",
        "icon.ico",
        "sky.hdr",
        "render.exr",
        "frame.qoi",
        "sprite.tga",
        "plot.pgm",
    ] {
        assert!(mdview_media::is_image_path(std::path::Path::new(name)), "{name}");
    }
}

