//! Document presentation modes.

/// How a tab presents its document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewMode {
    /// Source editing with the code editor.
    #[default]
    Edit,
    /// Rendered Markdown reading view.
    View,
    /// Editor and preview side by side.
    Split,
}

impl ViewMode {
    /// Parse a value stored in settings.
    pub fn from_setting(value: &str) -> Self {
        match value {
            "view" => Self::View,
            "split" => Self::Split,
            _ => Self::Edit,
        }
    }

    /// The value stored in settings.
    pub fn as_setting(self) -> &'static str {
        match self {
            Self::Edit => "edit",
            Self::View => "view",
            Self::Split => "split",
        }
    }

    /// The translation key for a menu label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Edit => "Edit mode",
            Self::View => "Reading view",
            Self::Split => "Split view",
        }
    }

    /// Whether the mode paints the rendered preview.
    pub fn shows_preview(self) -> bool {
        matches!(self, Self::View | Self::Split)
    }

    /// Whether the mode paints the source editor.
    pub fn shows_editor(self) -> bool {
        matches!(self, Self::Edit | Self::Split)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_round_trip_through_settings() {
        for mode in [ViewMode::Edit, ViewMode::View, ViewMode::Split] {
            assert_eq!(ViewMode::from_setting(mode.as_setting()), mode);
        }
        assert_eq!(ViewMode::from_setting("unknown"), ViewMode::Edit);
    }

    #[test]
    fn modes_describe_their_panes() {
        assert!(ViewMode::Edit.shows_editor());
        assert!(!ViewMode::Edit.shows_preview());
        assert!(ViewMode::View.shows_preview());
        assert!(ViewMode::Split.shows_editor() && ViewMode::Split.shows_preview());
    }
}
