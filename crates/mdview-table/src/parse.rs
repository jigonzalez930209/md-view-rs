//! RFC 4180 parsing that keeps every field's byte range.

use std::ops::Range;

use crate::Dialect;

/// One field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    /// Byte range of the field in the source, quotes included.
    pub span: Range<usize>,
    /// The field's value, unquoted and unescaped.
    pub value: String,
}

/// One record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// Byte range of the record without its line terminator.
    pub span: Range<usize>,
    /// Byte offset just past the record's terminator (equal to `span.end` on the last line).
    pub end: usize,
    /// The record's fields.
    pub cells: Vec<Cell>,
}

/// A parsed table and the dialect it was written in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table {
    /// Records in source order.
    pub rows: Vec<Row>,
    /// How the source is delimited and quoted.
    pub dialect: Dialect,
    /// Byte length of the source.
    pub len: usize,
    /// Whether the last record ends with a line terminator.
    pub trailing_newline: bool,
}

impl Table {
    /// Parse `text`. Malformed quoting never fails: an unterminated quote runs to the end, and
    /// text after a closing quote joins the field, as spreadsheet applications read it.
    pub fn parse(text: &str, dialect: Dialect) -> Self {
        let bytes = text.as_bytes();
        let delimiter = dialect.delimiter;
        let quote = dialect.quote;
        let mut rows = Vec::new();
        // Skip a UTF-8 byte order mark; it belongs to no field.
        let mut index = if text.starts_with('\u{feff}') { 3 } else { 0 };
        let mut trailing_newline = false;
        while index < bytes.len() {
            let row_start = index;
            let mut cells = Vec::new();
            let row_end;
            let next;
            loop {
                let (cell, after) = parse_field(text, index, delimiter, quote);
                cells.push(cell);
                index = after;
                match text[index..].chars().next() {
                    Some(character) if character == delimiter => index += character.len_utf8(),
                    Some('\r') if text[index..].starts_with("\r\n") => {
                        row_end = index;
                        next = index + 2;
                        break;
                    }
                    Some('\n') => {
                        row_end = index;
                        next = index + 1;
                        break;
                    }
                    _ => {
                        row_end = index;
                        next = index;
                        break;
                    }
                }
            }
            trailing_newline = next > row_end;
            index = next;
            rows.push(Row {
                span: row_start..row_end,
                end: next,
                cells,
            });
        }
        Self {
            rows,
            dialect,
            len: bytes.len(),
            trailing_newline,
        }
    }

    /// Number of columns: the widest row.
    pub fn columns(&self) -> usize {
        self.rows.iter().map(|row| row.cells.len()).max().unwrap_or(0)
    }

    /// The value at a row and column, if that field exists.
    pub fn value(&self, row: usize, column: usize) -> Option<&str> {
        self.rows.get(row)?.cells.get(column).map(|cell| cell.value.as_str())
    }
}

/// Parse one field starting at `start`; returns it and the byte offset after it.
fn parse_field(text: &str, start: usize, delimiter: char, quote: char) -> (Cell, usize) {
    let rest = &text[start..];
    if !rest.starts_with(quote) {
        let length = rest.find([delimiter, '\n', '\r']).unwrap_or(rest.len());
        // A lone `\r` that is not part of `\r\n` stays in the field.
        let mut end = start + length;
        while text[end..].starts_with('\r') && !text[end..].starts_with("\r\n") {
            end += 1;
            let more = text[end..].find([delimiter, '\n', '\r']).unwrap_or(text.len() - end);
            end += more;
        }
        return (
            Cell {
                span: start..end,
                value: text[start..end].to_owned(),
            },
            end,
        );
    }
    let mut value = String::new();
    let mut index = start + quote.len_utf8();
    loop {
        match text[index..].find(quote) {
            None => {
                value.push_str(&text[index..]);
                index = text.len();
                break;
            }
            Some(offset) => {
                value.push_str(&text[index..index + offset]);
                index += offset + quote.len_utf8();
                if text[index..].starts_with(quote) {
                    value.push(quote);
                    index += quote.len_utf8();
                } else {
                    break;
                }
            }
        }
    }
    // Text after the closing quote (malformed) joins the field.
    let tail = text[index..]
        .find([delimiter, '\n', '\r'])
        .unwrap_or(text.len() - index);
    value.push_str(&text[index..index + tail]);
    let end = index + tail;
    (
        Cell {
            span: start..end,
            value,
        },
        end,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Table {
        Table::parse(text, Dialect::default())
    }

    #[test]
    fn plain_rows_and_spans() {
        let text = "a,b\n1,22\n";
        let table = parse(text);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.value(1, 1), Some("22"));
        assert_eq!(&text[table.rows[1].cells[1].span.clone()], "22");
        assert_eq!(table.rows[1].span, 4..8);
        assert_eq!(table.rows[1].end, 9);
        assert!(table.trailing_newline);
    }

    #[test]
    fn quoted_fields_with_delimiters_quotes_and_newlines() {
        let text = "name,note\r\n\"Ada, L.\",\"said \"\"hi\"\"\nthen left\"\r\n";
        let table = parse(text);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.value(1, 0), Some("Ada, L."));
        assert_eq!(table.value(1, 1), Some("said \"hi\"\nthen left"));
        assert_eq!(&text[table.rows[1].cells[0].span.clone()], "\"Ada, L.\"");
    }

    #[test]
    fn last_row_without_terminator_and_empty_fields() {
        let table = parse("a,,c\n,,");
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[1].cells.len(), 3);
        assert_eq!(table.value(0, 1), Some(""));
        assert!(!table.trailing_newline);
        assert_eq!(table.columns(), 3);
    }

    #[test]
    fn ragged_rows_and_bom() {
        let table = parse("\u{feff}a,b,c\n1\n");
        assert_eq!(table.rows[0].span.start, 3);
        assert_eq!(table.value(0, 0), Some("a"));
        assert_eq!(table.rows[1].cells.len(), 1);
        assert_eq!(table.columns(), 3);
    }

    #[test]
    fn malformed_quotes_never_panic() {
        for text in ["\"open", "\"a\"b,c", "a\"b\",c", "\"\"\"", ",\n,\n", "\r", "\"x\"\r"] {
            let table = parse(text);
            for row in &table.rows {
                for cell in &row.cells {
                    assert!(cell.span.end <= text.len(), "{text:?}");
                }
            }
        }
    }

    #[test]
    fn multibyte_delimiters_and_text() {
        let dialect = Dialect {
            delimiter: '\t',
            ..Dialect::default()
        };
        let table = Table::parse("año\tñandú\n€\t\"ü\"\n", dialect);
        assert_eq!(table.value(0, 1), Some("ñandú"));
        assert_eq!(table.value(1, 1), Some("ü"));
    }
}
