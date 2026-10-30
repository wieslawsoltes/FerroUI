//! Port of `StorageProvider/Core/FolderPickerEventArgs.cs`.

use crate::StorageFolderRef;
use std::cell::Cell;

/// Event args raised after a folder pick operation completes.
#[derive(Debug)]
pub struct FolderPickerEventArgs {
    folders: Vec<StorageFolderRef>,
    handled: Cell<bool>,
}

impl FolderPickerEventArgs {
    /// Initializes a new instance with the collection of folders returned by the picker.
    pub fn new(folders: Vec<StorageFolderRef>) -> Self {
        Self { folders, handled: Cell::new(false) }
    }

    /// Gets the collection of folders returned by the picker.
    pub fn folders(&self) -> &[StorageFolderRef] {
        &self.folders
    }

    /// Gets or sets a value indicating whether the pick operation has been handled.
    pub fn handled(&self) -> bool {
        self.handled.get()
    }

    pub fn set_handled(&self, value: bool) {
        self.handled.set(value);
    }
}
