//! `ReaderWriter<T>` (`List/Internal/ReaderWriter.cs`): the items of a source list and the edits
//! that change them.

use super::ListEditor;
use crate::list::{clone_changes, ChangeAwareList, ChangeSet, IExtendedList};
use std::cell::{Cell, RefCell};

pub(crate) struct ReaderWriter<T> {
    data: RefCell<ChangeAwareList<T>>,
    update_in_progress: Cell<bool>,
}

impl<T: Clone + PartialEq + 'static> ReaderWriter<T> {
    pub(crate) fn new() -> Self {
        Self { data: RefCell::new(ChangeAwareList::new()), update_in_progress: Cell::new(false) }
    }

    pub(crate) fn count(&self) -> usize {
        self.data.borrow().count()
    }

    pub(crate) fn items(&self) -> Vec<T> {
        self.data.borrow().items().to_vec()
    }

    /// Applies a change set (`Write(IChangeSet<T>)`).
    pub(crate) fn write_changes(&self, changes: &ChangeSet<T>) -> ChangeSet<T> {
        let mut data = self.data.borrow_mut();
        clone_changes(&mut *data, changes);
        data.capture_changes()
    }

    /// Runs an edit (`Write(Action<IExtendedList<T>>)`).
    pub(crate) fn write(&self, update: &mut dyn FnMut(&mut dyn IExtendedList<T>)) -> ChangeSet<T> {
        self.update_in_progress.set(true);
        struct Done<'a>(&'a Cell<bool>);
        impl Drop for Done<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        {
            let _done = Done(&self.update_in_progress);
            update(&mut ListEditor::new(&self.data));
        }
        self.data.borrow_mut().capture_changes()
    }

    /// Runs an edit inside the edit in progress (`WriteNested`).
    ///
    /// # Panics
    /// When no edit is in progress (`InvalidOperationException`).
    pub(crate) fn write_nested(&self, update: &mut dyn FnMut(&mut dyn IExtendedList<T>)) {
        assert!(self.update_in_progress.get(), "WriteNested can only be used if another write is already in progress.");
        update(&mut ListEditor::new(&self.data));
    }

    /// Runs an edit, and the preview of its changes while the items are as they were
    /// (`WriteWithPreview`).
    pub(crate) fn write_with_preview(&self, update: &mut dyn FnMut(&mut dyn IExtendedList<T>), preview: &dyn Fn(&ChangeSet<T>)) -> ChangeSet<T> {
        let copy = ChangeAwareList::from_list(&self.data.borrow(), false);
        let result = self.write(update);
        let new_data = std::mem::replace(&mut *self.data.borrow_mut(), copy);
        preview(&result);
        *self.data.borrow_mut() = new_data;
        result
    }
}
