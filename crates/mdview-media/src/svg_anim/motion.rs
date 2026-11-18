//! `animateMotion`: moving an element along a path.

use svgtypes::{SimplePathSegment, SimplifyingPathParser};

/// Segments used to flatten each curve.
const CURVE_STEPS: usize = 24;

/// How the moving element is turned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Rotate {
    /// A fixed angle in degrees.
    Angle(f32),
    /// Along the path direction.
    Auto,
    /// Along the path direction, reversed.
    AutoReverse,
}

impl Rotate {
    pub(super) fn parse(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some("auto") => Self::Auto,
            Some("auto-reverse") => Self::AutoReverse,
            Some(angle) => Self::Angle(angle.parse().unwrap_or(0.0)),
            None => Self::Angle(0.0),
        }
    }
}

/// A path flattened into points with cumulative lengths.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct MotionPath {
    points: Vec<(f32, f32)>,
    lengths: Vec<f32>,
}

impl MotionPath {
    /// Flatten SVG path data. Returns `None` for empty or zero-length paths.
    pub(super) fn from_path_data(data: &str) -> Option<Self> {
        let mut points: Vec<(f32, f32)> = Vec::new();
        let mut current = (0.0_f32, 0.0_f32);
        let mut start = current;
        for segment in SimplifyingPathParser::from(data) {
            let segment = segment.ok()?;
            match segment {
                SimplePathSegment::MoveTo { x, y } => {
                    current = (x as f32, y as f32);
                    start = current;
                    if points.is_empty() {
                        points.push(current);
                    }
                }
                SimplePathSegment::LineTo { x, y } => {
                    current = (x as f32, y as f32);
                    points.push(current);
                }
                SimplePathSegment::Quadratic { x1, y1, x, y } => {
                    let (c, end) = ((x1 as f32, y1 as f32), (x as f32, y as f32));
                    for step in 1..=CURVE_STEPS {
                        let t = step as f32 / CURVE_STEPS as f32;
                        let u = 1.0 - t;
                        points.push((
                            u * u * current.0 + 2.0 * u * t * c.0 + t * t * end.0,
                            u * u * current.1 + 2.0 * u * t * c.1 + t * t * end.1,
                        ));
                    }
                    current = end;
                }
                SimplePathSegment::CurveTo { x1, y1, x2, y2, x, y } => {
                    let (c1, c2, end) = ((x1 as f32, y1 as f32), (x2 as f32, y2 as f32), (x as f32, y as f32));
                    for step in 1..=CURVE_STEPS {
                        let t = step as f32 / CURVE_STEPS as f32;
                        let u = 1.0 - t;
                        let point = |a: f32, b: f32, c: f32, d: f32| {
                            u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d
                        };
                        points.push((point(current.0, c1.0, c2.0, end.0), point(current.1, c1.1, c2.1, end.1)));
                    }
                    current = end;
                }
                SimplePathSegment::ClosePath => {
                    current = start;
                    points.push(current);
                }
            }
        }
        Self::from_points(points)
    }

    /// A polyline through `x,y` pairs (SMIL `values`, `from`/`to`).
    pub(super) fn from_values(values: &[String]) -> Option<Self> {
        let points = values
            .iter()
            .map(|value| {
                let mut numbers = value
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .filter(|part| !part.is_empty())
                    .map(str::parse::<f32>);
                Some((numbers.next()?.ok()?, numbers.next()?.ok()?))
            })
            .collect::<Option<Vec<_>>>()?;
        Self::from_points(points)
    }

    fn from_points(points: Vec<(f32, f32)>) -> Option<Self> {
        if points.len() < 2 {
            return None;
        }
        let mut lengths = Vec::with_capacity(points.len());
        let mut total = 0.0;
        lengths.push(0.0);
        for pair in points.windows(2) {
            total += ((pair[1].0 - pair[0].0).powi(2) + (pair[1].1 - pair[0].1).powi(2)).sqrt();
            lengths.push(total);
        }
        (total > 0.0).then_some(Self { points, lengths })
    }

    /// The point and direction (degrees) at `fraction` of the path's length.
    pub(super) fn at(&self, fraction: f32) -> (f32, f32, f32) {
        let total = self.lengths[self.lengths.len() - 1];
        let distance = fraction.clamp(0.0, 1.0) * total;
        let index = self
            .lengths
            .windows(2)
            .position(|pair| distance <= pair[1])
            .unwrap_or(self.lengths.len() - 2);
        let (from, to) = (self.points[index], self.points[index + 1]);
        let span = (self.lengths[index + 1] - self.lengths[index]).max(f32::EPSILON);
        let t = ((distance - self.lengths[index]) / span).clamp(0.0, 1.0);
        let angle = (to.1 - from.1).atan2(to.0 - from.0).to_degrees();
        (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t, angle)
    }

    /// The supplemental transform at `fraction`: a translation, then the rotation.
    pub(super) fn transform(&self, fraction: f32, rotate: Rotate) -> String {
        let (x, y, direction) = self.at(fraction);
        let angle = match rotate {
            Rotate::Angle(angle) => angle,
            Rotate::Auto => direction,
            Rotate::AutoReverse => direction + 180.0,
        };
        if angle.abs() < 1e-4 {
            format!("translate({x:.3} {y:.3})")
        } else {
            format!("translate({x:.3} {y:.3}) rotate({angle:.3})")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_sampled_by_length() {
        let path = MotionPath::from_path_data("M0 0 L10 0 L10 30").expect("path");
        assert_eq!(path.at(0.0), (0.0, 0.0, 0.0));
        let (x, y, angle) = path.at(0.25);
        assert!(
            (x - 10.0).abs() < 1e-3 && y.abs() < 1e-3 && angle.abs() < 1e-3,
            "{x} {y} {angle}"
        );
        let (x, y, angle) = path.at(1.0);
        assert!((x - 10.0).abs() < 1e-3 && (y - 30.0).abs() < 1e-3 && (angle - 90.0).abs() < 1e-3);
    }

    #[test]
    fn curves_and_relative_commands_flatten() {
        let path = MotionPath::from_path_data("m0 0 c0 10 10 10 10 0 z").expect("curve");
        let (x, y, _) = path.at(1.0);
        assert!(x.abs() < 1e-3 && y.abs() < 1e-3, "closes back at the start");
        assert!(
            MotionPath::from_path_data("M5 5").is_none(),
            "a lone point does not move"
        );
    }

    #[test]
    fn transforms_include_rotation_when_asked() {
        let path = MotionPath::from_values(&["0,0".into(), "0,10".into()]).expect("values");
        assert_eq!(path.transform(0.5, Rotate::Angle(0.0)), "translate(0.000 5.000)");
        assert_eq!(
            path.transform(0.5, Rotate::Auto),
            "translate(0.000 5.000) rotate(90.000)"
        );
        assert_eq!(Rotate::parse(Some("auto-reverse")), Rotate::AutoReverse);
    }
}
