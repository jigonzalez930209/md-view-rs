//! Grid edits as minimal text replacements.
//!
//! Every edit rewrites only the records it touches. In a rewritten record, fields that did not
//! change keep their original bytes, so quoting and spacing the user chose survive an edit
//! elsewhere in the row.

use std::ops::Range;

use crate::{Dialect, Row, Table};

/// Replace `range` (bytes in the source) with `text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    /// Byte range to replace.
    pub range: Range<usize>,
    /// Replacement text.
    pub text: String,
}

/// Apply non-overlapping edits to `text` (for tests and previews).
pub fn apply(text: &str, edits: &[TextEdit]) -> String {
    let mut sorted: Vec<&TextEdit> = edits.iter().collect();
    sorted.sort_by_key(|edit| edit.range.start);
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for edit in sorted {
        out.push_str(&text[cursor..edit.range.start]);
        out.push_str(&edit.text);
        cursor = edit.range.end;
    }
    out.push_str(&text[cursor..]);
    out
}

/// Quote a value when it holds the delimiter, the quote, a line break, or edge spaces.
fn quote(value: &str, dialect: Dialect) -> String {
    let needs = value.contains(dialect.delimiter)
        || value.contains(dialect.quote)
        || value.contains(['\n', '\r'])
        || value.starts_with(' ')
        || value.ends_with(' ');
    if !needs {
        return value.to_owned();
    }
    let doubled = format!("{0}{0}", dialect.quote);
    let escaped = value.replace(dialect.quote, &doubled);
    format!("{0}{escaped}{0}", dialect.quote)
}

