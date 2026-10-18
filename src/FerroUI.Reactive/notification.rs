//! `Notification<T>` and `NotificationKind` (`Notification.cs`).

use crate::concurrency::{IScheduler, SchedulerExt};
use crate::linq::Observable;
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::fmt;
use std::rc::Rc;

/// The kind of a notification (`NotificationKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NotificationKind {
    OnNext,
    OnError,
    OnCompleted,
}

/// A notification of an observable sequence as a value (`Notification<T>`).
///
/// Two notifications are equal when they are of the same kind and their values are equal; errors
/// compare by identity, as exceptions do upstream.
#[derive(Clone)]
pub enum Notification<T> {
    /// A value (`Notification.CreateOnNext`).
    OnNext(T),
    /// An error (`Notification.CreateOnError`).
    OnError(ObservableError),
    /// The end of the sequence (`Notification.CreateOnCompleted`).
    OnCompleted,
}

impl<T> Notification<T> {
    /// The kind of the notification (`Kind`).
    pub fn kind(&self) -> NotificationKind {
        match self {
            Self::OnNext(_) => NotificationKind::OnNext,
            Self::OnError(_) => NotificationKind::OnError,
            Self::OnCompleted => NotificationKind::OnCompleted,
        }
    }

    /// Whether the notification has a value (`HasValue`).
    pub fn has_value(&self) -> bool {
        matches!(self, Self::OnNext(_))
    }

    /// The value of an `OnNext` notification (`Value`); `None` for the others, where upstream throws
    /// the error of an `OnError` and `InvalidOperationException` for `OnCompleted`.
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::OnNext(value) => Some(value),
            _ => None,
        }
    }

    /// The error of an `OnError` notification (`Exception`).
    pub fn exception(&self) -> Option<&ObservableError> {
        match self {
            Self::OnError(error) => Some(error),
            _ => None,
        }
    }

    /// Delivers the notification to an observer (`Accept(IObserver<T>)`).
    pub fn accept(self, observer: &dyn IObserver<T>) {
        match self {
            Self::OnNext(value) => observer.on_next(value),
            Self::OnError(error) => observer.on_error(error),
            Self::OnCompleted => observer.on_completed(),
        }
    }

    /// Calls the function for the kind of the notification (`Accept(Func<T, TResult>, ...)`).
    pub fn accept_fns<R>(
        self,
        on_next: impl FnOnce(T) -> R,
        on_error: impl FnOnce(ObservableError) -> R,
        on_completed: impl FnOnce() -> R,
    ) -> R {
        match self {
            Self::OnNext(value) => on_next(value),
            Self::OnError(error) => on_error(error),
            Self::OnCompleted => on_completed(),
        }
    }
}

impl<T: Clone + 'static> Notification<T> {
    /// A sequence of the one notification, delivered on the immediate scheduler (`ToObservable()`).
    pub fn to_observable(&self) -> Rc<dyn IObservable<T>> {
        self.to_observable_on(crate::concurrency::Scheduler::immediate())
    }

    /// A sequence of the one notification, delivered on `scheduler`: a value is followed by the end
    /// of the sequence (`ToObservable(IScheduler)`).
    pub fn to_observable_on(&self, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
        let notification = self.clone();
        Observable::create(move |observer: Rc<dyn IObserver<T>>| {
            let notification = notification.clone();
            scheduler.schedule_action(move || {
                let is_next = notification.has_value();
                notification.accept(&*observer);
                if is_next {
                    observer.on_completed();
                }
            })
        })
    }
}

impl<T: PartialEq> PartialEq for Notification<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::OnNext(a), Self::OnNext(b)) => a == b,
            (Self::OnError(a), Self::OnError(b)) => Rc::ptr_eq(a, b),
            (Self::OnCompleted, Self::OnCompleted) => true,
            _ => false,
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Notification<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OnNext(value) => write!(f, "OnNext({value:?})"),
            Self::OnError(error) => write!(f, "OnError({error})"),
            Self::OnCompleted => f.write_str("OnCompleted()"),
        }
    }
}
