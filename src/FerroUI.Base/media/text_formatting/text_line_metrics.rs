/// Represents a metric for a `TextLine` object that is used to tell where
/// the line sits and how big it is.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextLineMetrics {
    /// Gets a value that indicates whether content of the line overflows the specified paragraph width.
    pub has_overflowed: bool,
    /// Gets the height of a line of text.
    pub height: f64,
    /// Gets the number of newline characters at the end of a line.
    pub newline_length: i32,
    /// Gets the distance from the start of a paragraph to the starting point of a line.
    pub start: f64,
    /// Gets the distance from the top to the baseline of the line of text.
    pub text_baseline: f64,
    /// Gets the number of whitespace characters at the end of a line.
    pub trailing_whitespace_length: i32,
    /// Gets the width of a line of text, excluding trailing whitespace characters.
    pub width: f64,
    /// Gets the width of a line of text, including trailing whitespace characters.
    pub width_including_trailing_whitespace: f64,
    /// Gets the distance from the top-most to bottom-most black pixel in a line.
    pub extent: f64,
    /// Gets the distance that black pixels extend beyond the bottom alignment edge of a line.
    pub overhang_after: f64,
    /// Gets the distance that black pixels extend prior to the left leading alignment edge of the line.
    pub overhang_leading: f64,
    /// Gets the distance that black pixels extend following the right trailing alignment edge of the line.
    pub overhang_trailing: f64,
}
