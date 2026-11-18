//! Delimiter, quote, and line-ending detection.

use std::path::Path;

/// Extensions opened in the grid view.
const TABLE_EXTENSIONS: &[&str] = &["csv", "tsv", "tab", "psv", "dsv", "ssv"];

/// Delimiters tried when the extension does not fix one, in preference order.
const CANDIDATES: &[char] = &[',', ';', '\t', '|'];

/// Lines inspected when sniffing the delimiter.
const SNIFF_LINES: usize = 64;

/// How a delimited file is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dialect {
    /// Field separator.
    pub delimiter: char,
    /// Quote character for fields that contain special characters.
    pub quote: char,
    /// Whether rows end with `\r\n` (otherwise `\n`).
    pub crlf: bool,
}

impl Default for Dialect {
    fn default() -> Self {
        Self {
            delimiter: ',',
            quote: '"',
            crlf: false,
        }
    }
}

impl Dialect {
    /// Detect the dialect of `text`. `path` fixes the delimiter for `.tsv`/`.tab` (tab) and
    /// `.psv` (pipe); otherwise the candidate that splits the most lines into the same, largest
    /// number of fields wins, so `;` (European CSV) is found without a setting.
    pub fn sniff(text: &str, path: Option<&Path>) -> Self {
        let crlf = text.find('\n').is_some_and(|newline| text[..newline].ends_with('\r'));
        let extension = path
            .and_then(|path| path.extension())
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        let fixed = match extension.as_deref() {
            Some("tsv" | "tab") => Some('\t'),
            Some("psv") => Some('|'),
            _ => None,
        };
        let delimiter = fixed.unwrap_or_else(|| best_delimiter(text));
        Self {
            delimiter,
            quote: '"',
            crlf,
        }
    }

    /// The row terminator.
    pub fn line_ending(&self) -> &'static str {
        if self.crlf { "\r\n" } else { "\n" }
    }
}

/// The candidate with the most lines that share one field count, preferring more fields.
fn best_delimiter(text: &str) -> char {
    let lines: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(SNIFF_LINES)
        .collect();
    let mut best = (',', 0usize, 0usize);
    for &candidate in CANDIDATES {
        let counts: Vec<usize> = lines
            .iter()
            .map(|line| fields_outside_quotes(line, candidate))
            .collect();
        let Some(&mode) = mode(&counts) else {
            continue;
        };
        if mode < 2 {
            continue;
        }
        let agreeing = counts.iter().filter(|&&count| count == mode).count();
        if (agreeing, mode) > (best.1, best.2) {
            best = (candidate, agreeing, mode);
        }
    }
    best.0
}

fn fields_outside_quotes(line: &str, delimiter: char) -> usize {
    let mut quoted = false;
    let mut fields = 1;
    for character in line.chars() {
        if character == '"' {
            quoted = !quoted;
        } else if character == delimiter && !quoted {
            fields += 1;
        }
    }
    fields
}

fn mode(values: &[usize]) -> Option<&usize> {
    values
        .iter()
        .max_by_key(|value| (values.iter().filter(|other| other == value).count(), **value))
}

/// Whether a path opens in the grid view.
pub fn is_table_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| TABLE_EXTENSIONS.contains(&extension.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_common_delimiters() {
        assert_eq!(Dialect::sniff("a,b,c\n1,2,3\n", None).delimiter, ',');
        assert_eq!(Dialect::sniff("a;b;c\n1,5;2,5;3\n", None).delimiter, ';');
        assert_eq!(Dialect::sniff("a\tb\n1\t2\n", None).delimiter, '\t');
        assert_eq!(Dialect::sniff("a|b|c\n1|2|3\n", None).delimiter, '|');
    }

    #[test]
    fn quoted_delimiters_do_not_count() {
        assert_eq!(
            Dialect::sniff("name;note\nAda;\"a, b, c\"\nBob;\"d, e\"\n", None).delimiter,
            ';'
        );
    }

    #[test]
    fn extensions_fix_the_delimiter() {
        assert_eq!(Dialect::sniff("a,b\n", Some(Path::new("x.tsv"))).delimiter, '\t');
        assert_eq!(Dialect::sniff("a,b\n", Some(Path::new("x.psv"))).delimiter, '|');
    }

    #[test]
    fn detects_crlf() {
        assert!(Dialect::sniff("a,b\r\n1,2\r\n", None).crlf);
        assert!(!Dialect::sniff("a,b\n1,2\n", None).crlf);
    }

    #[test]
    fn table_paths() {
        assert!(is_table_path(Path::new("data.CSV")));
        assert!(is_table_path(Path::new("data.tsv")));
        assert!(!is_table_path(Path::new("notes.md")));
    }
}
