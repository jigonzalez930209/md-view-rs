//! Bounded workspace text search worker.

use std::path::{Path, PathBuf};

use crate::search::SearchHit;

const MAX_FILES: usize = 5_000;
const MAX_BYTES: u64 = 512 * 1024;
const MAX_HITS: usize = 200;

/// Search the indexed files for a case-insensitive literal query.
pub fn search_workspace(root: &Path, files: &[PathBuf], query: &str) -> Result<Vec<SearchHit>, String> {
    let query = query.trim();
    if query.len() < 2 {
        return Ok(Vec::new());
    }
    let needle = query.to_lowercase();
    let mut hits = Vec::new();
    for path in files.iter().take(MAX_FILES) {
        if hits.len() >= MAX_HITS {
            break;
        }
        let Ok(metadata) = std::fs::metadata(path) else {
            continue;
        };
        if !metadata.is_file() || metadata.len() > MAX_BYTES {
            continue;
        }
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        if bytes.iter().take(1024).any(|byte| *byte == 0) {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        for (line_number, line) in text.lines().enumerate() {
            if line.len() > 2_000 {
                continue;
            }
            let lower = line.to_lowercase();
            let Some(byte_column) = lower.find(&needle) else {
                continue;
            };
            let Some(prefix) = line.get(..byte_column) else {
                continue;
            };
            let column = prefix.chars().count();
            hits.push(SearchHit {
                path: path.clone(),
                line: line_number,
                column,
                preview: line.trim().chars().take(160).collect(),
            });
            if hits.len() >= MAX_HITS {
                break;
            }
        }
        let _ = root;
    }
    hits.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.column.cmp(&right.column))
    });
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_case_insensitive_literal_matches() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("notes.md");
        std::fs::write(&path, "# One\nalpha Beta\nGamma\n").expect("write");
        let hits = search_workspace(directory.path(), &[path], "beta").expect("search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, 1);
        assert_eq!(hits[0].column, 6);
    }

    #[test]
    fn short_queries_and_binary_files_are_ignored() {
        let directory = tempfile::tempdir().expect("tempdir");
        let binary = directory.path().join("image.bin");
        std::fs::write(&binary, [0u8, 1, 2, 3]).expect("write");
        assert!(
            search_workspace(directory.path(), std::slice::from_ref(&binary), "a")
                .expect("search")
                .is_empty()
        );
        assert!(
            search_workspace(directory.path(), &[binary], "ab")
                .expect("search")
                .is_empty()
        );
    }
}
