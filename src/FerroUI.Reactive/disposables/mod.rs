//! `System.Reactive.Disposables`: the disposables that subscriptions are made of.
//!
//! Every disposable is single-threaded and shared by reference (`Rc`), like the observables of the
//! crate. Disposal is explicit and idempotent: dropping a handle disposes nothing. A disposable that
//! holds another one disposes it outside of any borrow of its own state, so that the disposal may
//! reach back into the holder (re-entrancy is the single-threaded form of the races upstream guards
//! with interlocked operations).

mod boolean_disposable;
mod cancellation_disposable;
mod composite_disposable;
mod disposable;
mod multiple_assignment_disposable;
mod ref_count_disposable;
mod scheduled_disposable;
mod serial_disposable;
mod single_assignment_disposable;
mod stable_composite_disposable;

pub use boolean_disposable::BooleanDisposable;
pub use cancellation_disposable::CancellationDisposable;
pub use composite_disposable::CompositeDisposable;
pub use disposable::{AnonymousDisposable, Disposable, DisposableExt, ICancelable};
pub use multiple_assignment_disposable::{MultipleAssignmentDisposable, MultipleAssignmentDisposableValue};
pub use ref_count_disposable::RefCountDisposable;
pub use scheduled_disposable::ScheduledDisposable;
pub use serial_disposable::{SerialDisposable, SerialDisposableValue};
pub use single_assignment_disposable::{SingleAssignmentDisposable, SingleAssignmentDisposableValue};
pub use stable_composite_disposable::StableCompositeDisposable;
