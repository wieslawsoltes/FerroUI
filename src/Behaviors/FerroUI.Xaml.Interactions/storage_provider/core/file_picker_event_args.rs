//! Port of `StorageProvider/Core/FilePickerEventArgs.cs`.

use crate::StorageFileRef;
use std::cell::Cell;

/// Event args raised after a file pick operation completes.
#[derive(Debug)]
pub struct FilePickerEventArgs {
    files: Vec<StorageFileRef>,
    handled: Cell<bool>,
}

impl FilePickerEventArgs {
    /// Initializes a new instance with the collection of files returned by the picker.
    pub fn new(files: Vec<StorageFileRef>) -> Self {
        Self { files, handled: Cell::new(false) }
    }

    /// Gets the collection of files returned by the picker.
    pub fn files(&self) -> &[StorageFileRef] {
        &self.files
    }

    /// Gets or sets a value indicating whether the pick operation has been handled.
    pub fn handled(&self) -> bool {
        self.handled.get()
    }

    pub fn set_handled(&self, value: bool) {
        self.handled.set(value);
    }
}
