/// Describes the orientation and direction along which items are placed
/// inside the `FlexPanel`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FlexDirection {
    /// Items are placed along the horizontal axis, starting from the left.
    ///
    /// This is the default value.
    #[default]
    Row = 0,

    /// Items are placed along the horizontal axis, starting from the right.
    RowReverse = 1,

    /// Items are placed along the vertical axis, starting from the top.
    Column = 2,

    /// Items are placed along the vertical axis, starting from the bottom.
    ColumnReverse = 3,
}
