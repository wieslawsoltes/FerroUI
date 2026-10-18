//! `Primitives/TreeDataGridRowUnrealizeReason.cs`.

/// Describes why a realized [`TreeDataGridRow`](super::TreeDataGridRow) is being unrealized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TreeDataGridRowUnrealizeReason {
    /// The row is leaving the realized viewport and may be recycled for another row.
    Recycle,
    /// The row's item was removed from the source.
    ItemRemoved,
}

impl std::fmt::Display for TreeDataGridRowUnrealizeReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TreeDataGridRowUnrealizeReason::Recycle => "Recycle",
            TreeDataGridRowUnrealizeReason::ItemRemoved => "ItemRemoved",
        })
    }
}
