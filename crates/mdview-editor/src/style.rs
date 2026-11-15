//! Editor typography, whitespace, and scrolling style.

/// How whitespace is made visible in the editor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WhitespaceMode {
    /// Never show whitespace.
    None,
    /// Show whitespace only inside the selection.
    #[default]
    Selection,
    /// Show every space and tab.
    All,
}

impl WhitespaceMode {
    /// Parse a persisted setting.
    pub fn from_setting(value: &str) -> Self {
        match value {
            "none" => Self::None,
            "all" => Self::All,
            _ => Self::Selection,
        }
    }

    /// The persisted setting value.
    pub fn as_setting(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Selection => "selection",
            Self::All => "all",
        }
    }
}

/// Visual and scrolling options shared by every editor tab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditorStyle {
    /// Monospace font size in points.
    pub font_size: f32,
    /// Line-height multiplier.
    pub line_height: f32,
    /// Allow the last line to reach the top of the viewport.
    pub scroll_beyond_last_line: bool,
    /// Draw indentation guides.
    pub indent_guides: bool,
    /// Whitespace visibility mode.
    pub whitespace: WhitespaceMode,
}

impl Default for EditorStyle {
    fn default() -> Self {
        Self {
            font_size: 13.0,
            line_height: 1.45,
            scroll_beyond_last_line: true,
            indent_guides: true,
            whitespace: WhitespaceMode::Selection,
        }
    }
}

impl EditorStyle {
    /// Clamp values to the supported range.
    pub fn sanitized(self) -> Self {
        Self {
            font_size: self.font_size.clamp(10.0, 22.0),
            line_height: self.line_height.clamp(1.1, 2.0),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_modes_round_trip() {
        for mode in [WhitespaceMode::None, WhitespaceMode::Selection, WhitespaceMode::All] {
            assert_eq!(WhitespaceMode::from_setting(mode.as_setting()), mode);
        }
        assert_eq!(WhitespaceMode::from_setting("unknown"), WhitespaceMode::Selection);
    }

    #[test]
    fn style_is_bounded() {
        let style = EditorStyle {
            font_size: 1.0,
            line_height: 9.0,
            ..EditorStyle::default()
        }
        .sanitized();
        assert_eq!(style.font_size, 10.0);
        assert_eq!(style.line_height, 2.0);
    }
}
