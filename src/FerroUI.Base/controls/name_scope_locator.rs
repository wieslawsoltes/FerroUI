use super::NameScopeRef;
use crate::reactive::{Disposable, IDisposable, IObservable, IObserver};
use crate::utilities::SynchronousCompletionAsyncResult;
use crate::{BoxedValue, FerroObject, Ref};
use std::cell::RefCell;
use std::rc::Rc;

type FindResult = Option<Ref<FerroObject>>;

/// Tracks a named element in a name scope.
pub struct NameScopeLocator;

impl NameScopeLocator {
    /// Tracks a named control relative to another control: the observable
    /// produces the element once it is registered (or `None` once the scope
    /// completes without it) and never completes.
    pub fn track(scope: &NameScopeRef, name: &str) -> Rc<dyn IObservable<FindResult>> {
        Self::observe(scope.find_async(name))
    }

    /// Tracks any named object, including a plain model, without completing the observable.
    pub fn track_value(scope: &NameScopeRef, name: &str) -> Rc<dyn IObservable<Option<BoxedValue>>> {
        Self::observe(scope.find_value_async(name))
    }

    fn observe<T: Clone + 'static>(task: SynchronousCompletionAsyncResult<T>) -> Rc<dyn IObservable<T>> {
        let state = if task.is_completed() { State::Value(task.get_result()) } else { State::Pending(task) };
        Rc::new(NeverEndingSynchronousCompletionAsyncResultObservable { state: RefCell::new(state) })
    }
}

enum State<T: Clone> {
    Value(T),
    Pending(SynchronousCompletionAsyncResult<T>),
}

// The binding system treats completion of a source as the end of the binding,
// so this observable never completes.
struct NeverEndingSynchronousCompletionAsyncResultObservable<T: Clone> {
    state: RefCell<State<T>>,
}

impl<T: Clone + 'static> IObservable<T> for NeverEndingSynchronousCompletionAsyncResultObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let pending = {
            let mut state = self.state.borrow_mut();
            if let State::Pending(task) = &*state {
                if task.is_completed() {
                    let result = task.get_result();
                    *state = State::Value(result);
                }
            }
            match &*state {
                State::Pending(task) => Ok(task.clone()),
                State::Value(value) => Err(value.clone()),
            }
        };

        match pending {
            Ok(task) => {
                let result = task.clone();
                task.on_completed(move || observer.on_next(result.get_result()));
            }
            Err(value) => observer.on_next(value),
        }

        Disposable::empty()
    }
}
