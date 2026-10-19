//! `ChangeSetBinder<T>` (`Shared/Platform/ChangeSetBinder.cs`).

use super::{ReactiveChange, ReactiveChangeReason, ReactiveChangeSet};
use ferroui_reactive::Observer;
use ferroui_system::{IDisposable, IObservable};
use std::cell::RefCell;
use std::rc::Rc;

/// A function told of each change as the binder applies it (`ReactiveChangeHandler<T>`).
pub type ReactiveChangeHandler<T> = Rc<dyn Fn(&ReactiveChange<T>)>;

/// Applies a stream of change sets to a list that it keeps (`ChangeSetBinder<T>`): the light
/// replacement of DynamicData's `SourceList` bind that the list adapters of the platforms use. It
/// subscribes once, applies each change in order, tells the adapter of each change and of each
/// batch, so that the adapter raises its own notifications, and unsubscribes when disposed.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's class is internal to the platform assemblies;
/// it is public here for the integration of the user interface, which lives in its own crate.
pub struct ChangeSetBinder<T> {
    items: Rc<RefCell<Vec<T>>>,
    subscription: Rc<dyn IDisposable>,
}

impl<T: Clone + PartialEq + 'static> ChangeSetBinder<T> {
    /// A binder of `source` (`ChangeSetBinder(IObservable<IReactiveChangeSet<T>>,
    /// ReactiveChangeHandler<T>?, Action?)`): `on_change` is told of each change as it is applied,
    /// and `on_batch` once after each change set.
    pub fn new(
        source: Rc<dyn IObservable<ReactiveChangeSet<T>>>,
        on_change: Option<ReactiveChangeHandler<T>>,
        on_batch: Option<Rc<dyn Fn()>>,
    ) -> Self {
        let items = Rc::new(RefCell::new(Vec::new()));
        let target = items.clone();
        // `OnChangeSet`.
        let subscription = source.subscribe(Observer::create_with_error(
            move |change_set: ReactiveChangeSet<T>| {
                for change in change_set.iter() {
                    apply(&target, change);
                    if let Some(on_change) = &on_change {
                        on_change(change);
                    }
                }
                if let Some(on_batch) = &on_batch {
                    on_batch();
                }
            },
            // `DelegateObserver` ignores the errors.
            |_| {},
        ));
        Self { items, subscription }
    }

    /// The number of items (`Count`).
    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    /// The item at `index` (the indexer).
    ///
    /// # Panics
    /// When `index` is out of range (`ArgumentOutOfRangeException`).
    pub fn get(&self, index: usize) -> T {
        self.items.borrow()[index].clone()
    }

    /// A copy of the items.
    pub fn to_vec(&self) -> Vec<T> {
        self.items.borrow().clone()
    }

    /// Unsubscribes from the change sets (`Dispose`).
    pub fn dispose(&self) {
        self.subscription.dispose();
    }
}

impl<T: Clone + PartialEq + 'static> IDisposable for ChangeSetBinder<T> {
    fn dispose(&self) {
        ChangeSetBinder::dispose(self);
    }
}

/// `Apply`: one change to the list.
fn apply<T: Clone + PartialEq>(items: &RefCell<Vec<T>>, change: &ReactiveChange<T>) {
    match change.reason() {
        ReactiveChangeReason::Add => insert(items, change.current_index(), change.current().clone()),
        ReactiveChangeReason::Remove => remove_at(items, change.current_index(), change.current()),
        ReactiveChangeReason::Replace => {
            let mut items = items.borrow_mut();
            let index = change.current_index();
            if index >= 0 && (index as usize) < items.len() {
                items[index as usize] = change.current().clone();
            }
        }
        ReactiveChangeReason::Move => {
            remove_at(items, change.previous_index(), change.current());
            insert(items, change.current_index(), change.current().clone());
        }
        ReactiveChangeReason::Refresh => {}
    }
}

/// `Insert`: at the index, or at the end when the index is out of range.
fn insert<T>(items: &RefCell<Vec<T>>, index: i32, item: T) {
    let mut items = items.borrow_mut();
    if index >= 0 && (index as usize) <= items.len() {
        items.insert(index as usize, item);
        return;
    }
    items.push(item);
}

/// `RemoveAt`: at the index, or the item by value when the index is out of range.
fn remove_at<T: PartialEq>(items: &RefCell<Vec<T>>, index: i32, item: &T) {
    let mut items = items.borrow_mut();
    if index >= 0 && (index as usize) < items.len() {
        items.remove(index as usize);
        return;
    }
    if let Some(position) = items.iter().position(|candidate| candidate == item) {
        items.remove(position);
    }
}
