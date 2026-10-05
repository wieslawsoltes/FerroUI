/// Enum specifying where a box should be positioned vertically.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BaselineAlignment {
    /// Align top toward the top of the container.
    Top,
    /// Center vertically.
    Center,
    /// Align bottom toward the bottom of the container.
    Bottom,
    /// Align at the baseline.
    #[default]
    Baseline,
    /// Align toward the text's top of the container.
    TextTop,
    /// Align toward the text's bottom of the container.
    TextBottom,
    /// Align the baseline to the subscript position of the container.
    Subscript,
    /// Align the baseline to the superscript position of the container.
    Superscript,
}
