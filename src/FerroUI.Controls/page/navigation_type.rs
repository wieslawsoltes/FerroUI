
/// Describes the type of navigation that occurred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NavigationType {
    /// A page was pushed onto the navigation stack.
    Push = 0,

    /// The top page was popped from the navigation stack.
    Pop = 1,

    /// All pages above the root were popped.
    PopToRoot = 2,

    /// A page was inserted into the navigation stack.
    Insert = 3,

    /// A page was removed from the navigation stack.
    Remove = 4,

    /// The current page was replaced.
    Replace = 5,

    /// A page was pushed onto the modal stack.
    PushModal = 6,

    /// The top page was popped from the modal stack.
    PopModal = 7,
}
