//! Markdown formatting actions: inline emphasis, links, code, and tables.
//!
//! Every action replaces one block of text, so a single undo reverts it, and keeps the
//! selection on the same text so actions can be combined (bold, then italic).

use crate::Editor;

/// A formatting action from the Markdown toolbar or a shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkdownAction {
    /// Set heading level 1–6 on the selected lines; 0 makes them normal text.
    Heading(u8),
    /// `**bold**`.
    Bold,
    /// `_italic_`.
    Italic,
    /// `<ins>underline</ins>`: GitHub shows inserted text underlined.
    Underline,
    /// `> ` before each line.
    Quote,
    /// Inline `` `code` ``, or a fenced block for several lines.
    Code,
    /// `[text](url)`.
    Link,
    /// `- ` items.
    BulletList,
    /// `1. ` items, numbered in order.
    NumberedList,
    /// `- [ ] ` task items.
    TaskList,
    /// A table with this many rows (including the header) and columns.
    Table {
        /// Rows including the header row.
        rows: usize,
        /// Columns.
        columns: usize,
    },
}

/// Which formats apply at the cursor, for toolbar buttons shown as pressed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MarkdownState {
    /// Heading level of the cursor line, 0 for normal text.
    pub heading: u8,
    /// Inside `**…**`.
    pub bold: bool,
    /// Inside `_…_` or `*…*`.
    pub italic: bool,
    /// Inside `<ins>…</ins>` or `<u>…</u>`.
    pub underline: bool,
    /// The cursor line is quoted.
    pub quote: bool,
    /// The cursor line is a bullet item (not a task).
    pub bullet: bool,
    /// The cursor line is a numbered item (not a task).
    pub numbered: bool,
    /// The cursor line is a task item.
    pub task: bool,
}

pub(crate) const BOLD: (&str, &str) = ("**", "**");
pub(crate) const ITALIC: (&str, &str) = ("_", "_");
pub(crate) const UNDERLINE: (&str, &str) = ("<ins>", "</ins>");
const CODE: (&str, &str) = ("`", "`");

/// Marker pairs recognised as already applied, per action.
fn pairs(marker: (&'static str, &'static str)) -> &'static [(&'static str, &'static str)] {
    match marker.0 {
        "**" => &[("**", "**"), ("__", "__")],
        "_" => &[("_", "_"), ("*", "*")],
        "<ins>" => &[("<ins>", "</ins>"), ("<u>", "</u>")],
        _ => &[("`", "`")],
    }
}

