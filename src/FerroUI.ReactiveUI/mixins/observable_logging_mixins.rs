//! `ObservableLoggingMixins` (`ReactiveUI.Shared/Mixins/ObservableLoggingMixins.cs`): log the
//! notifications of an observable, and catch its errors with a log entry, through the logger of an
//! object (Splat's `IEnableLogger`).
//!
//! Upstream's static constructor ensures that the builder of ReactiveUI ran
//! (`RxAppBuilder.EnsureInitialized()`), so that a log manager is registered. The logger here comes
//! from the log manager of the current resolver of `AppLocator`, as `IEnableLogger::log` reads it;
//! the application registers one, as the builder does upstream.

use crate::PanicException;
use ferroui_reactive::disposables::{SerialDisposable, SingleAssignmentDisposable};
use ferroui_reactive::linq::Observable;
#[allow(unused_imports)]
use ferroui_splat::{IEnableLogger, IFullLogger};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;

/// The extension methods of `ObservableLoggingMixins` on an observable.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the logging object is an `Rc` (the closures that log
/// keep it); a value is written with its `Debug` form where upstream calls `ToString()`; and the
/// overloads are named by what they take (`log`, `log_message`, `log_with_stringifier`;
/// `logged_catch`, `logged_catch_to`, `logged_catch_of`).
pub trait ObservableLoggingMixins<T> {
    /// Logs every notification at the level `Info` (`OnNext`, `OnCompleted`) or `Warn` (`OnError`)
    /// through the logger of `log_object` (`Log<TObj>(TObj)`).
    fn log<O: IEnableLogger + ?Sized + 'static>(&self, log_object: Rc<O>) -> Rc<dyn IObservable<T>>
    where
        T: fmt::Debug;

    /// [`log`](Self::log) with `message` before each entry (`Log<TObj>(TObj, string?)`).
    fn log_message<O: IEnableLogger + ?Sized + 'static>(&self, log_object: Rc<O>, message: Option<&str>) -> Rc<dyn IObservable<T>>
    where
        T: fmt::Debug;

    /// [`log_message`](Self::log_message) that writes each value with `stringifier`
    /// (`Log<TObj>(TObj, string?, Func<T, string>?)`).
    fn log_with_stringifier<O: IEnableLogger + ?Sized + 'static>(
        &self,
        log_object: Rc<O>,
        message: Option<&str>,
        stringifier: impl Fn(&T) -> String + 'static,
    ) -> Rc<dyn IObservable<T>>;

    /// Continues with `next` (a sequence of the default value when `None`) when the source fails,
    /// after a `Warn` entry with `message` in the logger of `class`
    /// (`LoggedCatch<TObj>(TObj)`, `LoggedCatch<TObj>(TObj, IObservable<T>?)`,
    /// `LoggedCatch<TObj>(TObj, IObservable<T>?, string?)`).
    fn logged_catch<O: IEnableLogger + ?Sized + 'static>(
        &self,
        class: Rc<O>,
        next: Option<Rc<dyn IObservable<T>>>,
        message: Option<&str>,
    ) -> Rc<dyn IObservable<T>>
    where
        T: Default + Clone;

    /// [`logged_catch`](Self::logged_catch) with the sequence to continue with, which needs no
    /// default value.
    fn logged_catch_to<O: IEnableLogger + ?Sized + 'static>(
        &self,
        class: Rc<O>,
        next: Rc<dyn IObservable<T>>,
        message: Option<&str>,
    ) -> Rc<dyn IObservable<T>>;

    /// Continues with the sequence `next` gives for the error when the source fails with an error
    /// of the type `E`, after a `Warn` entry; other errors pass through
    /// (`LoggedCatch<TObj, TException>(TObj, Func<TException, IObservable<T>>, string?)`). `next`
    /// receives the error, which `downcast_ref::<E>` reads.
    fn logged_catch_of<O: IEnableLogger + ?Sized + 'static, E: Error + 'static>(
        &self,
        class: Rc<O>,
        next: impl Fn(ObservableError) -> Rc<dyn IObservable<T>> + 'static,
        message: Option<&str>,
    ) -> Rc<dyn IObservable<T>>;
}

