//! Case-insensitive literal search highlighting.

/// Split text into plain and query-matching runs, case-insensitively.
pub(crate) fn highlight_runs(text: &str, query: &str) -> Vec<(String, bool)> {
    let query = query.trim();
    if query.is_empty() || text.is_empty() {
        return vec![(text.to_owned(), false)];
    }
    let lower = text.to_lowercase();
    let needle = query.to_lowercase();
    let mut runs = Vec::new();
    let mut start = 0usize;
    while start < text.len() {
        let Some(relative) = lower[start..].find(&needle) else {
            break;
        };
        let hit = start + relative;
        if !text.is_char_boundary(hit) || !text.is_char_boundary(hit + needle.len()) {
            break;
        }
        if hit > start {
            runs.push((text[start..hit].to_owned(), false));
        }
        runs.push((text[hit..hit + needle.len()].to_owned(), true));
        start = hit + needle.len();
    }
    if start < text.len() {
        runs.push((text[start..].to_owned(), false));
    }
    if runs.is_empty() {
        runs.push((text.to_owned(), false));
    }
    runs
}
