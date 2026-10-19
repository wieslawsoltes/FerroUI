//! `ReactiveChangeReason` (`ReactiveUI.Core/ChangeSets/ReactiveChangeReason.cs`).

/// Why a [`ReactiveChange`](super::ReactiveChange) happened (`ReactiveChangeReason`). Ranges and
/// resets are flattened to one change per item, so this is the reason of one item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReactiveChangeReason {
    /// An item was added (`Add`).
    Add = 0,
    /// An item was removed; a reset is one removal per item it had (`Remove`).
    Remove = 1,
    /// An item replaced the item at its index (`Replace`).
    Replace = 2,
    /// An item moved from one index to another; the count does not change (`Move`).
    Move = 3,
    /// An item asked to be evaluated again, without being added or removed (`Refresh`).
    Refresh = 4,
}
