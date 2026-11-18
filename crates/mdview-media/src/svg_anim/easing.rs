//! Timing curves and color parsing for animation interpolation.

/// How progress through one segment maps to the interpolation fraction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Easing {
    /// Straight line.
    Linear,
    /// A CSS/SMIL cubic Bézier through (0,0), (x1,y1), (x2,y2), (1,1).
    Bezier(f32, f32, f32, f32),
    /// CSS `steps(n, end)` / `steps(n, start)`.
    Steps(u32, bool),
}

impl Easing {
    /// Map a linear fraction in 0..=1 through the curve.
    pub(super) fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::Steps(count, start) => {
                let count = count.max(1) as f32;
                let step = if start { (t * count).ceil() } else { (t * count).floor() };
                (step / count).clamp(0.0, 1.0)
            }
            Self::Bezier(x1, y1, x2, y2) => {
                // Solve x(s) = t for s with Newton steps, falling back to bisection.
                let curve =
                    |a: f32, b: f32, s: f32| 3.0 * a * s * (1.0 - s).powi(2) + 3.0 * b * s * s * (1.0 - s) + s.powi(3);
                let slope = |a: f32, b: f32, s: f32| {
                    3.0 * a * (1.0 - s).powi(2) + 6.0 * (b - a) * s * (1.0 - s) + 3.0 * (1.0 - b) * s * s
                };
                let mut s = t;
                for _ in 0..8 {
                    let error = curve(x1, x2, s) - t;
                    let derivative = slope(x1, x2, s);
                    if error.abs() < 1e-5 || derivative.abs() < 1e-6 {
                        break;
                    }
                    s = (s - error / derivative).clamp(0.0, 1.0);
                }
                if (curve(x1, x2, s) - t).abs() > 1e-3 {
                    let (mut low, mut high) = (0.0_f32, 1.0_f32);
                    for _ in 0..30 {
                        s = (low + high) / 2.0;
                        if curve(x1, x2, s) < t {
                            low = s;
                        } else {
                            high = s;
                        }
                    }
                }
                curve(y1, y2, s)
            }
        }
    }

    /// A CSS `animation-timing-function` keyword or function.
    pub(super) fn from_css(value: &str) -> Option<Self> {
        let value = value.trim();
        Some(match value {
            "linear" => Self::Linear,
            "ease" => Self::Bezier(0.25, 0.1, 0.25, 1.0),
            "ease-in" => Self::Bezier(0.42, 0.0, 1.0, 1.0),
            "ease-out" => Self::Bezier(0.0, 0.0, 0.58, 1.0),
            "ease-in-out" => Self::Bezier(0.42, 0.0, 0.58, 1.0),
            "step-start" => Self::Steps(1, true),
            "step-end" => Self::Steps(1, false),
            _ => {
                if let Some(arguments) = function(value, "cubic-bezier") {
                    let numbers = numbers(arguments);
                    return (numbers.len() == 4).then(|| Self::Bezier(numbers[0], numbers[1], numbers[2], numbers[3]));
                }
                let arguments = function(value, "steps")?;
                let mut parts = arguments.split(',').map(str::trim);
                let count = parts.next()?.parse().ok()?;
                let start = matches!(parts.next(), Some("start" | "jump-start"));
                Self::Steps(count, start)
            }
        })
    }

    /// One SMIL `keySplines` entry: four numbers.
    pub(super) fn from_spline(value: &str) -> Option<Self> {
        let numbers = numbers(value);
        (numbers.len() == 4).then(|| Self::Bezier(numbers[0], numbers[1], numbers[2], numbers[3]))
    }
}

fn function<'a>(value: &'a str, name: &str) -> Option<&'a str> {
    value.strip_prefix(name)?.trim().strip_prefix('(')?.strip_suffix(')')
}

fn numbers(value: &str) -> Vec<f32> {
    value
        .split(|character: char| character == ',' || character.is_whitespace())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect()
}

