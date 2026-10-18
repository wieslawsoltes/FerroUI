//! `Presentation/CellModelPool.cs`: the cells of a presentation kept for reuse.
//!
//! One pool per presentation, keyed by column identity: bindings and options must never cross
//! columns or views. Both retained models and empty bucket metadata are bounded.
//!
//! The pool only stores cells; suspending, retargeting and disposing them call into the cells and
//! are done by [`TreeDataGridRows`](super::TreeDataGridRows) without the pool borrowed
//! (`realize_cell` and `try_recycle_cell` there).

use crate::models::{CellRef, ColumnRef};
use std::collections::VecDeque;
use std::rc::Rc;

fn column_key(column: &ColumnRef) -> *const () {
    Rc::as_ptr(column) as *const ()
}

struct Bucket {
    column: ColumnRef,
    cells: Vec<CellRef>,
}

/// The cells of the columns of a presentation suspended for reuse (`CellModelPool<TModel>`).
#[derive(Default)]
pub struct CellModelPool {
    // The buckets from the least to the most recently used.
    recent: VecDeque<Bucket>,
    count: usize,
}

impl CellModelPool {
    /// The most cells the pool keeps (`MaximumCells`).
    pub const MAXIMUM_CELLS: usize = 256;
    const MAXIMUM_COLUMNS: usize = 32;

    pub fn new() -> Self {
        Self::default()
    }

    fn position(&self, column: &ColumnRef) -> Option<usize> {
        let key = column_key(column);
        self.recent.iter().position(|bucket| column_key(&bucket.column) == key)
    }

    // Makes the bucket at `index` the most recently used one; returns its new index.
    fn touch(&mut self, index: usize) -> usize {
        let last = self.recent.len() - 1;
        if index != last {
            let bucket = self.recent.remove(index).expect("a bucket of the pool");
            self.recent.push_back(bucket);
        }
        last
    }

    // Removes the least recently used bucket and returns its cells.
    fn remove_first(&mut self) -> Vec<CellRef> {
        match self.recent.pop_front() {
            Some(bucket) => {
                self.count -= bucket.cells.len();
                bucket.cells
            }
            None => Vec::new(),
        }
    }

    /// Whether the pool has a bucket for `column` (`_columns.TryGetValue`); a hit makes the bucket
    /// the most recently used one.
    pub fn touch_column(&mut self, column: &ColumnRef) -> bool {
        match self.position(column) {
            Some(index) => {
                self.touch(index);
                true
            }
            None => false,
        }
    }

    /// Removes the most recently pooled cell of `column` (`node.Value.Cells.Pop()`).
    pub fn pop(&mut self, column: &ColumnRef) -> Option<CellRef> {
        let index = self.position(column)?;
        let cell = self.recent[index].cells.pop()?;
        self.count -= 1;
        Some(cell)
    }

    /// Keeps a suspended `cell` for `column` (the second half of `TryAdd`); the cells evicted to
    /// keep the pool within its bounds are added to `evicted`, for the caller to dispose.
    pub fn add(&mut self, column: &ColumnRef, cell: CellRef, evicted: &mut Vec<CellRef>) {
        while self.count >= Self::MAXIMUM_CELLS {
            evicted.extend(self.remove_first());
        }

        let index = match self.position(column) {
            Some(index) => self.touch(index),
            None => {
                if self.recent.len() >= Self::MAXIMUM_COLUMNS {
                    evicted.extend(self.remove_first());
                }
                self.recent.push_back(Bucket { column: column.clone(), cells: Vec::new() });
                self.recent.len() - 1
            }
        };

        self.recent[index].cells.push(cell);
        self.count += 1;
    }

    /// Removes every cell (`Clear`); returns them for the caller to dispose.
    pub fn clear(&mut self) -> Vec<CellRef> {
        let mut cells = Vec::new();
        while !self.recent.is_empty() {
            cells.extend(self.remove_first());
        }
        cells
    }

    /// The number of cells kept.
    pub fn count(&self) -> usize {
        self.count
    }
}
