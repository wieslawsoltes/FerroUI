//! mini-mvvm
//!
//! Port of the sample library `MiniMvvm`: the base of the view models of
//! the samples ([`ViewModelBase`]), commands built from closures
//! ([`MiniCommand`], with [`start_async`] for the
//! asynchronous methods of view models) and observables of the properties of a view model
//! ([`PropertyChangedExtensions`]).
//!
//! One file per upstream file.

mod mini_command;
mod property_changed_extensions;
mod view_model_base;

pub use mini_command::{start_async, MiniCommand, MiniCommandTask};
pub use property_changed_extensions::PropertyChangedExtensions;
pub use view_model_base::ViewModelBase;
