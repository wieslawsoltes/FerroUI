//! What every library implements: the layout of the strings of a corpus, cold and again, with
//! what the layout produced (the lines and the height), so that a benchmark consumes a result
//! the optimiser cannot drop and the side check can compare the libraries.

use crate::corpus::sweep_width;
use crate::LibraryKind;
use std::ops::AddAssign;

/// What laying out strings produced: the lines, and the height of the text.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Totals {
    pub lines: usize,
    pub height: f64,
}

impl Totals {
    pub fn new(lines: usize, height: f64) -> Self {
        Self { lines, height }
    }
}

impl AddAssign for Totals {
    fn add_assign(&mut self, other: Totals) {
        self.lines += other.lines;
        self.height += other.height;
    }
}

/// A library laying out the strings of a corpus with Roboto Regular at 14 px.
///
/// [`TextTarget::new`] creates the state an application keeps (the font loaded into the
/// library's font database, its shared contexts); nothing of a string is laid out yet.
pub trait TextTarget: Sized + 'static {
    /// The library.
    const KIND: LibraryKind;

    /// The state of the library with the strings, before any of them is laid out.
    fn new(texts: Vec<String>) -> Self;

    /// The number of strings.
    fn len(&self) -> usize;

    /// Lays out string `index` at `width` from its text, with no cache of that string: the cold
    /// path of the library (see `README.md`, "What each library caches").
    fn cold_one(&mut self, index: usize, width: f32) -> Totals;

    /// Lays out every string cold at `width`.
    fn cold(&mut self, width: f32) -> Totals {
        let mut totals = Totals::default();
        for index in 0..self.len() {
            totals += std::hint::black_box(self.cold_one(index, width));
        }
        totals
    }

    /// Prepares every string for [`TextTarget::relayout`]: shapes it once and lays it out at the
    /// width of the cold scenario, warming every cache of the library.
    fn prepare(&mut self);

    /// Lays out every prepared string at `width`, with every cache of the library warm.
    fn relayout(&mut self, width: f32) -> Totals;
}

/// A prepared target and the step of the width sweep it lays out at next.
pub struct Sweep<T> {
    pub target: T,
    step: usize,
}

impl<T: TextTarget> Sweep<T> {
    /// The target, prepared.
    pub fn prepared(mut target: T) -> Self {
        target.prepare();
        Self { target, step: 0 }
    }

    /// Lays out every string at the next width of the sweep.
    pub fn relayout_next(&mut self) -> Totals {
        let width = sweep_width(self.step);
        self.step += 1;
        self.target.relayout(width)
    }
}
