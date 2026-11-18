//! Map a position in TeX source to the formula part it produces.
//!
//! The formula editor highlights, in the rendered preview, the part of the formula under
//! the pointer in the source. RaTeX keeps byte locations on leaf nodes and braced groups
//! but not on composite nodes (`\frac`, `\sqrt`, scripts, `\left…\right`, environments),
//! so this module derives a span for every composite from its descendants and the command
//! around them, then picks the innermost span that can be colored without moving anything.
//!
//! Coloring must never change the formula's shape. Wrapping source in braces can: an
//! optional argument `[9]` stops being one, a big operator loses its limits above and below,
//! and a binary `+` loses its spacing. So each span is tried in several wrap forms, and a
//! form is accepted only when the colored formula lays out to exactly the same geometry as
//! the original.

use std::ops::Range;

use ratex_layout::{LayoutOptions, layout, to_display_list};
use ratex_parser::parser::Parser;
use serde_json::{Map, Value};

/// A source span and the TeX that renders it highlighted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathHighlight {
    /// Byte range of the highlighted part in the original source.
    pub span: Range<usize>,
    /// The source with that part wrapped in `\textcolor`.
    pub tex: String,
}

/// Byte spans of the formula parts containing `offset`, innermost first.
///
/// `offset` is a byte offset into `tex`. An empty list means the source does not parse or
/// the offset is outside every part (for example on whitespace between two terms).
pub fn math_spans_at(tex: &str, offset: usize) -> Vec<Range<usize>> {
    let Ok(nodes) = Parser::new(tex).parse() else {
        return Vec::new();
    };
    let Ok(Value::Array(nodes)) = serde_json::to_value(&nodes) else {
        return Vec::new();
    };
    let mut spans = Vec::new();
    collect(tex, &nodes, 0, &mut spans);
    spans.extend(command_at(tex, offset));
    let mut containing: Vec<Range<usize>> = spans
        .into_iter()
        .filter(|span| span.start <= offset && offset < span.end)
        .collect();
    containing.sort_by_key(|span| (span.len(), std::cmp::Reverse(span.start)));
    containing.dedup();
    containing
}

/// Color the innermost part at `offset` with `\textcolor{color}` and return the new source.
///
/// `color` is a CSS hex color such as `#0969da`. A part that cannot be colored without
/// changing the layout (a lone `\frac` command, an operator whose limits would move) is
/// skipped in favour of its parent.
pub fn highlight_math_at(tex: &str, offset: usize, color: &str) -> Option<MathHighlight> {
    let reference = geometry(tex)?;
    math_spans_at(tex, offset).into_iter().find_map(|span| {
        wrap_forms(tex, &span, color)
            .into_iter()
            .find(|wrapped| geometry(wrapped).is_some_and(|shape| same_shape(&shape, &reference)))
            .map(|tex| MathHighlight { span, tex })
    })
}

/// Ways to color `span`, the least intrusive first: inside its own braces or brackets, a
/// bare or braced `\textcolor`, then wrapped in the atom class it had.
fn wrap_forms(tex: &str, span: &Range<usize>, color: &str) -> Vec<String> {
    let (Some(before), Some(inside), Some(after)) = (tex.get(..span.start), tex.get(span.clone()), tex.get(span.end..))
    else {
        return Vec::new();
    };
    let mut forms = Vec::new();
    for (open, close) in [("{", "}"), ("[", "]")] {
        if let Some(inner) = inside.strip_prefix(open).and_then(|rest| rest.strip_suffix(close)) {
            forms.push(format!("{before}{open}\\textcolor{{{color}}}{{{inner}}}{close}{after}"));
        }
    }
    forms.push(format!("{before}\\textcolor{{{color}}}{{{inside}}}{after}"));
    forms.push(format!("{before}{{\\textcolor{{{color}}}{{{inside}}}}}{after}"));
    // A colored atom becomes ordinary; restore its class so spacing and limits stay put (a
    // fraction is an inner atom, with a thin space on each side).
    for class in ["mathinner", "mathbin", "mathrel", "mathopen", "mathclose", "mathpunct"] {
        forms.push(format!(
            "{before}\\{class}{{\\textcolor{{{color}}}{{{inside}}}}}{after}"
        ));
    }
    for limits in ["\\limits", "\\nolimits"] {
        forms.push(format!(
            "{before}\\mathop{{\\textcolor{{{color}}}{{{inside}}}}}{limits}{after}"
        ));
    }
    forms
}

