//! Grouped undo and redo history.

use std::time::{Duration, Instant};

use ropey::Rope;

/// How an edit is grouped with the preceding edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    /// Typed characters.
    Typing,
    /// A deletion.
    Delete,
    /// A replacement or a structural edit.
    Other,
}

#[derive(Clone, Debug)]
struct Edit {
    start: usize,
    removed: String,
    inserted: String,
    kind: EditKind,
}

/// Undo history for a rope.
#[derive(Debug)]
pub struct UndoStack {
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    last_at: Option<Instant>,
    limit: usize,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(10_000)
    }
}

impl UndoStack {
    /// Build a stack with at most `limit` edits.
    pub fn new(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            last_at: None,
            limit: limit.max(1),
        }
    }

    /// Record an insertion of `inserted` at `start`.
    pub fn inserted(&mut self, start: usize, inserted: &str, kind: EditKind) {
        self.push(Edit {
            start,
            removed: String::new(),
            inserted: inserted.to_owned(),
            kind,
        });
    }

    /// Record a deletion of `removed` at `start`.
    pub fn deleted(&mut self, start: usize, removed: &str) {
        self.push(Edit {
            start,
            removed: removed.to_owned(),
            inserted: String::new(),
            kind: EditKind::Delete,
        });
    }

    /// Record a replacement.
    pub fn replaced(&mut self, start: usize, removed: &str, inserted: &str) {
        self.push(Edit {
            start,
            removed: removed.to_owned(),
            inserted: inserted.to_owned(),
            kind: EditKind::Other,
        });
    }

    /// Forget the history (used when a different file is loaded).
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.last_at = None;
    }

    /// End the current typing group so the next edit starts a new undo step.
    pub fn clear_group(&mut self) {
        self.last_at = None;
    }

    /// Whether there is an edit to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether there is an edit to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Undo one edit in `rope`.
    pub fn undo(&mut self, rope: &mut Rope) -> bool {
        let Some(edit) = self.undo.pop() else {
            return false;
        };
        apply_remove(rope, edit.start, edit.inserted.chars().count());
        rope.insert(edit.start, &edit.removed);
        self.redo.push(edit);
        self.last_at = None;
        true
    }

    /// Redo one edit in `rope`.
    pub fn redo(&mut self, rope: &mut Rope) -> bool {
        let Some(edit) = self.redo.pop() else {
            return false;
        };
        apply_remove(rope, edit.start, edit.removed.chars().count());
        rope.insert(edit.start, &edit.inserted);
        self.undo.push(edit);
        self.last_at = None;
        true
    }

    fn push(&mut self, edit: Edit) {
        let now = Instant::now();
        let groupable = self
            .last_at
            .is_some_and(|last| now.duration_since(last) < Duration::from_millis(750))
            && edit.kind != EditKind::Other
            && self.undo.last().is_some_and(|previous| {
                previous.kind == edit.kind
                    && edit.removed.is_empty()
                    && edit.inserted.chars().count() == 1
                    && edit.start == previous.start + previous.inserted.chars().count()
            });
        if groupable && let Some(previous) = self.undo.last_mut() {
            previous.inserted.push_str(&edit.inserted);
            self.last_at = Some(now);
            self.redo.clear();
            return;
        }
        self.undo.push(edit);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.last_at = Some(now);
    }
}

fn apply_remove(rope: &mut Rope, start: usize, count: usize) {
    if count == 0 {
        return;
    }
    let start = start.min(rope.len_chars());
    let end = (start + count).min(rope.len_chars());
    if start < end {
        rope.remove(start..end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_and_redo_round_trip() {
        let mut rope = Rope::from_str("hello");
        let mut undo = UndoStack::default();
        undo.inserted(5, "!", EditKind::Typing);
        rope.insert(5, "!");
        assert!(undo.undo(&mut rope));
        assert_eq!(rope.to_string(), "hello");
        assert!(undo.redo(&mut rope));
        assert_eq!(rope.to_string(), "hello!");
    }

    #[test]
    fn deletion_round_trip() {
        let mut rope = Rope::from_str("abcdef");
        let mut undo = UndoStack::default();
        undo.deleted(2, "cd");
        rope.remove(2..4);
        assert!(undo.undo(&mut rope));
        assert_eq!(rope.to_string(), "abcdef");
        assert!(undo.redo(&mut rope));
        assert_eq!(rope.to_string(), "abef");
    }
}
