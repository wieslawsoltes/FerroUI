/// The arguments of the event raised when a change to the source collection
/// shifts the indexes of selected items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectionModelIndexesChangedEventArgs {
    start_index: i32,
    delta: i32,
}

impl SelectionModelIndexesChangedEventArgs {
    pub fn new(start_index: i32, delta: i32) -> Self {
        Self { start_index, delta }
    }

    /// The first index that was shifted.
    pub fn start_index(&self) -> i32 {
        self.start_index
    }

    /// If positive, the number of items inserted; if negative, the number
    /// of items removed.
    pub fn delta(&self) -> i32 {
        self.delta
    }
}
