//! Theme tokens and egui style application for MD View.
//!
//! Milestone H1 ships the light and dark base themes. The full GitHub, Dracula, and One Dark Pro
//! families are added in H8; all colors then come from the same token structure.

use craft_ui::theme::WidgetPalette;
use egui::{Color32, CornerRadius, Stroke};

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

/// The color tokens used by the shell, editor, and future Markdown renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    /// Window and panel background.
    pub background: Color32,
    /// Raised surfaces such as menus and dialogs.
    pub surface: Color32,
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
    /// Link color.
    pub link: Color32,
}

impl Theme {
    /// The light theme.
    pub fn light() -> Self {
        Self {
            background: Color32::from_rgb(0xfb, 0xfb, 0xfa),
            surface: Color32::from_rgb(0xff, 0xff, 0xff),
            sunken: Color32::from_rgb(0xf3, 0xf4, 0xf6),
            text: Color32::from_rgb(0x1f, 0x23, 0x28),
            text_muted: Color32::from_rgb(0x65, 0x6d, 0x76),
            accent: Color32::from_rgb(0x09, 0x69, 0xda),
            accent_text: Color32::from_rgb(0xff, 0xff, 0xff),
            border: Color32::from_rgb(0xd1, 0xd9, 0xe0),
            selection: Color32::from_rgb(0xcf, 0xe3, 0xfb),
            hover: Color32::from_rgb(0xef, 0xf2, 0xf5),
            danger: Color32::from_rgb(0xcf, 0x22, 0x2e),
            link: Color32::from_rgb(0x09, 0x69, 0xda),
        }
    }

    /// The dark theme.
    pub fn dark() -> Self {
        Self {
            background: Color32::from_rgb(0x1e, 0x1f, 0x22),
            surface: Color32::from_rgb(0x26, 0x28, 0x2c),
            sunken: Color32::from_rgb(0x15, 0x16, 0x18),
            text: Color32::from_rgb(0xe6, 0xe6, 0xe6),
            text_muted: Color32::from_rgb(0xa0, 0xa6, 0xad),
            accent: Color32::from_rgb(0x4c, 0x9a, 0xff),
            accent_text: Color32::from_rgb(0x0b, 0x1b, 0x2e),
            border: Color32::from_rgb(0x3a, 0x3d, 0x42),
            selection: Color32::from_rgb(0x2f, 0x4f, 0x74),
            hover: Color32::from_rgb(0x33, 0x36, 0x3b),
            danger: Color32::from_rgb(0xff, 0x7b, 0x72),
            link: Color32::from_rgb(0x7c, 0xb8, 0xff),
        }
    }

    /// The theme for a mode.
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        }
    }

    /// Whether the surface is dark, used by media and syntax palettes.
    pub fn is_dark(&self) -> bool {
        let luminance = 0.2126 * self.background.r() as f32
            + 0.7152 * self.background.g() as f32
            + 0.0722 * self.background.b() as f32;
        luminance < 128.0
    }
}

/// Apply a theme to an egui context.
pub fn apply(ctx: &egui::Context, mode: ThemeMode) {
    let theme = Theme::for_mode(mode);
    let mut visuals = match mode {
        ThemeMode::Light => egui::Visuals::light(),
        ThemeMode::Dark => egui::Visuals::dark(),
    };

    visuals.panel_fill = theme.background;
    visuals.window_fill = theme.surface;
    visuals.extreme_bg_color = theme.sunken;
    visuals.faint_bg_color = theme.hover;
    visuals.hyperlink_color = theme.link;
    visuals.selection.bg_fill = theme.selection;
    visuals.selection.stroke = Stroke::new(1.0, theme.text);
    visuals.window_stroke = Stroke::new(1.0, theme.border);

    WidgetPalette {
        noninteractive: theme.background,
        inactive: theme.surface,
        hovered: theme.hover,
        active: theme.accent,
        open: theme.surface,
    }
    .apply_to(
        &mut visuals.widgets,
        CornerRadius::same(6),
        Stroke::new(1.0, theme.text),
    );

    visuals.widgets.inactive.weak_bg_fill = theme.surface;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, theme.border);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, theme.border);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, theme.accent_text);
    visuals.override_text_color = Some(theme.text);

    ctx.all_styles_mut(|style| {
        style.visuals = visuals.clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.spacing.interact_size.y = 28.0;
    });
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
}
