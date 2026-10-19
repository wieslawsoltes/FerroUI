//! ferroui-reactiveui
//!
//! A port of ReactiveUI (<https://github.com/reactiveui/ReactiveUI>) to Rust, in the flavour that
//! runs on System.Reactive (here `ferroui-reactive`), together with the run time of
//! ReactiveUI.Binding: reactive objects, commands, the observation of properties and the bindings
//! of view models and views for MVVM. It depends on nothing of the user interface of FerroUI; the
//! integration with the framework (the scheduler of the dispatcher, the observation of the
//! properties of controls, the views) builds on it.
//!
//! The modules follow the folders of ReactiveUI.Core and ReactiveUI.Shared, in snake case, and the
//! run time of ReactiveUI.Binding is under `binding`. Everything public is re-exported at the
//! root, as upstream puts it in the namespaces `ReactiveUI` and `ReactiveUI.Binding`:
//!
//! - [`reactive_object`]: [`ReactiveObject`], the part of a view model that notifies the changes
//!   of its properties ([`IReactiveObject`], [`IReactiveObjectExt`]), [`ReactiveRecord`], and the
//!   macros [`reactive_object!`] and [`reactive_properties!`] that write a view model's traits and
//!   properties.
//! - [`interfaces`]: the change notifications ([`IReactiveNotifyPropertyChanged`],
//!   [`ReactivePropertyChangedEventArgs`]) and [`IHandleObservableErrors`].
//! - [`scheduler`]: [`ScheduledSubject`].
//! - The root: [`RxSchedulers`], [`RxState`], [`RxCacheSize`] and [`UnhandledErrorException`].
//!
//! A view model holds a [`ReactiveObject`] and is shared as an `Rc`:
//!
//! ```
//! use ferroui_reactiveui::{reactive_object, reactive_properties, IReactiveNotifyPropertyChanged, ReactiveObject};
//! use ferroui_reactive::ObservableExt;
//! use std::cell::Cell;
//! use std::rc::Rc;
//!
//! reactive_properties! {
//!     #[derive(Default)]
//!     pub struct Counter {
//!         reactive: ReactiveObject,
//!         #[reactive]
//!         count: i32,
//!     }
//! }
//! reactive_object!(Counter);
//!
//! let counter = Rc::new(Counter::default());
//! let changes = Rc::new(Cell::new(0));
//! let seen = changes.clone();
//! let _subscription = counter.changed().subscribe_next(move |_| seen.set(seen.get() + 1));
//! counter.set_count(1);
//! counter.set_count(1);
//! assert_eq!(changes.get(), 1);
//! ```
//!
//! Everything is single-threaded and reference counted, as the object model of FerroUI is
//! (`docs/porting/reactive.md`, section 5.1). An exception of user code is a panic; where upstream
//! catches one, the port catches the panic ([`PanicException`]).

pub mod interfaces;
pub(crate) mod internal;
pub mod reactive_object;
mod rx_cache_size;
mod rx_schedulers;
mod rx_state;
pub mod scheduler;
mod unhandled_error_exception;

pub mod binding;
pub mod mixins;
pub(crate) mod primitives;
pub mod reactive_command;

pub use binding::bindings::*;
pub use binding::builder::*;
pub use binding::command_binding::*;
pub use binding::interactions::*;
pub use binding::interfaces::*;
pub use binding::mixins::*;
pub use binding::observable_for_property::*;
pub use binding::observables::*;
pub use interfaces::*;
pub use mixins::*;
pub use reactive_command::*;
pub use internal::PanicException;
pub use reactive_object::*;
pub use rx_cache_size::RxCacheSize;
pub use rx_schedulers::RxSchedulers;
pub use rx_state::RxState;
pub use scheduler::*;
pub use unhandled_error_exception::UnhandledErrorException;

/// What the macros of the crate expand to, so that a crate that uses them needs no other
/// dependency.
#[doc(hidden)]
pub mod __private {
    pub use ferroui_splat::IEnableLogger;
    pub use ferroui_system::component_model::{property_name_of, INotifyPropertyChanged, INotifyPropertyChanging};
    pub use ferroui_system::events::Event;
    pub use ferroui_system::{IObservable, ObservableError};
    pub use paste::paste;
}
