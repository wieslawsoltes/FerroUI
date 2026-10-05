/// Represents a segment between two Unicode sentence boundaries (UAX #29).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SentenceSegment<'a> {
    offset: usize,
    text: &'a [u16],
}

impl<'a> SentenceSegment<'a> {
    /// Initializes a new instance of the [`SentenceSegment`] struct.
    ///
    /// * `offset` - The segment offset in UTF-16 code units within the source span.
    /// * `text` - The slice of the source span that makes up this segment.
    #[inline]
    pub const fn new(offset: usize, text: &'a [u16]) -> Self {
        Self { offset, text }
    }

    /// Gets the segment start offset in UTF-16 code units within the source span.
    #[inline]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Gets the text content of this segment as a slice of the source span.
    #[inline]
    pub const fn text(&self) -> &'a [u16] {
        self.text
    }
}
