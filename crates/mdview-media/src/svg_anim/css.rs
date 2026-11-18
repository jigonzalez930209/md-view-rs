//! A small CSS `@keyframes` and animation shorthand parser.

use std::collections::HashMap;

use super::*;

type Declarations = Vec<(String, String)>;
type Frames = Vec<(f32, Declarations)>;
type Keyframes = HashMap<String, Frames>;

pub(super) fn parse_css(document: &roxmltree::Document<'_>, clips: &mut Vec<Clip>) {
    let mut keyframes: Keyframes = HashMap::new();
    let mut rules: Vec<(String, String)> = Vec::new();
    for node in document.descendants().filter(|node| node.has_tag_name("style")) {
        let Some(text) = node.text() else {
            continue;
        };
        collect_keyframes(text, &mut keyframes);
        collect_rules(text, &mut rules);
    }
    if keyframes.is_empty() {
        return;
    }
    for node in document.descendants().filter(|node| node.is_element()) {
        if node.has_tag_name("style") {
            continue;
        }
        let Some(animation) = element_animation(&node, &rules) else {
            continue;
        };
        let Some(frames) = keyframes.get(&animation.name) else {
            continue;
        };
        for property in PROPERTIES {
            let mut values = Vec::new();
            let mut times = Vec::new();
            for (time, declarations) in frames {
                if let Some((_, value)) = declarations.iter().find(|(key, _)| key == property) {
                    times.push(*time);
                    values.push(if *property == "transform" {
                        svg_transform(value)
                    } else {
                        value.trim_end_matches("px").to_owned()
                    });
                }
            }
            if values.len() < 2 {
                continue;
            }
            clips.push(Clip {
                target: node.range().start,
                property: if *property == "transform" {
                    Property::Transform
                } else {
                    Property::Attribute((*property).to_owned())
                },
                values,
                begin: animation.delay,
                duration: animation.duration,
                repeat: animation.repeat.clone(),
                key_times: times,
                calc: CalcMode::Linear,
                set_like: false,
                easing: vec![animation.easing],
                direction: animation.direction,
                freeze: animation.freeze,
            });
        }
    }
}

/// Properties CSS keyframes may animate.
const PROPERTIES: &[&str] = &[
    "opacity",
    "fill",
    "stroke",
    "transform",
    "fill-opacity",
    "stroke-opacity",
    "stroke-width",
    "stroke-dashoffset",
    "stroke-dasharray",
];

/// One element's CSS animation, from the shorthand and any longhands after it.
#[derive(Clone, Debug, PartialEq)]
struct CssAnimation {
    name: String,
    duration: f32,
    delay: f32,
    repeat: Repeat,
    direction: Direction,
    easing: Easing,
    freeze: bool,
}

impl Default for CssAnimation {
    fn default() -> Self {
        Self {
            name: String::new(),
            duration: 0.0,
            delay: 0.0,
            repeat: Repeat::Once,
            direction: Direction::Normal,
            // CSS's initial timing function is `ease`.
            easing: Easing::Bezier(0.25, 0.1, 0.25, 1.0),
            freeze: false,
        }
    }
}

fn element_animation(node: &roxmltree::Node<'_, '_>, rules: &[(String, String)]) -> Option<CssAnimation> {
    let mut bodies = Vec::new();
    let id = node.attribute("id");
    let classes = node
        .attribute("class")
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>();
    for (selector, body) in rules {
        let matches = if let Some(id_selector) = selector.strip_prefix('#') {
            id == Some(id_selector)
        } else if let Some(class_selector) = selector.strip_prefix('.') {
            classes.contains(&class_selector)
        } else {
            selector == node.tag_name().name()
        };
        if matches {
            bodies.push(body.clone());
        }
    }
    // The inline style wins, so it is applied last.
    if let Some(style) = node.attribute("style") {
        bodies.push(style.to_owned());
    }
    let mut animation = CssAnimation::default();
    for body in bodies {
        for (property, value) in parse_declarations(&body) {
            apply_declaration(&mut animation, &property, &value);
        }
    }
    (!animation.name.is_empty() && animation.name != "none").then(|| CssAnimation {
        duration: animation.duration.max(0.001),
        ..animation
    })
}

fn apply_declaration(animation: &mut CssAnimation, property: &str, value: &str) {
    // Only the first of several comma-separated animations is played.
    let value = first_layer(value);
    match property {
        "animation" => {
            let mut times = Vec::new();
            *animation = CssAnimation::default();
            for token in tokens(value) {
                if let Some(seconds) = css_time(&token) {
                    times.push(seconds);
                } else if !apply_keyword(animation, &token) {
                    animation.name = token;
                }
            }
            if let Some(duration) = times.first() {
                animation.duration = *duration;
            }
            if let Some(delay) = times.get(1) {
                animation.delay = *delay;
            }
        }
        "animation-name" => animation.name = value.trim().to_owned(),
        "animation-duration" => animation.duration = css_time(value.trim()).unwrap_or(animation.duration),
        "animation-delay" => animation.delay = css_time(value.trim()).unwrap_or(0.0),
        "animation-iteration-count" | "animation-direction" | "animation-fill-mode" | "animation-timing-function" => {
            apply_keyword(animation, value.trim());
        }
        _ => {}
    }
}

