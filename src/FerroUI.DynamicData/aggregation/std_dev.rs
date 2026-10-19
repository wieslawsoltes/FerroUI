//! `StdDev<TValue>` (`Aggregation/StdDev.cs`).

/// The state of a running standard deviation: the count of the values, their sum and the sum of
/// their squares (`StdDev<TValue>`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StdDev<V> {
    count: i32,
    sum_of_items: V,
    sum_of_squares: V,
}

impl<V: Copy> StdDev<V> {
    pub(crate) fn new(count: i32, sum_of_items: V, sum_of_squares: V) -> Self {
        Self { count, sum_of_items, sum_of_squares }
    }

    /// The count of the values (`Count`).
    pub(crate) fn count(&self) -> i32 {
        self.count
    }

    /// The sum of the values (`SumOfItems`).
    pub(crate) fn sum_of_items(&self) -> V {
        self.sum_of_items
    }

    /// The sum of the squares of the values (`SumOfSquares`).
    pub(crate) fn sum_of_squares(&self) -> V {
        self.sum_of_squares
    }
}
