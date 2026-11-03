//! `{x:Bind}`: the compiled bindings of WinUI's XAML compiler, as one markup extension of the
//! XAML runtime library that the compiler and the run-time loader both build (docs/porting/x-bind.md;
//! the pinned WinUI sources are in the NOTICE.md of the crate).
//!
//! The compiler (`ferroui-markup-xaml-loader`) resolves the path of an `x:Bind` against the type
//! of the root of its document, or against the `x:DataType` of the `DataTemplate` it is in, and
//! builds it as a [`CompiledBindingPath`](ferroui_base::data::CompiledBindingPath) of typed
//! accessors: the Rust emitter writes the builder calls, the run-time loader makes the same calls.
//! [`XBindExtension`] then gives each target an [`XBinding`], whose expression reads the path from
//! the root source of the [`XBindings`] of its root: the per-root lifecycle object that WinUI
//! generates as the `Bindings` of a page (`Initialize`, `Update`, `StopTracking`).

mod binding;
mod bindings;
mod extension;
mod root_node;

pub use binding::XBinding;
pub use bindings::{XBindDiagnostic, XBindErrors, XBindings};
pub use extension::XBindExtension;
