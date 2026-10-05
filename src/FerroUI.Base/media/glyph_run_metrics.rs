/// Describes the metrics of a glyph run.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GlyphRunMetrics {
    /// The baseline of the run.
    pub baseline: f64,
    /// The width of the run without trailing whitespace.
    pub width: f64,
    /// The width of the run including trailing whitespace.
    pub width_including_trailing_whitespace: f64,
    /// The height of the run.
    pub height: f64,
    /// The number of trailing whitespace characters (UTF-16 code units).
    pub trailing_whitespace_length: i32,
    /// The number of trailing line break characters (UTF-16 code units).
    pub new_line_length: i32,
    /// The first cluster of the run.
    pub first_cluster: i32,
    /// The last cluster of the run.
    pub last_cluster: i32,
}
