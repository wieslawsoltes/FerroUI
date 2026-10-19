//! `ChangeReason` (`Cache/ChangeReason.cs`).

/// The reason of a change of a cache (`ChangeReason`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChangeReason {
    /// An item was added (`Add`).
    Add = 0,
    /// An item replaced the item of its key (`Update`).
    Update = 1,
    /// An item was removed (`Remove`).
    Remove = 2,
    /// An item asks to be evaluated again by the operators downstream: filters, sorts, groups
    /// (`Refresh`).
    Refresh = 3,
    /// An item moved within a sorted collection (`Moved`).
    Moved = 4,
}
