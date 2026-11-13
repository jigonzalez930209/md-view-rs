//! The nine theme variants.

use egui::Color32;

use crate::{Theme, ThemeMode};

/// A stable theme identifier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ThemeId {
    /// GitHub Light.
    #[default]
    GithubLight,
    /// GitHub Dark.
    GithubDark,
    /// GitHub Dark Dimmed.
    GithubDarkDimmed,
    /// Dracula.
    Dracula,
    /// Dracula Soft.
    DraculaSoft,
    /// Alucard (Dracula's light theme).
    Alucard,
    /// One Dark Pro.
    OneDarkPro,
    /// One Dark Pro Darker.
    OneDarkProDarker,
    /// One Dark Pro Light (a derived variant).
    OneDarkProLight,
}

impl ThemeId {
    /// Every variant in display order.
    pub const ALL: [Self; 9] = [
        Self::GithubLight,
        Self::GithubDark,
        Self::GithubDarkDimmed,
        Self::Dracula,
        Self::DraculaSoft,
        Self::Alucard,
        Self::OneDarkPro,
        Self::OneDarkProDarker,
        Self::OneDarkProLight,
    ];

    /// Parse a stored identifier, including the old `light`/`dark` values.
    pub fn from_setting(value: &str) -> Self {
        match value {
            "github-light" | "light" => Self::GithubLight,
            "github-dark" | "dark" => Self::GithubDark,
            "github-dark-dimmed" => Self::GithubDarkDimmed,
            "dracula" => Self::Dracula,
            "dracula-soft" => Self::DraculaSoft,
            "alucard" => Self::Alucard,
            "one-dark-pro" => Self::OneDarkPro,
            "one-dark-pro-darker" => Self::OneDarkProDarker,
            "one-dark-pro-light" => Self::OneDarkProLight,
            _ => Self::GithubLight,
        }
    }

    /// The stored identifier.
    pub fn as_setting(self) -> &'static str {
        match self {
            Self::GithubLight => "github-light",
            Self::GithubDark => "github-dark",
            Self::GithubDarkDimmed => "github-dark-dimmed",
            Self::Dracula => "dracula",
            Self::DraculaSoft => "dracula-soft",
            Self::Alucard => "alucard",
            Self::OneDarkPro => "one-dark-pro",
            Self::OneDarkProDarker => "one-dark-pro-darker",
            Self::OneDarkProLight => "one-dark-pro-light",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::GithubLight => "GitHub Light",
            Self::GithubDark => "GitHub Dark",
            Self::GithubDarkDimmed => "GitHub Dark Dimmed",
            Self::Dracula => "Dracula",
            Self::DraculaSoft => "Dracula Soft",
            Self::Alucard => "Alucard",
            Self::OneDarkPro => "One Dark Pro",
            Self::OneDarkProDarker => "One Dark Pro Darker",
            Self::OneDarkProLight => "One Dark Pro Light",
        }
    }

    /// The family label.
    pub fn family(self) -> &'static str {
        match self {
            Self::GithubLight | Self::GithubDark | Self::GithubDarkDimmed => "GitHub",
            Self::Dracula | Self::DraculaSoft | Self::Alucard => "Dracula",
            Self::OneDarkPro | Self::OneDarkProDarker | Self::OneDarkProLight => "One Dark Pro",
        }
    }

    /// The light/dark mode.
    pub fn mode(self) -> ThemeMode {
        match self {
            Self::GithubLight | Self::Alucard | Self::OneDarkProLight => ThemeMode::Light,
            _ => ThemeMode::Dark,
        }
    }

    /// Choose the light or dark base variant of a family.
    pub fn follow_system(self, dark: bool) -> Self {
        match (self.family(), dark) {
            ("Dracula", false) => Self::Alucard,
            ("Dracula", true) => Self::Dracula,
            ("One Dark Pro", false) => Self::OneDarkProLight,
            ("One Dark Pro", true) => Self::OneDarkPro,
            (_, false) => Self::GithubLight,
            (_, true) => Self::GithubDark,
        }
    }
}

