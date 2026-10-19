//! `SortExpressionComparer<T>` and `SortExpression<T>` (`Binding/SortExpressionComparer.cs`,
//! `Binding/SortExpression.cs`).

use super::SortDirection;
use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

/// One key of a sort: a value read from each item, and the direction (`SortExpression<T>`).
///
/// Deviation (DEVIATIONS.md, DynamicData): upstream holds the getter `Func<T, IComparable>`; Rust
/// has no common type of the values of different getters, so the expression is held as the
/// comparison of the values it reads. The values need `PartialOrd`; `Option` puts `None` first,
/// as upstream puts `null` first, and two values without an order (a `NaN`) compare as equal.
pub struct SortExpression<T> {
    direction: SortDirection,
    expression: Rc<dyn Fn(&T, &T) -> Ordering>,
}

impl<T> Clone for SortExpression<T> {
    fn clone(&self) -> Self {
        Self { direction: self.direction, expression: self.expression.clone() }
    }
}

impl<T> fmt::Debug for SortExpression<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SortExpression").field("direction", &self.direction).finish_non_exhaustive()
    }
}

impl<T: 'static> SortExpression<T> {
    /// The key `expression` in `direction` (`SortExpression(Func<T, IComparable>, SortDirection)`).
    pub fn new<V: PartialOrd + 'static>(expression: impl Fn(&T) -> V + 'static, direction: SortDirection) -> Self {
        Self { direction, expression: Rc::new(move |x: &T, y: &T| expression(x).partial_cmp(&expression(y)).unwrap_or(Ordering::Equal)) }
    }

    /// The key `expression`, ascending (`SortExpression(Func<T, IComparable>)`).
    pub fn ascending<V: PartialOrd + 'static>(expression: impl Fn(&T) -> V + 'static) -> Self {
        Self::new(expression, SortDirection::Ascending)
    }
}

impl<T> SortExpression<T> {
    /// The direction (`Direction`).
    pub fn direction(&self) -> SortDirection {
        self.direction
    }

    /// The comparison of the values that the expression reads from `x` and `y`, ascending
    /// (`Expression(x).CompareTo(Expression(y))`).
    pub fn compare_values(&self, x: &T, y: &T) -> Ordering {
        (self.expression)(x, y)
    }
}

/// A comparer of items by a list of keys, each ascending or descending
/// (`SortExpressionComparer<T>`): the first key that differs decides.
///
/// It builds as upstream's does, `SortExpressionComparer::ascending(|p: &Person| p.age())
/// .then_by_ascending(|p| p.name())`, and becomes the comparer that the operators take,
/// `Rc<dyn Fn(&T, &T) -> Ordering>`, with [`comparer`](Self::comparer) or `into()`.
pub struct SortExpressionComparer<T> {
    expressions: Vec<SortExpression<T>>,
}

impl<T> Clone for SortExpressionComparer<T> {
    fn clone(&self) -> Self {
        Self { expressions: self.expressions.clone() }
    }
}

impl<T> Default for SortExpressionComparer<T> {
    /// A comparer without keys, which finds every pair equal (`new SortExpressionComparer<T>()`).
    fn default() -> Self {
        Self { expressions: Vec::new() }
    }
}

impl<T> fmt::Debug for SortExpressionComparer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.expressions.iter()).finish()
    }
}

impl<T: 'static> SortExpressionComparer<T> {
    /// A comparer by `expression`, ascending (`SortExpressionComparer<T>.Ascending`).
    pub fn ascending<V: PartialOrd + 'static>(expression: impl Fn(&T) -> V + 'static) -> Self {
        Self { expressions: vec![SortExpression::new(expression, SortDirection::Ascending)] }
    }

    /// A comparer by `expression`, descending (`SortExpressionComparer<T>.Descending`).
    pub fn descending<V: PartialOrd + 'static>(expression: impl Fn(&T) -> V + 'static) -> Self {
        Self { expressions: vec![SortExpression::new(expression, SortDirection::Descending)] }
    }

    /// The comparer, then by `expression`, ascending (`ThenByAscending`).
    pub fn then_by_ascending<V: PartialOrd + 'static>(mut self, expression: impl Fn(&T) -> V + 'static) -> Self {
        self.expressions.push(SortExpression::new(expression, SortDirection::Ascending));
        self
    }

    /// The comparer, then by `expression`, descending (`ThenByDescending`).
    pub fn then_by_descending<V: PartialOrd + 'static>(mut self, expression: impl Fn(&T) -> V + 'static) -> Self {
        self.expressions.push(SortExpression::new(expression, SortDirection::Descending));
        self
    }

    /// The comparer as the function that the operators take (`IComparer<T>`).
    pub fn comparer(&self) -> Rc<dyn Fn(&T, &T) -> Ordering> {
        let comparer = self.clone();
        Rc::new(move |x: &T, y: &T| comparer.compare(x, y))
    }
}

impl<T> SortExpressionComparer<T> {
    /// Adds a key (`Add`, of the list upstream's comparer is).
    pub fn push(&mut self, expression: SortExpression<T>) {
        self.expressions.push(expression);
    }

    /// The keys, in order.
    pub fn expressions(&self) -> &[SortExpression<T>] {
        &self.expressions
    }

    /// Compares `x` and `y` by the keys in order (`Compare`).
    pub fn compare(&self, x: &T, y: &T) -> Ordering {
        for item in &self.expressions {
            let result = item.compare_values(x, y);
            if result == Ordering::Equal {
                continue;
            }
            return match item.direction() {
                SortDirection::Ascending => result,
                SortDirection::Descending => result.reverse(),
            };
        }
        Ordering::Equal
    }
}

impl<T: 'static> From<SortExpressionComparer<T>> for Rc<dyn Fn(&T, &T) -> Ordering> {
    fn from(comparer: SortExpressionComparer<T>) -> Self {
        Rc::new(move |x: &T, y: &T| comparer.compare(x, y))
    }
}
