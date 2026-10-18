//! `System.Reactive.Linq`: the operators of observable sequences.
//!
//! The static members of upstream's `Observable` (the factories: `Create`, `Return`, `Range`, ...)
//! are associated functions of [`Observable`]; its extension methods on `IObservable<T>` (the
//! operators: `Select`, `Where`, `Merge`, ...) are methods of [`ObservableExt`] on
//! `Rc<dyn IObservable<T>>`, the handle of a sequence. The names are upstream's in Rust case
//! (`select`, `where_`, `distinct_until_changed`). Each family of operators is a module of its own:
//! its implementation, and the methods it adds to [`ObservableExt`] (a `*_methods!` macro the trait
//! invokes).

mod creation;
pub(crate) mod append_prepend;
pub(crate) mod as_observable;
pub(crate) mod cast;
pub(crate) mod default_if_empty;
pub(crate) mod dematerialize;
pub(crate) mod do_;
pub(crate) mod finally;
pub(crate) mod ignore_elements;
pub(crate) mod materialize;
pub(crate) mod of_type;
pub(crate) mod scan;
pub(crate) mod skip;
pub(crate) mod skip_last;
pub(crate) mod skip_until;
pub(crate) mod skip_while;
pub(crate) mod start_with;
pub(crate) mod take;
pub(crate) mod take_last;
pub(crate) mod take_last_buffer;
pub(crate) mod take_until;
pub(crate) mod take_while;
mod observable_ext;
pub(crate) mod select;
pub(crate) mod subscribe;
pub(crate) mod where_;

pub use cast::{AnyObject, InvalidCastException};
pub use creation::Observable;
pub use dematerialize::NotificationValue;
pub use observable_ext::ObservableExt;
