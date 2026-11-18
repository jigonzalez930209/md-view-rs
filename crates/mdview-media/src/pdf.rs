//! PDF page rendering with hayro, a pure-Rust PDF rasterizer.
//!
//! A [`PdfDocument`] owns the file's bytes and renders one page at a time onto white paper, at a
//! scale chosen by the caller and bounded by [`MAX_PAGE_PIXELS`]. Encrypted files are refused
//! with a clear error; pages hayro cannot fully interpret still render what it supports.

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings, render};

use crate::{MediaError, Raster};

/// Largest rendered page, in pixels (about 8K × 8K).
pub const MAX_PAGE_PIXELS: f32 = 64.0e6;
/// Longest rendered side; hayro's canvas uses 16-bit sides.
const MAX_SIDE: f32 = 16_000.0;

/// An open PDF document.
pub struct PdfDocument {
    pdf: Pdf,
}

impl std::fmt::Debug for PdfDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PdfDocument")
            .field("pages", &self.page_count())
            .finish()
    }
}

impl PdfDocument {
    /// Parse a PDF from its bytes.
    pub fn open(bytes: Vec<u8>) -> Result<Self, MediaError> {
        let pdf = Pdf::new(bytes).map_err(|error| MediaError::Decode(format!("Could not open the PDF: {error:?}")))?;
        if pdf.pages().is_empty() {
            return Err(MediaError::Decode("The PDF has no pages.".to_owned()));
        }
        Ok(Self { pdf })
    }

    /// Number of pages.
    pub fn page_count(&self) -> usize {
        self.pdf.pages().len()
    }

    /// Size of a page in PDF points (1/72 in), after its rotation.
    pub fn page_size(&self, index: usize) -> Option<(f32, f32)> {
        self.pdf.pages().get(index).map(|page| page.render_dimensions())
    }

    /// Render page `index` at `scale` pixels per point onto white paper. The scale is reduced
    /// when the page would exceed [`MAX_PAGE_PIXELS`] or the longest side the canvas allows.
    pub fn render_page(&self, index: usize, scale: f32) -> Result<Raster, MediaError> {
        let page = self
            .pdf
            .pages()
            .get(index)
            .ok_or_else(|| MediaError::Decode(format!("The PDF has no page {}.", index + 1)))?;
        let (width, height) = page.render_dimensions();
        let mut scale = scale.max(0.05);
        let pixels = width * height * scale * scale;
        if pixels > MAX_PAGE_PIXELS {
            scale *= (MAX_PAGE_PIXELS / pixels).sqrt();
        }
        scale = scale.min(MAX_SIDE / width.max(height).max(1.0));
        let pixmap = render(
            page,
            &RenderCache::new(),
            &InterpreterSettings::default(),
            &RenderSettings::default(),
            &PixmapSettings {
                x_scale: scale,
                y_scale: scale,
                bg_color: WHITE,
            },
        );
        let (out_width, out_height) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
        if out_width == 0 || out_height == 0 {
            return Err(MediaError::EmptyImage);
        }
        // The background is opaque white, so premultiplied pixels equal straight ones.
        let rgba = pixmap
            .data()
            .iter()
            .flat_map(|pixel| [pixel.r, pixel.g, pixel.b, pixel.a])
            .collect();
        Ok(Raster {
            width: out_width,
            height: out_height,
            rgba,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid PDF: `pages` US Letter pages, each filled with a red square at the
    /// bottom-left quarter, with a correct cross-reference table.
    pub(crate) fn sample_pdf(pages: usize) -> Vec<u8> {
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            format!(
                "<< /Type /Pages /Kids [{}] /Count {pages} >>",
                (0..pages)
                    .map(|i| format!("{} 0 R", 3 + i * 2))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        ];
        let content = "1 0 0 rg 0 0 306 396 re f";
        for i in 0..pages {
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {} 0 R >>",
                4 + i * 2
            ));
            objects.push(format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()));
        }
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
        }
        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
        for offset in offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );
        out
    }

    #[test]
    fn renders_pages_at_the_requested_scale() {
        let document = PdfDocument::open(sample_pdf(3)).expect("open");
        assert_eq!(document.page_count(), 3);
        assert_eq!(document.page_size(0), Some((612.0, 792.0)));
        let raster = document.render_page(2, 0.5).expect("render");
        assert_eq!((raster.width, raster.height), (306, 396));
        assert!(raster.is_consistent());
        // Bottom-left is the red square; top-right is white paper.
        let pixel = |x: u32, y: u32| {
            let at = ((y * raster.width + x) * 4) as usize;
            &raster.rgba[at..at + 4]
        };
        assert_eq!(pixel(10, 390), [255, 0, 0, 255]);
        assert_eq!(pixel(300, 10), [255, 255, 255, 255]);
    }

    #[test]
    fn huge_scales_are_bounded() {
        let document = PdfDocument::open(sample_pdf(1)).expect("open");
        let raster = document.render_page(0, 100.0).expect("render");
        assert!((raster.width as f32 * raster.height as f32) <= MAX_PAGE_PIXELS * 1.01);
    }

    #[test]
    fn bad_input_is_an_error() {
        assert!(PdfDocument::open(b"not a pdf".to_vec()).is_err());
        let document = PdfDocument::open(sample_pdf(1)).expect("open");
        assert!(document.render_page(5, 1.0).is_err());
    }
}
