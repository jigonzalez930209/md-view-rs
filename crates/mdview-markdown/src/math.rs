//! Locate formulas in Markdown source for editing.
//!
//! The formula editor opens on a double-click inside a formula and writes the edited TeX
//! back over the same source. It uses the parser the preview uses, so `$…$`, `$$…$$`, and
//! ```` ```math ```` fences are found exactly where the preview renders them, and dollar
//! signs inside code spans or escaped as `\$` are never mistaken for formulas.

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};

/// How a formula is delimited in the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathDelimiter {
    /// `$…$` inside a paragraph.
    Inline,
    /// `$$…$$`.
    Display,
    /// A ```` ```math ```` fenced block.
    Fence,
}

/// One formula in Markdown source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathSource {
    /// Byte range of the whole formula, delimiters included.
    pub range: Range<usize>,
    /// The TeX between the delimiters.
    pub tex: String,
    /// How the formula is delimited.
    pub delimiter: MathDelimiter,
}

impl MathSource {
    /// Whether the formula renders as a centered block.
    pub fn is_display(&self) -> bool {
        self.delimiter != MathDelimiter::Inline
    }
}

/// Every formula in `source`, in document order.
pub fn math_sources(source: &str) -> Vec<MathSource> {
    let mut formulas = Vec::new();
    let mut fence: Option<(Range<usize>, String)> = None;
    for (event, range) in Parser::new_ext(source, crate::options()).into_offset_iter() {
        match event {
            Event::InlineMath(tex) => formulas.push(MathSource {
                range,
                tex: tex.into_string(),
                delimiter: MathDelimiter::Inline,
            }),
            Event::DisplayMath(tex) => formulas.push(MathSource {
                range,
                tex: tex.trim().to_owned(),
                delimiter: MathDelimiter::Display,
            }),
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(language)))
                if language.split_whitespace().next() == Some("math") =>
            {
                fence = Some((range, String::new()));
            }
            Event::Text(text) => {
                if let Some((_, tex)) = fence.as_mut() {
                    tex.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((range, tex)) = fence.take() {
                    formulas.push(MathSource {
                        range,
                        tex: tex.trim().to_owned(),
                        delimiter: MathDelimiter::Fence,
                    });
                }
            }
            _ => {}
        }
    }
    formulas
}

/// The formula whose source contains the byte `offset`, if any.
pub fn math_source_at(source: &str, offset: usize) -> Option<MathSource> {
    math_sources(source)
        .into_iter()
        .find(|formula| formula.range.start <= offset && offset < formula.range.end)
}

/// Markdown source for a formula, delimited for `display` or inline use.
///
/// A display formula keeps a fence when `fence` is set, so editing a ```` ```math ```` block
/// does not rewrite it as `$$`.
pub fn format_math(tex: &str, display: bool, fence: bool) -> String {
    let tex = tex.trim();
    match (display, fence) {
        (true, true) => format!("```math\n{tex}\n```"),
        (true, false) => format!("$$\n{tex}\n$$"),
        // Inline math cannot span lines; join them so the formula stays one token.
        (false, _) => format!("${}$", tex.lines().map(str::trim).collect::<Vec<_>>().join(" ")),
    }
}

#[cfg(test)]
mod tests {
    use super::{MathDelimiter, format_math, math_source_at, math_sources};

    #[test]
    fn finds_inline_display_and_fenced_math() {
        let source = "Mass $E = mc^2$ here.\n\n$$\n\\frac{a}{b}\n$$\n\n```math\nx^2\n```\n";
        let formulas = math_sources(source);
        assert_eq!(formulas.len(), 3);
        assert_eq!(formulas[0].tex, "E = mc^2");
        assert_eq!(&source[formulas[0].range.clone()], "$E = mc^2$");
        assert_eq!(formulas[0].delimiter, MathDelimiter::Inline);
        assert_eq!(formulas[1].tex, "\\frac{a}{b}");
        assert_eq!(&source[formulas[1].range.clone()], "$$\n\\frac{a}{b}\n$$");
        assert_eq!(formulas[2].tex, "x^2");
        assert_eq!(formulas[2].delimiter, MathDelimiter::Fence);
        assert!(formulas[2].is_display());
    }

    #[test]
    fn ignores_code_spans_and_escaped_dollars() {
        assert!(math_sources("Price `$5 and $6` and \\$7 or \\$8.").is_empty());
    }

    #[test]
    fn finds_the_formula_under_an_offset() {
        let source = "a $x$ b $y$";
        assert_eq!(
            math_source_at(source, 9).map(|formula| formula.tex),
            Some("y".to_owned())
        );
        assert_eq!(math_source_at(source, 0), None);
    }

    #[test]
    fn formats_each_delimiter() {
        assert_eq!(format_math(" x ", false, false), "$x$");
        assert_eq!(format_math("a\n+ b", false, false), "$a + b$");
        assert_eq!(format_math("x", true, false), "$$\nx\n$$");
        assert_eq!(format_math("x", true, true), "```math\nx\n```");
    }
}
