//! `Avg<TValue>` (`Aggregation/Avg.cs`).

/// The state of a running average: the count of the values and their sum (`Avg<TValue>`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Avg<V> {
    count: i32,
    sum: V,
}

impl<V: Copy> Avg<V> {
    pub(crate) fn new(count: i32, sum: V) -> Self {
        Self { count, sum }
    }

    /// The count of the values (`Count`).
    pub(crate) fn count(&self) -> i32 {
        self.count
    }

    /// The sum of the values (`Sum`).
    pub(crate) fn sum(&self) -> V {
        self.sum
    }
}
