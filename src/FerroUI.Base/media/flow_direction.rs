/// Describes the flow direction of content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FlowDirection {
    /// Text and other elements flow from left to right.
    #[default]
    LeftToRight,
    /// Text and other elements flow from right to left.
    RightToLeft,
}
