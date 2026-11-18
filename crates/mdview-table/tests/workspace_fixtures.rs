//! The delimited samples in `examples/v1-workspace/media/tables` parse with the expected shape.

#![allow(clippy::expect_used)]

use std::path::Path;

use mdview_table::{Dialect, Table, is_table_path};

fn table(name: &str) -> (Table, String) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/v1-workspace/media/tables")
        .join(name);
    assert!(is_table_path(&path), "{name} opens in the grid");
    let text = std::fs::read_to_string(&path).expect("fixture");
    let dialect = Dialect::sniff(&text, Some(&path));
    (Table::parse(&text, dialect), text)
}

#[test]
fn quoted_multiline_and_unicode_cells() {
    let (table, _) = table("people.csv");
    assert_eq!((table.rows.len(), table.columns()), (7, 5));
    assert_eq!(table.value(1, 3), Some("Wrote the first program, \"Note G\""));
    assert_eq!(table.value(2, 3), Some("Two lines:\nsecond line"));
    assert_eq!(table.value(3, 1), Some("李小龍"));
    assert_eq!(table.value(5, 3), Some(""));
}

#[test]
fn every_dialect_is_sniffed() {
    for (name, delimiter, rows, columns) in [
        ("metrics.tsv", '\t', 4, 4),
        ("inventory.psv", '|', 4, 4),
        ("european.csv", ';', 4, 4),
    ] {
        let (table, _) = table(name);
        assert_eq!(table.dialect.delimiter, delimiter, "{name}");
        assert_eq!((table.rows.len(), table.columns()), (rows, columns), "{name}");
    }
    let (european, _) = table("european.csv");
    assert_eq!(european.value(3, 1), Some("Agua; con gas"));
}

#[test]
fn the_large_sample_keeps_crlf() {
    let (table, _) = table("large.csv");
    assert_eq!((table.rows.len(), table.columns()), (5001, 5));
    assert!(table.dialect.crlf);
    assert_eq!(table.value(5000, 4), Some("item-05000"));
}
