/// Specifies a logical direction in which to perform certain text operations,
/// such as inserting, retrieving or navigating through text relative to a
/// specified position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LogicalDirection {
    /// Backward, or from right to left.
    Backward,
    /// Forward, or from left to right.
    Forward,
}
