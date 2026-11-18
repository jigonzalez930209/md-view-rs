//! Grid selection, navigation, and clipboard text: the logic without painting.

/// A cell position, zero-based.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CellPos {
    /// Row index.
    pub row: usize,
    /// Column index.
    pub column: usize,
}

impl CellPos {
    /// A cell position.
    pub fn new(row: usize, column: usize) -> Self {
        Self { row, column }
    }
}

/// A rectangular selection between an anchor and the cursor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// Where the selection started; stays put while Shift extends it.
    pub anchor: CellPos,
    /// The active cell.
    pub cursor: CellPos,
}

/// A movement on the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    /// One cell up.
    Up,
    /// One cell down.
    Down,
    /// One cell left.
    Left,
    /// One cell right.
    Right,
    /// First column of the row.
    RowStart,
    /// Last column of the row.
    RowEnd,
    /// A page of rows up.
    PageUp(usize),
    /// A page of rows down.
    PageDown(usize),
    /// The first cell.
    Start,
    /// The last cell.
    End,
}

impl Selection {
    /// Inclusive row range, top to bottom.
    pub fn rows(&self) -> (usize, usize) {
        (
            self.anchor.row.min(self.cursor.row),
            self.anchor.row.max(self.cursor.row),
        )
    }

    /// Inclusive column range, left to right.
    pub fn columns(&self) -> (usize, usize) {
        (
            self.anchor.column.min(self.cursor.column),
            self.anchor.column.max(self.cursor.column),
        )
    }

    /// Whether a cell is inside the selection.
    pub fn contains(&self, cell: CellPos) -> bool {
        let (top, bottom) = self.rows();
        let (left, right) = self.columns();
        (top..=bottom).contains(&cell.row) && (left..=right).contains(&cell.column)
    }

    /// Select one cell.
    pub fn select(&mut self, cell: CellPos) {
        self.anchor = cell;
        self.cursor = cell;
    }

    /// Move the cursor within a `rows` × `columns` grid; `extend` keeps the anchor (Shift).
    ///
    /// The grid always allows one row and one column past the data, so the cursor can reach
    /// the empty cells where new data is typed.
    pub fn apply(&mut self, motion: Motion, rows: usize, columns: usize, extend: bool) {
        let last_row = rows;
        let last_column = columns;
        let cursor = &mut self.cursor;
        match motion {
            Motion::Up => cursor.row = cursor.row.saturating_sub(1),
            Motion::Down => cursor.row = (cursor.row + 1).min(last_row),
            Motion::Left => cursor.column = cursor.column.saturating_sub(1),
            Motion::Right => cursor.column = (cursor.column + 1).min(last_column),
            Motion::RowStart => cursor.column = 0,
            Motion::RowEnd => cursor.column = columns.saturating_sub(1),
            Motion::PageUp(page) => cursor.row = cursor.row.saturating_sub(page.max(1)),
            Motion::PageDown(page) => cursor.row = (cursor.row + page.max(1)).min(last_row),
            Motion::Start => *cursor = CellPos::new(0, 0),
            Motion::End => *cursor = CellPos::new(rows.saturating_sub(1), columns.saturating_sub(1)),
        }
        if !extend {
            self.anchor = self.cursor;
        }
    }
}

/// Spreadsheet column name: A, B, …, Z, AA, AB, …
pub fn column_name(mut index: usize) -> String {
    let mut name = Vec::new();
    loop {
        name.push(b'A' + (index % 26) as u8);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    name.reverse();
    String::from_utf8(name).unwrap_or_default()
}

/// The selection as tab-separated text, the clipboard format spreadsheets exchange.
/// Fields with tabs, line breaks, or quotes are quoted, as Excel and LibreOffice do.
pub fn to_clipboard(values: &[Vec<String>]) -> String {
    values
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| {
                    if value.contains(['\t', '\n', '\r', '"']) {
                        format!("\"{}\"", value.replace('"', "\"\""))
                    } else {
                        value.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse clipboard text from a spreadsheet (tab-separated, quoted fields) into a block.
pub fn from_clipboard(text: &str) -> Vec<Vec<String>> {
    let text = text
        .strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .unwrap_or(text);
    let dialect = mdview_table::Dialect {
        delimiter: '\t',
        quote: '"',
        crlf: false,
    };
    mdview_table::Table::parse(text, dialect)
        .rows
        .into_iter()
        .map(|row| row.cells.into_iter().map(|cell| cell.value).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_names_follow_spreadsheets() {
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(25), "Z");
        assert_eq!(column_name(26), "AA");
        assert_eq!(column_name(27), "AB");
        assert_eq!(column_name(701), "ZZ");
        assert_eq!(column_name(702), "AAA");
    }

    #[test]
    fn motions_stay_inside_the_grid_plus_one() {
        let mut selection = Selection::default();
        selection.apply(Motion::Up, 3, 2, false);
        assert_eq!(selection.cursor, CellPos::new(0, 0));
        for _ in 0..10 {
            selection.apply(Motion::Down, 3, 2, false);
        }
        assert_eq!(selection.cursor.row, 3, "one row past the data");
        selection.apply(Motion::End, 3, 2, false);
        assert_eq!(selection.cursor, CellPos::new(2, 1));
        selection.apply(Motion::Start, 3, 2, true);
        assert_eq!(selection.rows(), (0, 2));
        assert_eq!(selection.columns(), (0, 1));
        assert!(selection.contains(CellPos::new(1, 1)));
    }

    #[test]
    fn clipboard_round_trips_through_spreadsheet_tsv() {
        let block = vec![
            vec!["a".to_owned(), "tab\there".to_owned()],
            vec!["line\nbreak".to_owned(), "say \"hi\"".to_owned()],
        ];
        let text = to_clipboard(&block);
        assert_eq!(from_clipboard(&text), block);
        assert_eq!(from_clipboard("x\ty\n1\t2\n"), vec![vec!["x", "y"], vec!["1", "2"]]);
    }
}
