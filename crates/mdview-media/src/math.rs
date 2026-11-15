//! LaTeX math layout and rasterization with RaTeX.

use ratex_layout::{LayoutOptions, layout, to_display_list};
use ratex_parser::parser::Parser;
use ratex_svg::{SvgOptions, render_to_svg};
use ratex_types::color::Color;

use crate::{MediaError, MediaKind, MediaRequest, Raster, rasterize_svg};

/// Raster pixels per em. Formulas are rasterized at one pixel per SVG unit, so a
/// consumer scales the texture by `font_size / MATH_UNITS_PER_EM` to match text.
pub const MATH_UNITS_PER_EM: f32 = 40.0;

/// Raster padding around a formula, in pixels at one pixel per SVG unit.
const MATH_PADDING: f32 = 6.0;

/// Baseline of a formula as a fraction of its raster height, measured from the top.
///
/// Inline formulas use it to sit on the text baseline instead of being centered on
/// the line, the way MathJax aligns inline math on GitHub.
pub fn math_baseline(tex: &str) -> Option<f32> {
    let tex = tex.trim();
    if tex.is_empty() {
        return None;
    }
    let nodes = Parser::new(tex).parse().ok()?;
    let display = to_display_list(&layout(&nodes, &LayoutOptions::default()));
    let height = display.height as f32 * MATH_UNITS_PER_EM;
    let total = (display.height + display.depth) as f32 * MATH_UNITS_PER_EM + MATH_PADDING * 2.0;
    (total > 0.0).then(|| ((height + MATH_PADDING) / total).clamp(0.0, 1.0))
}

/// Render a TeX expression to an RGBA raster.
pub fn render_math(tex: &str, dark: bool, max_width: u32) -> Result<Raster, MediaError> {
    let tex = tex.trim();
    if tex.is_empty() {
        return Err(MediaError::Empty);
    }
    let mut parser = Parser::new(tex);
    let nodes = parser.parse().map_err(|error| MediaError::Math(error.to_string()))?;
    let options = LayoutOptions {
        color: if dark { Color::WHITE } else { Color::BLACK },
        ..LayoutOptions::default()
    };
    let root = layout(&nodes, &options);
    let display = to_display_list(&root);
    let svg_options = SvgOptions {
        font_size: f64::from(MATH_UNITS_PER_EM),
        padding: f64::from(MATH_PADDING),
        embed_glyphs: true,
        ..SvgOptions::default()
    };
    let svg = render_to_svg(&display, &svg_options);
    let mut request = MediaRequest::new(MediaKind::Svg, tex);
    request.max_width = max_width.max(32);
    request.max_height = max_width.max(32);
    request.dark = dark;
    rasterize_svg(svg.as_bytes(), &request).map_err(|error| match error {
        MediaError::Svg(message) => MediaError::Math(message),
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_formulas_render() {
        for formula in [
            "a^2 + b^2 = c^2",
            "\\frac{1}{2}",
            "\\sqrt{x + y}",
            "\\int_0^1 x^2 dx",
            "\\sum_{i=1}^{n} i",
            "\\begin{matrix} a & b \\\\ c & d \\end{matrix}",
            "\\begin{cases} x & x > 0 \\\\ -x & x \\le 0 \\end{cases}",
            "\\alpha + \\beta = \\gamma",
            "\\text{hello}",
            "\\left( \\frac{a}{b} \\right)",
        ] {
            let raster = render_math(formula, false, 600).unwrap_or_else(|error| panic!("{formula}: {error}"));
            assert!(raster.width > 0 && raster.height > 0, "{formula}");
            assert!(raster.is_consistent(), "{formula}");
        }
    }

    #[test]
    fn one_hundred_formulas_render() {
        let mut formulas = Vec::new();
        for index in 1..=20 {
            formulas.push(format!("x^{{{index}}} + y_{{{index}}}"));
            formulas.push(format!("\\frac{{{index}}}{{{}}}", index + 1));
            formulas.push(format!("\\sqrt[{index}]{{x}}"));
            formulas.push(format!("\\sum_{{i=1}}^{{{index}}} i"));
            formulas.push(format!("\\int_0^{{{index}}} x dx"));
        }
        for formula in &formulas {
            let raster = render_math(formula, false, 500).unwrap_or_else(|error| panic!("{formula}: {error}"));
            assert!(raster.is_consistent());
        }
        assert_eq!(formulas.len(), 100);
    }

    #[test]
    fn baseline_sits_above_descenders() {
        let plain = math_baseline("x").expect("baseline");
        let fraction = math_baseline("\\frac{1}{2}").expect("baseline");
        let raster = render_math("x", false, 400).expect("render");
        assert!(plain > 0.5 && plain < 1.0, "{plain}");
        assert!(fraction > 0.3 && fraction < 0.8, "{fraction}");
        assert!(raster.height > 0);
    }

    #[test]
    fn invalid_math_reports_an_error() {
        assert!(matches!(render_math("\\frac{1", false, 400), Err(MediaError::Math(_))));
    }

    #[test]
    fn dark_math_has_light_pixels() {
        let raster = render_math("x", true, 200).expect("render");
        let has_light = raster
            .rgba
            .chunks_exact(4)
            .any(|pixel| pixel[3] > 0 && pixel[0] > 200 && pixel[1] > 200 && pixel[2] > 200);
        assert!(has_light);
    }
}
