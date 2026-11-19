use std::collections::VecDeque;

#[derive(Clone)]
struct Entry<T> {
    label: String,
    before: T,
    after: T,
}

/// Bounded undo and redo for immutable authored source snapshots.
#[derive(Clone)]
pub struct SourceHistory<T> {
    undo: VecDeque<Entry<T>>,
    redo: VecDeque<Entry<T>>,
    limit: usize,
}

impl<T> Default for SourceHistory<T> {
    fn default() -> Self {
        Self {
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            limit: 256,
        }
    }
}

impl<T: Clone> SourceHistory<T> {
    pub fn without_entries(&self) -> Self {
        Self {
            limit: self.limit,
            ..Self::default()
        }
    }

    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        trim(&mut self.undo, limit);
        trim(&mut self.redo, limit);
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo.back().map(|entry| entry.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo.back().map(|entry| entry.label.as_str())
    }

    pub fn commit(&mut self, label: String, before: T, after: T) {
        if self.limit > 0 {
            self.undo.push_back(Entry {
                label,
                before,
                after,
            });
            trim(&mut self.undo, self.limit);
        }
        self.redo.clear();
    }

    pub fn undo(&mut self) -> Option<(String, T)> {
        let entry = self.undo.pop_back()?;
        let result = (format!("Undo {}", entry.label), entry.before.clone());
        self.redo.push_back(entry);
        trim(&mut self.redo, self.limit);
        Some(result)
    }

    pub fn redo(&mut self) -> Option<(String, T)> {
        let entry = self.redo.pop_back()?;
        let result = (format!("Redo {}", entry.label), entry.after.clone());
        self.undo.push_back(entry);
        trim(&mut self.undo, self.limit);
        Some(result)
    }
}

fn trim<T>(history: &mut VecDeque<T>, limit: usize) {
    if history.len() > limit {
        history.drain(..history.len() - limit);
    }
}
