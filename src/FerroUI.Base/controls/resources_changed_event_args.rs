use std::cell::Cell;

/// Describes a change to the resources visible to a resource host.
///
/// Each notification carries a new sequence number, which lets a host that is
/// reached through more than one path ignore a notification it has already
/// handled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ResourcesChangedEventArgs {
    pub sequence_number: i32,
}

thread_local! {
    static LAST_SEQUENCE_NUMBER: Cell<i32> = const { Cell::new(0) };
}

impl ResourcesChangedEventArgs {
    pub fn new(sequence_number: i32) -> Self {
        Self { sequence_number }
    }

    /// Creates event arguments with the next sequence number.
    pub fn create() -> Self {
        let next = LAST_SEQUENCE_NUMBER.get().wrapping_add(1);
        LAST_SEQUENCE_NUMBER.set(next);
        Self { sequence_number: next }
    }
}
