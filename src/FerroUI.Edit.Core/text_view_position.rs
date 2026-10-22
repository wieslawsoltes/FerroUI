use crate::TextLocation;
use std::{cmp::Ordering, fmt};

/// A logical UTF-16 location with an optional zero-based visual column.
///
/// At a soft line break, `is_at_end_of_line` distinguishes the trailing edge
/// of the previous visual row from the leading edge of the next one. `None`
/// means that the formatter has not resolved a visual column yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextViewPosition {
    pub line: usize,
    pub column: usize,
    pub visual_column: Option<usize>,
    pub is_at_end_of_line: bool,
}

impl TextViewPosition {
    pub const fn new(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            visual_column: None,
            is_at_end_of_line: false,
        }
    }

    pub const fn from_location(location: TextLocation) -> Self {
        Self::new(location.line, location.column)
    }

    pub const fn with_visual_column(location: TextLocation, visual_column: usize) -> Self {
        Self {
            visual_column: Some(visual_column),
            ..Self::from_location(location)
        }
    }

    pub const fn location(self) -> TextLocation {
        TextLocation::new(self.line, self.column)
    }

    pub fn set_location(&mut self, location: TextLocation) {
        self.line = location.line;
        self.column = location.column;
    }
}

impl Ord for TextViewPosition {
    fn cmp(&self, other: &Self) -> Ordering {
        self.location()
            .cmp(&other.location())
            .then_with(|| self.visual_column.cmp(&other.visual_column))
            .then_with(|| other.is_at_end_of_line.cmp(&self.is_at_end_of_line))
    }
}

impl PartialOrd for TextViewPosition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for TextViewPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[TextViewPosition Line={} Column={} VisualColumn={} IsAtEndOfLine={}]",
            self.line,
            self.column,
            self.visual_column
                .map_or_else(|| "-1".to_owned(), |v| v.to_string()),
            self.is_at_end_of_line
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_break_ordering_and_location_updates_preserve_visual_affinity() {
        let mut previous = TextViewPosition::with_visual_column(TextLocation::new(2, 4), 6);
        let next = previous;
        previous.is_at_end_of_line = true;
        assert!(previous < next);
        assert!(TextViewPosition::new(2, 4) < previous);
        previous.set_location(TextLocation::new(3, 5));
        assert_eq!(previous.visual_column, Some(6));
        assert!(previous.is_at_end_of_line);
        assert_eq!(previous.location(), TextLocation::new(3, 5));
    }
}
