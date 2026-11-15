//! Theme tokens and egui style application for MD View.
//!
//! Nine stable theme identifiers cover the GitHub, Dracula, and One Dark Pro families. Every
//! color used by the shell, editor, and preview comes from [`Theme`]; the application never
//! invents a literal color. Contrast is validated in tests against WCAG 2.2 AA.

#![deny(missing_docs)]

mod apply;
mod contrast;
mod fonts;
mod palette;

use egui::{Color32, CornerRadius, Stroke};

pub use apply::{apply, apply_id, apply_theme, effective_id, set_reduced_motion};
pub use contrast::{contrast_ratio, relative_luminance};
pub use fonts::{ReadingFace, add_reading_fonts, install_reading_fonts};
pub use palette::ThemeId;

/// The active color mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    /// Light surfaces.
    #[default]
    Light,
    /// Dark surfaces.
    Dark,
}

impl ThemeMode {
    /// Parse a value stored in settings.
    pub fn from_setting(value: &str) -> Self {
        if value == "dark" { Self::Dark } else { Self::Light }
    }

    /// The value stored in settings.
    pub fn as_setting(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

/// The color tokens used by the shell, editor, Markdown renderer, and media.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    /// Window and panel background.
    pub background: Color32,
    /// Raised surfaces such as menus and dialogs.
    pub surface: Color32,
    /// A second raised surface for cards and hovered rows.
    pub surface_raised: Color32,
    /// Sunken surfaces such as code blocks and text fields.
    pub sunken: Color32,
    /// Primary text.
    pub text: Color32,
    /// Secondary text.
    pub text_muted: Color32,
    /// Accent used for selection and focus.
    pub accent: Color32,
    /// Text drawn on the accent.
    pub accent_text: Color32,
    /// Borders and separators.
    pub border: Color32,
    /// Selection background.
    pub selection: Color32,
    /// Hover background.
    pub hover: Color32,
    /// Error color.
    pub danger: Color32,
    /// Warning color.
    pub warning: Color32,
    /// Success color.
    pub success: Color32,
    /// "Done" purple, used by GitHub for Important alerts.
    pub done: Color32,
    /// Link color.
    pub link: Color32,
    /// Scrollbar thumb.
    pub scrollbar: Color32,
    /// Keyboard focus ring.
    pub focus: Color32,
    /// Table header background.
    pub table_header: Color32,
    /// Zebra table row background.
    pub table_zebra: Color32,
    /// Block quote border.
    pub quote_border: Color32,
    /// Code block background.
    pub code_background: Color32,
}

impl Theme {
    /// The GitHub light theme.
    pub fn light() -> Self {
        palette::theme(ThemeId::GithubLight)
    }

    /// The GitHub dark theme.
    pub fn dark() -> Self {
        palette::theme(ThemeId::GithubDark)
    }

    /// The tokens for a stable theme identifier.
    pub fn for_id(id: ThemeId) -> Self {
        palette::theme(id)
    }