/// The laid-out formula with colors removed, for comparing shapes.
fn geometry(tex: &str) -> Option<Value> {
    let nodes = Parser::new(tex).parse().ok()?;
    let display = to_display_list(&layout(&nodes, &LayoutOptions::default()));
    let mut value = serde_json::to_value(&display).ok()?;
    strip_colors(&mut value);
    Some(value)
}

fn strip_colors(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("color");
            map.values_mut().for_each(strip_colors);
        }
        Value::Array(items) => items.iter_mut().for_each(strip_colors),
        _ => {}
    }
}

/// Whether two layouts match, allowing for floating-point noise.
fn same_shape(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(a), Some(b)) => (a - b).abs() < 1e-6,
            _ => a == b,
        },
        (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_shape(a, b)),
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| same_shape(value, other)))
        }
        _ => a == b,
    }
}

/// Push the spans of `nodes` (a sibling sequence) and their descendants, and return the
/// sequence's overall extent.
///
/// `low` is the earliest byte a node may claim: the end of the previous sibling, so a
/// composite never swallows the term before it.
fn collect(tex: &str, nodes: &[Value], mut low: usize, spans: &mut Vec<Range<usize>>) -> Option<Range<usize>> {
    let mut extent: Option<Range<usize>> = None;
    for node in nodes {
        let Value::Object(map) = node else {
            continue;
        };
        if let Some(span) = node_span(tex, map, low, spans) {
            low = low.max(span.end);
            extent = Some(union(extent, &span));
            spans.push(span);
        }
    }
    extent
}

fn union(range: Option<Range<usize>>, other: &Range<usize>) -> Range<usize> {
    match range {
        Some(range) => range.start.min(other.start)..range.end.max(other.end),
        None => other.clone(),
    }
}

/// Child nodes grouped into sibling sequences: a list field (`body`) is one sequence, a
/// single field (`numer`, `base`, `sup`) is its own, and array cells are one each.
fn child_groups(map: &Map<String, Value>) -> Vec<Vec<Value>> {
    let mut groups = Vec::new();
    for (key, value) in map {
        if key == "loc" {
            continue;
        }
        match value {
            Value::Object(_) if is_node(value) => groups.push(vec![value.clone()]),
            Value::Array(items) => {
                let nodes: Vec<Value> = items.iter().filter(|item| is_node(item)).cloned().collect();
                if !nodes.is_empty() {
                    groups.push(nodes);
                }
                for item in items {
                    if let Value::Array(cells) = item {
                        groups.extend(cells.iter().filter(|cell| is_node(cell)).map(|cell| vec![cell.clone()]));
                    }
                }
            }
            _ => {}
        }
    }
    groups
}

fn is_node(value: &Value) -> bool {
    matches!(value, Value::Object(map) if map.contains_key("type"))
}

fn own_loc(map: &Map<String, Value>) -> Option<Range<usize>> {
    let loc = map.get("loc")?;
    let start = usize::try_from(loc.get("start")?.as_u64()?).ok()?;
    let end = usize::try_from(loc.get("end")?.as_u64()?).ok()?;
    (start < end).then_some(start..end)
}

/// The source span of one node.
///
/// Children are measured first, so a composite starts before the command of its first
/// child (`\left[ \frac…` opens at `\left`, not at `\frac`). A located node uses its
/// location. A composite extends its children's extent back over the command that opens
/// it and forward over `\right…` or `\end{…}`. Transparent wrappers (`styling`, an
/// unlocated `ordgroup`) keep their children's extent.
fn node_span(tex: &str, map: &Map<String, Value>, low: usize, spans: &mut Vec<Range<usize>>) -> Option<Range<usize>> {
    let own = own_loc(map);
    let child_low = own.as_ref().map_or(low, |own| own.start);
    let mut inner: Option<Range<usize>> = None;
    for group in child_groups(map) {
        if let Some(extent) = collect(tex, &group, child_low, spans) {
            inner = Some(union(inner, &extent));
        }
    }
    if own.is_some() {
        return own;
    }
    let inner = inner?;
    let transparent = matches!(
        map.get("type").and_then(Value::as_str),
        Some("styling" | "ordgroup" | "sizing")
    );
    if transparent {
        return Some(inner);
    }
    let start = open_composite(tex, inner.start, low);
    let end = close_composite(tex, inner.end);
    (start < end).then_some(start..end)
}

