/// Describes the wrap behavior of the `FlexPanel`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FlexWrap {
    /// The `FlexPanel` is single line.
    ///
    /// This is the default value.
    #[default]
    NoWrap = 0,

    /// The `FlexPanel` is multi line.
    Wrap = 1,

    /// Same as [`FlexWrap::Wrap`] but new lines are added in the opposite
    /// cross-axis direction.
    WrapReverse = 2,
}
