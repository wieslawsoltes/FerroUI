//! `ChangeSetMergeTracker<TObject>` (`List/Internal/ChangeSetMergeTracker.cs`): the merge of the
//! change sets of several lists into those of one.

use crate::list::{clear_or_remove_many, remove_many, replace_or_add, Change, ChangeAwareList, ChangeSet, IList, ListChangeReason};
use ferroui_system::IObserver;
use std::cell::RefCell;
use std::rc::Rc;

/// The tracker (`ChangeSetMergeTracker<TObject>`).
pub(crate) struct ChangeSetMergeTracker<T> {
    result_list: RefCell<ChangeAwareList<T>>,
}

impl<T: Clone + PartialEq + 'static> Default for ChangeSetMergeTracker<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + PartialEq + 'static> ChangeSetMergeTracker<T> {
    pub(crate) fn new() -> Self {
        Self { result_list: RefCell::new(ChangeAwareList::new()) }
    }

    /// Applies the change set of one of the lists (`ProcessChangeSet`); moves are ignored, as the
    /// indexes of the merged list are not those of the lists.
    pub(crate) fn process_change_set(&self, changes: &ChangeSet<T>, observer: Option<&Rc<dyn IObserver<ChangeSet<T>>>>) {
        {
            let mut result_list = self.result_list.borrow_mut();
            for change in changes {
                match change.reason() {
                    ListChangeReason::Add => result_list.add(change.item().current().clone()),
                    ListChangeReason::Remove => {
                        result_list.remove(change.item().current());
                    }
                    ListChangeReason::Replace => {
                        let item = change.item();
                        let previous = item.previous().expect("a replacement has a previous item");
                        replace_or_add(&mut *result_list, previous, item.current().clone());
                    }
                    ListChangeReason::Refresh => {
                        result_list.refresh(change.item().current());
                    }
                    ListChangeReason::AddRange => result_list.add_range(change.range().items().to_vec()),
                    ListChangeReason::RemoveRange => remove_many(&mut *result_list, change.range().items()),
                    ListChangeReason::Clear => on_clear(&mut result_list, change),
                    // Ignored: the indexes of the merged list are not those of the lists.
                    ListChangeReason::Moved => {}
                }
            }
        }
        if let Some(observer) = observer {
            self.emit_changes(observer);
        }
    }

    /// Removes the items (`RemoveItems`).
    pub(crate) fn remove_items(&self, remove_items: &[T], observer: Option<&Rc<dyn IObserver<ChangeSet<T>>>>) {
        remove_many(&mut *self.result_list.borrow_mut(), remove_items);
        if let Some(observer) = observer {
            self.emit_changes(observer);
        }
    }

    /// Emits the changes since the last emission (`EmitChanges`).
    pub(crate) fn emit_changes(&self, observer: &Rc<dyn IObserver<ChangeSet<T>>>) {
        let change_set = self.result_list.borrow_mut().capture_changes();
        if change_set.count() != 0 {
            observer.on_next(change_set);
        }
    }
}

/// `OnClear`.
fn on_clear<T: Clone + PartialEq>(result_list: &mut ChangeAwareList<T>, change: &Change<T>) {
    clear_or_remove_many(result_list as &mut dyn IList<T>, change);
}
