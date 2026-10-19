//! `ListChangeReason` and `ChangeType` (`List/ListChangeReason.cs`, `List/ChangeType.cs`,
//! `ChangeSetEx.GetChangeType`).

/// The reason of a change of a list (`ListChangeReason`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListChangeReason {
    /// An item was inserted (`Add`).
    Add,
    /// Items were inserted together (`AddRange`).
    AddRange,
    /// An item replaced the item at its index (`Replace`).
    Replace,
    /// An item was removed (`Remove`).
    Remove,
    /// Items were removed together (`RemoveRange`).
    RemoveRange,
    /// An item asks to be evaluated again (`Refresh`).
    Refresh,
    /// An item moved (`Moved`).
    Moved,
    /// Every item was removed (`Clear`).
    Clear,
}

/// Whether a change is of one item or of a range (`ChangeType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChangeType {
    Item,
    Range,
}

impl ListChangeReason {
    /// The kind of change of the reason (`GetChangeType`).
    pub fn change_type(self) -> ChangeType {
        match self {
            ListChangeReason::AddRange | ListChangeReason::RemoveRange | ListChangeReason::Clear => ChangeType::Range,
            _ => ChangeType::Item,
        }
    }
}
