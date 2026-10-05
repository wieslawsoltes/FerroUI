/// Specifies the type of `ScrollBar` scroll event that occurred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollEventType {
    /// The thumb moved a distance determined by the value of `SmallChange`:
    /// to the left for a horizontal scroll bar or upward for a vertical one.
    SmallDecrement = 0,
    /// The thumb moved a distance determined by the value of `SmallChange`:
    /// to the right for a horizontal scroll bar or downward for a vertical
    /// one.
    SmallIncrement = 1,
    /// The thumb moved a distance determined by the value of `LargeChange`:
    /// to the left for a horizontal scroll bar or upward for a vertical one.
    LargeDecrement = 2,
    /// The thumb moved a distance determined by the value of `LargeChange`:
    /// to the right for a horizontal scroll bar or downward for a vertical
    /// one.
    LargeIncrement = 3,
    /// The thumb was dragged and caused a pointer move event. A scroll event
    /// of this type may occur more than one time when the thumb is dragged
    /// in the scroll bar.
    ThumbTrack = 4,
    /// The thumb was dragged to a new position and is now no longer being
    /// dragged by the user.
    EndScroll = 5,
}
