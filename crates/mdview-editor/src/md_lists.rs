//! Line-level Markdown editing: headings, quotes, lists, and list continuation.
//!
//! Enter at the end of a list item starts the next item (numbers increase, tasks start
//! unchecked). Enter on an empty item leaves the list: a nested item moves up to its
//! parent's level, a top-level item loses its marker. Tab on an empty item nests it.
//! Following numbered items are renumbered in the same edit, so one undo reverts it.

use std::collections::BTreeMap;

use crate::Editor;
use crate::md_syntax::{ListItem, ListKind, heading, list_item, quote};

/// The list a toolbar button or shortcut applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListStyle {
    Bullet,
    Numbered,
    Task,
}

/// One rewritten line: its new text and where the text after the prefix starts, before
/// and after the change, so the cursor stays on the same character.
struct Rewrite {
    text: String,
    old_prefix: usize,
    new_prefix: usize,
}

fn leading(line: &str) -> String {
    line.chars()
        .take_while(|character| *character == ' ' || *character == '\t')
        .collect()
}

impl Editor {
    /// The selected lines; a selection ending at column 0 does not include that line.
    fn selected_lines(&self) -> (usize, usize) {
        let (start, end) = self.selection();
        let first = self.document.line_col(start).0;
        let (mut last, column) = self.document.line_col(end);
        if last > first && column == 0 {
            last -= 1;
        }
        (first, last)
    }

