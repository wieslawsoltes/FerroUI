use crate::utilities::DateTimeOffset;

/// Provides access to the content-related properties of an item (like a
/// file or folder).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StorageItemProperties {
    size: Option<u64>,
    date_created: Option<DateTimeOffset>,
    date_modified: Option<DateTimeOffset>,
}

impl StorageItemProperties {
    /// Creates the properties of an item. A value the platform cannot
    /// provide is `None`.
    pub fn new(size: Option<u64>, date_created: Option<DateTimeOffset>, date_modified: Option<DateTimeOffset>) -> Self {
        Self { size, date_created, date_modified }
    }

    /// The size of the file in bytes.
    ///
    /// Can be `None` if the property is not available.
    pub fn size(&self) -> Option<u64> {
        self.size
    }

    /// The date and time that the current folder was created.
    ///
    /// Can be `None` if the property is not available.
    pub fn date_created(&self) -> Option<DateTimeOffset> {
        self.date_created
    }

    /// The date and time of the last time the file was modified.
    ///
    /// Can be `None` if the property is not available.
    pub fn date_modified(&self) -> Option<DateTimeOffset> {
        self.date_modified
    }
}