impl Editor {
    /// Whether the document is Markdown, by its file extension.
    pub fn is_markdown(&self) -> bool {
        self.document
            .path()
            .and_then(|path| path.extension())
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "md" | "markdown" | "mdown" | "mkd" | "mkdn" | "mdx"
                )
            })
    }

    /// Apply a formatting action to the selection or the cursor line.
    pub fn apply_markdown(&mut self, action: MarkdownAction) {
        match action {
            MarkdownAction::Heading(level) => self.set_heading(level.min(6)),
            MarkdownAction::Bold => self.toggle_inline(BOLD),
            MarkdownAction::Italic => self.toggle_inline(ITALIC),
            MarkdownAction::Underline => self.toggle_inline(UNDERLINE),
            MarkdownAction::Quote => self.toggle_quote(),
            MarkdownAction::Code => self.toggle_code(),
            MarkdownAction::Link => self.insert_link(),
            MarkdownAction::BulletList => self.toggle_list(crate::md_lists::ListStyle::Bullet),
            MarkdownAction::NumberedList => self.toggle_list(crate::md_lists::ListStyle::Numbered),
            MarkdownAction::TaskList => self.toggle_list(crate::md_lists::ListStyle::Task),
            MarkdownAction::Table { rows, columns } => self.insert_table(rows, columns),
        }
        self.preferred_column = None;
        self.scroll_to_cursor = true;
        self.focus_requested = true;
        self.cursor_blink_start = std::time::Instant::now();
        self.document.break_undo_group();
    }

    /// The formats active at the cursor or selection.
    pub fn markdown_state(&self) -> MarkdownState {
        let (start, end) = self.selection();
        let line = self.document.line_col(start).0;
        let text = self.document.line(line);
        let item = crate::md_syntax::list_item(&text);
        let layers = self.enclosing_layers(start, end);
        let inline = |marker: (&str, &str)| layers.iter().any(|layer| layer.0 == marker.0);
        MarkdownState {
            heading: crate::md_syntax::heading(&text).map_or(0, |(level, _)| level as u8),
            bold: inline(BOLD),
            italic: inline(ITALIC),
            underline: inline(UNDERLINE),
            quote: crate::md_syntax::quote(&text).is_some(),
            bullet: item
                .as_ref()
                .is_some_and(|item| item.task.is_none() && matches!(item.kind, crate::md_syntax::ListKind::Bullet(_))),
            numbered: item.as_ref().is_some_and(|item| {
                item.task.is_none() && matches!(item.kind, crate::md_syntax::ListKind::Ordered(..))
            }),
            task: item.as_ref().is_some_and(|item| item.task.is_some()),
        }
    }

    /// Every marker pair enclosing `[start, end)`, innermost first, as
    /// `(kind, open_start, close_end, open_len, close_len)`. `kind` is the canonical opening
    /// marker (`**`, `_`, `<ins>`, or `` ` ``), so `**_<ins>x</ins>_**` reports all three.
    fn enclosing_layers(&self, start: usize, end: usize) -> Vec<(&'static str, usize, usize, usize, usize)> {
        let mut layers = Vec::new();
        let (mut inner_start, mut inner_end) = (start, end);
        'outer: loop {
            for marker in [BOLD, ITALIC, UNDERLINE, CODE] {
                if layers
                    .iter()
                    .any(|layer: &(&str, usize, usize, usize, usize)| layer.0 == marker.0)
                {
                    continue;
                }
                if let Some((open_start, close_end, ol, cl)) =
                    self.inline_wrapped(inner_start, inner_end, pairs(marker))
                {
                    layers.push((marker.0, open_start, close_end, ol, cl));
                    (inner_start, inner_end) = (open_start, close_end);
                    continue 'outer;
                }
            }
            return layers;
        }
    }

    /// The marker pair around `[start, end)`: `(open_start, close_end, open, close)`.
    ///
    /// Markers count directly outside the range, or as the range's own first and last
    /// characters. A lone `*` next to another `*` is part of `**`, not italic.
    fn inline_wrapped(&self, start: usize, end: usize, pairs: &[(&str, &str)]) -> Option<(usize, usize, usize, usize)> {
        let (line, _) = self.document.line_col(start);
        let line_start = self.document.line_start_char(line);
        let line_end = line_start + self.document.line_len(line);
        if self.document.line_col(end).0 != line {
            return None;
        }
        let before = self.document.slice(line_start, start);
        let selected = self.document.slice(start, end);
        let after = self.document.slice(end, line_end);
        for (open, close) in pairs {
            let (ol, cl) = (open.chars().count(), close.chars().count());
            let single = ol == 1 && open == close;
            let clean = |outer: Option<char>| !single || outer != open.chars().next();
            if before.ends_with(open) && after.starts_with(close) {
                let outer_before = before.chars().rev().nth(ol);
                let outer_after = after.chars().nth(cl);
                if clean(outer_before) && clean(outer_after) {
                    return Some((start - ol, end + cl, ol, cl));
                }
            }
            let count = selected.chars().count();
            if count >= ol + cl && selected.starts_with(open) && selected.ends_with(close) {
                let inner_first = selected.chars().nth(ol);
                let inner_last = selected.chars().nth(count - cl - 1);
                if count > ol + cl && clean(inner_first) && clean(inner_last) {
                    return Some((start, end, ol, cl));
                }
            }
        }
        None
    }

    fn toggle_inline(&mut self, marker: (&'static str, &'static str)) {
        let (start, end) = self.selection();
        let (first_line, _) = self.document.line_col(start);
        let (last_line, _) = self.document.line_col(end);
        if first_line != last_line {
            self.toggle_inline_lines(marker, first_line, last_line);
            return;
        }
        let (open, close) = marker;
        let ol = open.chars().count();
        let backwards = self.anchor > self.cursor;
        let layer = self
            .enclosing_layers(start, end)
            .into_iter()
            .find(|layer| layer.0 == marker.0);
        if let Some((_, outer_start, outer_end, wl, wr)) = layer {
            // Already formatted, possibly under other formats: remove this format's markers
            // and keep the same text selected.
            let inner = self.document.slice(outer_start + wl, outer_end - wr);
            self.document.replace_range(outer_start, outer_end, &inner);
            let (new_start, new_end) = if outer_start < start {
                (start - wl, end - wl)
            } else {
                (start, end - wl - wr)
            };
            self.set_selection_ordered(new_start, new_end, backwards);
            return;
        }
        if start == end {
            let (word_start, word_end) = {
                let (line, column) = self.document.line_col(start);
                self.word_at(line, column)
            };
            if word_start < start && start < word_end {
                // Inside a word: format the whole word and keep the cursor in place.
                let word = self.document.slice(word_start, word_end);
                self.document
                    .replace_range(word_start, word_end, &format!("{open}{word}{close}"));
                self.cursor = start + ol;
                self.anchor = self.cursor;
                return;
            }
            // Nothing selected: insert the pair and type between the markers.
            self.document.insert_text(start, &format!("{open}{close}"));
            self.cursor = start + ol;
            self.anchor = self.cursor;
            return;
        }
        // Keep surrounding spaces outside the markers: `** bold **` is not bold.
        let selected = self.document.slice(start, end);
        let lead = selected
            .chars()
            .take_while(|character| character.is_whitespace())
            .count();
        let trail = selected
            .chars()
            .rev()
            .take_while(|character| character.is_whitespace())
            .count();
        let total = selected.chars().count();
        if lead == total {
            return;
        }
        let (inner_start, inner_end) = (start + lead, end - trail);
        let inner = self.document.slice(inner_start, inner_end);
        self.document
            .replace_range(inner_start, inner_end, &format!("{open}{inner}{close}"));
        self.set_selection_ordered(inner_start + ol, inner_end + ol, backwards);
    }

    /// Bold, italic, or underline on several lines: each line's selected text is formatted on
    /// its own (emphasis cannot span paragraphs). If every part is formatted, all are cleared.
    fn toggle_inline_lines(&mut self, marker: (&'static str, &'static str), first_line: usize, last_line: usize) {
        let (start, end) = self.selection();
        let block_start = self.document.line_start_char(first_line);
        let block_end = self.document.line_start_char(last_line) + self.document.line_len(last_line);
        let mut parts = Vec::new();
        for line in first_line..=last_line {
            let line_start = self.document.line_start_char(line);
            let line_end = line_start + self.document.line_len(line);
            let (from, to) = (start.max(line_start), end.min(line_end));
            let text = self.document.slice(from, to);
            let lead = text.chars().take_while(|character| character.is_whitespace()).count();
            let trail = text
                .chars()
                .rev()
                .take_while(|character| character.is_whitespace())
                .count();
            if lead < text.chars().count() {
                parts.push((from + lead, to - trail));
            }
        }
        let wrapped: Vec<_> = parts
            .iter()
            .map(|(from, to)| self.inline_wrapped(*from, *to, pairs(marker)))
            .collect();
        let remove = !parts.is_empty() && wrapped.iter().all(Option::is_some);
        let mut text = self.document.slice(block_start, block_end);
        // Edit from the end so earlier offsets stay valid.
        for (index, (from, to)) in parts.iter().enumerate().rev() {
            let (from, to) = (from - block_start, to - block_start);
            let chars: Vec<char> = text.chars().collect();
            let replaced: String = if remove {
                let (outer_start, outer_end, wl, wr) = wrapped[index].unwrap_or((0, 0, 0, 0));
                let (outer_start, outer_end) = (outer_start - block_start, outer_end - block_start);
                let mut out: String = chars[..outer_start].iter().collect();
                out.extend(&chars[outer_start + wl..outer_end - wr]);
                out.extend(&chars[outer_end..]);
                out
            } else if wrapped[index].is_some() {
                continue;
            } else {
                let mut out: String = chars[..from].iter().collect();
                out.push_str(marker.0);
                out.extend(&chars[from..to]);
                out.push_str(marker.1);
                out.extend(&chars[to..]);
                out
            };
            text = replaced;
        }
        self.document.replace_range(block_start, block_end, &text);
        self.anchor = block_start;
        self.cursor = block_start + text.chars().count();
    }

    /// Select `[start, end)`, keeping the cursor on the side it was on.
    fn set_selection_ordered(&mut self, start: usize, end: usize, backwards: bool) {
        if backwards {
            self.anchor = end;
            self.cursor = start;
        } else {
            self.anchor = start;
            self.cursor = end;
        }
    }

    fn toggle_code(&mut self) {
        let (start, end) = self.selection();
        let (first_line, _) = self.document.line_col(start);
        let (mut last_line, end_column) = self.document.line_col(end);
        if first_line == last_line {
            self.toggle_inline(CODE);
            return;
        }
        if end_column == 0 && last_line > first_line {
            last_line -= 1;
        }
        let fenced = first_line > 0
            && self.document.line(first_line - 1).trim_start().starts_with("```")
            && self.document.line(last_line + 1).trim() == "```";
        let block_start = self.document.line_start_char(first_line);
        let block_end = self.document.line_start_char(last_line) + self.document.line_len(last_line);
        let body = self.document.slice(block_start, block_end);
        if fenced {
            let outer_start = self.document.line_start_char(first_line - 1);
            let outer_end = self.document.line_start_char(last_line + 1) + self.document.line_len(last_line + 1);
            self.document.replace_range(outer_start, outer_end, &body);
            self.anchor = outer_start;
            self.cursor = outer_start + body.chars().count();
        } else {
            self.document
                .replace_range(block_start, block_end, &format!("```\n{body}\n```"));
            self.anchor = block_start + 4;
            self.cursor = self.anchor + body.chars().count();
        }
    }

    fn insert_link(&mut self) {
        let (start, end) = self.selection();
        let selected = self.document.slice(start, end);
        if selected.contains('\n') {
            return;
        }
        let looks_like_url = selected.starts_with("http://") || selected.starts_with("https://");
        if looks_like_url {
            // A selected address becomes the target; type the text between the brackets.
            self.document.replace_range(start, end, &format!("[]({selected})"));
            self.cursor = start + 1;
            self.anchor = self.cursor;
        } else if selected.is_empty() {
            self.document.insert_text(start, "[](url)");
            self.cursor = start + 1;
            self.anchor = self.cursor;
        } else {
            // Select the `url` placeholder so typing replaces it.
            let text = format!("[{selected}](url)");
            self.document.replace_range(start, end, &text);
            let url = start + selected.chars().count() + 3;
            self.anchor = url;
            self.cursor = url + 3;
        }
    }

    fn insert_table(&mut self, rows: usize, columns: usize) {
        let (rows, columns) = (rows.clamp(1, 64), columns.clamp(1, 32));
        let headers: Vec<String> = (1..=columns).map(|column| format!("Column {column}")).collect();
        let cell = |text: &str, width: usize| format!(" {text:<width$} ");
        let widths: Vec<usize> = headers.iter().map(|header| header.chars().count()).collect();
        let row = |cells: Vec<String>| format!("|{}|", cells.join("|"));
        let mut lines = vec![row(headers
            .iter()
            .zip(&widths)
            .map(|(text, width)| cell(text, *width))
            .collect())];
        lines.push(row(widths
            .iter()
            .map(|width| format!(" {} ", "-".repeat(*width)))
            .collect()));
        for _ in 1..rows {
            lines.push(row(widths.iter().map(|width| cell("", *width)).collect()));
        }
        let table = lines.join("\n");

        // A table needs its own lines, with a blank line before and after it.
        let (start, end) = self.selection();
        let (line, column) = self.document.line_col(start);
        let line_start = self.document.line_start_char(line);
        let before_empty = self.document.slice(line_start, start).trim().is_empty();
        let prefix = if column == 0 || before_empty {
            if line > 0 && !self.document.line(line - 1).trim().is_empty() && before_empty {
                "\n"
            } else {
                ""
            }
        } else {
            "\n\n"
        };
        let (end_line, end_column) = self.document.line_col(end);
        let rest_empty = end_column >= self.document.line_len(end_line);
        let next_blank =
            end_line + 1 >= self.document.line_count() || self.document.line(end_line + 1).trim().is_empty();
        let suffix = if !rest_empty {
            "\n\n"
        } else if next_blank {
            ""
        } else {
            "\n"
        };
        let replace_from = if before_empty { line_start } else { start };
        let text = format!("{prefix}{table}{suffix}");
        self.document.replace_range(replace_from, end, &text);
        // Select the first header so typing names the column.
        let first = replace_from + prefix.chars().count() + 2;
        self.anchor = first;
        self.cursor = first + headers[0].chars().count();
    }
}
