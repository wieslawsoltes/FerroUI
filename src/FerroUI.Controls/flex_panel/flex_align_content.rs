/// Defines the alignment mode of the lines inside a `FlexPanel` along the
/// cross-axis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FlexAlignContent {
    /// Lines are packed toward the start of the container.
    #[default]
    FlexStart = 0,

    /// Lines are packed toward the end of the container.
    FlexEnd = 1,

    /// Lines are packed toward the center of the container.
    Center = 2,

    /// Lines are stretched to take up the remaining space.
    ///
    /// This is the default value of `FlexPanel::align_content`.
    Stretch = 3,

    /// Lines are evenly distributed in the container, with no space on either
    /// end.
    SpaceBetween = 4,

    /// Lines are evenly distributed in the container, with half-size spaces
    /// on either end.
    SpaceAround = 5,

    /// Lines are evenly distributed in the container, with equal-size spaces
    /// between each line and on either end.
    SpaceEvenly = 6,
}
