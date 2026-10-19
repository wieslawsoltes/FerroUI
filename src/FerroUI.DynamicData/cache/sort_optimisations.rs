//! `SortOptimisations` (`Cache/SortOptimisations.cs`).

use std::fmt;
use std::ops::{BitAnd, BitOr, BitOrAssign};

/// The optimisations of a sort of a cache (`[Flags] enum SortOptimisations`): a set of flags.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SortOptimisations(u32);

impl SortOptimisations {
    /// No optimisation (`None`).
    pub const NONE: Self = Self(0);

    /// The comparer reads values that never change, so the current position of an item is found
    /// with a binary search (`ComparesImmutableValuesOnly`). The comparer must also give each item
    /// a unique position.
    pub const COMPARES_IMMUTABLE_VALUES_ONLY: Self = Self(1);

    /// A refresh does not move items: the collection is sorted again without moves
    /// (`IgnoreEvaluates`).
    pub const IGNORE_EVALUATES: Self = Self(2);

    /// No longer used upstream; the value of both other flags (`InsertAtEndThenSort`, obsolete).
    #[deprecated(note = "This is no longer being used. Use one of the other options instead.")]
    pub const INSERT_AT_END_THEN_SORT: Self = Self(3);

    /// The flags as a number.
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Whether every flag of `flag` is set (`Enum.HasFlag`).
    pub const fn has_flag(self, flag: Self) -> bool {
        self.0 & flag.0 == flag.0
    }
}

impl BitOr for SortOptimisations {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for SortOptimisations {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for SortOptimisations {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl fmt::Debug for SortOptimisations {
    /// The names of the flags, as `Enum.ToString` of a flags enum writes them.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            0 => f.write_str("None"),
            1 => f.write_str("ComparesImmutableValuesOnly"),
            2 => f.write_str("IgnoreEvaluates"),
            3 => f.write_str("InsertAtEndThenSort"),
            other => write!(f, "{other}"),
        }
    }
}
