//! `ItemChange<T>` (`List/ItemChange.cs`).

/// The change of one item of a list (`ItemChange<T>`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemChange<T> {
    reason: super::ListChangeReason,
    current: T,
    previous: Option<T>,
    current_index: Option<usize>,
    previous_index: Option<usize>,
}

impl<T> ItemChange<T> {
    /// `ItemChange(ListChangeReason, T, in Optional<T>, int, int)`.
    pub fn new(reason: super::ListChangeReason, current: T, previous: Option<T>, current_index: Option<usize>, previous_index: Option<usize>) -> Self {
        Self { reason, current, previous, current_index, previous_index }
    }

    /// The reason (`Reason`).
    pub fn reason(&self) -> super::ListChangeReason {
        self.reason
    }

    /// The item (`Current`).
    pub fn current(&self) -> &T {
        &self.current
    }

    /// The item that was replaced (`Previous`).
    pub fn previous(&self) -> Option<&T> {
        self.previous.as_ref()
    }

    /// The index of the item (`CurrentIndex`; `-1` upstream when none).
    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    /// The previous index of a moved item (`PreviousIndex`).
    pub fn previous_index(&self) -> Option<usize> {
        self.previous_index
    }

    pub(crate) fn set_current_index(&mut self, index: Option<usize>) {
        self.current_index = index;
    }
}
