use std::fmt;
use std::str::FromStr;

/// A one-based line and UTF-16 column. `(0, 0)` denotes an empty location.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextLocation {
    pub line: usize,
    pub column: usize,
}

impl TextLocation {
    pub const EMPTY: Self = Self::new(0, 0);
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
    pub const fn is_empty(self) -> bool {
        self.line == 0 && self.column == 0
    }
}

impl fmt::Display for TextLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(Line {}, Col {})", self.line, self.column)
    }
}

impl FromStr for TextLocation {
    type Err = super::DocumentError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split([';', ',']);
        let line = parts
            .next()
            .and_then(|p| p.trim().parse().ok())
            .ok_or(super::DocumentError::InvalidLocation)?;
        let column = parts
            .next()
            .and_then(|p| p.trim().parse().ok())
            .ok_or(super::DocumentError::InvalidLocation)?;
        if parts.next().is_some() {
            return Err(super::DocumentError::InvalidLocation);
        }
        Ok(Self::new(line, column))
    }
}

/// A UTF-16 offset/length pair. A zero-length segment contains its insertion
/// position; otherwise ranges have an exclusive end.
pub trait ISegment {
    fn offset(&self) -> usize;
    fn length(&self) -> usize;
    fn end_offset(&self) -> usize {
        self.offset().saturating_add(self.length())
    }
    fn contains(&self, offset: usize, length: usize) -> bool {
        offset >= self.offset()
            && offset
                .checked_add(length)
                .is_some_and(|end| end <= self.end_offset())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SimpleSegment {
    pub offset: usize,
    pub length: usize,
}

impl SimpleSegment {
    pub const fn new(offset: usize, length: usize) -> Self {
        Self { offset, length }
    }
    pub const fn end_offset(self) -> usize {
        self.offset.saturating_add(self.length)
    }
    pub fn contains(self, offset: usize, length: usize) -> bool {
        ISegment::contains(&self, offset, length)
    }
    pub fn overlap(self, other: Self) -> Option<Self> {
        let start = self.offset.max(other.offset);
        let end = self.end_offset().min(other.end_offset());
        (end >= start).then(|| Self::new(start, end - start))
    }
}

impl ISegment for SimpleSegment {
    fn offset(&self) -> usize {
        self.offset
    }
    fn length(&self) -> usize {
        self.length
    }
}
