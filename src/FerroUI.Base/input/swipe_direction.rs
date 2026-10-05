/// Specifies the direction of a swipe gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SwipeDirection {
    /// The swipe moved to the left.
    Left,
    /// The swipe moved to the right.
    Right,
    /// The swipe moved upward.
    Up,
    /// The swipe moved downward.
    Down,
}
