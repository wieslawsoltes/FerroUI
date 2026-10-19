//! `ForEachAsync` (`Linq/QueryLanguage.Imperative.cs`, `Linq/Observable.Imperative.cs`): runs an
//! action for each value of a sequence and gives a task that ends with the sequence
//! (docs/porting/reactive.md, section 5.8).
//!
//! Upstream's `Task` is `Task<()>`. An action that throws is the `try_` form, whose `Err` faults
//! the task and disposes the subscription, as upstream's `catch` around the action does. Upstream
//! also catches a `Dispose` that throws and faults the task with it; disposables do not fail here.

use crate::disposables::SingleAssignmentDisposableValue;
use crate::internal::TokenRegistration;
use crate::linq::SequenceError;
use ferroui_system::threading::tasks::{Task, TaskCompletionSource};
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// What the observer, the cancellation and the caller share (the locals of `ForEachAsync_`).
struct ForEachState {
    completion: TaskCompletionSource<()>,
    subscription: SingleAssignmentDisposableValue,
    registration: RefCell<Option<Rc<TokenRegistration>>>,
}

impl ForEachState {
    /// `dispose`: removes the registration and disposes the subscription, then runs `action`.
    fn dispose_then(&self, action: impl FnOnce(&TaskCompletionSource<()>)) {
        let registration = self.registration.borrow().clone();
        if let Some(registration) = registration {
            registration.dispose();
        }
        self.subscription.dispose();
        action(&self.completion);
    }
}

/// The observer of `ForEachAsync_` (the `AnonymousObserver<TSource>` of upstream).
struct ForEachObserver<F> {
    state: Rc<ForEachState>,
    on_next: F,
    is_stopped: Cell<bool>,
}

impl<T, F: Fn(T) -> Result<(), ObservableError>> IObserver<T> for ForEachObserver<F> {
    fn on_next(&self, value: T) {
        if self.is_stopped.get() || self.state.subscription.is_disposed() {
            return;
        }
        if let Err(error) = (self.on_next)(value) {
            self.state.dispose_then(move |completion| {
                completion.try_set_exception(error);
            });
        }
    }

    fn on_error(&self, error: ObservableError) {
        if self.is_stopped.replace(true) {
            return;
        }
        self.state.dispose_then(move |completion| {
            completion.try_set_exception(error);
        });
    }

    fn on_completed(&self) {
        if self.is_stopped.replace(true) {
            return;
        }
        self.state.dispose_then(|completion| {
            completion.try_set_result(());
        });
    }
}

/// `ForEachAsync_(IObservable<TSource>, Action<TSource>, CancellationToken)`: the cancellation of
/// `token` cancels the task and disposes the subscription; a token canceled already gives a
/// canceled task without subscribing.
pub(crate) fn for_each_async<T: 'static>(
    source: Rc<dyn IObservable<T>>,
    on_next: impl Fn(T) -> Result<(), ObservableError> + 'static,
    token: CancellationToken,
) -> Task<()> {
    let state = Rc::new(ForEachState {
        completion: TaskCompletionSource::new(),
        subscription: SingleAssignmentDisposableValue::new(),
        registration: RefCell::new(None),
    });
    if token.can_be_canceled() {
        let target = state.clone();
        let registration = TokenRegistration::register(&token, move || {
            target.completion.try_set_canceled();
            target.subscription.dispose();
        });
        *state.registration.borrow_mut() = Some(registration);
    }
    if !token.is_cancellation_requested() {
        let observer = Rc::new(ForEachObserver { state: state.clone(), on_next, is_stopped: Cell::new(false) });
        let subscription = source.subscribe(observer);
        state.subscription.set_disposable(Some(subscription));
    }
    state.completion.task()
}

/// The action of the overloads with an index: `x => onNext(x, checked(i++))`. The count past the
/// largest index fails with [`SequenceError::Overflow`] (`OverflowException`).
pub(crate) fn with_index<T: 'static>(
    on_next: impl Fn(T, usize) -> Result<(), ObservableError> + 'static,
) -> impl Fn(T) -> Result<(), ObservableError> + 'static {
    let index = Cell::new(0usize);
    move |value| {
        let current = index.get();
        let Some(next) = current.checked_add(1) else {
            return Err(Rc::new(SequenceError::Overflow));
        };
        index.set(next);
        on_next(value, current)
    }
}

