//! Attribute rewriting on SVG start tags.

pub(super) fn start_tag_end(source: &str, start: usize) -> Option<usize> {
    if start >= source.len() {
        return None;
    }
    let mut quote = None;
    for (offset, character) in source[start..].char_indices() {
        match character {
            '\'' | '"' => {
                if quote == Some(character) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(character);
                }
            }
            '>' if quote.is_none() => return Some(start + offset),
            _ => {}
        }
    }
    None
}

pub(super) fn set_attribute(tag: &str, name: &str, value: &str) -> String {
    let mut tag = remove_attribute(tag, name);
    let trimmed = tag.trim_end();
    let closing = trimmed.len();
    let insert = if trimmed.ends_with("/>") {
        closing - 2
    } else if trimmed.ends_with('>') {
        closing - 1
    } else {
        return tag;
    };
    let addition = format!(" {name}=\"{}\"", escape(value));
    tag.insert_str(insert, &addition);
    tag
}

fn remove_attribute(tag: &str, name: &str) -> String {
    let mut result = tag.to_owned();
    let mut search = 0;
    while let Some(found) = result[search..].find(name) {
        let index = search + found;
        let previous = result[..index].chars().next_back();
        let after = index + name.len();
        let next = result[after..].chars().next();
        let boundary = previous.is_none_or(char::is_whitespace)
            && next.is_some_and(|character| character == '=' || character.is_whitespace());
        if !boundary {
            search = index + name.len();
            continue;
        }
        let Some(equal) = result[after..].find('=') else {
            search = index + name.len();
            continue;
        };
        let value_start = after + equal + 1;
        let bytes = result.as_bytes();
        let mut value_end = value_start;
        while value_end < bytes.len() && bytes[value_end].is_ascii_whitespace() {
            value_end += 1;
        }
        if bytes.get(value_end) == Some(&b'"') || bytes.get(value_end) == Some(&b'\'') {
            let quote = bytes[value_end];
            value_end += 1;
            while value_end < bytes.len() && bytes[value_end] != quote {
                value_end += 1;
            }
            value_end = (value_end + 1).min(bytes.len());
        } else {
            while value_end < bytes.len()
                && !bytes[value_end].is_ascii_whitespace()
                && bytes[value_end] != b'>'
                && bytes[value_end] != b'/'
            {
                value_end += 1;
            }
        }
        let mut start = index;
        while start > 0 && result.as_bytes()[start - 1].is_ascii_whitespace() {
            start -= 1;
        }
        result.replace_range(start..value_end, "");
        search = start;
    }
    result
}

fn escape(value: &str) -> String {
    value.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;")
}

/// The value of attribute `name` on a start tag, matching the whole name (so `transform` is
/// not found inside `gradientTransform`).
pub(super) fn get_attribute(tag: &str, name: &str) -> Option<String> {
    let mut search = 0;
    while let Some(found) = tag[search..].find(name) {
        let start = search + found;
        let before = tag[..start].chars().next_back();
        let after = &tag[start + name.len()..];
        let after = after.trim_start();
        if before.is_some_and(char::is_whitespace)
            && let Some(rest) = after.strip_prefix('=')
        {
            let rest = rest.trim_start();
            let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
            let value = &rest[1..];
            return value.find(quote).map(|end| value[..end].to_owned());
        }
        search = start + name.len();
    }
    None
}
