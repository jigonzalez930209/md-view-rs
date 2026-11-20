//! Text measurement for Mermaid layout with the font the rasterizer draws.
//!
//! Mermaid sizes every node, label, and message from the measured width of its text.
//! The layout engine's built-in metrics describe Mermaid's default "trebuchet ms"
//! stack; when the system substitutes a wider face, labels overflow their boxes and are
//! clipped. Measuring with the exact face that resvg will use, and naming that face in
//! the SVG, keeps the layout and the pixels in agreement.

use std::sync::{Arc, OnceLock};

use fontdb::{Database, Family, Query, Style, Weight};

/// Families tried in order: Mermaid's own stack first (as GitHub shows it), then the
/// common sans-serif faces of each desktop platform.
const CANDIDATES: [&str; 11] = [
    "Trebuchet MS",
    "Verdana",
    "Arial",
    "Helvetica Neue",
    "Helvetica",
    "Segoe UI",
    "Noto Sans",
    "DejaVu Sans",
    "Liberation Sans",
    "Cantarell",
    "Ubuntu",
];

/// Advance used for a character the face has no glyph for, in em.
const MISSING_ADVANCE_EM: f64 = 0.6;
/// Advance used for a wide (CJK) character the face has no glyph for, in em.
const MISSING_WIDE_ADVANCE_EM: f64 = 1.0;

/// One font face's raw data, parsed again for each measurement (parsing only reads
/// table offsets, so it is cheap and keeps the type `Send + Sync`).
struct FaceData {
    data: Arc<[u8]>,
    index: u32,
}

impl FaceData {
    fn load(database: &Database, id: fontdb::ID) -> Option<Self> {
        let (data, index) = database.with_face_data(id, |data, index| (Arc::<[u8]>::from(data), index))?;
        ttf_parser::Face::parse(&data, index).ok()?;
        Some(Self { data, index })
    }

    /// The advance of `text` in em, without kerning (as SVG text without shaping).
    fn advance_em(&self, text: &str) -> f64 {
        let Ok(face) = ttf_parser::Face::parse(&self.data, self.index) else {
            return fallback_advance_em(text);
        };
        let units = f64::from(face.units_per_em().max(1));
        text.chars()
            .map(|character| {
                face.glyph_index(character)
                    .and_then(|glyph| face.glyph_hor_advance(glyph))
                    .map_or_else(|| missing_advance_em(character), |advance| f64::from(advance) / units)
            })
            .sum()
    }
}

/// The sans-serif face Mermaid text is measured and drawn with.
pub(crate) struct DiagramFont {
    family: String,
    regular: FaceData,
    bold: Option<FaceData>,
}

impl DiagramFont {
    /// The installed diagram font, resolved once per process.
    pub(crate) fn shared(database: &Database) -> Option<Arc<Self>> {
        static FONT: OnceLock<Option<Arc<DiagramFont>>> = OnceLock::new();
        FONT.get_or_init(|| Self::resolve(database).map(Arc::new)).clone()
    }

    fn resolve(database: &Database) -> Option<Self> {
        let (family, regular_id) = CANDIDATES
            .iter()
            .find_map(|name| query(database, Family::Name(name), Weight::NORMAL).map(|id| ((*name).to_owned(), id)))
            .or_else(|| {
                let id = query(database, Family::SansSerif, Weight::NORMAL)?;
                let family = database.face(id)?.families.first()?.0.clone();
                Some((family, id))
            })?;
        let regular = FaceData::load(database, regular_id)?;
        let bold = query(database, Family::Name(&family), Weight::BOLD)
            .filter(|id| *id != regular_id)
            .and_then(|id| FaceData::load(database, id));
        Some(Self { family, regular, bold })
    }

    /// The CSS font-family value that names this face in the SVG.
    pub(crate) fn css_family(&self) -> String {
        let escaped = self.family.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\", sans-serif")
    }

    /// A stable identity for the measurement profile.
    pub(crate) fn identity(&self) -> String {
        format!("{}:{}", self.family, self.regular.data.len())
    }

    /// The width of `text` in CSS pixels at `font_size`.
    ///
    /// A weight of 600 or more (or `bold`) uses the bold face when one is installed;
    /// otherwise the regular advances are widened the way a synthetic bold would be.
    pub(crate) fn width_px(&self, text: &str, font_size: f64, weight: Option<&str>) -> f64 {
        if text.is_empty() {
            return 0.0;
        }
        let size = if font_size.is_finite() && font_size > 0.0 {
            font_size
        } else {
            16.0
        };
        let em = match (is_bold(weight), &self.bold) {
            (true, Some(bold)) => bold.advance_em(text),
            (true, None) => self.regular.advance_em(text) * 1.05,
            (false, _) => self.regular.advance_em(text),
        };
        (em * size).max(0.0)
    }
}

fn query(database: &Database, family: Family<'_>, weight: Weight) -> Option<fontdb::ID> {
    let id = database.query(&Query {
        families: &[family],
        weight,
        style: Style::Normal,
        ..Query::default()
    })?;
    // `query` falls back to the nearest weight; a named family must really match.
    match family {
        Family::Name(name) => database
            .face(id)?
            .families
            .iter()
            .any(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
            .then_some(id),
        _ => Some(id),
    }
}

fn is_bold(weight: Option<&str>) -> bool {
    match weight.map(str::trim) {
        Some(value) if value.eq_ignore_ascii_case("bold") || value.eq_ignore_ascii_case("bolder") => true,
        Some(value) => value.parse::<u16>().is_ok_and(|weight| weight >= 600),
        None => false,
    }
}

fn missing_advance_em(character: char) -> f64 {
    if is_wide(character) {
        MISSING_WIDE_ADVANCE_EM
    } else {
        MISSING_ADVANCE_EM
    }
}

fn fallback_advance_em(text: &str) -> f64 {
    text.chars().map(missing_advance_em).sum()
}

/// East Asian wide ranges: CJK, Hangul, kana, full-width forms.
fn is_wide(character: char) -> bool {
    matches!(u32::from(character),
        0x1100..=0x115F | 0x2E80..=0xA4CF | 0xAC00..=0xD7A3 | 0xF900..=0xFAFF | 0xFE30..=0xFE4F | 0xFF00..=0xFF60 | 0xFFE0..=0xFFE6
        | 0x1F300..=0x1FAFF | 0x20000..=0x3FFFD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_from_css_values() {
        assert!(is_bold(Some("bold")));
        assert!(is_bold(Some("700")));
        assert!(is_bold(Some(" 600 ")));
        assert!(!is_bold(Some("normal")));
        assert!(!is_bold(Some("400")));
        assert!(!is_bold(None));
    }

    #[test]
    fn missing_glyphs_have_a_width() {
        assert_eq!(fallback_advance_em("ab"), 2.0 * MISSING_ADVANCE_EM);
        assert_eq!(fallback_advance_em("漢"), MISSING_WIDE_ADVANCE_EM);
    }

    #[test]
    fn an_installed_face_measures_longer_text_wider() {
        let mut database = Database::new();
        database.load_system_fonts();
        let Some(font) = DiagramFont::resolve(&database) else {
            // A host without any sans-serif font keeps Mermaid's built-in metrics.
            return;
        };
        let short = font.width_px("Auth", 16.0, None);
        let long = font.width_px("Authentication service", 16.0, None);
        assert!(short > 0.0 && long > short * 3.0, "{short} {long}");
        assert!(font.width_px("Auth", 16.0, Some("bold")) >= short);
        assert!(font.css_family().ends_with("sans-serif"));
    }
}
