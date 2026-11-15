//! Tokenizer for the HTML subset: tags with attributes, decoded text, no comments.

pub(super) struct Tag<'a> {
    pub(super) name: String,
    pub(super) closing: bool,
    attributes: Vec<(String, String)>,
    _raw: &'a str,
}

impl Tag<'_> {
    pub(super) fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

pub(super) enum Token<'a> {
    Tag(Tag<'a>),
    Text(String),
}

/// Split HTML into tags and decoded text, dropping comments.
pub(super) fn tokenize(html: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("<!--") {
            rest = after.find("-->").map_or("", |end| &after[end + 3..]);
            continue;
        }
        if rest.starts_with('<')
            && rest[1..]
                .chars()
                .next()
                .is_some_and(|next| next.is_ascii_alphabetic() || next == '/')
            && let Some(end) = tag_end(rest)
        {
            tokens.push(Token::Tag(parse_tag(&rest[..=end])));
            rest = &rest[end + 1..];
            continue;
        }
        let next = rest[1..].find('<').map_or(rest.len(), |index| index + 1);
        tokens.push(Token::Text(decode_entities(&rest[..next])));
        rest = &rest[next..];
    }
    tokens
}

/// Index of the `>` that closes the tag at the start of `text`, honoring quotes.
fn tag_end(text: &str) -> Option<usize> {
    let mut quote = None;
    for (index, character) in text.char_indices().skip(1) {
        match (quote, character) {
            (Some(open), character) if character == open => quote = None,
            (None, '"' | '\'') => quote = Some(character),
            (None, '>') => return Some(index),
            _ => {}
        }
    }
    None
}

fn parse_tag(raw: &str) -> Tag<'_> {
    let inner = raw.trim_start_matches('<').trim_end_matches('>').trim_end_matches('/');
    let closing = inner.starts_with('/');
    let inner = inner.trim_start_matches('/');
    let name_end = inner
        .find(|character: char| character.is_whitespace())
        .unwrap_or(inner.len());
    let name = inner[..name_end].to_ascii_lowercase();
    let mut attributes = Vec::new();
    let mut rest = inner[name_end..].trim_start();
    while !rest.is_empty() {
        let key_end = rest
            .find(|character: char| character == '=' || character.is_whitespace())
            .unwrap_or(rest.len());
        let key = rest[..key_end].to_ascii_lowercase();
        rest = rest[key_end..].trim_start();
        let mut value = String::new();
        if let Some(after) = rest.strip_prefix('=') {
            let after = after.trim_start();
            if let Some(quote) = after.chars().next().filter(|quote| *quote == '"' || *quote == '\'') {
                let body = &after[1..];
                let end = body.find(quote).unwrap_or(body.len());
                value = decode_entities(&body[..end]);
                rest = body.get(end + 1..).unwrap_or("").trim_start();
            } else {
                let end = after.find(char::is_whitespace).unwrap_or(after.len());
                value = decode_entities(&after[..end]);
                rest = after[end..].trim_start();
            }
        }
        if !key.is_empty() {
            attributes.push((key, value));
        }
    }
    Tag {
        name,
        closing,
        attributes,
        _raw: raw,
    }
}

/// Decode the named and numeric character references common in READMEs.
pub(crate) fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let candidate = &rest[start..];
        let Some(end) = candidate.find(';').filter(|end| *end <= 10) else {
            out.push('&');
            rest = &candidate[1..];
            continue;
        };
        let entity = &candidate[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            "copy" => Some('©'),
            "reg" => Some('®'),
            "trade" => Some('™'),
            "mdash" => Some('—'),
            "ndash" => Some('–'),
            "hellip" => Some('…'),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|decimal| decimal.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(character) => {
                out.push(character);
                rest = &candidate[end + 1..];
            }
            None => {
                out.push('&');
                rest = &candidate[1..];
            }
        }
    }
    out.push_str(rest);
    out
}