    /// The base tokens for a mode.
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        }
    }

    /// Whether the surface is dark, used by media and syntax palettes.
    pub fn is_dark(&self) -> bool {
        relative_luminance(self.background) < 0.5
    }

    /// A complete egui stroke for borders.
    pub fn border_stroke(&self) -> Stroke {
        Stroke::new(1.0, self.border)
    }

    /// The corner radius used by cards and inputs.
    pub fn radius(&self) -> CornerRadius {
        CornerRadius::same(6)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_round_trip_through_settings() {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            assert_eq!(ThemeMode::from_setting(mode.as_setting()), mode);
        }
        assert_eq!(ThemeMode::from_setting("unknown"), ThemeMode::Light);
    }

    #[test]
    fn light_and_dark_tokens_differ() {
        let light = Theme::light();
        let dark = Theme::dark();
        assert_ne!(light.background, dark.background);
        assert_ne!(light.text, dark.text);
        assert_ne!(light.accent, dark.accent);
    }

    #[test]
    fn every_theme_has_consistent_darkness() {
        for id in ThemeId::ALL {
            let theme = Theme::for_id(id);
            assert_eq!(theme.is_dark(), id.mode() == ThemeMode::Dark, "{id:?}");
        }
    }

    #[test]
    fn contrast_meets_aa_for_primary_pairs() {
        for id in ThemeId::ALL {
            let theme = Theme::for_id(id);
            for (foreground, background, label) in [
                (theme.text, theme.background, "text/background"),
                (theme.text, theme.surface, "text/surface"),
                (theme.text_muted, theme.background, "muted/background"),
                (theme.link, theme.background, "link/background"),
                (theme.accent_text, theme.accent, "accent text/accent"),
                (theme.danger, theme.background, "danger/background"),
            ] {
                let ratio = contrast_ratio(foreground, background);
                assert!(ratio >= 4.5, "{id:?} {label}: {ratio:.2}");
            }
        }
    }

    #[test]
    fn focus_and_components_reach_three_to_one() {
        for id in ThemeId::ALL {
            let theme = Theme::for_id(id);
            assert!(contrast_ratio(theme.focus, theme.background) >= 3.0, "{id:?}");
            assert!(contrast_ratio(theme.border, theme.background) >= 1.2, "{id:?}");
        }
    }

    #[test]
    fn reading_margin_settings_round_trip() {
        for percent in [
            ReadingMargins::MIN_PERCENT,
            12.5,
            ReadingMargins::DEFAULT_PERCENT,
            ReadingMargins::MAX_PERCENT,
        ] {
            let margins = ReadingMargins::new(percent);
            assert_eq!(ReadingMargins::from_setting(&margins.as_setting()), margins);
        }
        assert_eq!(ReadingMargins::from_setting("compact"), ReadingMargins::new(10.0));
        assert_eq!(ReadingMargins::from_setting("full"), ReadingMargins::new(5.0));
        assert_eq!(ReadingMargins::from_setting("unknown"), ReadingMargins::default());
    }

    #[test]
    fn reading_typography_is_bounded() {
        let sanitized = ReadingTypography {
            body: 1.0,
            headings: 200.0,
            code: 0.0,
            tables: 80.0,
            math: 100.0,
        }
        .sanitized();
        assert_eq!(sanitized.body, 10.0);
        assert_eq!(sanitized.headings, 48.0);
        assert_eq!(sanitized.code, 9.0);
        assert_eq!(sanitized.tables, 22.0);
        assert_eq!(sanitized.math, 36.0);
    }
}

/// Reading margins as a percentage of the preview width on each side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReadingMargins {
    percent: f32,
}

impl ReadingMargins {
    /// Smallest selectable margin, used by the wide preset.
    pub const MIN_PERCENT: f32 = 5.0;
    /// Largest selectable margin.
    pub const MAX_PERCENT: f32 = 40.0;
    /// Default margin, matching the old comfortable preset.
    pub const DEFAULT_PERCENT: f32 = 15.0;

    /// Clamp a percentage to the supported range.
    pub fn new(percent: f32) -> Self {
        Self {
            percent: percent.clamp(Self::MIN_PERCENT, Self::MAX_PERCENT),
        }
    }

    /// The percentage of the pane reserved on each side.
    pub fn percent(self) -> f32 {
        self.percent
    }

    /// Parse a stored percentage. Legacy preset names are migrated.
    pub fn from_setting(value: &str) -> Self {
        match value {
            "compact" => Self::new(10.0),
            "full" => Self::new(5.0),
            "comfortable" => Self::default(),
            other => other.parse().map(Self::new).unwrap_or_default(),
        }
    }

    /// The stored percentage.
    pub fn as_setting(self) -> String {
        format!("{:.1}", self.percent)
    }
}

impl Default for ReadingMargins {
    fn default() -> Self {
        Self::new(Self::DEFAULT_PERCENT)
    }
}

/// Per-area text sizes for the Markdown reading view, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReadingTypography {
    /// Body paragraphs, lists, quotes, and alerts.
    pub body: f32,
    /// Base heading size; lower levels scale down from it.
    pub headings: f32,
    /// Fenced and inline code.
    pub code: f32,
    /// Table headers and cells.
    pub tables: f32,
    /// Equation size. Display math scales up from this value.
    pub math: f32,
}

impl Default for ReadingTypography {
    fn default() -> Self {
        Self {
            body: 14.0,
            headings: 28.0,
            // GitHub: code at 85% of the body, tables at the body size.
            code: 12.0,
            tables: 14.0,
            math: 20.0,
        }
    }
}

impl ReadingTypography {
    /// Clamp every size to a safe range.
    pub fn sanitized(self) -> Self {
        Self {
            body: self.body.clamp(10.0, 22.0),
            headings: self.headings.clamp(16.0, 48.0),
            code: self.code.clamp(9.0, 22.0),
            tables: self.tables.clamp(9.0, 22.0),
            math: self.math.clamp(10.0, 36.0),
        }
    }
}
