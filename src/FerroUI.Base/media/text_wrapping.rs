/// Controls the wrapping mode of text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextWrapping {
    /// Text should not wrap.
    #[default]
    NoWrap,
    /// Text can wrap.
    Wrap,
    /// Line-breaking occurs if the line overflows the available block width.
    /// However, a line may overflow the block width if the line breaking
    /// algorithm cannot determine a break opportunity, as in the case of a very
    /// long word.
    WrapWithOverflow,
}
