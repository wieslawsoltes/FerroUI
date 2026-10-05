/// Describes the main-axis alignment of items inside a `FlexPanel` line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FlexJustifyContent {
    /// Child items are packed toward the start of the line.
    ///
    /// This is the default value.
    #[default]
    FlexStart = 0,

    /// Child items are packed toward the end of the line.
    FlexEnd = 1,

    /// Child items are packed toward the center of the line.
    ///
    /// If the leftover free-space is negative, the child items will overflow
    /// equally in both directions.
    Center = 2,

    /// Child items are evenly distributed in the line, with no space on
    /// either end.
    ///
    /// If the leftover free-space is negative or there is only a single child
    /// item on the line, this value is identical to
    /// [`FlexJustifyContent::FlexStart`].
    SpaceBetween = 3,

    /// Child items are evenly distributed in the line, with half-size spaces
    /// on either end.
    ///
    /// If the leftover free-space is negative or there is only a single child
    /// item on the line, this value is identical to
    /// [`FlexJustifyContent::Center`].
    SpaceAround = 4,

    /// Child items are evenly distributed in the line, with equal-size spaces
    /// between each item and on either end.
    SpaceEvenly = 5,
}
