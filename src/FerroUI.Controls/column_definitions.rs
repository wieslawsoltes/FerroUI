use crate::{ColumnDefinition, DefinitionList, GridLength};
use ferroui_base::utilities::FormatError;
use ferroui_base::Ref;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

/// A collection of [`ColumnDefinition`]s.
///
/// The value is a shared handle: clones refer to the same collection, and
/// handles compare by identity.
#[derive(Clone, PartialEq, Default)]
pub struct ColumnDefinitions {
    base: DefinitionList<ColumnDefinition>,
}

impl ColumnDefinitions {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self {
            base: DefinitionList::new(),
        }
    }

    /// Creates a collection with the initial items.
    pub fn from_items(items: impl IntoIterator<Item = Ref<ColumnDefinition>>) -> Self {
        let result = Self::new();
        result.add_range(items);
        result
    }

    /// Parses a string representation of a column definitions collection.
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        let lengths = GridLength::parse_lengths(s)?;
        Ok(Self::from_items(lengths.into_iter().map(ColumnDefinition::with_width)))
    }
}

impl Deref for ColumnDefinitions {
    type Target = DefinitionList<ColumnDefinition>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl fmt::Display for ColumnDefinitions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, definition) in self.snapshot().iter().enumerate() {
            if index > 0 {
                f.write_str(",")?;
            }
            definition.width().fmt(f)?;
        }
        Ok(())
    }
}

impl FromStr for ColumnDefinitions {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
