use crate::{Ref, StyledElement};

/// Describes the action that caused a child index changed notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChildIndexChangedAction {
    /// The index of a single child changed.
    ChildIndexChanged,
    /// The index of multiple children changed and all children should be
    /// re-evaluated.
    ChildIndexesReset,
    /// The total count of children changed.
    TotalCountChanged,
}

/// Event args for the child index changed notification of a child index
/// provider.
#[derive(Clone)]
pub struct ChildIndexChangedEventArgs {
    action: ChildIndexChangedAction,
    child: Option<Ref<StyledElement>>,
    index: i32,
}

impl ChildIndexChangedEventArgs {
    /// Creates the arguments for a change of the index of a single child.
    pub fn new(child: Ref<StyledElement>, index: i32) -> Self {
        Self { action: ChildIndexChangedAction::ChildIndexChanged, child: Some(child), index }
    }

    /// The arguments for a reset of all child indexes.
    pub fn child_indexes_reset() -> Self {
        Self { action: ChildIndexChangedAction::ChildIndexesReset, child: None, index: -1 }
    }

    /// The arguments for a change of the total number of children.
    pub fn total_count_changed() -> Self {
        Self { action: ChildIndexChangedAction::TotalCountChanged, child: None, index: -1 }
    }

    /// The type of change that occurred.
    pub fn action(&self) -> ChildIndexChangedAction {
        self.action
    }

    /// The logical child whose index was changed, or `None` if all children
    /// should be re-evaluated.
    pub fn child(&self) -> Option<&Ref<StyledElement>> {
        self.child.as_ref()
    }

    /// The new index of the child, or -1 if all children should be
    /// re-evaluated.
    pub fn index(&self) -> i32 {
        self.index
    }
}
