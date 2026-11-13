//! Link destination classification.

use crate::ast::LinkKind;

/// Classify a link or image destination without touching the filesystem.
pub fn classify(destination: &str) -> LinkKind {
    let destination = destination.trim();
    if destination.is_empty() {
        return LinkKind::Invalid;
    }
    if destination.starts_with('#') {
        return LinkKind::Anchor;
    }
    let lower = destination.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:") {
        return LinkKind::External;
    }
    if lower.starts_with("file:") {
        return LinkKind::FileUrl;
    }
    if lower.starts_with("javascript:") || lower.starts_with("vbscript:") || lower.starts_with("data:text/html") {
        return LinkKind::Invalid;
    }
    if lower.starts_with("data:") {
        return LinkKind::Data;
    }
    if has_scheme(destination) {
        return LinkKind::External;
    }
    LinkKind::LocalFile
}

fn has_scheme(destination: &str) -> bool {
    let Some((scheme, _)) = destination.split_once(':') else {
        return false;
    };
    !scheme.is_empty()
        && scheme.starts_with(|character: char| character.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.'))
}
