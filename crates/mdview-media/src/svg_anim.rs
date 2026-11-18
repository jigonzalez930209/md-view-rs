//! A bounded SMIL and CSS animation subset for SVG.
//!
//! The runtime parses `<animate>`, `<animateTransform>`, `<set>`, and a small `@keyframes` CSS
//! subset with `roxmltree`, then rewrites the animated attributes for a requested time before the
//! SVG is rasterized. It does not execute scripts, events, transitions, or external resources.
//!
//! SMIL `<animate>` drives any attribute with numbers, lists of numbers, path data of the same
//! shape, or colors; `<animateTransform>` and `<animateMotion>` move elements; CSS `@keyframes`
//! drive opacity, colors, stroke dashes and widths, and transforms. Timing covers `begin`, `dur`,
//! `repeatCount`, `fill="freeze"`, `keyTimes`, `keySplines`, CSS delays, iteration counts,
//! directions, and timing functions.

use std::collections::BTreeMap;

mod css;
mod easing;
mod eval;
mod motion;
mod tag;

use css::parse_css;
use easing::Easing;
use motion::{MotionPath, Rotate};
use tag::{get_attribute, set_attribute, start_tag_end};

/// A parsed animation timeline.
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    /// Total timeline duration in seconds.
    pub duration: f32,
    /// Whether the document uses animation features outside the supported subset.
    pub unsupported: bool,
    clips: Vec<Clip>,
}

