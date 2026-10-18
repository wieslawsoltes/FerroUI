//! `Do` (`Linq/Observable/Do.cs`).

use crate::internal::{impl_sink_dispose, unary, OperatorSink, Sink};
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::rc::Rc;

pub(crate) type OnNextAction<T> = Rc<dyn Fn(&T) -> Result<(), ObservableError>>;
pub(crate) type OnErrorAction = Rc<dyn Fn(ObservableError) -> Result<(), ObservableError>>;
pub(crate) type OnCompletedAction = Rc<dyn Fn() -> Result<(), ObservableError>>;

/// The sink of `Do` (`Do<TSource>.OnNext._`, `Do<TSource>.Actions._` and `Do<TSource>.Observer._`):
/// runs the action of each notification before it forwards the notification; an action that fails
/// ends the sequence with its error in place of the notification.
struct DoSink<T> {
    sink: Sink<T>,
    on_next: OnNextAction<T>,
    on_error: OnErrorAction,
    on_completed: OnCompletedAction,
}

impl<T: 'static> OperatorSink<T, T> for DoSink<T> {
    fn sink(&self) -> &Sink<T> {
        &self.sink
    }
}

impl_sink_dispose!(DoSink<T>);

impl<T: 'static> IObserver<T> for DoSink<T> {
    fn on_next(&self, value: T) {
        match (self.on_next)(&value) {
            Ok(()) => self.sink.forward_on_next(value),
            Err(error) => self.sink.forward_on_error(error),
        }
    }

    fn on_error(&self, error: ObservableError) {
        match (self.on_error)(error.clone()) {
            Ok(()) => self.sink.forward_on_error(error),
            Err(failure) => self.sink.forward_on_error(failure),
        }
    }

    fn on_completed(&self) {
        match (self.on_completed)() {
            Ok(()) => self.sink.forward_on_completed(),
            Err(error) => self.sink.forward_on_error(error),
        }
    }
}

/// `Do` with the actions of the three notifications (`Do<TSource>.Actions`; `Do<TSource>.OnNext`
/// is this with actions that do nothing for the end of the sequence).
pub(crate) fn do_<T: 'static>(
    source: Rc<dyn IObservable<T>>,
    on_next: OnNextAction<T>,
    on_error: OnErrorAction,
    on_completed: OnCompletedAction,
) -> Rc<dyn IObservable<T>> {
    unary(source, move |observer| {
        Rc::new(DoSink { sink: Sink::new(observer), on_next: on_next.clone(), on_error: on_error.clone(), on_completed: on_completed.clone() })
    })
}

/// An action for the error that does nothing (`Stubs<Exception>.Ignore`).
pub(crate) fn ignore_error() -> OnErrorAction {
    Rc::new(|_: ObservableError| Ok(()))
}

/// An action for the completion that does nothing (`Stubs.Nop`).
pub(crate) fn nop() -> OnCompletedAction {
    Rc::new(|| Ok(()))
}

