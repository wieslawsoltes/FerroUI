//! `CombineOperator` (`Cache/Internal/CombineOperator.cs`): the logical operators of the
//! combiners of caches and lists.

/// How the items of several collections are combined (`CombineOperator`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CombineOperator {
    /// The items that are in every collection (`And`).
    And,
    /// The items that are in any collection (`Or`).
    Or,
    /// The items that are in exactly one collection (`Xor`).
    Xor,
    /// The items of the first collection that are in none of the others (`Except`).
    Except,
}
