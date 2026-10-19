//! `AsyncDisposeManyFixture` (`DynamicData.Tests/Cache/AsyncDisposeManyFixture.cs`): the items of
//! the tests of `AsyncDisposeMany`, which `async_dispose_many_fixture_unit_tests` and
//! `async_dispose_many_fixture_integration_tests` share.
//!
//! Upstream's abstract record `ItemBase` and its four records (`PlainItem`, `DisposableItem`,
//! `AsyncDisposableItem`, `ImmediateAsyncDisposableItem`) are one struct with its [`ItemType`]:
//! the item is an `IDisposable` and an `IAsyncDisposable` here, and [`ItemBase::to_item_disposal`]
//! says which of the two upstream's record is (the function of `async_dispose_many_with`).

use crate::utilities::{ObservableExtensions, ValueRecordingObserver};
use ferroui_dynamic_data::{IAsyncDisposable, ItemDisposal};
use ferroui_system::threading::tasks::{Task, TaskCompletionSource};
use ferroui_system::{IDisposable, IObservable, ObservableError};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// `SourceType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceType {
    Subject,
    Immediate,
}

/// `ItemType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ItemType {
    Plain,
    Disposable,
    AsyncDisposable,
    ImmediateAsyncDisposable,
}

/// `ItemBase` and its records.
pub(crate) struct ItemBase {
    id: i32,
    version: i32,
    item_type: ItemType,
    has_been_disposed: Cell<bool>,
    /// `_disposeError` of `DisposableItem` and `ImmediateAsyncDisposableItem`.
    dispose_error: RefCell<Option<ObservableError>>,
    /// `_disposeCompletionSource` of `AsyncDisposableItem`.
    dispose_completion_source: TaskCompletionSource<()>,
}

impl ItemBase {
    /// `ItemBase.Create(ItemType, int id, int version)`.
    pub(crate) fn create(item_type: ItemType, id: i32, version: i32) -> Rc<Self> {
        Rc::new(Self {
            id,
            version,
            item_type,
            has_been_disposed: Cell::new(false),
            dispose_error: RefCell::new(None),
            dispose_completion_source: TaskCompletionSource::new(),
        })
    }

    /// `Id`.
    pub(crate) fn id(&self) -> i32 {
        self.id
    }

    /// `Version`.
    #[allow(dead_code)]
    pub(crate) fn version(&self) -> i32 {
        self.version
    }

    /// The key of an item (`static item => item.Id`).
    pub(crate) fn select_id(item: &Rc<ItemBase>) -> i32 {
        item.id
    }

    /// `CanBeDisposed`.
    pub(crate) fn can_be_disposed(&self) -> bool {
        self.item_type != ItemType::Plain
    }

    /// `HasBeenDisposed`.
    pub(crate) fn has_been_disposed(&self) -> bool {
        self.item_type != ItemType::Plain && self.has_been_disposed.get()
    }

    /// `CompleteDisposal`: ends the disposal task of an `AsyncDisposableItem`.
    pub(crate) fn complete_disposal(&self) {
        if self.item_type == ItemType::AsyncDisposable {
            self.dispose_completion_source.set_result(());
        }
    }

    /// `FailDisposal`: the error of the disposal.
    pub(crate) fn fail_disposal(&self, error: ObservableError) {
        match self.item_type {
            ItemType::Plain => {}
            ItemType::Disposable | ItemType::ImmediateAsyncDisposable => *self.dispose_error.borrow_mut() = Some(error),
            ItemType::AsyncDisposable => self.dispose_completion_source.set_exception(error),
        }
    }

    /// How the record of the item is disposed (`item is IDisposable`, `item is IAsyncDisposable`).
    pub(crate) fn to_item_disposal(item: &Rc<ItemBase>) -> Option<ItemDisposal> {
        match item.item_type {
            ItemType::Plain => None,
            ItemType::Disposable => {
                let disposable: Rc<dyn IDisposable> = item.clone();
                Some(ItemDisposal::Disposable(disposable))
            }
            ItemType::AsyncDisposable | ItemType::ImmediateAsyncDisposable => {
                let disposable: Rc<dyn IAsyncDisposable> = item.clone();
                Some(ItemDisposal::AsyncDisposable(disposable))
            }
        }
    }
}

impl IDisposable for ItemBase {
    /// `DisposableItem.Dispose`: an error of the disposal is thrown, which is a panic here.
    fn dispose(&self) {
        if let Some(error) = self.dispose_error.borrow().as_ref() {
            panic!("{error}");
        }
        self.has_been_disposed.set(true);
    }
}

impl IAsyncDisposable for ItemBase {
    /// `AsyncDisposableItem.DisposeAsync` (the task of the completion source) and
    /// `ImmediateAsyncDisposableItem.DisposeAsync` (a task that has ended, or failed).
    fn dispose_async(&self) -> Task<()> {
        self.has_been_disposed.set(true);
        match self.item_type {
            ItemType::AsyncDisposable => self.dispose_completion_source.task(),
            _ => match self.dispose_error.borrow().clone() {
                Some(error) => Task::from_exception(error),
                None => Task::from_result(()),
            },
        }
    }
}

impl PartialEq for ItemBase {
    /// The equality of the records: the same record type, identifier and version.
    fn eq(&self, other: &Self) -> bool {
        self.item_type == other.item_type && self.id == other.id && self.version == other.version
    }
}

impl fmt::Debug for ItemBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {{ Id = {}, Version = {} }}", self.item_type, self.id, self.version)
    }
}

/// The recorder of the `disposalsCompleted` sequence that the accessor receives, once per
/// subscription.
#[derive(Default)]
pub(crate) struct DisposalsCompletedRecorder {
    results: RefCell<Option<(Rc<dyn IDisposable>, Rc<ValueRecordingObserver<()>>)>>,
}

impl DisposalsCompletedRecorder {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The accessor: `disposalsCompleted.RecordValues(out disposalsCompletedResults)`.
    pub(crate) fn accessor(self: &Rc<Self>) -> impl Fn(Rc<dyn IObservable<()>>) + 'static {
        let recorder = self.clone();
        move |disposals_completed: Rc<dyn IObservable<()>>| {
            assert!(recorder.results.borrow().is_none(), "disposalsCompletedAccessor should only be invoked once per subscription");
            let recorded = disposals_completed.record_values();
            *recorder.results.borrow_mut() = Some(recorded);
        }
    }

    /// `disposalsCompletedResults`.
    pub(crate) fn results(&self) -> Rc<ValueRecordingObserver<()>> {
        self.results.borrow().as_ref().map(|(_, results)| results.clone()).expect("disposalsCompletedAccessor should have been invoked")
    }
}
