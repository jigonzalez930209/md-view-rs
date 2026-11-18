use super::{highlight_math_at, math_spans_at};
use crate::render_math;

/// The innermost highlighted source text for the first occurrence of `needle`.
fn highlighted<'a>(tex: &'a str, needle: &str) -> &'a str {
    let offset = tex.find(needle).unwrap_or_else(|| panic!("{needle} in {tex}"));
    let highlight = highlight_math_at(tex, offset, "#ff0000").unwrap_or_else(|| panic!("{needle} in {tex}"));
    &tex[highlight.span]
}

#[test]
fn leaves_highlight_themselves() {
    assert_eq!(highlighted("x^2 + y", "y"), "y");
    assert_eq!(highlighted("x^2 + y", "+"), "+");
    // A colored base would lose its italic correction and shift the scripts, so the base
    // of a scripted symbol highlights the whole symbol.
    assert_eq!(highlighted("\\alpha_{i}^{n}", "\\alpha"), "\\alpha_{i}^{n}");
    assert_eq!(highlighted("\\alpha + \\beta", "\\alpha"), "\\alpha");
    assert_eq!(highlighted("\\alpha_{i}^{n}", "i"), "i");
}

#[test]
fn scripts_highlight_their_part() {
    assert_eq!(highlighted("x^2 + y", "2"), "2");
    assert_eq!(highlighted("x^2 + y", "^"), "x^2");
}

#[test]
fn commands_highlight_the_whole_construct() {
    assert_eq!(highlighted("a + \\frac{b}{c}", "\\frac"), "\\frac{b}{c}");
    assert_eq!(highlighted("a + \\frac{b}{c}", "b"), "b");
    assert_eq!(highlighted("\\sqrt[3]{x} + 1", "sqrt"), "\\sqrt[3]{x}");
    // An operator with limits highlights together with them, so the limits stay in place.
    assert_eq!(highlighted("\\int_0^1 f(x)\\,dx", "\\int"), "\\int_0^1");
    assert_eq!(highlighted("\\int_0^1 f(x)\\,dx", "_"), "\\int_0^1");
    assert_eq!(highlighted("\\sum x", "sum"), "\\sum");
}

#[test]
fn delimiters_and_environments_close() {
    assert_eq!(highlighted("\\left( a \\right) + b", "\\left"), "\\left( a \\right)");
    assert_eq!(highlighted("\\left( a \\right) + b", "\\right"), "\\left( a \\right)");
    let matrix = "\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}";
    assert_eq!(highlighted(matrix, "c"), "c");
    assert_eq!(highlighted(matrix, "\\begin"), matrix);
    assert_eq!(highlighted(matrix, "\\end"), matrix);
}

#[test]
fn highlighted_source_renders() {
    for (tex, needle) in [
        ("x^2 + \\frac{a+b}{c}", "frac"),
        ("\\sqrt{x_1}", "1"),
        ("\\left[ \\frac{1}{2} \\right]", "left"),
        ("\\begin{cases} x & x > 0 \\\\ -x & x \\le 0 \\end{cases}", "le"),
        ("\\lim_{n \\to \\infty} \\left(1 + \\frac{1}{n}\\right)^n", "infty"),
    ] {
        let offset = tex.find(needle).unwrap_or_default();
        let highlight = highlight_math_at(tex, offset, "#0969da").unwrap_or_else(|| panic!("{tex}"));
        assert!(highlight.tex.contains("\\textcolor{#0969da}"));
        render_math(&highlight.tex, false, 600).unwrap_or_else(|error| panic!("{tex}: {error}"));
    }
}

#[test]
fn coloring_never_changes_the_layout() {
    // The case from the formula editor: coloring `[9]` must keep it the root's index.
    let tex = "e^x = \\sum_{n=0}^{\\infty} \\frac{x^n}{n-3!}x\\sqrt[9]{10}";
    let reference = super::geometry(tex).expect("layout");
    for offset in 0..tex.len() {
        if let Some(highlight) = highlight_math_at(tex, offset, "#0969da") {
            let shape = super::geometry(&highlight.tex).expect("layout");
            assert!(
                super::same_shape(&shape, &reference),
                "offset {offset}: {}",
                highlight.tex
            );
        }
    }
    assert_eq!(highlighted(tex, "9"), "9");
    assert_eq!(highlighted(tex, "\\sum"), "\\sum_{n=0}^{\\infty}");
    assert_eq!(highlighted(tex, "="), "=");
}

#[test]
fn fractions_keep_their_inner_spacing() {
    // A fraction is an inner atom with a thin space on each side; a colored one must keep it.
    let tex = "i\\hbar\\frac{\\partial}{\\partial t}\\Psi = -\\frac{\\hbar^2}{2m}\\nabla^2\\Psi";
    assert_eq!(highlighted(tex, "\\frac"), "\\frac{\\partial}{\\partial t}");
    assert_eq!(highlighted(tex, "-\\frac"), "-");
    let second = tex.rfind("\\frac").expect("second fraction");
    let highlight = highlight_math_at(tex, second, "#ff0000").expect("highlight");
    assert_eq!(&tex[highlight.span], "\\frac{\\hbar^2}{2m}");
}

#[test]
fn invalid_or_blank_positions_find_nothing() {
    assert!(math_spans_at("\\frac{1", 2).is_empty());
    assert!(highlight_math_at("a  +  b", 2, "#000000").is_none());
    assert!(highlight_math_at("", 0, "#000000").is_none());
}

#[test]
fn spans_nest_from_innermost() {
    let spans = math_spans_at("\\frac{a+b}{c}", 6);
    let texts: Vec<&str> = spans.iter().map(|span| &"\\frac{a+b}{c}"[span.clone()]).collect();
    assert_eq!(texts.first(), Some(&"a"));
    assert!(texts.contains(&"{a+b}"));
    assert_eq!(texts.last(), Some(&"\\frac{a+b}{c}"));
}
