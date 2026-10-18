//! Lightweight reactive primitives: observables, observers and disposables.
//!
//! Everything here is single-threaded and reference counted, matching the
//! thread affinity of the object model.

mod anonymous_observer;
mod anonymous_observer_non_generic_helper;
mod combined_subject;
mod composite_disposable;
mod disposable;
mod disposable_mixin;
mod i_ferro_subject;
mod lightweight_observable_base;
mod lightweight_subject;
mod observable;
mod operators;
mod single_subscriber_observable_base;
mod weak_observer_subscription;

pub use anonymous_observer::AnonymousObserver;
pub use combined_subject::CombinedSubject;
pub use composite_disposable::CompositeDisposable;
pub use disposable::{Disposable, SerialDisposable};
pub use disposable_mixin::DisposableMixin;
pub use i_ferro_subject::IFerroSubject;
pub use lightweight_observable_base::{LightweightObservable, LightweightObservableBase};
pub use lightweight_subject::LightweightSubject;
pub use observable::{Observable, ObservableExt};
pub use single_subscriber_observable_base::SingleSubscriberObservableBase;
pub use weak_observer_subscription::WeakObserverSubscription;

/// The interfaces of push-based notification of .NET (`System.IObservable<T>`, `System.IObserver<T>`,
/// `System.IDisposable`), shared with the reusable libraries (`ferroui-system`).
pub use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
