//! Back and forward navigation for documents and internal anchors.

use std::path::PathBuf;

/// One location the user visited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavEntry {
    /// Document path.
    pub path: PathBuf,
    /// Zero-based source line.
    pub line: usize,
    /// Optional heading or footnote anchor.
    pub anchor: Option<String>,
}

/// Bounded back/forward stacks.
#[derive(Default)]
pub struct HistoryState {
    back: Vec<NavEntry>,
    forward: Vec<NavEntry>,
    /// Set while the application performs a history navigation.
    pub suppress: bool,
}

impl HistoryState {
    /// Record a location before navigating away from it.
    pub fn record(&mut self, entry: NavEntry) {
        if self.suppress {
            return;
        }
        if self.back.last() == Some(&entry) {
            return;
        }
        self.back.push(entry);
        const LIMIT: usize = 100;
        if self.back.len() > LIMIT {
            self.back.remove(0);
        }
        self.forward.clear();
    }

    /// Whether a back entry exists.
    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    /// Whether a forward entry exists.
    pub fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// Move back, storing the current location for forward navigation.
    pub fn back(&mut self, current: NavEntry) -> Option<NavEntry> {
        let entry = self.back.pop()?;
        self.forward.push(current);
        Some(entry)
    }

    /// Move forward, storing the current location for back navigation.
    pub fn forward(&mut self, current: NavEntry) -> Option<NavEntry> {
        let entry = self.forward.pop()?;
        self.back.push(current);
        Some(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn entry(name: &str, line: usize) -> NavEntry {
        NavEntry {
            path: PathBuf::from(name),
            line,
            anchor: None,
        }
    }

    #[test]
    fn back_and_forward_are_symmetric() {
        let mut history = HistoryState::default();
        history.record(entry("a.md", 1));
        history.record(entry("b.md", 2));
        let previous = history.back(entry("c.md", 3)).expect("back");
        assert_eq!(previous.path, Path::new("b.md"));
        assert!(history.can_forward());
        let next = history.forward(previous).expect("forward");
        assert_eq!(next.path, Path::new("c.md"));
        assert!(!history.can_forward());
        assert!(history.can_back());
    }
}
