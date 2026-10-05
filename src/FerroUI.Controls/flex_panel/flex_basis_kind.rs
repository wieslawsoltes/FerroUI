/// Determines how a `FlexBasis` affects the size of the flex item.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FlexBasisKind {
    /// Uses the measured width and height of the `FlexPanel` to determine the
    /// initial size of the item.
    #[default]
    Auto = 0,

    /// The initial size of the item is set to the `FlexBasis` value.
    Absolute = 1,

    /// Indicates the `FlexBasis` value is a percentage, and the size of the
    /// flex item is scaled by it.
    Relative = 2,
}
