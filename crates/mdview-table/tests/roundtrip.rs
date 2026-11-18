//! Values written through grid edits parse back unchanged, for every dialect.

use mdview_table::{Dialect, Table, apply};

/// A small deterministic generator (xorshift), so the test needs no dependency and is stable.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[(self.next() % items.len() as u64) as usize]
    }
}

const PIECES: &[&str] = &[
    "a", "Zz", " ", ",", ";", "\t", "|", "\"", "\"\"", "\n", "\r\n", "ñ", "€", "1.5", "",
];

fn value(rng: &mut Rng) -> String {
    let length = (rng.next() % 5) as usize;
    let text: String = (0..length).map(|_| rng.pick(PIECES)).collect();
    // A lone `\r` is not a line break any spreadsheet keeps; normalise it like they do.
    text.replace('\r', "")
}

#[test]
fn random_tables_round_trip() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for delimiter in [',', ';', '\t', '|'] {
        for crlf in [false, true] {
            let dialect = Dialect {
                delimiter,
                quote: '"',
                crlf,
            };
            for _ in 0..40 {
                let rows = 1 + (rng.next() % 5) as usize;
                let columns = 1 + (rng.next() % 4) as usize;
                let values: Vec<Vec<String>> = (0..rows)
                    .map(|_| (0..columns).map(|_| value(&mut rng)).collect())
                    .collect();
                let mut text = String::new();
                for (r, row) in values.iter().enumerate() {
                    for (c, cell) in row.iter().enumerate() {
                        let table = Table::parse(&text, dialect);
                        text = apply(&text, &table.set_cell(&text, r, c, cell));
                    }
                }
                let table = Table::parse(&text, dialect);
                for (r, row) in values.iter().enumerate() {
                    for (c, expected) in row.iter().enumerate() {
                        assert_eq!(
                            table.value(r, c).unwrap_or(""),
                            expected,
                            "delimiter {delimiter:?} crlf {crlf} cell ({r},{c}) in {text:?}"
                        );
                    }
                }
            }
        }
    }
}
