//! Table and literal-block parsing.

use super::*;

impl<'a> EventParser<'a> {
    pub(super) fn parse_table(&mut self) -> (Vec<Vec<Inline>>, Vec<Vec<Vec<Inline>>>) {
        let mut head = Vec::new();
        let mut rows = Vec::new();
        loop {
            match self.peek().cloned() {
                Some((Event::Start(Tag::TableHead), _)) => {
                    self.cursor += 1;
                    while matches!(self.peek(), Some((Event::Start(Tag::TableCell), _))) {
                        self.cursor += 1;
                        let (cell, _) = self.parse_inlines(InlineEnd::Cell);
                        head.push(cell);
                    }
                    if matches!(self.peek(), Some((Event::End(TagEnd::TableHead), _))) {
                        self.cursor += 1;
                    }
                }
                Some((Event::Start(Tag::TableRow), _)) => {
                    self.cursor += 1;
                    let mut row = Vec::new();
                    while matches!(self.peek(), Some((Event::Start(Tag::TableCell), _))) {
                        self.cursor += 1;
                        let (cell, _) = self.parse_inlines(InlineEnd::Cell);
                        row.push(cell);
                    }
                    if matches!(self.peek(), Some((Event::End(TagEnd::TableRow), _))) {
                        self.cursor += 1;
                    }
                    rows.push(row);
                }
                Some((Event::End(TagEnd::Table), _)) => {
                    self.cursor += 1;
                    break;
                }
                Some(_) => self.cursor += 1,
                None => break,
            }
        }
        (head, rows)
    }

    /// Collect text for code and HTML blocks, stopping at the first end tag.
    pub(super) fn collect_literal(&mut self, end: BlockEnd) -> String {
        let mut text = String::new();
        while let Some((event, _)) = self.peek().cloned() {
            match event {
                Event::Text(value) | Event::Code(value) | Event::Html(value) => {
                    text.push_str(&value);
                    self.cursor += 1;
                }
                Event::End(tag_end) => {
                    self.cursor += 1;
                    if end.matches(&tag_end) || end == BlockEnd::Any {
                        break;
                    }
                }
                _ => self.cursor += 1,
            }
        }
        text
    }
}
