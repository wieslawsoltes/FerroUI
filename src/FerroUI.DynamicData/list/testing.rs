//! The aids for tests that DynamicData ships with the library (`List/Tests`, namespace
//! `DynamicData.Tests`): `ChangeSetAggregator<TObject>`.

use super::{ChangeSet, IObservableList, ObservableList, ObservableListEx};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObservable, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct State<T> {
    messages: RefCell<Vec<ChangeSet<T>>>,
    error: RefCell<Option<ObservableError>>,
    is_completed: Cell<bool>,
}

/// Records what a sequence of change sets of a list produces (`ChangeSetAggregator<TObject>`).
pub struct ChangeSetAggregator<T> {
    state: Rc<State<T>>,
    data: ObservableList<T>,
    disposer: Rc<dyn IDisposable>,
}

impl<T: Clone + PartialEq + 'static> ChangeSetAggregator<T> {
    /// Subscribes to `source` (`ChangeSetAggregator(IObservable<IChangeSet<TObject>>)`).
    pub fn new(source: Rc<dyn IObservable<ChangeSet<T>>>) -> Self {
        let state = Rc::new(State { messages: RefCell::new(Vec::new()), error: RefCell::new(None), is_completed: Cell::new(false) });
        let published = source.publish();
        let published_source: Rc<dyn IObservable<ChangeSet<T>>> = published.clone();
        let data = published_source.as_observable_list();
        let (on_next, on_error, on_completed) = (state.clone(), state.clone(), state.clone());
        let results = published_source.subscribe_all(
            move |changes| on_next.messages.borrow_mut().push(changes),
            move |error| *on_error.error.borrow_mut() = Some(error),
            move || on_completed.is_completed.set(true),
        );
        let connected = published.connect();
        let list = data.clone();
        let disposer = Disposable::create(move || {
            list.dispose();
            connected.dispose();
            results.dispose();
        });
        Self { state, data, disposer }
    }

    /// The items the change sets make (`Data`).
    pub fn data(&self) -> &ObservableList<T> {
        &self.data
    }

    /// The change sets so far (`Messages`).
    pub fn messages(&self) -> Vec<ChangeSet<T>> {
        self.state.messages.borrow().clone()
    }

    /// The error of the sequence (`Exception`).
    pub fn error(&self) -> Option<ObservableError> {
        self.state.error.borrow().clone()
    }

    /// Whether the sequence has ended (`IsCompleted`).
    pub fn is_completed(&self) -> bool {
        self.state.is_completed.get()
    }

    /// The number of items the change sets make.
    pub fn count(&self) -> usize {
        self.data.count()
    }
}

impl<T> IDisposable for ChangeSetAggregator<T> {
    fn dispose(&self) {
        self.disposer.dispose();
    }
}

impl<T> Drop for ChangeSetAggregator<T> {
    fn drop(&mut self) {
        self.disposer.dispose();
    }
}