impl Animation {
    /// Whether the timeline has at least one usable clip.
    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }

    /// The number of clips, for diagnostics and tests.
    pub fn clip_count(&self) -> usize {
        self.clips.len()
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Property {
    Attribute(String),
    Transform,
    /// `animateMotion`: a supplemental translation (and rotation) along a path.
    Motion(MotionPath, Rotate),
}

/// Which way successive iterations run (CSS `animation-direction`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Direction {
    #[default]
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

#[derive(Clone, Debug, PartialEq)]
enum Repeat {
    Once,
    Count(u32),
    Indefinite,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum CalcMode {
    Linear,
    Discrete,
}

#[derive(Clone, Debug, PartialEq)]
struct Clip {
    target: usize,
    property: Property,
    values: Vec<String>,
    begin: f32,
    duration: f32,
    repeat: Repeat,
    key_times: Vec<f32>,
    calc: CalcMode,
    set_like: bool,
    /// Easing per segment (SMIL `keySplines`) or one curve for every segment (CSS).
    easing: Vec<Easing>,
    direction: Direction,
    /// Keep the final value after the active duration (`fill="freeze"`, CSS `forwards`).
    freeze: bool,
}

/// Parse animations from an SVG document. Returns `Ok(None)` when the SVG is static.
pub fn parse(svg: &str) -> Result<Option<Animation>, String> {
    let document = roxmltree::Document::parse(svg).map_err(|error| error.to_string())?;
    let mut clips = Vec::new();
    for node in document.descendants() {
        if !node.is_element() {
            continue;
        }
        match node.tag_name().name() {
            "animate" => parse_animate(&node, &mut clips),
            "animateTransform" => parse_animate_transform(&node, &mut clips),
            "set" => parse_set(&node, &mut clips),
            "animateMotion" => parse_animate_motion(&document, &node, &mut clips),
            _ => {}
        }
    }
    parse_css(&document, &mut clips);
    let unsupported = document.descendants().any(|node| {
        node.is_element()
            && (matches!(node.tag_name().name(), "animateColor" | "discard")
                || node.attribute("keyPoints").is_some()
                || node.attribute("additive") == Some("sum")
                || node.attribute("accumulate") == Some("sum"))
    });
    if clips.is_empty() {
        return Ok(None);
    }
    let duration = clips
        .iter()
        .map(|clip| {
            if clip.duration.is_finite() {
                clip.begin + clip.duration
            } else {
                clip.begin + 1.0
            }
        })
        .fold(1.0_f32, f32::max);
    Ok(Some(Animation {
        duration,
        unsupported,
        clips,
    }))
}

fn parse_animate(node: &roxmltree::Node<'_, '_>, clips: &mut Vec<Clip>) {
    let Some(target) = node.parent() else {
        return;
    };
    let Some(attribute) = node.attribute("attributeName") else {
        return;
    };
    let values = values_of(node);
    if values.is_empty() {
        return;
    }
    push_clip(node, target, Property::Attribute(attribute.to_owned()), values, clips);
}

fn parse_animate_transform(node: &roxmltree::Node<'_, '_>, clips: &mut Vec<Clip>) {
    let Some(target) = node.parent() else {
        return;
    };
    let kind = node.attribute("type").unwrap_or("translate");
    let raw = values_of(node);
    if raw.is_empty() {
        return;
    }
    let values = raw
        .into_iter()
        .map(|value| {
            if value.contains('(') {
                value
            } else {
                format!("{kind}({value})")
            }
        })
        .collect();
    push_clip(node, target, Property::Transform, values, clips);
}

fn parse_animate_motion(document: &roxmltree::Document<'_>, node: &roxmltree::Node<'_, '_>, clips: &mut Vec<Clip>) {
    let Some(target) = node.parent() else {
        return;
    };
    // An `<mpath href="#id">` child wins over the `path` attribute, which wins over `values`.
    let referenced = node
        .children()
        .find(|child| child.has_tag_name("mpath"))
        .and_then(|mpath| {
            mpath
                .attribute("href")
                .or_else(|| mpath.attribute(("http://www.w3.org/1999/xlink", "href")))
        })
        .and_then(|href| href.strip_prefix('#'))
        .and_then(|id| {
            document
                .descendants()
                .find(|element| element.attribute("id") == Some(id))
        })
        .and_then(|element| element.attribute("d"));
    let path = referenced
        .or_else(|| node.attribute("path"))
        .and_then(MotionPath::from_path_data)
        .or_else(|| MotionPath::from_values(&values_of(node)));
    let Some(path) = path else {
        return;
    };
    let rotate = Rotate::parse(node.attribute("rotate"));
    push_clip(
        node,
        target,
        Property::Motion(path, rotate),
        vec![String::new(), String::new()],
        clips,
    );
}

fn parse_set(node: &roxmltree::Node<'_, '_>, clips: &mut Vec<Clip>) {
    let Some(target) = node.parent() else {
        return;
    };
    let Some(attribute) = node.attribute("attributeName") else {
        return;
    };
    let Some(value) = node.attribute("to") else {
        return;
    };
    let begin = parse_time(node.attribute("begin").unwrap_or("0s"));
    let duration = parse_time(node.attribute("dur").unwrap_or("0s"));
    clips.push(Clip {
        target: target.range().start,
        property: Property::Attribute(attribute.to_owned()),
        values: vec![value.to_owned()],
        begin,
        duration,
        repeat: Repeat::Indefinite,
        key_times: Vec::new(),
        calc: CalcMode::Discrete,
        set_like: true,
        easing: Vec::new(),
        direction: Direction::Normal,
        freeze: true,
    });
}

fn push_clip(
    node: &roxmltree::Node<'_, '_>,
    target: roxmltree::Node<'_, '_>,
    property: Property,
    values: Vec<String>,
    clips: &mut Vec<Clip>,
) {
    let duration = parse_time(node.attribute("dur").unwrap_or("1s")).max(0.001);
    let begin = parse_time(node.attribute("begin").unwrap_or("0s"));
    let repeat = match node.attribute("repeatCount").unwrap_or("1") {
        "indefinite" => Repeat::Indefinite,
        value => Repeat::Count(value.parse::<u32>().unwrap_or(1).max(1)),
    };
    let key_times = node
        .attribute("keyTimes")
        .map(|times| {
            times
                .split(';')
                .filter_map(|time| time.trim().parse::<f32>().ok())
                .map(|time| time.clamp(0.0, 1.0))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let calc = match node.attribute("calcMode").unwrap_or("linear") {
        "discrete" => CalcMode::Discrete,
        _ => CalcMode::Linear,
    };
    let easing = if node.attribute("calcMode") == Some("spline") {
        node.attribute("keySplines")
            .map(|splines| splines.split(';').filter_map(Easing::from_spline).collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    clips.push(Clip {
        target: target.range().start,
        property,
        values,
        begin,
        duration,
        repeat,
        key_times,
        calc,
        set_like: false,
        easing,
        direction: Direction::Normal,
        freeze: node.attribute("fill") == Some("freeze"),
    });
}

fn values_of(node: &roxmltree::Node<'_, '_>) -> Vec<String> {
    if let Some(values) = node.attribute("values") {
        return values
            .split(';')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect();
    }
    match (node.attribute("from"), node.attribute("to")) {
        (Some(from), Some(to)) => vec![from.to_owned(), to.to_owned()],
        (None, Some(to)) => vec![to.to_owned(), to.to_owned()],
        (Some(from), None) => vec![from.to_owned(), from.to_owned()],
        (None, None) => Vec::new(),
    }
}

fn parse_time(value: &str) -> f32 {
    let value = value.split(';').next().unwrap_or(value).trim().trim_start_matches('+');
    if let Some(millis) = value.strip_suffix("ms") {
        return millis.trim().parse::<f32>().unwrap_or(0.0) / 1000.0;
    }
    if let Some(seconds) = value.strip_suffix('s') {
        return seconds.trim().parse::<f32>().unwrap_or(0.0);
    }
    value.parse::<f32>().unwrap_or(0.0)
}

/// Produce the SVG for one animation time. Returns the source unchanged when no clip applies.
pub fn frame_svg(svg: &str, animation: &Animation, time: f32) -> String {
    let mut patches: BTreeMap<usize, BTreeMap<String, String>> = BTreeMap::new();
    let mut motions: BTreeMap<usize, String> = BTreeMap::new();
    for clip in &animation.clips {
        let Some(value) = clip.value_at(time) else {
            continue;
        };
        let attribute = match &clip.property {
            Property::Attribute(name) => name.clone(),
            Property::Transform => "transform".to_owned(),
            Property::Motion(..) => {
                motions.insert(clip.target, value);
                continue;
            }
        };
        let entry = patches.entry(clip.target).or_default();
        if attribute == "transform" {
            entry
                .entry(attribute)
                .and_modify(|current| {
                    current.push(' ');
                    current.push_str(&value);
                })
                .or_insert(value);
        } else {
            entry.insert(attribute, value);
        }
    }
    // Motion is supplemental: it goes in front of the element's own (or animated) transform.
    for (target, motion) in motions {
        let base = patches
            .get(&target)
            .and_then(|attributes| attributes.get("transform").cloned())
            .or_else(|| start_tag_end(svg, target).and_then(|end| get_attribute(&svg[target..=end], "transform")))
            .unwrap_or_default();
        let combined = format!("{motion} {base}").trim().to_owned();
        patches
            .entry(target)
            .or_default()
            .insert("transform".to_owned(), combined);
    }
    let mut output = svg.to_owned();
    for (target, attributes) in patches.iter().rev() {
        let target = *target;
        let Some(end) = start_tag_end(&output, target) else {
            continue;
        };
        let mut tag = output[target..=end].to_owned();
        for (name, value) in attributes {
            tag = set_attribute(&tag, name, value);
        }
        output.replace_range(target..=end, &tag);
    }
    output
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod extended_tests;
