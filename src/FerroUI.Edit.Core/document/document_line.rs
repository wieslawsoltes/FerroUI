use super::ISegment;

/// Immutable line information at the revision where it was queried. The
/// delimiter is excluded from `length` and included in `total_length()`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DocumentLine {
    pub line_number: usize,
    pub offset: usize,
    pub length: usize,
    pub delimiter_length: usize,
}

impl DocumentLine {
    pub const fn line_number(self) -> usize {
        self.line_number
    }
    pub const fn offset(self) -> usize {
        self.offset
    }
    pub const fn length(self) -> usize {
        self.length
    }
    pub const fn delimiter_length(self) -> usize {
        self.delimiter_length
    }
    pub const fn total_length(self) -> usize {
        self.length + self.delimiter_length
    }
    pub const fn end_offset(self) -> usize {
        self.offset + self.length
    }
}

impl ISegment for DocumentLine {
    fn offset(&self) -> usize {
        self.offset
    }
    fn length(&self) -> usize {
        self.length
    }
}