/// The methods of `ForEachAsync` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! for_each_async_methods {
    () => {
        /// Runs `on_next` for each value of the sequence and gives a task that ends when the
        /// sequence ends, or faults with its error (`ForEachAsync(Action<TSource>)`).
        fn for_each_async(&self, on_next: impl Fn(T) + 'static) -> ferroui_system::threading::tasks::Task<()> {
            crate::linq::for_each_async::for_each_async(
                self.observable(),
                move |value| {
                    on_next(value);
                    Ok(())
                },
                ferroui_system::threading::CancellationToken::none(),
            )
        }

        /// [`for_each_async`](Self::for_each_async) that the cancellation of `token` cancels,
        /// disposing the subscription (`ForEachAsync(Action<TSource>, CancellationToken)`).
        fn for_each_async_with_token(
            &self,
            on_next: impl Fn(T) + 'static,
            token: ferroui_system::threading::CancellationToken,
        ) -> ferroui_system::threading::tasks::Task<()> {
            crate::linq::for_each_async::for_each_async(
                self.observable(),
                move |value| {
                    on_next(value);
                    Ok(())
                },
                token,
            )
        }

        /// [`for_each_async`](Self::for_each_async) with the index of each value
        /// (`ForEachAsync(Action<TSource, int>)`).
        fn for_each_async_with_index(&self, on_next: impl Fn(T, usize) + 'static) -> ferroui_system::threading::tasks::Task<()> {
            crate::linq::for_each_async::for_each_async(
                self.observable(),
                crate::linq::for_each_async::with_index(move |value, index| {
                    on_next(value, index);
                    Ok(())
                }),
                ferroui_system::threading::CancellationToken::none(),
            )
        }

        /// [`for_each_async_with_index`](Self::for_each_async_with_index) that the cancellation of
        /// `token` cancels (`ForEachAsync(Action<TSource, int>, CancellationToken)`).
        fn for_each_async_with_index_with_token(
            &self,
            on_next: impl Fn(T, usize) + 'static,
            token: ferroui_system::threading::CancellationToken,
        ) -> ferroui_system::threading::tasks::Task<()> {
            crate::linq::for_each_async::for_each_async(
                self.observable(),
                crate::linq::for_each_async::with_index(move |value, index| {
                    on_next(value, index);
                    Ok(())
                }),
                token,
            )
        }

        /// [`for_each_async`](Self::for_each_async) with an action that may fail: an `Err` faults
        /// the task with it and disposes the subscription (`ForEachAsync(Action<TSource>)` with an
        /// action that throws).
        fn try_for_each_async(
            &self,
            on_next: impl Fn(T) -> Result<(), ferroui_system::ObservableError> + 'static,
        ) -> ferroui_system::threading::tasks::Task<()> {
            crate::linq::for_each_async::for_each_async(self.observable(), on_next, ferroui_system::threading::CancellationToken::none())
        }

        /// [`try_for_each_async`](Self::try_for_each_async) that the cancellation of `token`
        /// cancels (`ForEachAsync(Action<TSource>, CancellationToken)` with an action that throws).
        fn try_for_each_async_with_token(
            &self,
            on_next: impl Fn(T) -> Result<(), ferroui_system::ObservableError> + 'static,
            token: ferroui_system::threading::CancellationToken,
        ) -> ferroui_system::threading::tasks::Task<()> {
            crate::linq::for_each_async::for_each_async(self.observable(), on_next, token)
        }

        /// [`try_for_each_async`](Self::try_for_each_async) with the index of each value
        /// (`ForEachAsync(Action<TSource, int>)` with an action that throws).
        fn try_for_each_async_with_index(
            &self,
            on_next: impl Fn(T, usize) -> Result<(), ferroui_system::ObservableError> + 'static,
        ) -> ferroui_system::threading::tasks::Task<()> {
            crate::linq::for_each_async::for_each_async(
                self.observable(),
                crate::linq::for_each_async::with_index(on_next),
                ferroui_system::threading::CancellationToken::none(),
            )
        }

        /// [`try_for_each_async_with_index`](Self::try_for_each_async_with_index) that the
        /// cancellation of `token` cancels (`ForEachAsync(Action<TSource, int>, CancellationToken)`
        /// with an action that throws).
        fn try_for_each_async_with_index_with_token(
            &self,
            on_next: impl Fn(T, usize) -> Result<(), ferroui_system::ObservableError> + 'static,
            token: ferroui_system::threading::CancellationToken,
        ) -> ferroui_system::threading::tasks::Task<()> {
            crate::linq::for_each_async::for_each_async(self.observable(), crate::linq::for_each_async::with_index(on_next), token)
        }
    };
}
pub(crate) use for_each_async_methods;
