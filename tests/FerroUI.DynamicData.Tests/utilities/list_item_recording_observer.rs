//! `ListItemRecordingObserver<T>` (`Utilities/ListItemRecordingObserver.cs`).

use super::RecordingObserverBase;
use ferroui_dynamic_data::list::{ChangeSet, ListChangeReason};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_system::{IObserver, ObservableError};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

/// Records the change sets of a list and the items they make, until the sequence ends
/// (`ListItemRecordingObserver<T>`). It dereferences to [`RecordingObserverBase`] for the error,
/// the end and the notifications.
pub struct ListItemRecordingObserver<T> {
    base: RecordingObserverBase<ChangeSet<T>>,
    recorded_change_sets: RefCell<Vec<ChangeSet<T>>>,
    recorded_items: RefCell<Vec<T>>,
}

impl<T: Clone + 'static> ListItemRecordingObserver<T> {
    /// An observer that stamps the notifications with the time of `scheduler`.
    pub fn new(scheduler: Rc<dyn IScheduler>) -> Rc<Self> {
        Rc::new(Self { base: RecordingObserverBase::new(scheduler), recorded_change_sets: RefCell::new(Vec::new()), recorded_items: RefCell::new(Vec::new()) })
    }

    /// The change sets received before the end (`RecordedChangeSets`).
    pub fn recorded_change_sets(&self) -> Vec<ChangeSet<T>> {
        self.recorded_change_sets.borrow().clone()
    }

    /// The items the change sets make (`RecordedItems`).
    pub fn recorded_items(&self) -> Vec<T> {
        self.recorded_items.borrow().clone()
    }

    /// Forgets the change sets received so far (`ClearChangeSets`).
    pub fn clear_change_sets(&self) {
        self.recorded_change_sets.borrow_mut().clear();
    }
}

impl<T> Deref for ListItemRecordingObserver<T> {
    type Target = RecordingObserverBase<ChangeSet<T>>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<T: Clone + 'static> IObserver<ChangeSet<T>> for ListItemRecordingObserver<T> {
    fn on_next(&self, value: ChangeSet<T>) {
        self.base.record_next(&value);
        if self.base.has_finalized() {
            return;
        }
        self.recorded_change_sets.borrow_mut().push(value.clone());
        let mut items = self.recorded_items.borrow_mut();
        for change in &value {
            match change.reason() {
                ListChangeReason::Add => {
                    let item = change.item();
                    match item.current_index() {
                        None => items.push(item.current().clone()),
                        Some(index) => items.insert(index, item.current().clone()),
                    }
                }
                ListChangeReason::AddRange => {
                    let range = change.range();
                    match range.index() {
                        None => items.extend(range.iter().cloned()),
                        Some(index) => {
                            let tail = items.split_off(index);
                            items.extend(range.iter().cloned());
                            items.extend(tail);
                        }
                    }
                }
                ListChangeReason::Clear => items.clear(),
                ListChangeReason::Moved => {
                    let item = change.item();
                    items.remove(item.previous_index().expect("a move has a previous index"));
                    items.insert(item.current_index().expect("a move has a current index"), item.current().clone());
                }
                ListChangeReason::Remove => {
                    items.remove(change.item().current_index().expect("a removal has an index"));
                }
                ListChangeReason::RemoveRange => {
                    let range = change.range();
                    let index = range.index().expect("a removal of a range has an index");
                    items.drain(index..index + range.count());
                }
                ListChangeReason::Replace => {
                    let item = change.item();
                    items[item.current_index().expect("a replacement has an index")] = item.current().clone();
                }
                ListChangeReason::Refresh => {}
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.base.record_error(error);
    }

    fn on_completed(&self) {
        self.base.record_completed();
    }
}