/// The methods of `Do` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! do_methods {
    () => {
        /// Runs `on_next` for each value before it is forwarded (`Do(Action<TSource>)`).
        fn do_(&self, on_next: impl Fn(&T) + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::do_::do_(
                self.observable(),
                std::rc::Rc::new(move |value: &T| {
                    on_next(value);
                    Ok(())
                }),
                crate::linq::do_::ignore_error(),
                crate::linq::do_::nop(),
            )
        }

        /// Runs `on_next` for each value and `on_completed` for the completion before they are
        /// forwarded (`Do(Action<TSource>, Action)`).
        fn do_with_completed(
            &self,
            on_next: impl Fn(&T) + 'static,
            on_completed: impl Fn() + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::do_::do_(
                self.observable(),
                std::rc::Rc::new(move |value: &T| {
                    on_next(value);
                    Ok(())
                }),
                crate::linq::do_::ignore_error(),
                std::rc::Rc::new(move || {
                    on_completed();
                    Ok(())
                }),
            )
        }

        /// Runs `on_next` for each value and `on_error` for the error before they are forwarded
        /// (`Do(Action<TSource>, Action<Exception>)`).
        fn do_with_error(
            &self,
            on_next: impl Fn(&T) + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::do_::do_(
                self.observable(),
                std::rc::Rc::new(move |value: &T| {
                    on_next(value);
                    Ok(())
                }),
                std::rc::Rc::new(move |error: ferroui_system::ObservableError| {
                    on_error(error);
                    Ok(())
                }),
                crate::linq::do_::nop(),
            )
        }

        /// Runs the action of each notification before it is forwarded
        /// (`Do(Action<TSource>, Action<Exception>, Action)`).
        fn do_all(
            &self,
            on_next: impl Fn(&T) + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) + 'static,
            on_completed: impl Fn() + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::do_::do_(
                self.observable(),
                std::rc::Rc::new(move |value: &T| {
                    on_next(value);
                    Ok(())
                }),
                std::rc::Rc::new(move |error: ferroui_system::ObservableError| {
                    on_error(error);
                    Ok(())
                }),
                std::rc::Rc::new(move || {
                    on_completed();
                    Ok(())
                }),
            )
        }

        /// Notifies `observer` of each notification before it is forwarded
        /// (`Do(IObserver<TSource>)`).
        fn do_observer(&self, observer: std::rc::Rc<dyn ferroui_system::IObserver<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            let (next, error, completed) = (observer.clone(), observer.clone(), observer);
            crate::linq::do_::do_(
                self.observable(),
                std::rc::Rc::new(move |value: &T| {
                    next.on_next(value.clone());
                    Ok(())
                }),
                std::rc::Rc::new(move |e: ferroui_system::ObservableError| {
                    error.on_error(e);
                    Ok(())
                }),
                std::rc::Rc::new(move || {
                    completed.on_completed();
                    Ok(())
                }),
            )
        }

        /// [`do_`](Self::do_) with an action that may fail; a failure ends the sequence with its
        /// error in place of the value (`Do(Action<TSource>)` with an action that throws).
        fn try_do(
            &self,
            on_next: impl Fn(&T) -> Result<(), ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::do_::do_(self.observable(), std::rc::Rc::new(on_next), crate::linq::do_::ignore_error(), crate::linq::do_::nop())
        }

        /// [`do_with_completed`](Self::do_with_completed) with actions that may fail
        /// (`Do(Action<TSource>, Action)` with actions that throw).
        fn try_do_with_completed(
            &self,
            on_next: impl Fn(&T) -> Result<(), ferroui_system::ObservableError> + 'static,
            on_completed: impl Fn() -> Result<(), ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::do_::do_(
                self.observable(),
                std::rc::Rc::new(on_next),
                crate::linq::do_::ignore_error(),
                std::rc::Rc::new(on_completed),
            )
        }

        /// [`do_with_error`](Self::do_with_error) with actions that may fail
        /// (`Do(Action<TSource>, Action<Exception>)` with actions that throw).
        fn try_do_with_error(
            &self,
            on_next: impl Fn(&T) -> Result<(), ferroui_system::ObservableError> + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) -> Result<(), ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::do_::do_(self.observable(), std::rc::Rc::new(on_next), std::rc::Rc::new(on_error), crate::linq::do_::nop())
        }

        /// [`do_all`](Self::do_all) with actions that may fail
        /// (`Do(Action<TSource>, Action<Exception>, Action)` with actions that throw).
        fn try_do_all(
            &self,
            on_next: impl Fn(&T) -> Result<(), ferroui_system::ObservableError> + 'static,
            on_error: impl Fn(ferroui_system::ObservableError) -> Result<(), ferroui_system::ObservableError> + 'static,
            on_completed: impl Fn() -> Result<(), ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::do_::do_(
                self.observable(),
                std::rc::Rc::new(on_next),
                std::rc::Rc::new(on_error),
                std::rc::Rc::new(on_completed),
            )
        }
    };
}
pub(crate) use do_methods;
