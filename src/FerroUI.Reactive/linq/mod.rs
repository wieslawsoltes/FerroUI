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
mod observable_ext;
pub(crate) mod select;
pub(crate) mod subscribe;
pub(crate) mod where_;

pub use creation::Observable;
pub use observable_ext::ObservableExt;
