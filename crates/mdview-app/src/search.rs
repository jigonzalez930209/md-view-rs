//! Document and workspace search.
//!
//! Document search runs on the UI thread against the active buffer. Workspace search
//! runs on a short-lived worker, reads the indexed file list, and drops results from an
//! older query generation. The worker never touches egui or the document model.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use mdview_ui::SearchScope;

/// One search match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    /// File containing the match.
    pub path: PathBuf,
    /// Zero-based line.
    pub line: usize,
    /// Zero-based character column.
    pub column: usize,
    /// Trimmed line preview.
    pub preview: String,
}

type SearchResult = (u64, Result<Vec<SearchHit>, String>);

/// Persistent search popover state.
pub struct SearchState {
    /// Whether the popover is visible.
    pub open: bool,
    /// Current scope.
    pub scope: SearchScope,
    /// Query as typed.
    pub query: String,
    /// Ask the next paint to focus the query field.
    pub focus: bool,
    /// Current results.
    pub hits: Vec<SearchHit>,
    /// Selected result.
    pub selected: usize,
    /// Whether a workspace search is running.
    pub running: bool,
    /// Last worker error.
    pub error: Option<String>,
    /// Query used for the current results.
    last_query: String,
    /// Document revision used for the current document results.
    last_revision: Option<u64>,
    /// Pending workspace query, debounced until `deadline`.
    pending: Option<(String, Instant)>,
    /// Generation counter; stale worker results are ignored.
    generation: u64,
    receiver: Option<Receiver<SearchResult>>,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            open: false,
            scope: SearchScope::Document,
            query: String::new(),
            focus: false,
            hits: Vec::new(),
            selected: 0,
            running: false,
            error: None,
            last_query: String::new(),
            last_revision: None,
            pending: None,
            generation: 0,
            receiver: None,
        }
    }
}

impl SearchState {
    /// Open the popover with a scope.
    pub fn open(&mut self, scope: SearchScope) {
        self.open = true;
        self.scope = scope;
        self.focus = true;
        self.selected = 0;
    }

    /// Close the popover.
    pub fn close(&mut self) {
        self.open = false;
        self.running = false;
        self.receiver = None;
        self.pending = None;
        self.generation = self.generation.wrapping_add(1);
    }

    /// Switch scope and discard results from the other scope.
    pub fn set_scope(&mut self, scope: SearchScope) {
        if self.scope == scope {
            return;
        }
        self.scope = scope;
        self.focus = true;
        self.last_query.clear();
        self.last_revision = None;
        self.hits.clear();
        self.selected = 0;
        self.error = None;
        self.receiver = None;
        self.running = false;
        self.pending = None;
        self.generation = self.generation.wrapping_add(1);
    }

    /// Mark the query dirty.
    pub fn query_changed(&mut self) {
        self.last_query.clear();
        self.last_revision = None;
        self.selected = 0;
        if self.scope == SearchScope::Workspace {
            self.pending = Some((self.query.clone(), Instant::now() + Duration::from_millis(160)));
        }
    }

    /// The debounced query to launch, if any.
    pub fn take_workspace_query(&mut self) -> Option<String> {
        let (query, deadline) = self.pending.as_ref()?;
        if Instant::now() < *deadline {
            return None;
        }
        let query = query.clone();
        self.pending = None;
        Some(query)
    }

    /// Whether document results must be recomputed.
    pub fn needs_document_refresh(&self, query: &str, revision: u64) -> bool {
        self.scope == SearchScope::Document && (self.last_query != query || self.last_revision != Some(revision))
    }

    /// Store document results.
    pub fn set_document_results(&mut self, query: String, revision: u64, hits: Vec<SearchHit>) {
        self.last_query = query;
        self.last_revision = Some(revision);
        self.hits = hits;
        self.selected = self.selected.min(self.hits.len().saturating_sub(1));
        self.error = None;
    }

    /// Start a workspace worker.
    pub fn start_workspace(&mut self, query: String, root: PathBuf, files: Vec<PathBuf>) {
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        self.running = true;
        self.error = None;
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        let query_for_worker = query.clone();
        std::thread::Builder::new()
            .name("mdview-search".to_owned())
            .spawn(move || {
                let result = crate::search_worker::search_workspace(&root, &files, &query_for_worker);
                let _ = sender.send((generation, result));
            })
            .ok();
        log::debug!("workspace search query={query}");
        self.last_query = query;
        self.last_revision = None;
    }

    /// Poll for a completed worker result.
    pub fn poll(&mut self) {
        let Some(receiver) = &self.receiver else {
            return;
        };
        match receiver.try_recv() {
            Ok((generation, result)) => {
                if generation != self.generation {
                    return;
                }
                self.receiver = None;
                self.running = false;
                match result {
                    Ok(hits) => {
                        log::debug!("workspace search hits={}", hits.len());
                        self.hits = hits;
                        self.error = None;
                    }
                    Err(error) => {
                        self.hits.clear();
                        self.error = Some(error);
                    }
                }
                self.selected = self.selected.min(self.hits.len().saturating_sub(1));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.receiver = None;
                self.running = false;
                self.error = Some("Search worker stopped unexpectedly.".to_owned());
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
}

/// Search the active document buffer.
pub fn search_document(document: &mdview_model::Document, query: &str) -> Vec<SearchHit> {
    let query = query.trim();
    if query.len() < 2 {
        return Vec::new();
    }
    let needle = query.to_lowercase();
    let path = document.path().map(PathBuf::from).unwrap_or_default();
    let mut hits = Vec::new();
    for line_number in 0..document.line_count() {
        let line = document.line(line_number);
        let lower = line.to_lowercase();
        if let Some(byte_column) = lower.find(&needle) {
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
            if hits.len() >= MAX_DOCUMENT_HITS {
                break;
            }
        }
    }
    hits
}

const MAX_DOCUMENT_HITS: usize = 200;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_search_reports_line_and_column() {
        let document = mdview_model::Document::from_loaded(
            PathBuf::from("notes.md"),
            "one\ntwo target\nthree\n".to_owned(),
            mdview_model::Eol::Lf,
            mdview_model::Encoding::Utf8,
            false,
        );
        let hits = search_document(&document, "target");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, 1);
        assert_eq!(hits[0].column, 4);
    }

    #[test]
    fn short_document_queries_are_ignored() {
        let document = mdview_model::Document::from_loaded(
            PathBuf::from("notes.md"),
            "one\n".to_owned(),
            mdview_model::Eol::Lf,
            mdview_model::Encoding::Utf8,
            false,
        );
        assert!(search_document(&document, "o").is_empty());
    }
}
