use super::PickerOptions;
use std::ops::{Deref, DerefMut};

/// Options class for the
/// [`IStorageProvider::open_folder_picker_async`](super::IStorageProvider::open_folder_picker_async)
/// method.
#[derive(Clone, Default)]
pub struct FolderPickerOpenOptions {
    base: PickerOptions,
    allow_multiple: bool,
}

impl Deref for FolderPickerOpenOptions {
    type Target = PickerOptions;

    fn deref(&self) -> &PickerOptions {
        &self.base
    }
}

impl DerefMut for FolderPickerOpenOptions {
    fn deref_mut(&mut self) -> &mut PickerOptions {
        &mut self.base
    }
}

impl From<PickerOptions> for FolderPickerOpenOptions {
    fn from(base: PickerOptions) -> Self {
        Self { base, ..Self::default() }
    }
}

impl FolderPickerOpenOptions {
    /// Creates options with nothing set.
    pub fn new() -> Self {
        Self::default()
    }

    /// An option indicating whether the open picker allows users to select
    /// multiple folders.
    pub fn allow_multiple(&self) -> bool {
        self.allow_multiple
    }

    pub fn set_allow_multiple(&mut self, value: bool) {
        self.allow_multiple = value;
    }

    /// Sets [`allow_multiple`](Self::allow_multiple) and returns the
    /// options.
    pub fn with_allow_multiple(mut self, value: bool) -> Self {
        self.allow_multiple = value;
        self
    }
}
