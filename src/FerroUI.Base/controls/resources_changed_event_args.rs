use std::cell::Cell;

/// Describes a change to the resources visible to a resource host.
///
/// Each notification carries a new sequence number, which lets a host that is
/// reached through more than one path ignore a notification it has already
/// handled.
///
/// A notification is made after the change it describes: a resource looked up
/// after [`create`](Self::create) made the arguments of a change sees that
/// change. A dynamic resource remembers the
/// [`last_sequence_number`](Self::last_sequence_number) when it looks its
/// resource up and does not look it up again for a notification that is not
/// [newer](Self::is_newer_than).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ResourcesChangedEventArgs {
    pub sequence_number: i32,
}

crate::ferro_thread_local! {
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

    /// The sequence number of the last arguments [`create`](Self::create)
    /// made on this thread; 0 before the first.
    pub fn last_sequence_number() -> i32 {
        LAST_SEQUENCE_NUMBER.get()
    }

    /// Whether these arguments describe a change made after the moment at
    /// which `sequence_number` was the
    /// [`last_sequence_number`](Self::last_sequence_number). Arguments that
    /// [`create`](Self::create) did not number (the default, sequence number
    /// 0) are newer than any moment.
    pub fn is_newer_than(&self, sequence_number: i32) -> bool {
        self.sequence_number == 0 || self.sequence_number.wrapping_sub(sequence_number) > 0
    }
}
