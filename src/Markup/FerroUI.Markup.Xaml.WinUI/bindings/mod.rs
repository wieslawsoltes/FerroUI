//! Opt-in WinUI property-path binding and its explicit host lifecycle.

mod binding;
mod bindings;
mod path_node;
mod register_types;

pub use binding::{WinUIBindExtension, WinUIBinding, WinUIBindingScope};
pub use bindings::{WinUIBindingDiagnostic, WinUIBindingErrors, WinUIBindings};
pub(crate) use register_types::register_markup_types;
