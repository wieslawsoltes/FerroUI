/// Defines vertical or horizontal orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Orientation {
    /// Horizontal orientation.
    Horizontal = 0,
    /// Vertical orientation.
    Vertical = 1,
}
