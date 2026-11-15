//! Reading fonts for the Markdown preview.
//!
//! GitHub's Markdown body uses the system UI font stack, which resolves to Noto Sans on
//! Linux. The preview bundles Noto Sans (SIL OFL 1.1, see `assets/fonts/OFL.txt`) in
//! five static faces (Noto Sans v2.015) so semibold, bold, and italic text use real
//! weights and slants instead of the single-weight UI font.

use egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use egui::{FontData, FontFamily};

const REGULAR: &[u8] = include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf");
const SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/NotoSans-SemiBold.ttf");
const BOLD: &[u8] = include_bytes!("../../../assets/fonts/NotoSans-Bold.ttf");
const ITALIC: &[u8] = include_bytes!("../../../assets/fonts/NotoSans-Italic.ttf");
const BOLD_ITALIC: &[u8] = include_bytes!("../../../assets/fonts/NotoSans-BoldItalic.ttf");

/// A face of the reading font.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReadingFace {
    /// Upright regular text.
    Regular,
    /// Upright semibold (600) text, used by GitHub for headings, table headers,
    /// and alert titles.
    SemiBold,
    /// Upright bold text.
    Bold,
    /// Italic text.
    Italic,
    /// Bold italic text.
    BoldItalic,
}

impl ReadingFace {
    /// Pick a face from inline emphasis flags.
    pub fn from_style(strong: bool, emphasis: bool) -> Self {
        match (strong, emphasis) {
            (false, false) => Self::Regular,
            (true, false) => Self::Bold,
            (false, true) => Self::Italic,
            (true, true) => Self::BoldItalic,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Regular => "mdview-reading-regular",
            Self::SemiBold => "mdview-reading-semibold",
            Self::Bold => "mdview-reading-bold",
            Self::Italic => "mdview-reading-italic",
            Self::BoldItalic => "mdview-reading-bold-italic",
        }
    }

    fn data(self) -> &'static [u8] {
        match self {
            Self::Regular => REGULAR,
            Self::SemiBold => SEMIBOLD,
            Self::Bold => BOLD,
            Self::Italic => ITALIC,
            Self::BoldItalic => BOLD_ITALIC,
        }
    }

    /// The egui font family for this face.
    pub fn family(self) -> FontFamily {
        FontFamily::Name(self.name().into())
    }

    /// A font id for this face at a size.
    pub fn font(self, size: f32) -> egui::FontId {
        egui::FontId::new(size, self.family())
    }
}

/// Register the reading faces. Safe before the first frame; egui skips fonts it
/// already has, so repeated calls are cheap.
pub fn add_reading_fonts(ctx: &egui::Context) {
    for face in [
        ReadingFace::Regular,
        ReadingFace::SemiBold,
        ReadingFace::Bold,
        ReadingFace::Italic,
        ReadingFace::BoldItalic,
    ] {
        ctx.add_font(FontInsert {
            name: face.name().to_owned(),
            data: FontData::from_static(face.data()),
            families: vec![InsertFontFamily {
                family: face.family(),
                priority: FontPriority::Highest,
            }],
        });
    }
}

/// Register the reading faces and report whether they are active in this pass.
///
/// Call it only while a frame is running. Fonts added after text was laid out in
/// the current pass become active on the next pass, so callers must not lay out
/// reading text while this returns `false`.
pub fn install_reading_fonts(ctx: &egui::Context) -> bool {
    if reading_fonts_ready(ctx) {
        return true;
    }
    add_reading_fonts(ctx);
    reading_fonts_ready(ctx)
}

fn reading_fonts_ready(ctx: &egui::Context) -> bool {
    ctx.fonts(|fonts| {
        fonts
            .definitions()
            .families
            .get(&ReadingFace::BoldItalic.family())
            .is_some_and(|faces| !faces.is_empty())
    })
}
