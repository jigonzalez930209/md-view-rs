//! Moving between Markdown table cells with Tab, as in a word processor.
//!
//! Tab selects the next cell's text (skipping the `| --- |` delimiter row); Tab in the last
//! cell adds a row with the same column widths. Shift+Tab goes back one cell.

use crate::Editor;

/// Character ranges between the pipes of a table row, or `None` for a line that is not one.
fn cells(line: &str) -> Option<Vec<(usize, usize)>> {
    let indent = line.chars().take_while(|character| *character == ' ').count();
    if indent > 3 || !line[indent..].starts_with('|') {
        return None;
    }
    let mut pipes = Vec::new();
    let mut escaped = false;
    for (index, character) in line.chars().enumerate() {
        match character {
            '\\' if !escaped => escaped = true,
            '|' if !escaped => {
                pipes.push(index);
                escaped = false;
            }
            _ => escaped = false,
        }
    }
    let length = line.chars().count();
    // A row without a closing pipe still has a last cell up to the end of the line.
    if pipes
        .last()
        .is_some_and(|last| *last + 1 < length && !line.trim_end().ends_with('|'))
    {
        pipes.push(length);
    }
    let ranges: Vec<(usize, usize)> = pipes.windows(2).map(|pair| (pair[0] + 1, pair[1])).collect();
    (!ranges.is_empty()).then_some(ranges)
}

fn is_delimiter(line: &str) -> bool {
    cells(line).is_some_and(|ranges| {
        ranges.iter().all(|(start, end)| {
            let cell: String = line.chars().skip(*start).take(end - start).collect();
            let cell = cell.trim();
            !cell.is_empty() && cell.trim_matches(':').chars().all(|character| character == '-')
        })
    })
}

/// Cell texts of a row, trimmed.
fn row_cells(line: &str) -> Vec<String> {
    cells(line)
        .unwrap_or_default()
        .iter()
        .map(|(start, end)| {
            line.chars()
                .skip(*start)
                .take(end - start)
                .collect::<String>()
                .trim()
                .to_owned()
        })
        .collect()
}

/// Rewrite table rows with every column padded to its widest cell. The delimiter row keeps
/// its `:` alignment marks; missing cells are added so every row has the same columns.
fn align_table(lines: &[String]) -> Vec<String> {
    let indent: String = lines
        .first()
        .map(|line| line.chars().take_while(|character| *character == ' ').collect())
        .unwrap_or_default();
    let rows: Vec<Vec<String>> = lines.iter().map(|line| row_cells(line)).collect();
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let delimiter: Vec<bool> = lines.iter().map(|line| is_delimiter(line)).collect();
    let mut widths = vec![3usize; columns];
    for (row, is_delimiter) in rows.iter().zip(&delimiter) {
        if *is_delimiter {
            continue;
        }
        for (column, cell) in row.iter().enumerate() {
            widths[column] = widths[column].max(crate::display_width(cell));
        }
    }
    rows.iter()
        .zip(&delimiter)
        .map(|(row, is_delimiter)| {
            let cells: Vec<String> = (0..columns)
                .map(|column| {
                    let cell = row.get(column).map_or("", String::as_str);
                    let width = widths[column];
                    if *is_delimiter {
                        let left = cell.starts_with(':');
                        let right = cell.len() > 1 && cell.ends_with(':');
                        let dashes = width - usize::from(left) - usize::from(right);
                        format!(
                            "{}{}{}",
                            if left { ":" } else { "" },
                            "-".repeat(dashes),
                            if right { ":" } else { "" }
                        )
                    } else {
                        let pad = width - crate::display_width(cell);
                        format!("{cell}{}", " ".repeat(pad))
                    }
                })
                .collect();
            format!("{indent}| {} |", cells.join(" | "))
        })
        .collect()
}

impl Editor {
    /// Tab or Shift+Tab inside a Markdown table. Returns `false` outside a table.
    pub(crate) fn markdown_table_tab(&mut self, back: bool) -> bool {
        if !self.is_markdown() {
            return false;
        }
        let (line, column) = self.document.line_col(self.cursor);
        let text = self.document.line(line);
        let Some(ranges) = cells(&text) else {
            return false;
        };
        if is_delimiter(&text) {
            return false;
        }
        // The whole table: the contiguous rows around the cursor.
        let mut first = line;
        while first > 0 && cells(&self.document.line(first - 1)).is_some() {
            first -= 1;
        }
        let mut last = line;
        while last + 1 < self.document.line_count() && cells(&self.document.line(last + 1)).is_some() {
            last += 1;
        }
        let mut rows: Vec<String> = (first..=last).map(|index| self.document.line(index)).collect();
        let delimiters: Vec<bool> = rows.iter().map(|row| is_delimiter(row)).collect();
        let current = ranges.iter().rposition(|(start, _)| *start <= column).unwrap_or(0);
        let row = line - first;
        let next_row = |from: usize, forward: bool| -> Option<usize> {
            let mut index = from;
            loop {
                index = if forward {
                    (index + 1 < delimiters.len()).then_some(index + 1)?
                } else {
                    index.checked_sub(1)?
                };
                if !delimiters[index] {
                    return Some(index);
                }
            }
        };
        let (target_row, target_cell) = if back {
            if current > 0 {
                (row, current - 1)
            } else if let Some(previous) = next_row(row, false) {
                (previous, usize::MAX)
            } else {
                (row, 0)
            }
        } else if current + 1 < ranges.len() {
            (row, current + 1)
        } else if let Some(next) = next_row(row, true) {
            (next, 0)
        } else {
            // Tab in the last cell adds a row.
            rows.push(format!("|{}", "|".repeat(ranges.len())));
            (rows.len() - 1, 0)
        };
        let aligned = align_table(&rows);
        let block_start = self.document.line_start_char(first);
        let block_end = self.document.line_start_char(last) + self.document.line_len(last);
        self.document.replace_range(block_start, block_end, &aligned.join("\n"));

        let target_text = &aligned[target_row];
        let Some(target_ranges) = cells(target_text) else {
            return true;
        };
        let (start, end) = target_ranges[target_cell.min(target_ranges.len() - 1)];
        let content: String = target_text.chars().skip(start).take(end - start).collect();
        let trimmed = content.trim().chars().count();
        let lead = if trimmed == 0 {
            1
        } else {
            content.chars().take_while(|character| *character == ' ').count()
        };
        let line_start = self.document.line_start_char(first + target_row);
        self.anchor = line_start + start + lead.min(end - start);
        // An empty cell gets the cursor after its leading space; a filled one is selected.
        self.cursor = self.anchor + trimmed;
        self.preferred_column = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{align_table, cells, is_delimiter};

    #[test]
    fn finds_cells_between_pipes() {
        assert_eq!(cells("| a | b |"), Some(vec![(1, 4), (5, 8)]));
        assert_eq!(cells("| a \\| b | c"), Some(vec![(1, 9), (10, 12)]));
        assert_eq!(cells("not | a row"), None);
        assert!(is_delimiter("| --- | :-: |"));
        assert!(!is_delimiter("| a | b |"));
    }

    #[test]
    fn aligns_columns_and_keeps_alignment_marks() {
        let rows = ["| Name | Age |", "|:--|--:|", "| Ana Lucía | 30 |", "| 中文 |"].map(str::to_owned);
        assert_eq!(
            align_table(&rows),
            vec![
                "| Name      | Age |",
                "| :-------- | --: |",
                "| Ana Lucía | 30  |",
                "| 中文      |     |",
            ]
        );
    }
}
