//! `AutoRefreshOnObservableFixture` (`DynamicData.Tests/Cache/AutoRefreshOnObservableFixture.cs`):
//! the types shared by the tests of `AutoRefreshOnObservable` (`auto_refresh_on_observable_fixture_base`,
//! `auto_refresh_on_observable_fixture_with_key` and `auto_refresh_on_observable_fixture_without_key`).

use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

/// When a test notifies the source or a reevaluator: before the subscription of the operator
/// (`Immediate`) or after it (`Asynchronous`) (`NotificationStrategy`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NotificationStrategy {
    Immediate,
    Asynchronous,
}

/// An item whose `Value` notifies a subject when it changes (`Item`). Items are held in `Rc` and
/// are equal only to themselves, as the reference type of upstream.
pub(crate) struct Item {
    id: i32,
    value: Cell<i32>,
    value_changed: Rc<Subject<i32>>,
    has_disposed: Cell<bool>,
}

impl Item {
    /// `new Item() { Id = id }`.
    pub(crate) fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { id, value: Cell::new(0), value_changed: Subject::new(), has_disposed: Cell::new(false) })
    }

    /// The current value, then its changes (`ObserveValue`).
    pub(crate) fn observe_value(item: &Rc<Item>) -> Rc<dyn IObservable<i32>> {
        let item = item.clone();
        Observable::create(move |observer: Rc<dyn IObserver<i32>>| {
            observer.on_next(item.value.get());
            item.value_changed.as_observable().subscribe_safe(observer)
        })
    }

    /// The changes of the value, without the value (`ObserveValueChanged`).
    pub(crate) fn observe_value_changed(item: &Rc<Item>) -> Rc<dyn IObservable<()>> {
        item.value_changed.as_observable().select(|_: i32| ())
    }

    /// The key of an item (`SelectId`).
    pub(crate) fn select_id(item: &Rc<Item>) -> i32 {
        item.id
    }

    /// The key (`Id`).
    pub(crate) fn id(&self) -> i32 {
        self.id
    }

    /// Whether the subject of the changes has observers (`HasObservers`).
    pub(crate) fn has_observers(&self) -> bool {
        self.value_changed.has_observers()
    }

    /// `Value`.
    pub(crate) fn value(&self) -> i32 {
        self.value.get()
    }

    /// Sets `Value`, notifying the changes when it changes.
    pub(crate) fn set_value(&self, value: i32) {
        if self.value.get() == value {
            return;
        }
        self.value.set(value);
        self.value_changed.on_next(value);
    }

    /// Completes the changes (`Complete`).
    pub(crate) fn complete(&self) {
        self.value_changed.on_completed();
    }

    /// Ends the changes with `error` (`SetError`).
    pub(crate) fn set_error(&self, error: ObservableError) {
        self.value_changed.on_error(error);
    }
}

impl IDisposable for Item {
    /// Completes and disposes the subject of the changes, once (`Dispose`).
    fn dispose(&self) {
        if self.has_disposed.replace(true) {
            return;
        }
        self.value_changed.on_completed();
        self.value_changed.dispose();
    }
}

impl PartialEq for Item {
    /// Reference equality, as upstream's class.
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Item").field("id", &self.id).field("value", &self.value.get()).finish()
    }
}