/// Build the token set for a theme identifier.
pub fn theme(id: ThemeId) -> Theme {
    match id {
        ThemeId::GithubLight => Theme {
            background: rgb(0xff, 0xff, 0xff),
            surface: rgb(0xf6, 0xf8, 0xfa),
            surface_raised: rgb(0xff, 0xff, 0xff),
            sunken: rgb(0xf6, 0xf8, 0xfa),
            text: rgb(0x1f, 0x23, 0x28),
            text_muted: rgb(0x59, 0x60, 0x69),
            accent: rgb(0x09, 0x69, 0xda),
            accent_text: rgb(0xff, 0xff, 0xff),
            border: rgb(0xd1, 0xd9, 0xe0),
            selection: rgb(0xb6, 0xd7, 0xff),
            hover: rgb(0xea, 0xee, 0xf2),
            danger: rgb(0xcf, 0x22, 0x2e),
            warning: rgb(0x9a, 0x67, 0x00),
            success: rgb(0x1a, 0x7f, 0x37),
            link: rgb(0x09, 0x69, 0xda),
            scrollbar: rgb(0x8c, 0x95, 0x9f),
            focus: rgb(0x09, 0x69, 0xda),
            table_header: rgb(0xea, 0xee, 0xf2),
            table_zebra: rgb(0xf6, 0xf8, 0xfa),
            quote_border: rgb(0xd1, 0xd9, 0xe0),
            code_background: rgb(0xf6, 0xf8, 0xfa),
        },
        ThemeId::GithubDark => Theme {
            background: rgb(0x0d, 0x11, 0x17),
            surface: rgb(0x16, 0x1b, 0x22),
            surface_raised: rgb(0x21, 0x26, 0x2d),
            sunken: rgb(0x16, 0x1b, 0x22),
            text: rgb(0xe6, 0xed, 0xf3),
            text_muted: rgb(0x9f, 0xa7, 0xb2),
            accent: rgb(0x1f, 0x6f, 0xeb),
            accent_text: rgb(0xff, 0xff, 0xff),
            border: rgb(0x30, 0x36, 0x3d),
            selection: rgb(0x26, 0x4f, 0x78),
            hover: rgb(0x21, 0x26, 0x2d),
            danger: rgb(0xff, 0x7b, 0x72),
            warning: rgb(0xe3, 0xb3, 0x41),
            success: rgb(0x3f, 0xb9, 0x50),
            link: rgb(0x58, 0xa6, 0xff),
            scrollbar: rgb(0x6e, 0x76, 0x81),
            focus: rgb(0x58, 0xa6, 0xff),
            table_header: rgb(0x21, 0x26, 0x2d),
            table_zebra: rgb(0x16, 0x1b, 0x22),
            quote_border: rgb(0x30, 0x36, 0x3d),
            code_background: rgb(0x16, 0x1b, 0x22),
        },
        ThemeId::GithubDarkDimmed => Theme {
            background: rgb(0x22, 0x27, 0x2e),
            surface: rgb(0x2d, 0x33, 0x3b),
            surface_raised: rgb(0x37, 0x3e, 0x47),
            sunken: rgb(0x2d, 0x33, 0x3b),
            text: rgb(0xad, 0xba, 0xc7),
            text_muted: rgb(0x8b, 0x94, 0x9e),
            accent: rgb(0x53, 0x9b, 0xf5),
            accent_text: rgb(0x22, 0x27, 0x2e),
            border: rgb(0x44, 0x4c, 0x56),
            selection: rgb(0x3b, 0x50, 0x75),
            hover: rgb(0x37, 0x3e, 0x47),
            danger: rgb(0xf4, 0x70, 0x67),
            warning: rgb(0xda, 0xaa, 0x3d),
            success: rgb(0x57, 0xab, 0x5a),
            link: rgb(0x53, 0x9b, 0xf5),
            scrollbar: rgb(0x63, 0x6e, 0x7b),
            focus: rgb(0x53, 0x9b, 0xf5),
            table_header: rgb(0x37, 0x3e, 0x47),
            table_zebra: rgb(0x2d, 0x33, 0x3b),
            quote_border: rgb(0x44, 0x4c, 0x56),
            code_background: rgb(0x2d, 0x33, 0x3b),
        },
        ThemeId::Dracula => Theme {
            background: rgb(0x28, 0x2a, 0x36),
            surface: rgb(0x34, 0x37, 0x46),
            surface_raised: rgb(0x3c, 0x40, 0x52),
            sunken: rgb(0x21, 0x22, 0x2c),
            text: rgb(0xf8, 0xf8, 0xf2),
            text_muted: rgb(0xbf, 0xbf, 0xd0),
            accent: rgb(0xbd, 0x93, 0xf9),
            accent_text: rgb(0x28, 0x2a, 0x36),
            border: rgb(0x44, 0x47, 0x5a),
            selection: rgb(0x44, 0x47, 0x5a),
            hover: rgb(0x3c, 0x40, 0x52),
            danger: rgb(0xff, 0x55, 0x55),
            warning: rgb(0xff, 0xb8, 0x6c),
            success: rgb(0x50, 0xfa, 0x7b),
            link: rgb(0x8b, 0xe9, 0xfd),
            scrollbar: rgb(0x62, 0x72, 0xa4),
            focus: rgb(0xbd, 0x93, 0xf9),
            table_header: rgb(0x3c, 0x40, 0x52),
            table_zebra: rgb(0x34, 0x37, 0x46),
            quote_border: rgb(0x62, 0x72, 0xa4),
            code_background: rgb(0x21, 0x22, 0x2c),
        },
        ThemeId::DraculaSoft => Theme {
            background: rgb(0x2b, 0x2d, 0x3a),
            surface: rgb(0x36, 0x39, 0x48),
            surface_raised: rgb(0x40, 0x44, 0x55),
            sunken: rgb(0x24, 0x26, 0x31),
            text: rgb(0xf2, 0xf2, 0xec),
            text_muted: rgb(0xb8, 0xb8, 0xc8),
            accent: rgb(0xb4, 0x8c, 0xf0),
            accent_text: rgb(0x2b, 0x2d, 0x3a),
            border: rgb(0x4a, 0x4d, 0x60),
            selection: rgb(0x4a, 0x4d, 0x60),
            hover: rgb(0x40, 0x44, 0x55),
            danger: rgb(0xf0, 0x70, 0x70),
            warning: rgb(0xed, 0xb8, 0x7a),
            success: rgb(0x6a, 0xd9, 0x8a),
            link: rgb(0x9a, 0xd8, 0xe8),
            scrollbar: rgb(0x66, 0x70, 0x90),
            focus: rgb(0xb4, 0x8c, 0xf0),
            table_header: rgb(0x40, 0x44, 0x55),
            table_zebra: rgb(0x36, 0x39, 0x48),
            quote_border: rgb(0x66, 0x70, 0x90),
            code_background: rgb(0x24, 0x26, 0x31),
        },
        ThemeId::Alucard => Theme {
            background: rgb(0xff, 0xff, 0xff),
            surface: rgb(0xf5, 0xf5, 0xf5),
            surface_raised: rgb(0xff, 0xff, 0xff),
            sunken: rgb(0xee, 0xee, 0xee),
            text: rgb(0x1f, 0x1f, 0x1f),
            text_muted: rgb(0x62, 0x62, 0x62),
            accent: rgb(0x64, 0x4a, 0xc9),
            accent_text: rgb(0xff, 0xff, 0xff),
            border: rgb(0xcf, 0xcf, 0xcf),
            selection: rgb(0xdd, 0xd2, 0xfb),
            hover: rgb(0xf0, 0xf0, 0xf0),
            danger: rgb(0xcb, 0x3a, 0x2f),
            warning: rgb(0x84, 0x6e, 0x15),
            success: rgb(0x14, 0x7d, 0x30),
            link: rgb(0x64, 0x4a, 0xc9),
            scrollbar: rgb(0x8a, 0x8a, 0x8a),
            focus: rgb(0x64, 0x4a, 0xc9),
            table_header: rgb(0xf0, 0xf0, 0xf0),
            table_zebra: rgb(0xf5, 0xf5, 0xf5),
            quote_border: rgb(0xcf, 0xcf, 0xcf),
            code_background: rgb(0xee, 0xee, 0xee),
        },
        ThemeId::OneDarkPro => Theme {
            background: rgb(0x28, 0x2c, 0x34),
            surface: rgb(0x21, 0x25, 0x2b),
            surface_raised: rgb(0x2c, 0x31, 0x3a),
            sunken: rgb(0x1e, 0x22, 0x27),
            text: rgb(0xab, 0xb2, 0xbf),
            text_muted: rgb(0x9a, 0xa2, 0xb1),
            accent: rgb(0x61, 0xaf, 0xef),
            accent_text: rgb(0x1e, 0x22, 0x27),
            border: rgb(0x3e, 0x44, 0x51),
            selection: rgb(0x3e, 0x44, 0x51),
            hover: rgb(0x2c, 0x31, 0x3a),
            danger: rgb(0xf2, 0x83, 0x8c),
            warning: rgb(0xe5, 0xc0, 0x7b),
            success: rgb(0x98, 0xc3, 0x79),
            link: rgb(0x61, 0xaf, 0xef),
            scrollbar: rgb(0x4b, 0x52, 0x64),
            focus: rgb(0x61, 0xaf, 0xef),
            table_header: rgb(0x2c, 0x31, 0x3a),
            table_zebra: rgb(0x21, 0x25, 0x2b),
            quote_border: rgb(0x3e, 0x44, 0x51),
            code_background: rgb(0x1e, 0x22, 0x27),
        },
        ThemeId::OneDarkProDarker => Theme {
            background: rgb(0x1e, 0x22, 0x27),
            surface: rgb(0x18, 0x1b, 0x20),
            surface_raised: rgb(0x25, 0x2a, 0x31),
            sunken: rgb(0x15, 0x18, 0x1c),
            text: rgb(0xb0, 0xb7, 0xc4),
            text_muted: rgb(0x9a, 0xa2, 0xb1),
            accent: rgb(0x5c, 0xa7, 0xe6),
            accent_text: rgb(0x15, 0x18, 0x1c),
            border: rgb(0x35, 0x3b, 0x45),
            selection: rgb(0x35, 0x3b, 0x45),
            hover: rgb(0x25, 0x2a, 0x31),
            danger: rgb(0xec, 0x7d, 0x86),
            warning: rgb(0xdc, 0xb7, 0x72),
            success: rgb(0x91, 0xbc, 0x72),
            link: rgb(0x5c, 0xa7, 0xe6),
            scrollbar: rgb(0x43, 0x49, 0x58),
            focus: rgb(0x5c, 0xa7, 0xe6),
            table_header: rgb(0x25, 0x2a, 0x31),
            table_zebra: rgb(0x18, 0x1b, 0x20),
            quote_border: rgb(0x35, 0x3b, 0x45),
            code_background: rgb(0x15, 0x18, 0x1c),
        },
        ThemeId::OneDarkProLight => Theme {
            background: rgb(0xfa, 0xfa, 0xfa),
            surface: rgb(0xf0, 0xf0, 0xf0),
            surface_raised: rgb(0xff, 0xff, 0xff),
            sunken: rgb(0xe5, 0xe5, 0xe6),
            text: rgb(0x38, 0x3a, 0x42),
            text_muted: rgb(0x69, 0x6c, 0x77),
            accent: rgb(0x2f, 0x6b, 0xd8),
            accent_text: rgb(0xff, 0xff, 0xff),
            border: rgb(0xd3, 0xd3, 0xd6),
            selection: rgb(0xd0, 0xe4, 0xff),
            hover: rgb(0xec, 0xec, 0xee),
            danger: rgb(0xc0, 0x2b, 0x20),
            warning: rgb(0x98, 0x64, 0x00),
            success: rgb(0x22, 0x7d, 0x3a),
            link: rgb(0x2f, 0x6b, 0xd8),
            scrollbar: rgb(0x9a, 0x9a, 0xa0),
            focus: rgb(0x2f, 0x6b, 0xd8),
            table_header: rgb(0xec, 0xec, 0xee),
            table_zebra: rgb(0xf0, 0xf0, 0xf0),
            quote_border: rgb(0xd3, 0xd3, 0xd6),
            code_background: rgb(0xe5, 0xe5, 0xe6),
        },
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_round_trips_and_has_a_family() {
        for id in ThemeId::ALL {
            assert_eq!(ThemeId::from_setting(id.as_setting()), id);
            assert!(!id.family().is_empty());
            assert!(!id.label().is_empty());
        }
        assert_eq!(ThemeId::from_setting("light"), ThemeId::GithubLight);
        assert_eq!(ThemeId::from_setting("dark"), ThemeId::GithubDark);
    }

    #[test]
    fn follow_system_keeps_the_family() {
        assert_eq!(ThemeId::Dracula.follow_system(false), ThemeId::Alucard);
        assert_eq!(ThemeId::Alucard.follow_system(true), ThemeId::Dracula);
        assert_eq!(ThemeId::OneDarkProLight.follow_system(true), ThemeId::OneDarkPro);
        assert_eq!(ThemeId::GithubDark.follow_system(true), ThemeId::GithubDark);
    }
}