/// Apply a shorthand keyword; false when the token is not one (it is then the name).
fn apply_keyword(animation: &mut CssAnimation, token: &str) -> bool {
    if let Some(easing) = Easing::from_css(token) {
        animation.easing = easing;
        return true;
    }
    match token {
        "infinite" => animation.repeat = Repeat::Indefinite,
        "normal" => animation.direction = Direction::Normal,
        "reverse" => animation.direction = Direction::Reverse,
        "alternate" => animation.direction = Direction::Alternate,
        "alternate-reverse" => animation.direction = Direction::AlternateReverse,
        "forwards" | "both" => animation.freeze = true,
        "none" | "backwards" | "running" | "paused" => {}
        _ => match token.parse::<f32>() {
            Ok(count) if count > 0.0 => animation.repeat = Repeat::Count(count.ceil() as u32),
            _ => return false,
        },
    }
    true
}

/// Split a shorthand on spaces outside parentheses, so `cubic-bezier(0, 0, 1, 1)` stays whole.
fn tokens(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    for character in value.chars() {
        match character {
            '(' => {
                depth += 1;
                current.push(character);
            }
            ')' => {
                depth -= 1;
                current.push(character);
            }
            c if c.is_whitespace() && depth == 0 => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn first_layer(value: &str) -> &str {
    let mut depth = 0;
    for (index, character) in value.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => return &value[..index],
            _ => {}
        }
    }
    value
}

/// A CSS transform as an SVG `transform` attribute: SVG takes user units and degrees, so the
/// `px` and `deg` units CSS requires are dropped (`translate(0, -24px)` → `translate(0, -24)`).
fn svg_transform(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;
    while !rest.is_empty() {
        let unit = ["px", "deg"].iter().find(|unit| rest.starts_with(**unit));
        let after_number = output
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_digit() || c == '.');
        if let (Some(unit), true) = (unit, after_number) {
            rest = &rest[unit.len()..];
            continue;
        }
        let character = rest.chars().next().unwrap_or(' ');
        output.push(character);
        rest = &rest[character.len_utf8()..];
    }
    output
}

fn css_time(token: &str) -> Option<f32> {
    if let Some(millis) = token.strip_suffix("ms") {
        return millis.parse::<f32>().ok().map(|value| value / 1000.0);
    }
    token.strip_suffix('s')?.parse::<f32>().ok()
}

fn collect_keyframes(text: &str, out: &mut Keyframes) {
    let mut rest = text;
    while let Some(index) = rest.find("@keyframes") {
        let after = &rest[index + "@keyframes".len()..];
        let name = after
            .trim_start()
            .split(|character: char| character.is_whitespace() || character == '{')
            .next()
            .unwrap_or("")
            .to_owned();
        let Some(open) = after.find('{') else {
            break;
        };
        let Some(close) = matching_brace(after, open) else {
            break;
        };
        let body = &after[open + 1..close];
        let mut frames = Vec::new();
        let mut cursor = 0;
        while let Some(frame_open) = body[cursor..].find('{') {
            let frame_open = cursor + frame_open;
            let selector = body[cursor..frame_open].trim().trim_end_matches(',').trim();
            let Some(frame_close) = body[frame_open..].find('}') else {
                break;
            };
            let frame_close = frame_open + frame_close;
            let time = parse_keyframe_time(selector);
            let declarations = parse_declarations(&body[frame_open + 1..frame_close]);
            frames.push((time, declarations));
            cursor = frame_close + 1;
        }
        if !name.is_empty() && !frames.is_empty() {
            out.insert(name, frames);
        }
        rest = &after[close + 1..];
    }
}

fn collect_rules(text: &str, out: &mut Vec<(String, String)>) {
    let stripped = strip_keyframes(text);
    let mut cursor = 0;
    while let Some(open) = stripped[cursor..].find('{') {
        let open = cursor + open;
        let Some(close) = stripped[open..].find('}') else {
            break;
        };
        let close = open + close;
        let selector = stripped[cursor..open].trim().trim_start_matches('}').trim();
        let body = stripped[open + 1..close].trim();
        if !selector.is_empty() && body.contains("animation") {
            out.push((selector.to_owned(), body.to_owned()));
        }
        cursor = close + 1;
    }
}

fn strip_keyframes(text: &str) -> String {
    let mut result = String::new();
    let mut rest = text;
    while let Some(index) = rest.find("@keyframes") {
        result.push_str(&rest[..index]);
        let after = &rest[index..];
        if let Some(open) = after.find('{')
            && let Some(close) = matching_brace(after, open)
        {
            rest = &after[close + 1..];
            continue;
        }
        break;
    }
    result.push_str(rest);
    result
}

fn matching_brace(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (index, character) in text[open..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + index);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_keyframe_time(selector: &str) -> f32 {
    let selector = selector.split(',').next().unwrap_or(selector).trim();
    if selector == "from" {
        return 0.0;
    }
    if selector == "to" {
        return 1.0;
    }
    selector.trim_end_matches('%').parse::<f32>().unwrap_or(0.0) / 100.0
}

fn parse_declarations(body: &str) -> Vec<(String, String)> {
    body.split(';')
        .filter_map(|declaration| {
            let (key, value) = declaration.split_once(':')?;
            let key = key.trim().to_owned();
            let value = value.trim().to_owned();
            if key.is_empty() || value.is_empty() {
                None
            } else {
                Some((key, value))
            }
        })
        .collect()
}