impl<T: 'static> ObservableLoggingMixins<T> for Rc<dyn IObservable<T>> {
    fn log<O: IEnableLogger + ?Sized + 'static>(&self, log_object: Rc<O>) -> Rc<dyn IObservable<T>>
    where
        T: fmt::Debug,
    {
        self.log_message(log_object, None)
    }

    fn log_message<O: IEnableLogger + ?Sized + 'static>(&self, log_object: Rc<O>, message: Option<&str>) -> Rc<dyn IObservable<T>>
    where
        T: fmt::Debug,
    {
        self.log_with_stringifier(log_object, message, |value| format!("{value:?}"))
    }

    fn log_with_stringifier<O: IEnableLogger + ?Sized + 'static>(
        &self,
        log_object: Rc<O>,
        message: Option<&str>,
        stringifier: impl Fn(&T) -> String + 'static,
    ) -> Rc<dyn IObservable<T>> {
        let message: Rc<str> = Rc::from(message.unwrap_or(""));
        let (next_object, next_message) = (log_object.clone(), message.clone());
        let (error_object, error_message) = (log_object.clone(), message.clone());
        let (completed_object, completed_message) = (log_object, message);
        Rc::new(LoggingTeeObservable {
            source: self.clone(),
            on_next: Rc::new(move |value: &T| {
                next_object.log().info_format(format_args!("{} OnNext: {}", next_message, stringifier(value)));
            }),
            on_error: Rc::new(move |error: &ObservableError| {
                error_object.log().warn_exception(&**error, &format!("{error_message} OnError"));
            }),
            on_completed: Rc::new(move || {
                completed_object.log().info_format(format_args!("{} OnCompleted", completed_message));
            }),
        })
    }

    fn logged_catch<O: IEnableLogger + ?Sized + 'static>(
        &self,
        class: Rc<O>,
        next: Option<Rc<dyn IObservable<T>>>,
        message: Option<&str>,
    ) -> Rc<dyn IObservable<T>>
    where
        T: Default + Clone,
    {
        let next = next.unwrap_or_else(|| Observable::return_(T::default()));
        self.logged_catch_to(class, next, message)
    }

    fn logged_catch_to<O: IEnableLogger + ?Sized + 'static>(
        &self,
        class: Rc<O>,
        next: Rc<dyn IObservable<T>>,
        message: Option<&str>,
    ) -> Rc<dyn IObservable<T>> {
        let message = message.unwrap_or("").to_string();
        Rc::new(LoggedCatchObservable::<T, dyn Error> {
            source: self.clone(),
            handler: Rc::new(move |error: ObservableError| {
                class.log().warn_exception(&*error, &message);
                next.clone()
            }),
            matches: Rc::new(|_: &ObservableError| true),
            _error: PhantomData,
        })
    }

    fn logged_catch_of<O: IEnableLogger + ?Sized + 'static, E: Error + 'static>(
        &self,
        class: Rc<O>,
        next: impl Fn(ObservableError) -> Rc<dyn IObservable<T>> + 'static,
        message: Option<&str>,
    ) -> Rc<dyn IObservable<T>> {
        let message = message.unwrap_or("").to_string();
        Rc::new(LoggedCatchObservable::<T, E> {
            source: self.clone(),
            handler: Rc::new(move |error: ObservableError| {
                class.log().warn_exception(&*error, &message);
                next(error)
            }),
            matches: Rc::new(|error: &ObservableError| error.downcast_ref::<E>().is_some()),
            _error: PhantomData,
        })
    }
}

