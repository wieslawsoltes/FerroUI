//! Port of `StorageProvider/Core/SaveFilePickerEventArgs.cs`.

use crate::StorageFileRef;
use std::cell::Cell;

/// Event args raised after a save file pick operation completes.
#[derive(Debug)]
pub struct SaveFilePickerEventArgs {
    file: StorageFileRef,
    handled: Cell<bool>,
}

impl SaveFilePickerEventArgs {
    /// Initializes a new instance with the file returned by the picker.
    pub fn new(file: StorageFileRef) -> Self {
        Self { file, handled: Cell::new(false) }
    }

    /// Gets the file returned by the picker.
    pub fn file(&self) -> &StorageFileRef {
        &self.file
    }

    /// Gets or sets a value indicating whether the pick operation has been handled.
    pub fn handled(&self) -> bool {
        self.handled.get()
    }

    pub fn set_handled(&self, value: bool) {
        self.handled.set(value);
    }
}
