/// Represents a segment between two Unicode word boundaries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WordSegment<'a> {
    offset: usize,
    length: usize,
    codepoint_offset: usize,
    codepoint_length: usize,
    text: &'a [u16],
}

impl<'a> WordSegment<'a> {
    /// Initializes a new instance of the [`WordSegment`] struct.
    ///
    /// * `offset` - The segment offset in UTF-16 code units.
    /// * `length` - The segment length in UTF-16 code units.
    /// * `codepoint_offset` - The segment offset in Unicode code points.
    /// * `codepoint_length` - The segment length in Unicode code points.
    ///
    /// Segments created through this constructor carry no [`WordSegment::text`] slice.
    pub const fn new(offset: usize, length: usize, codepoint_offset: usize, codepoint_length: usize) -> Self {
        Self { offset, length, codepoint_offset, codepoint_length, text: &[] }
    }

    /// Initializes a new instance of the [`WordSegment`] struct.
    ///
    /// * `offset` - The segment start offset in UTF-16 code units within the source span.
    /// * `text` - The slice of the source span that makes up this segment.
    /// * `codepoint_offset` - The segment offset in Unicode code points.
    /// * `codepoint_length` - The segment length in Unicode code points.
    pub const fn with_text(offset: usize, text: &'a [u16], codepoint_offset: usize, codepoint_length: usize) -> Self {
        Self { offset, length: text.len(), codepoint_offset, codepoint_length, text }
    }

    /// Gets the segment offset in UTF-16 code units.
    #[inline]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Gets the segment length in UTF-16 code units.
    #[inline]
    pub const fn length(&self) -> usize {
        self.length
    }

    /// Gets the segment offset in Unicode code points.
    #[inline]
    pub const fn codepoint_offset(&self) -> usize {
        self.codepoint_offset
    }

    /// Gets the segment length in Unicode code points.
    #[inline]
    pub const fn codepoint_length(&self) -> usize {
        self.codepoint_length
    }

    /// Gets the text content of this segment as a slice of the source span.
    ///
    /// Empty for segments created through the constructor that takes a code-unit length
    /// instead of a slice, whose source span is unknown. Use [`WordSegment::length`] when the
    /// segment may come from an arbitrary caller rather than from the word break enumerator.
    #[inline]
    pub const fn text(&self) -> &'a [u16] {
        self.text
    }
}
