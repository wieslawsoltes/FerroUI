//! ferroui-system
//!
//! The parts of the base class library of .NET that FerroUI and its reusable libraries share, in
//! Rust: thread-local variables that work on every target, events, change notification and the
//! cancellation of work. Nothing here depends on the user interface; the base
//! library of FerroUI re-exports every item at the path it had there.
//!
//! The modules follow the namespaces of .NET where an item ports one of its types; the items of the
//! namespace `System` are at the root (`IDisposable`, `IObservable<T>`, `IObserver<T>`,
//! `IProgress<T>`), with the guard of a subscription ([`Subscription`]) and the extension that
//! subscribes with it ([`SubscribeGuard`]).
//!
//! - [`component_model`]: `System.ComponentModel` (`INotifyPropertyChanged`).
//! - [`collections::generic`]: `System.Collections.Generic` (`Dictionary` with the order of
//!   enumeration of .NET).
//! - [`collections::object_model`]: `System.Collections.ObjectModel` (`ObservableCollection`).
//! - [`collections::specialized`]: `System.Collections.Specialized` (`INotifyCollectionChanged` and
//!   the arguments of a change).
//! - [`threading`]: `System.Threading` (`CancellationToken` and its source) and
//!   `System.Threading.Tasks` (`Task.Run`; the deprecated `Task<T>` and `TaskCompletionSource<T>`,
//!   wrappers over [`task::Shared`] since 0.5.0).
//! - [`task`]: the executor of the workspace (named after `std::task`): futures spawned on their
//!   thread, their handles ([`task::JoinHandle`], [`task::Shared`]) and where their polls run
//!   ([`task::Schedule`]), and the one-shot channel ([`task::oneshot`]); `threading::tasks` runs
//!   on it.
//! - [`events`]: the multicast events and the lists of handlers that FerroUI's events are built on.
//! - [`ferro_thread_local!`]: thread-local variables, `std::thread_local!` on every target with
//!   thread-local statics and one table per thread on Android ([`mod@ferro_thread_local`]).

pub mod collections;
pub mod component_model;
pub mod events;
pub mod ferro_thread_local;
mod i_disposable;
mod subscription;
mod i_observable;
mod i_progress;
pub mod task;
pub mod threading;

pub use i_disposable::IDisposable;
pub use subscription::Subscription;
pub use i_observable::{IObservable, IObserver, ObservableError, SubscribeGuard};
pub use i_progress::IProgress;

#[cfg(test)]
mod tests;
