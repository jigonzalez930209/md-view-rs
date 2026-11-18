//! Formula templates: the Word-style gallery of common equations and the editor palette.
//!
//! Templates use `\square` as the placeholder. It is valid TeX, so a template renders as
//! soon as it is inserted, and Tab in the formula editor moves to the next one.

/// The placeholder a template leaves for the user to fill.
pub const PLACEHOLDER: &str = "\\square";

/// A named equation from the gallery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GalleryEquation {
    /// English name, translated by the caller.
    pub name: &'static str,
    /// TeX source.
    pub tex: &'static str,
}

/// The common equations offered by the formatting bar, as in Word's equation gallery.
pub const GALLERY: &[GalleryEquation] = &[
    GalleryEquation {
        name: "Area of circle",
        tex: "A = \\pi r^2",
    },
    GalleryEquation {
        name: "Binomial theorem",
        tex: "(x + a)^n = \\sum_{k=0}^{n} \\binom{n}{k} x^k a^{n-k}",
    },
    GalleryEquation {
        name: "Expansion of a sum",
        tex: "(1 + x)^n = 1 + \\frac{nx}{1!} + \\frac{n(n-1)x^2}{2!} + \\cdots",
    },
    GalleryEquation {
        name: "Fourier series",
        tex: "f(x) = a_0 + \\sum_{n=1}^{\\infty} \\left( a_n \\cos\\frac{n\\pi x}{L} + b_n \\sin\\frac{n\\pi x}{L} \\right)",
    },
    GalleryEquation {
        name: "Pythagorean theorem",
        tex: "a^2 + b^2 = c^2",
    },
    GalleryEquation {
        name: "Quadratic formula",
        tex: "x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}",
    },
    GalleryEquation {
        name: "Taylor expansion",
        tex: "e^x = 1 + \\frac{x}{1!} + \\frac{x^2}{2!} + \\frac{x^3}{3!} + \\cdots, \\quad -\\infty < x < \\infty",
    },
    GalleryEquation {
        name: "Trig identity",
        tex: "\\sin \\alpha \\pm \\sin \\beta = 2 \\sin \\frac{1}{2}(\\alpha \\pm \\beta) \\cos \\frac{1}{2}(\\alpha \\mp \\beta)",
    },
    GalleryEquation {
        name: "Euler's identity",
        tex: "e^{i\\pi} + 1 = 0",
    },
    GalleryEquation {
        name: "Definite integral",
        tex: "\\int_a^b f(x)\\,dx = F(b) - F(a)",
    },
    GalleryEquation {
        name: "Derivative",
        tex: "f'(x) = \\lim_{h \\to 0} \\frac{f(x + h) - f(x)}{h}",
    },
    GalleryEquation {
        name: "Normal distribution",
        tex: "f(x) = \\frac{1}{\\sigma \\sqrt{2\\pi}} e^{-\\frac{(x - \\mu)^2}{2\\sigma^2}}",
    },
];

/// A palette category in the formula editor.
#[derive(Clone, Copy, Debug)]
pub struct PaletteCategory {
    /// English name, translated by the caller.
    pub name: &'static str,
    /// TeX templates; each is also drawn as its own button face.
    pub templates: &'static [&'static str],
}