/// Move `start` back over the source that opens a composite: scripts markers, optional
/// arguments, `\left` with its delimiter, `\begin{name}`, and one command name.
fn open_composite(tex: &str, mut start: usize, low: usize) -> usize {
    loop {
        let before = tex.get(low..start).unwrap_or("");
        let trimmed = before.trim_end();
        if let Some(rest) = trimmed.strip_suffix(['_', '^']) {
            start = low + rest.len();
        } else if trimmed.ends_with(']')
            && let Some(open) = trimmed.rfind('[')
        {
            start = low + open;
        } else if let Some(begin) = begin_environment(trimmed) {
            return low + begin;
        } else if let Some(left) = left_delimiter(trimmed) {
            return low + left;
        } else {
            return command_before(trimmed).map_or(start, |command| low + command);
        }
    }
}

/// Byte index of `\begin{name}` when `text` ends with it.
fn begin_environment(text: &str) -> Option<usize> {
    let body = text.strip_suffix('}')?;
    let open = body.rfind("\\begin{")?;
    let name = &body[open + "\\begin{".len()..];
    name.chars()
        .all(|c| c.is_ascii_alphabetic() || c == '*')
        .then_some(open)
}

/// Byte index of `\left` when `text` ends with `\left` and a delimiter.
fn left_delimiter(text: &str) -> Option<usize> {
    let without = match command_before(text) {
        Some(command) => &text[..command],
        None => {
            let last = text.chars().next_back()?;
            let without = &text[..text.len() - last.len_utf8()];
            without.strip_suffix('\\').unwrap_or(without)
        }
    };
    without.trim_end().strip_suffix("\\left").map(str::len)
}

/// Byte index of the backslash when `text` ends with a command name such as `\frac`.
fn command_before(text: &str) -> Option<usize> {
    let name = text.len() - text.trim_end_matches(|c: char| c.is_ascii_alphabetic()).len();
    let backslash = text.len().checked_sub(name + 1)?;
    (name > 0 && text[backslash..].starts_with('\\')).then_some(backslash)
}

/// Extend `end` over `\right` with its delimiter and over `\end{name}`.
fn close_composite(tex: &str, mut end: usize) -> usize {
    loop {
        let rest = tex.get(end..).unwrap_or("");
        let trimmed = rest.trim_start();
        let skipped = rest.len() - trimmed.len();
        if let Some(after) = trimmed.strip_prefix("\\right") {
            end += skipped + "\\right".len() + delimiter_len(after);
        } else if let Some(after) = trimmed.strip_prefix("\\end{")
            && let Some(close) = after.find('}')
        {
            end += skipped + "\\end{".len() + close + 1;
        } else {
            return end;
        }
    }
}

fn delimiter_len(text: &str) -> usize {
    let trimmed = text.trim_start();
    let leading = text.len() - trimmed.len();
    let mut chars = trimmed.chars();
    match chars.next() {
        None => leading,
        Some('\\') => {
            let name: usize = chars
                .clone()
                .take_while(char::is_ascii_alphabetic)
                .map(char::len_utf8)
                .sum();
            // `\|`, `\{` and `\}` are one-character control symbols.
            let name = if name == 0 {
                chars.next().map_or(0, char::len_utf8)
            } else {
                name
            };
            leading + 1 + name
        }
        Some(other) => leading + other.len_utf8(),
    }
}

/// The span of a `\command` token under `offset`, for operators RaTeX does not locate.
fn command_at(tex: &str, offset: usize) -> Option<Range<usize>> {
    let probe = tex
        .get(offset..)?
        .chars()
        .next()
        .map_or(offset, |next| offset + next.len_utf8());
    let start = tex.get(..probe)?.rfind('\\')?;
    let name: usize = tex[start + 1..]
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .map(char::len_utf8)
        .sum();
    let end = start + 1 + name;
    (name > 0 && offset < end).then_some(start..end)
}

#[cfg(test)]
mod tests;