/// `LoggingTeeObservable<T>`: calls the logging handlers, then forwards each notification; a
/// panic of a handler is caught and forwarded as the error of the sequence, as upstream forwards
/// the exception of a handler.
struct LoggingTeeObservable<T> {
    source: Rc<dyn IObservable<T>>,
    on_next: Rc<dyn Fn(&T)>,
    on_error: Rc<dyn Fn(&ObservableError)>,
    on_completed: Rc<dyn Fn()>,
}

impl<T: 'static> IObservable<T> for LoggingTeeObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        self.source.subscribe(Rc::new(LoggingTeeSink {
            on_next: self.on_next.clone(),
            on_error: self.on_error.clone(),
            on_completed: self.on_completed.clone(),
            downstream: observer,
        }))
    }
}

struct LoggingTeeSink<T> {
    on_next: Rc<dyn Fn(&T)>,
    on_error: Rc<dyn Fn(&ObservableError)>,
    on_completed: Rc<dyn Fn()>,
    downstream: Rc<dyn IObserver<T>>,
}

impl<T> IObserver<T> for LoggingTeeSink<T> {
    fn on_next(&self, value: T) {
        if let Err((error, _)) = PanicException::catch(|| (self.on_next)(&value)) {
            self.downstream.on_error(error);
            return;
        }
        self.downstream.on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        if let Err((handler_error, _)) = PanicException::catch(|| (self.on_error)(&error)) {
            self.downstream.on_error(handler_error);
            return;
        }
        self.downstream.on_error(error);
    }

    fn on_completed(&self) {
        if let Err((error, _)) = PanicException::catch(|| (self.on_completed)()) {
            self.downstream.on_error(error);
            return;
        }
        self.downstream.on_completed();
    }
}

/// `LoggedCatchObservable<T, TException>`: on an error of the type `E` (`matches`), disposes the
/// source and continues with the sequence of the handler; other errors pass through.
struct LoggedCatchObservable<T, E: ?Sized> {
    source: Rc<dyn IObservable<T>>,
    handler: Rc<dyn Fn(ObservableError) -> Rc<dyn IObservable<T>>>,
    matches: Rc<dyn Fn(&ObservableError) -> bool>,
    _error: PhantomData<fn() -> Box<E>>,
}

impl<T: 'static, E: ?Sized> IObservable<T> for LoggedCatchObservable<T, E> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let sink = Rc::new(LoggedCatchSink {
            handler: self.handler.clone(),
            matches: self.matches.clone(),
            downstream: observer,
            source: SingleAssignmentDisposable::new(),
            continuation: SerialDisposable::new(),
        });
        // `Run(IObservable<T>)`.
        let subscription = self.source.subscribe(sink.clone());
        sink.source.set_disposable(Some(subscription));
        sink
    }
}

struct LoggedCatchSink<T> {
    handler: Rc<dyn Fn(ObservableError) -> Rc<dyn IObservable<T>>>,
    matches: Rc<dyn Fn(&ObservableError) -> bool>,
    downstream: Rc<dyn IObserver<T>>,
    /// `_source`.
    source: Rc<SingleAssignmentDisposable>,
    /// `_continuation`.
    continuation: Rc<SerialDisposable>,
}

impl<T> IObserver<T> for LoggedCatchSink<T> {
    fn on_next(&self, value: T) {
        self.downstream.on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        if !(self.matches)(&error) {
            self.downstream.on_error(error);
            return;
        }
        let continuation = match PanicException::catch(|| (self.handler)(error)) {
            Ok(continuation) => continuation,
            Err((handler_error, _)) => {
                self.downstream.on_error(handler_error);
                return;
            }
        };
        self.source.dispose();
        self.continuation.set_disposable(Some(continuation.subscribe(self.downstream.clone())));
    }

    fn on_completed(&self) {
        self.downstream.on_completed();
    }
}

impl<T> IDisposable for LoggedCatchSink<T> {
    fn dispose(&self) {
        self.source.dispose();
        self.continuation.dispose();
    }
}
