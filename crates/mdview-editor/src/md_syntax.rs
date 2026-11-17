//! Recognising Markdown list items and block prefixes on a single line.

/// The kind of a list item marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListKind {
    /// `-`, `*`, or `+`.
    Bullet(char),
    /// A number followed by `.` or `)`.
    Ordered(u64, char),
}

/// A list item at the start of a line: `<indent><marker> [task] content`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ListItem {
    /// Leading whitespace before the marker.
    pub(crate) indent: String,
    pub(crate) kind: ListKind,
    /// `Some(checked)` for a task item (`- [ ]` or `- [x]`).
    pub(crate) task: Option<bool>,
    /// Character column where the item's text starts.
    pub(crate) content_start: usize,
}

impl ListItem {
    /// The marker text after the indentation, including the space(s) after it.
    pub(crate) fn marker(&self) -> String {
        let mut marker = match self.kind {
            ListKind::Bullet(bullet) => format!("{bullet} "),
            ListKind::Ordered(number, delimiter) => format!("{number}{delimiter} "),
        };
        if let Some(checked) = self.task {
            marker.push_str(if checked { "[x] " } else { "[ ] " });
        }
        marker
    }

    /// Width of the marker without a task box: how far nested items are indented.
    pub(crate) fn nest_width(&self) -> usize {
        match self.kind {
            ListKind::Bullet(_) => 2,
            ListKind::Ordered(number, _) => number.to_string().len() + 2,
        }
    }

    /// The marker for the item that follows this one in the same list.
    pub(crate) fn next(&self) -> ListItem {
        let kind = match self.kind {
            ListKind::Ordered(number, delimiter) => ListKind::Ordered(number.saturating_add(1), delimiter),
            bullet => bullet,
        };
        let mut next = ListItem {
            indent: self.indent.clone(),
            kind,
            // A new task starts unchecked.
            task: self.task.map(|_| false),
            content_start: 0,
        };
        next.content_start = next.indent.chars().count() + next.marker().chars().count();
        next
    }
}

/// Parse a list item at the start of `line`.
pub(crate) fn list_item(line: &str) -> Option<ListItem> {
    let indent: String = line
        .chars()
        .take_while(|character| *character == ' ' || *character == '\t')
        .collect();
    let rest = &line[indent.len()..];
    let mut chars = rest.chars();
    let first = chars.next()?;
    let (kind, marker_len) = if matches!(first, '-' | '*' | '+') {
        (ListKind::Bullet(first), 1)
    } else if first.is_ascii_digit() {
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if digits.len() > 9 {
            return None;
        }
        let delimiter = rest[digits.len()..].chars().next()?;
        if !matches!(delimiter, '.' | ')') {
            return None;
        }
        (ListKind::Ordered(digits.parse().ok()?, delimiter), digits.len() + 1)
    } else {
        return None;
    };
    let after = &rest[marker_len..];
    // A marker needs a space after it, or nothing at all (an empty item).
    if !after.is_empty() && !after.starts_with(' ') {
        return None;
    }
    // `---`, `***`, and `* * *` are thematic breaks, not items.
    let bare: String = rest.chars().filter(|character| !character.is_whitespace()).collect();
    if matches!(kind, ListKind::Bullet(_)) && bare.len() >= 3 && bare.chars().all(|character| character == first) {
        return None;
    }
    // Five or more spaces start indented code inside the item: its text begins after one.
    let run = after.chars().take_while(|character| *character == ' ').count();
    let spaces = if run > 4 { 1 } else { run };
    let mut content = indent.chars().count() + marker_len + spaces;
    let body = &after[spaces.min(after.len())..];
    let task = if matches!(kind, ListKind::Bullet(_) | ListKind::Ordered(..)) {
        if body.starts_with("[ ]") && (body.len() == 3 || body[3..].starts_with(' ')) {
            content += 3 + usize::from(body.len() > 3);
            Some(false)
        } else if (body.starts_with("[x]") || body.starts_with("[X]"))
            && (body.len() == 3 || body[3..].starts_with(' '))
        {
            content += 3 + usize::from(body.len() > 3);
            Some(true)
        } else {
            None
        }
    } else {
        None
    };
    Some(ListItem {
        indent,
        kind,
        task,
        content_start: content,
    })
}

/// Heading level (`#` count) and the column where the heading text starts.
pub(crate) fn heading(line: &str) -> Option<(usize, usize)> {
    let indent = line.chars().take_while(|character| *character == ' ').count();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let level = rest.chars().take_while(|character| *character == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let after = &rest[level..];
    if !after.is_empty() && !after.starts_with(' ') {
        return None;
    }
    let spaces = after.chars().take_while(|character| *character == ' ').count();
    Some((level, indent + level + spaces))
}

/// Column where the text after a `> ` quote prefix starts.
pub(crate) fn quote(line: &str) -> Option<usize> {
    let indent = line.chars().take_while(|character| *character == ' ').count();
    if indent > 3 || !line[indent..].starts_with('>') {
        return None;
    }
    Some(indent + 1 + usize::from(line[indent + 1..].starts_with(' ')))
}

#[cfg(test)]
mod tests {
    use super::{ListKind, heading, list_item, quote};

    #[test]
    fn recognises_bullets_numbers_and_tasks() {
        let item = list_item("- item").expect("bullet");
        assert_eq!(
            (item.kind, item.task, item.content_start),
            (ListKind::Bullet('-'), None, 2)
        );
        let item = list_item("  12. twelve").expect("ordered");
        assert_eq!(
            (item.kind, item.indent.as_str(), item.content_start),
            (ListKind::Ordered(12, '.'), "  ", 6)
        );
        let item = list_item("* [x] done").expect("task");
        assert_eq!((item.task, item.content_start), (Some(true), 6));
        let item = list_item("1) [ ] todo").expect("ordered task");
        assert_eq!(
            (item.kind, item.task, item.content_start),
            (ListKind::Ordered(1, ')'), Some(false), 7)
        );
        assert_eq!(list_item("- ").map(|item| item.content_start), Some(2));
        assert_eq!(list_item("-").map(|item| item.content_start), Some(1));
    }

    #[test]
    fn rejects_text_that_is_not_an_item() {
        assert_eq!(list_item("-item"), None);
        assert_eq!(list_item("---"), None);
        assert_eq!(list_item("* * *"), None);
        assert_eq!(list_item("2024 was"), None);
        assert_eq!(list_item("1.5 apples"), None);
        assert_eq!(list_item("plain"), None);
    }

    #[test]
    fn next_marker_continues_the_list() {
        let next = list_item("9. [x] nine").expect("item").next();
        assert_eq!(next.marker(), "10. [ ] ");
        assert_eq!(next.content_start, 8);
        assert_eq!(list_item("  + a").expect("item").next().marker(), "+ ");
    }

    #[test]
    fn headings_and_quotes() {
        assert_eq!(heading("## Title"), Some((2, 3)));
        assert_eq!(heading("#hashtag"), None);
        assert_eq!(heading("####### seven"), None);
        assert_eq!(quote("> quoted"), Some(2));
        assert_eq!(quote(">x"), Some(1));
        assert_eq!(quote("x > y"), None);
    }
}
