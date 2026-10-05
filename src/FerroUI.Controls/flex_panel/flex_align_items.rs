/// Defines the alignment mode along the cross-axis of `FlexPanel` child
/// items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FlexAlignItems {
    /// Items are aligned to the cross-axis start margin edge of the line.
    #[default]
    FlexStart = 0,

    /// Items are aligned to the cross-axis end margin edge of the line.
    FlexEnd = 1,

    /// Items are aligned to the cross-axis center of the line.
    ///
    /// If the cross size of the line is less than that of the child item, it
    /// will overflow equally in both directions.
    Center = 2,

    /// Items are stretched to fill the cross size of the line.
    ///
    /// This is the default value of `FlexPanel::align_items`.
    Stretch = 3,
}
