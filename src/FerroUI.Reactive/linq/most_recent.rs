//! `MostRecent` (`Linq/Observable/MostRecent.cs`).

use super::push_to_pull_adapter::{PullEnumerable, PullSink};
use crate::disposables::SingleAssignmentDisposableValue;
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

/// The last notification of the source (`_kind`, `_value`, `_error`).
enum Latest<T> {
    Next(T),
    Error(ObservableError),
    Completed,
}

/// The sink of `MostRecent` (`MostRecent<TSource>._`).
struct MostRecentSink<T> {
    latest: RefCell<Latest<T>>,
    upstream: SingleAssignmentDisposableValue,
}

impl<T> IObserver<T> for MostRecentSink<T> {
    fn on_next(&self, value: T) {
        *self.latest.borrow_mut() = Latest::Next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.upstream.dispose();
        *self.latest.borrow_mut() = Latest::Error(error);
    }

    fn on_completed(&self) {
        self.upstream.dispose();
        *self.latest.borrow_mut() = Latest::Completed;
    }
}

impl<T: Clone> PullSink<T> for MostRecentSink<T> {
    fn try_move_next(&self) -> Result<Option<T>, ObservableError> {
        match &*self.latest.borrow() {
            Latest::Next(value) => Ok(Some(value.clone())),
            Latest::Error(error) => Err(error.clone()),
            Latest::Completed => Ok(None),
        }
    }

    fn upstream(&self) -> &SingleAssignmentDisposableValue {
        &self.upstream
    }
}

pub(crate) fn most_recent<T: Clone + 'static>(source: Rc<dyn IObservable<T>>, initial_value: T) -> PullEnumerable<T> {
    PullEnumerable::new(source, move || {
        Ok(Rc::new(MostRecentSink { latest: RefCell::new(Latest::Next(initial_value.clone())), upstream: SingleAssignmentDisposableValue::new() }))
    })
}

macro_rules! most_recent_methods {
    () => {
        /// A sequence whose each step gives the latest value of the source, `initial_value` until
        /// the source produces one; it never waits (`MostRecent(TSource)`).
        fn most_recent(&self, initial_value: T) -> crate::linq::PullEnumerable<T>
        where
            T: Clone,
        {
            crate::linq::most_recent::most_recent(self.observable(), initial_value)
        }
    };
}
pub(crate) use most_recent_methods;
