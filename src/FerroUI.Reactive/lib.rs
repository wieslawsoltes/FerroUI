//! ferroui-reactive
//!
//! A port of Rx.NET's System.Reactive (<https://github.com/dotnet/reactive>) to Rust: observable
//! sequences, their operators, the schedulers that decide when work runs, the disposables that end
//! subscriptions, and subjects. It depends on nothing of the user interface of FerroUI; the
//! interfaces it implements (`IObservable<T>`, `IObserver<T>`, `IDisposable`) are those of
//! `ferroui-system`, which the framework uses too, so the sequences of this crate bind to the
//! properties of controls as they are.
//!
//! The modules follow the namespaces of System.Reactive:
//!
//! - [`concurrency`]: `System.Reactive.Concurrency` (schedulers).
//! - [`disposables`]: `System.Reactive.Disposables`.
//! - [`subjects`]: `System.Reactive.Subjects`.
//! - [`linq`]: `System.Reactive.Linq` (the operators: [`Observable`] and [`ObservableExt`]).
//! - The root: `Notification<T>`, `Unit`, `TimeInterval<T>`, `Timestamped<T>`, the observers
//!   (`Observer`, `AnonymousObserver<T>`, `ObserverBase<T>`), `ObservableBase<T>`.
//!
//! Everything is single-threaded and reference counted, as the object model of FerroUI is: a
//! sequence and its subscriptions live on the thread that created them, and the schedulers of the
//! crate run their work on that thread (`docs/porting/reactive.md`, section 5.1).

pub mod concurrency;
pub mod disposables;
// The machinery of the operators; parts of it are used by the operator families as they are ported.
#[allow(unused_imports, dead_code)]
pub(crate) mod internal;
pub mod linq;
mod notification;
mod observable_base;
mod observer;
mod observer_base;
pub mod subjects;
mod time_interval;

pub use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
pub use linq::{ConnectableObservableExt, Observable, ObservableExt, TaskObservableExt};
pub use notification::{IObserverWithResult, Notification, NotificationKind};
pub use observable_base::{ObservableBase, ObservableCore};
pub use observer::{AnonymousObserver, Observer, ObserverExt};
pub use observer_base::{ObserverBase, ObserverCore};
pub use time_interval::{TimeInterval, Timestamped};

/// The type of a notification without a value (`System.Reactive.Unit`): Rust's unit type.
pub type Unit = ();

#[cfg(test)]
mod tests;