    /// Replace lines `first..=last` in one edit and map the selection onto the new text.
    fn rewrite_lines(&mut self, first: usize, rewrites: Vec<Rewrite>) {
        if rewrites.is_empty() {
            return;
        }
        let last = first + rewrites.len() - 1;
        let block_start = self.document.line_start_char(first);
        let block_end = self.document.line_start_char(last) + self.document.line_len(last);
        let text = rewrites
            .iter()
            .map(|rewrite| rewrite.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if text == self.document.slice(block_start, block_end) {
            return;
        }
        let map = |index: usize, rewrites: &[Rewrite], document: &mdview_model::Document| -> usize {
            let (line, column) = document.line_col(index);
            if line < first || line > last {
                return index;
            }
            let offset: usize = rewrites[..line - first]
                .iter()
                .map(|rewrite| rewrite.text.chars().count() + 1)
                .sum();
            let rewrite = &rewrites[line - first];
            let column = if column >= rewrite.old_prefix {
                column - rewrite.old_prefix + rewrite.new_prefix
            } else {
                rewrite.new_prefix
            };
            block_start + offset + column.min(rewrite.text.chars().count())
        };
        let anchor = map(self.anchor, &rewrites, &self.document);
        let cursor = map(self.cursor, &rewrites, &self.document);
        self.document.replace_range(block_start, block_end, &text);
        self.anchor = anchor;
        self.cursor = cursor;
    }

    pub(crate) fn set_heading(&mut self, level: u8) {
        let (first, last) = self.selected_lines();
        let lines: Vec<String> = (first..=last).map(|line| self.document.line(line)).collect();
        let level = usize::from(level);
        let filled = |text: &String| !text.trim().is_empty() || first == last;
        // Pressing the level a line already has turns it back into normal text.
        let level = if level > 0
            && lines
                .iter()
                .filter(|text| filled(text))
                .all(|text| heading(text).is_some_and(|(current, _)| current == level))
        {
            0
        } else {
            level
        };
        let rewrites = lines
            .iter()
            .map(|text| {
                let old_prefix = heading(text).map_or(0, |(_, start)| start);
                let body: String = text.chars().skip(old_prefix).collect();
                if !filled(text) {
                    return Rewrite {
                        text: text.clone(),
                        old_prefix: 0,
                        new_prefix: 0,
                    };
                }
                let prefix = if level == 0 {
                    String::new()
                } else {
                    format!("{} ", "#".repeat(level))
                };
                Rewrite {
                    text: format!("{prefix}{body}"),
                    old_prefix,
                    new_prefix: prefix.chars().count(),
                }
            })
            .collect();
        self.rewrite_lines(first, rewrites);
    }

    pub(crate) fn toggle_quote(&mut self) {
        let (first, last) = self.selected_lines();
        let lines: Vec<String> = (first..=last).map(|line| self.document.line(line)).collect();
        let remove = lines
            .iter()
            .filter(|text| !text.trim().is_empty())
            .all(|text| quote(text).is_some())
            && lines.iter().any(|text| quote(text).is_some());
        let rewrites = lines
            .iter()
            .map(|text| match (remove, quote(text)) {
                (true, Some(start)) => Rewrite {
                    text: text.chars().skip(start).collect(),
                    old_prefix: start,
                    new_prefix: 0,
                },
                (false, None) => {
                    let prefix = if text.is_empty() { ">" } else { "> " };
                    Rewrite {
                        text: format!("{prefix}{text}"),
                        old_prefix: 0,
                        new_prefix: prefix.len(),
                    }
                }
                _ => Rewrite {
                    text: text.clone(),
                    old_prefix: 0,
                    new_prefix: 0,
                },
            })
            .collect();
        self.rewrite_lines(first, rewrites);
    }

    pub(crate) fn toggle_list(&mut self, style: ListStyle) {
        let (first, last) = self.selected_lines();
        let lines: Vec<String> = (first..=last).map(|line| self.document.line(line)).collect();
        let is_style = |item: &ListItem| match style {
            ListStyle::Bullet => item.task.is_none() && matches!(item.kind, ListKind::Bullet(_)),
            ListStyle::Numbered => item.task.is_none() && matches!(item.kind, ListKind::Ordered(..)),
            ListStyle::Task => item.task.is_some(),
        };
        let filled: Vec<bool> = lines
            .iter()
            .map(|text| !text.trim().is_empty() || first == last)
            .collect();
        let remove = lines
            .iter()
            .zip(&filled)
            .filter(|(_, filled)| **filled)
            .all(|(text, _)| list_item(text).is_some_and(|item| is_style(&item)));
        // Numbers continue an ordered list right above the selection, per indentation.
        let mut counters: BTreeMap<usize, u64> = BTreeMap::new();
        if first > 0
            && let Some(above) = list_item(&self.document.line(first - 1))
            && let ListKind::Ordered(number, _) = above.kind
        {
            counters.insert(above.indent.chars().count(), number);
        }
        let rewrites = lines
            .iter()
            .zip(&filled)
            .map(|(text, filled)| {
                let item = list_item(text);
                let indent = item.as_ref().map_or_else(|| leading(text), |item| item.indent.clone());
                let old_prefix = item.as_ref().map_or(indent.chars().count(), |item| item.content_start);
                let body: String = text.chars().skip(old_prefix).collect();
                if !*filled {
                    return Rewrite {
                        text: text.clone(),
                        old_prefix: 0,
                        new_prefix: 0,
                    };
                }
                let width = indent.chars().count();
                let marker = if remove {
                    String::new()
                } else {
                    match style {
                        ListStyle::Bullet => "- ".to_owned(),
                        ListStyle::Task => {
                            let checked = item.as_ref().and_then(|item| item.task).unwrap_or(false);
                            if checked { "- [x] " } else { "- [ ] " }.to_owned()
                        }
                        ListStyle::Numbered => {
                            counters.retain(|level, _| *level <= width);
                            let number = counters.entry(width).or_insert(0);
                            *number += 1;
                            format!("{number}. ")
                        }
                    }
                };
                let new_prefix = width + marker.chars().count();
                Rewrite {
                    text: format!("{indent}{marker}{body}"),
                    old_prefix,
                    new_prefix,
                }
            })
            .collect();
        self.rewrite_lines(first, rewrites);
    }

    /// Enter in a Markdown list. Returns `false` when the line is not a list item, so the
    /// plain newline applies.
    pub(crate) fn markdown_newline(&mut self) -> bool {
        let (start, end) = self.selection();
        if !self.is_markdown() {
            return false;
        }
        let (line, column) = self.document.line_col(start);
        let Some(item) = list_item(&self.document.line(line)) else {
            return false;
        };
        if column < item.content_start || self.document.line_col(end).0 != line {
            return false;
        }
        if start != end {
            // Typing Enter over a selection inside an item replaces it, then continues.
            self.document.delete_range(start, end);
            self.cursor = start;
            self.anchor = start;
        }
        let text = self.document.line(line);
        let length = text.chars().count();
        let content_empty = text.chars().skip(item.content_start).all(char::is_whitespace);
        if content_empty && column == length {
            if item.indent.is_empty() {
                // Enter on an empty top-level item ends the list.
                let line_start = self.document.line_start_char(line);
                self.document.replace_range(line_start, line_start + length, "");
                self.cursor = line_start;
                self.anchor = line_start;
                return true;
            }
            return self.outdent_item(line, &item);
        }
        let next = item.next();
        let before: String = text.chars().take(column).collect();
        let after: String = text.chars().skip(column).collect::<String>().trim_start().to_owned();
        let new_line = format!("{}{}{}", next.indent, next.marker(), after);
        let mut lines = vec![before.trim_end().to_owned(), new_line];
        let renumbered = self.renumber_after(line, &next, &mut lines);
        self.replace_lines(line, line + renumbered, &lines);
        let line_start = self.document.line_start_char(line);
        self.cursor = line_start + lines[0].chars().count() + 1 + next.content_start;
        self.anchor = self.cursor;
        true
    }

    /// Tab and Shift+Tab on a list item: nest an empty item one level, or move an item up
    /// to its parent's level. Returns `false` when this is not such a case.
    pub(crate) fn markdown_tab(&mut self, dedent: bool) -> bool {
        let (start, end) = self.selection();
        if !self.is_markdown() || start != end {
            return false;
        }
        let (line, column) = self.document.line_col(start);
        let text = self.document.line(line);
        let Some(item) = list_item(&text) else {
            return false;
        };
        if dedent {
            return !item.indent.is_empty() && self.outdent_item(line, &item);
        }
        if column != item.content_start.min(text.chars().count()) {
            return false;
        }
        // Nest under the item above: indent to its text and restart numbering.
        let parent = (line > 0).then(|| list_item(&self.document.line(line - 1))).flatten();
        let width = parent.as_ref().map_or(item.nest_width(), ListItem::nest_width);
        let mut nested = item.clone();
        nested.indent = format!("{}{}", item.indent, " ".repeat(width));
        if let ListKind::Ordered(_, delimiter) = item.kind {
            nested.kind = ListKind::Ordered(1, delimiter);
        }
        let body: String = text.chars().skip(item.content_start).collect();
        let new_text = format!("{}{}{}", nested.indent, nested.marker(), body);
        let new_prefix = nested.indent.chars().count() + nested.marker().chars().count();
        self.rewrite_lines(
            line,
            vec![Rewrite {
                text: new_text,
                old_prefix: item.content_start,
                new_prefix,
            }],
        );
        true
    }

    /// Move an item to its parent's level, continuing the parent's list.
    fn outdent_item(&mut self, line: usize, item: &ListItem) -> bool {
        let width = item.indent.chars().count();
        let parent = (0..line)
            .rev()
            .map(|index| self.document.line(index))
            .take_while(|text| !text.trim().is_empty())
            .filter_map(|text| list_item(&text))
            .find(|candidate| candidate.indent.chars().count() < width);
        let mut moved = match &parent {
            Some(parent) => parent.next(),
            None => ListItem {
                indent: String::new(),
                ..item.clone()
            },
        };
        // The item keeps its own kind of box: a task stays a task at the parent level.
        moved.task = item.task;
        moved.content_start = moved.indent.chars().count() + moved.marker().chars().count();
        let text = self.document.line(line);
        let body: String = text.chars().skip(item.content_start).collect();
        let column = self.document.line_col(self.cursor).1;
        let mut lines = vec![format!("{}{}{}", moved.indent, moved.marker(), body)];
        let renumbered = self.renumber_after(line, &moved, &mut lines);
        self.replace_lines(line, line + renumbered, &lines);
        let line_start = self.document.line_start_char(line);
        self.cursor = line_start + column.saturating_sub(item.content_start) + moved.content_start;
        self.cursor = self.cursor.min(line_start + self.document.line_len(line));
        self.anchor = self.cursor;
        true
    }

    /// Renumber the ordered siblings that follow `item`, which now ends `lines`. The old
    /// lines after line `from` are scanned; their new text is appended to `lines`, and the
    /// number of old lines covered is returned.
    fn renumber_after(&self, from: usize, item: &ListItem, lines: &mut Vec<String>) -> usize {
        let ListKind::Ordered(mut number, _) = item.kind else {
            return 0;
        };
        let width = item.indent.chars().count();
        let mut covered = 0;
        let mut index = from + 1;
        while index < self.document.line_count() {
            let text = self.document.line(index);
            if text.trim().is_empty() {
                break;
            }
            match list_item(&text) {
                Some(sibling) if sibling.indent.chars().count() == width => {
                    let ListKind::Ordered(current, delimiter) = sibling.kind else {
                        break;
                    };
                    number += 1;
                    let fixed = if current == number {
                        text
                    } else {
                        let renumbered = ListItem {
                            kind: ListKind::Ordered(number, delimiter),
                            ..sibling.clone()
                        };
                        let body: String = text.chars().skip(sibling.content_start).collect();
                        format!("{}{}{}", renumbered.indent, renumbered.marker(), body)
                    };
                    lines.push(fixed);
                }
                Some(nested) if nested.indent.chars().count() > width => lines.push(text),
                None if leading(&text).chars().count() > width => lines.push(text),
                _ => break,
            }
            covered += 1;
            index += 1;
        }
        covered
    }

    /// Replace lines `first..=last` with `lines` in one edit.
    fn replace_lines(&mut self, first: usize, last: usize, lines: &[String]) {
        let start = self.document.line_start_char(first);
        let end = self.document.line_start_char(last) + self.document.line_len(last);
        self.document.replace_range(start, end, &lines.join("\n"));
    }
}
