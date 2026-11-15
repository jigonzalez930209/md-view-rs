//! HTML `<table>` cells collected into GFM table shape.

use mdview_markdown::{Alignment, Inline};

use super::trim_edges;

/// Cells of an HTML table, shaped like a GFM table.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TableData {
    pub alignments: Vec<Alignment>,
    pub head: Vec<Vec<Inline>>,
    pub rows: Vec<Vec<Vec<Inline>>>,
}

#[derive(Debug, Default)]
pub(super) struct TableBuilder {
    pub(super) data: TableData,
    pub(super) row: Vec<Vec<Inline>>,
    pub(super) cell: Option<Vec<Inline>>,
    pub(super) row_is_head: bool,
    rows: Vec<(bool, Vec<Vec<Inline>>)>,
}

impl TableBuilder {
    pub(super) fn finish_cell(&mut self) {
        if let Some(mut cell) = self.cell.take() {
            trim_edges(&mut cell);
            self.row.push(cell);
        }
    }

    pub(super) fn finish_row(&mut self) {
        self.finish_cell();
        if !self.row.is_empty() {
            let row = std::mem::take(&mut self.row);
            self.rows.push((self.row_is_head, row));
        }
        self.row_is_head = false;
    }

    pub(super) fn finish(mut self) -> TableData {
        self.finish_row();
        let columns = self.rows.iter().map(|(_, row)| row.len()).max().unwrap_or(0);
        let mut rows = self.rows.into_iter();
        let mut data = self.data;
        data.alignments.resize(columns, Alignment::None);
        if let Some((_, mut head)) = rows.next() {
            head.resize(columns, Vec::new());
            data.head = head;
        }
        data.rows = rows
            .map(|(_, mut row)| {
                row.resize(columns, Vec::new());
                row
            })
            .collect();
        data
    }
}