impl Table {
    /// Raw text of the fields of `row`, original bytes for each.
    fn raw_fields<'a>(&self, text: &'a str, row: &Row) -> Vec<&'a str> {
        row.cells.iter().map(|cell| &text[cell.span.clone()]).collect()
    }

    fn join(&self, fields: &[String]) -> String {
        fields.join(&self.dialect.delimiter.to_string())
    }

    /// Where a new record goes after the last one: past its terminator, adding one if missing.
    fn append_point(&self) -> (usize, String) {
        match self.rows.last() {
            Some(last) if !self.trailing_newline => (last.span.end, self.dialect.line_ending().to_owned()),
            _ => (self.len, String::new()),
        }
    }

    /// Set one field. Rows and columns that do not exist yet are created.
    pub fn set_cell(&self, text: &str, row: usize, column: usize, value: &str) -> Vec<TextEdit> {
        self.set_block(text, row, column, &[vec![value.to_owned()]])
    }

    /// Set a block of fields with its top-left corner at (`row`, `column`), as a paste does.
    /// Short rows are padded with empty fields; rows past the end are appended.
    pub fn set_block(&self, text: &str, row: usize, column: usize, block: &[Vec<String>]) -> Vec<TextEdit> {
        let mut edits = Vec::new();
        let mut appended = Vec::new();
        for (offset, values) in block.iter().enumerate() {
            let index = row + offset;
            let mut fields: Vec<String> = match self.rows.get(index) {
                Some(existing) => self.raw_fields(text, existing).into_iter().map(str::to_owned).collect(),
                None => Vec::new(),
            };
            let width = (column + values.len()).max(fields.len());
            fields.resize(width, String::new());
            for (i, value) in values.iter().enumerate() {
                fields[column + i] = quote(value, self.dialect);
            }
            let line = self.join(&fields);
            match self.rows.get(index) {
                Some(existing) => edits.push(TextEdit {
                    range: existing.span.clone(),
                    text: line,
                }),
                None => {
                    // Rows between the table end and the block are created empty.
                    while self.rows.len() + appended.len() < index {
                        appended.push(String::new());
                    }
                    appended.push(line);
                }
            }
        }
        if !appended.is_empty() {
            edits.push(self.append_rows(&appended));
        }
        edits
    }

    fn append_rows(&self, lines: &[String]) -> TextEdit {
        let (at, prefix) = self.append_point();
        let ending = self.dialect.line_ending();
        let mut text = prefix;
        for line in lines {
            text.push_str(line);
            text.push_str(ending);
        }
        TextEdit { range: at..at, text }
    }

    /// Insert `count` empty records before `row` (at the end when `row` is past it).
    pub fn insert_rows(&self, row: usize, count: usize) -> Vec<TextEdit> {
        let empty = self.join(&vec![String::new(); self.columns().max(1)]);
        let lines = vec![empty; count];
        match self.rows.get(row) {
            Some(existing) => {
                let ending = self.dialect.line_ending();
                let text: String = lines.iter().map(|line| format!("{line}{ending}")).collect();
                vec![TextEdit {
                    range: existing.span.start..existing.span.start,
                    text,
                }]
            }
            None => vec![self.append_rows(&lines)],
        }
    }

    /// Delete the records in `rows`.
    pub fn delete_rows(&self, rows: Range<usize>) -> Vec<TextEdit> {
        let end = rows.end.min(self.rows.len());
        if rows.start >= end {
            return Vec::new();
        }
        let first = &self.rows[rows.start];
        let last = &self.rows[end - 1];
        // Deleting through the last record also removes the terminator before it, so the file
        // does not grow a blank line.
        let start = if end == self.rows.len() && !self.trailing_newline && rows.start > 0 {
            self.rows[rows.start - 1].span.end
        } else {
            first.span.start
        };
        vec![TextEdit {
            range: start..last.end,
            text: String::new(),
        }]
    }

    /// Insert `count` empty columns before `column` in every record.
    pub fn insert_columns(&self, text: &str, column: usize, count: usize) -> Vec<TextEdit> {
        self.rewrite_rows(text, |fields| {
            let at = column.min(fields.len());
            for _ in 0..count {
                fields.insert(at, String::new());
            }
        })
    }

    /// Delete the columns in `columns` from every record.
    pub fn delete_columns(&self, text: &str, columns: Range<usize>) -> Vec<TextEdit> {
        self.rewrite_rows(text, |fields| {
            let start = columns.start.min(fields.len());
            let end = columns.end.min(fields.len());
            fields.drain(start..end);
            if fields.is_empty() {
                fields.push(String::new());
            }
        })
    }

    fn rewrite_rows(&self, text: &str, change: impl Fn(&mut Vec<String>)) -> Vec<TextEdit> {
        self.rows
            .iter()
            .filter_map(|row| {
                let mut fields: Vec<String> = self.raw_fields(text, row).into_iter().map(str::to_owned).collect();
                change(&mut fields);
                let line = self.join(&fields);
                (line != text[row.span.clone()]).then(|| TextEdit {
                    range: row.span.clone(),
                    text: line,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> Table {
        Table::parse(text, Dialect::sniff(text, None))
    }

    #[test]
    fn set_cell_keeps_other_fields_byte_for_byte() {
        let text = "a,\"b, c\",d\n1,2,3\n";
        let t = table(text);
        let out = apply(text, &t.set_block(text, 0, 2, &[vec!["x".into()]]));
        assert_eq!(out, "a,\"b, c\",x\n1,2,3\n");
    }

    #[test]
    fn values_are_quoted_only_when_needed() {
        let text = "a,b\n";
        let t = table(text);
        let edit = |value: &str| apply(text, &t.set_block(text, 0, 1, &[vec![value.into()]]));
        assert_eq!(edit("plain"), "a,plain\n");
        assert_eq!(edit("x, y"), "a,\"x, y\"\n");
        assert_eq!(edit("say \"hi\""), "a,\"say \"\"hi\"\"\"\n");
        assert_eq!(edit("two\nlines"), "a,\"two\nlines\"\n");
        assert_eq!(edit(" pad"), "a,\" pad\"\n");
    }

    #[test]
    fn edits_past_the_end_grow_the_table() {
        let text = "a,b\n1,2";
        let t = table(text);
        let out = apply(text, &t.set_block(text, 3, 2, &[vec!["z".into()]]));
        assert_eq!(out, "a,b\n1,2\n\n,,z\n");
        let wide = apply(text, &t.set_block(text, 0, 3, &[vec!["w".into()]]));
        assert_eq!(wide, "a,b,,w\n1,2");
    }

    #[test]
    fn paste_block_spans_rows_and_columns() {
        let text = "a,b,c\n1,2,3\n";
        let t = table(text);
        let block = vec![vec!["x".into(), "y".into()], vec!["p".into(), "q".into()]];
        assert_eq!(apply(text, &t.set_block(text, 0, 1, &block)), "a,x,y\n1,p,q\n");
    }

    #[test]
    fn insert_and_delete_rows() {
        let text = "a,b\n1,2\n3,4\n";
        let t = table(text);
        assert_eq!(apply(text, &t.insert_rows(1, 1)), "a,b\n,\n1,2\n3,4\n");
        assert_eq!(apply(text, &t.insert_rows(9, 1)), "a,b\n1,2\n3,4\n,\n");
        assert_eq!(apply(text, &t.delete_rows(1..2)), "a,b\n3,4\n");
        let open = "a,b\n1,2";
        assert_eq!(apply(open, &table(open).delete_rows(1..2)), "a,b");
    }

    #[test]
    fn insert_and_delete_columns() {
        let text = "a,\"b,1\",c\r\n1,2,3\r\n";
        let t = table(text);
        assert_eq!(apply(text, &t.insert_columns(text, 1, 1)), "a,,\"b,1\",c\r\n1,,2,3\r\n");
        assert_eq!(apply(text, &t.delete_columns(text, 0..1)), "\"b,1\",c\r\n2,3\r\n");
    }

    #[test]
    fn tsv_and_crlf_round_trip() {
        let text = "x\ty\r\n1\t2\r\n";
        let t = table(text);
        assert_eq!(
            apply(text, &t.set_block(text, 2, 0, &[vec!["n".into()]])),
            "x\ty\r\n1\t2\r\nn\r\n"
        );
    }
}