/// The palette, ported from the sci-notebook visual formula builder.
pub const PALETTE: &[PaletteCategory] = &[
    PaletteCategory {
        name: "Structures",
        templates: &[
            "\\frac{\\square}{\\square}",
            "\\sqrt{\\square}",
            "\\sqrt[\\square]{\\square}",
            "{\\square}^{\\square}",
            "{\\square}_{\\square}",
            "{\\square}_{\\square}^{\\square}",
            "\\hat{\\square}",
            "\\bar{\\square}",
            "\\vec{\\square}",
            "\\tilde{\\square}",
            "\\dot{\\square}",
            "\\ddot{\\square}",
            "\\binom{\\square}{\\square}",
            "\\overline{\\square}",
        ],
    },
    PaletteCategory {
        name: "Integrals",
        templates: &[
            "\\int \\square \\, d\\square",
            "\\int_{\\square}^{\\square} \\square \\, d\\square",
            "\\iint \\square",
            "\\iiint \\square",
            "\\oint \\square",
        ],
    },
    PaletteCategory {
        name: "Sums and limits",
        templates: &[
            "\\sum_{\\square}^{\\square} \\square",
            "\\prod_{\\square}^{\\square} \\square",
            "\\sum_{n=\\square}^{\\infty} \\square",
            "\\lim_{\\square \\to \\square} \\square",
            "\\frac{d}{d\\square} \\square",
            "\\frac{\\partial \\square}{\\partial \\square}",
        ],
    },
    PaletteCategory {
        name: "Matrices",
        templates: &[
            "\\begin{pmatrix} \\square & \\square \\\\ \\square & \\square \\end{pmatrix}",
            "\\begin{pmatrix} \\square & \\square & \\square \\\\ \\square & \\square & \\square \\\\ \\square & \\square & \\square \\end{pmatrix}",
            "\\begin{bmatrix} \\square & \\square \\\\ \\square & \\square \\end{bmatrix}",
            "\\begin{vmatrix} \\square & \\square \\\\ \\square & \\square \\end{vmatrix}",
            "\\begin{cases} \\square & \\text{if } \\square \\\\ \\square & \\text{otherwise} \\end{cases}",
        ],
    },
    PaletteCategory {
        name: "Greek",
        templates: &[
            "\\alpha",
            "\\beta",
            "\\gamma",
            "\\delta",
            "\\epsilon",
            "\\zeta",
            "\\eta",
            "\\theta",
            "\\lambda",
            "\\mu",
            "\\pi",
            "\\rho",
            "\\sigma",
            "\\tau",
            "\\phi",
            "\\psi",
            "\\omega",
            "\\Gamma",
            "\\Delta",
            "\\Theta",
            "\\Lambda",
            "\\Sigma",
            "\\Phi",
            "\\Psi",
            "\\Omega",
        ],
    },
    PaletteCategory {
        name: "Operators",
        templates: &[
            "\\pm",
            "\\mp",
            "\\times",
            "\\div",
            "\\cdot",
            "\\circ",
            "\\otimes",
            "\\oplus",
            "\\partial",
            "\\nabla",
            "\\infty",
            "\\approx",
            "\\neq",
            "\\leq",
            "\\geq",
            "\\equiv",
            "\\propto",
            "\\in",
            "\\notin",
            "\\subset",
            "\\supset",
            "\\cup",
            "\\cap",
            "\\emptyset",
            "\\forall",
            "\\exists",
        ],
    },
    PaletteCategory {
        name: "Arrows",
        templates: &[
            "\\rightarrow",
            "\\leftarrow",
            "\\leftrightarrow",
            "\\Rightarrow",
            "\\Leftarrow",
            "\\Leftrightarrow",
            "\\mapsto",
            "\\uparrow",
            "\\downarrow",
        ],
    },
    PaletteCategory {
        name: "Functions",
        templates: &[
            "\\sin \\square",
            "\\cos \\square",
            "\\tan \\square",
            "\\log \\square",
            "\\ln \\square",
            "\\exp \\square",
            "\\max \\square",
            "\\min \\square",
            "\\det \\square",
        ],
    },
    PaletteCategory {
        name: "Delimiters",
        templates: &[
            "\\left( \\square \\right)",
            "\\left[ \\square \\right]",
            "\\left\\{ \\square \\right\\}",
            "\\left| \\square \\right|",
            "\\left\\| \\square \\right\\|",
            "\\lfloor \\square \\rfloor",
            "\\lceil \\square \\rceil",
        ],
    },
];

/// The byte range of the first placeholder at or after `from`, wrapping to the start.
pub fn next_placeholder(tex: &str, from: usize) -> Option<std::ops::Range<usize>> {
    let from = from.min(tex.len());
    let after = tex
        .get(from..)
        .and_then(|rest| rest.find(PLACEHOLDER))
        .map(|at| from + at);
    let start = after.or_else(|| tex.find(PLACEHOLDER))?;
    Some(start..start + PLACEHOLDER.len())
}

#[cfg(test)]
mod tests {
    use super::{GALLERY, PALETTE, next_placeholder};

    #[test]
    fn every_template_renders() {
        let templates = GALLERY
            .iter()
            .map(|equation| equation.tex)
            .chain(PALETTE.iter().flat_map(|category| category.templates.iter().copied()));
        for tex in templates {
            mdview_media::render_math(tex, false, 600).unwrap_or_else(|error| panic!("{tex}: {error}"));
        }
    }

    #[test]
    fn placeholders_cycle() {
        let tex = "\\frac{\\square}{\\square}";
        assert_eq!(next_placeholder(tex, 0), Some(6..13));
        assert_eq!(next_placeholder(tex, 13), Some(15..22));
        assert_eq!(next_placeholder(tex, 22), Some(6..13));
        assert_eq!(next_placeholder("x", 0), None);
    }
}