/// Named colors common in animated icons; the full CSS list is not needed to interpolate them.
const NAMED: &[(&str, [u8; 3])] = &[
    ("black", [0, 0, 0]),
    ("white", [255, 255, 255]),
    ("red", [255, 0, 0]),
    ("green", [0, 128, 0]),
    ("lime", [0, 255, 0]),
    ("blue", [0, 0, 255]),
    ("yellow", [255, 255, 0]),
    ("cyan", [0, 255, 255]),
    ("aqua", [0, 255, 255]),
    ("magenta", [255, 0, 255]),
    ("fuchsia", [255, 0, 255]),
    ("orange", [255, 165, 0]),
    ("purple", [128, 0, 128]),
    ("gray", [128, 128, 128]),
    ("grey", [128, 128, 128]),
    ("silver", [192, 192, 192]),
    ("navy", [0, 0, 128]),
    ("teal", [0, 128, 128]),
    ("maroon", [128, 0, 0]),
    ("olive", [128, 128, 0]),
    ("pink", [255, 192, 203]),
    ("gold", [255, 215, 0]),
];

/// `#rgb`, `#rrggbb`, `rgb(r, g, b)` (numbers or percentages), or a common color name.
pub(super) fn parse_color(value: &str) -> Option<[u8; 3]> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        let digits: Vec<u8> = hex
            .chars()
            .map(|c| c.to_digit(16).map(|d| d as u8))
            .collect::<Option<_>>()?;
        return match digits.as_slice() {
            [r, g, b] => Some([r * 17, g * 17, b * 17]),
            [r1, r2, g1, g2, b1, b2] => Some([r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2]),
            _ => None,
        };
    }
    if let Some(arguments) = function(value, "rgb") {
        let channels: Vec<u8> = arguments
            .split(',')
            .map(str::trim)
            .map(|part| match part.strip_suffix('%') {
                Some(percent) => percent
                    .parse::<f32>()
                    .ok()
                    .map(|p| (p.clamp(0.0, 100.0) * 2.55).round() as u8),
                None => part.parse::<f32>().ok().map(|c| c.clamp(0.0, 255.0).round() as u8),
            })
            .collect::<Option<_>>()?;
        return (channels.len() == 3).then(|| [channels[0], channels[1], channels[2]]);
    }
    let lower = value.to_ascii_lowercase();
    NAMED.iter().find(|(name, _)| *name == lower).map(|(_, rgb)| *rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves_hit_their_ends_and_bend() {
        for easing in [
            Easing::Linear,
            Easing::from_css("ease").expect("ease"),
            Easing::from_css("ease-in-out").expect("io"),
        ] {
            assert!(easing.apply(0.0).abs() < 1e-3);
            assert!((easing.apply(1.0) - 1.0).abs() < 1e-3);
        }
        let ease_in = Easing::from_css("ease-in").expect("ease-in");
        assert!(ease_in.apply(0.25) < 0.25, "slow start");
        let ease_out = Easing::from_css("cubic-bezier(0, 0, 0.58, 1)").expect("bezier");
        assert!(ease_out.apply(0.25) > 0.25, "fast start");
        assert_eq!(Easing::from_css("steps(4)").expect("steps").apply(0.3), 0.25);
        assert_eq!(
            Easing::from_spline("0.42 0 0.58 1"),
            Some(Easing::Bezier(0.42, 0.0, 0.58, 1.0))
        );
    }

    #[test]
    fn colors_in_every_common_notation() {
        assert_eq!(parse_color("#f80"), Some([255, 136, 0]));
        assert_eq!(parse_color("#FF8800"), Some([255, 136, 0]));
        assert_eq!(parse_color("rgb(255, 136, 0)"), Some([255, 136, 0]));
        assert_eq!(parse_color("rgb(100%, 0%, 50%)"), Some([255, 0, 128]));
        assert_eq!(parse_color("Orange"), Some([255, 165, 0]));
        assert_eq!(parse_color("url(#g)"), None);
    }
}
