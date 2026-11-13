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
        let animation = element_animation(&node, &rules);
        let Some((name, duration, repeat)) = animation else {
            continue;
        };
        let Some(frames) = keyframes.get(&name) else {
            continue;
        };
        for property in ["opacity", "fill", "stroke", "transform"] {
            let mut values = Vec::new();
            let mut times = Vec::new();
            for (time, declarations) in frames {
                if let Some((_, value)) = declarations.iter().find(|(key, _)| key == property) {
                    times.push(*time);
                    values.push(value.clone());
                }
            }
            if values.len() < 2 {
                continue;
            }
            clips.push(Clip {
                target: node.range().start,
                property: if property == "transform" {
                    Property::Transform
                } else {
                    Property::Attribute(property.to_owned())
                },
                values,
                begin: 0.0,
                duration,
                repeat: repeat.clone(),
                key_times: times,
                calc: CalcMode::Linear,
                set_like: false,
            });
        }
    }
}

fn element_animation(node: &roxmltree::Node<'_, '_>, rules: &[(String, String)]) -> Option<(String, f32, Repeat)> {
    let mut declarations = Vec::new();
    if let Some(style) = node.attribute("style") {
        declarations.push(style.to_owned());
    }
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
            declarations.push(body.clone());
        }
    }
    for declaration in declarations {
        if let Some(animation) = animation_shorthand(&declaration) {
            return Some(animation);
        }
    }
    None
}

fn animation_shorthand(declaration: &str) -> Option<(String, f32, Repeat)> {
    let index = declaration
        .find("animation:")
        .or_else(|| declaration.find("animation-name:"))?;
    let tail = &declaration[index..];
    let value = tail.split(';').next()?.split(':').nth(1)?.trim();
    if value.is_empty() || value == "none" {
        return None;
    }
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    let name = tokens.first()?.to_string();
    let mut duration = 2.0;
    let mut repeat = Repeat::Once;
    for token in &tokens[1..] {
        if let Some(seconds) = token.strip_suffix("ms") {
            duration = seconds.parse::<f32>().unwrap_or(2.0) / 1000.0;
        } else if let Some(seconds) = token.strip_suffix('s') {
            duration = seconds.parse::<f32>().unwrap_or(2.0);
        } else if *token == "infinite" {
            repeat = Repeat::Indefinite;
        }
    }
    Some((name, duration.max(0.001), repeat))
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
