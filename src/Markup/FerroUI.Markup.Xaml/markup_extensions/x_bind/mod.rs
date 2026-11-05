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
//! generates as the `Bindings` of a page (`Initialize`, `Update`, `StopTracking`). On an event
//! the path ends with the method the event calls, and the binding subscribes its handler while
//! the bindings of the root are initialized.
//!
//! Step X5 adds the directives around the bindings: `x:DefaultBindMode` (the default mode of
//! the bindings of a subtree), `x:Phase` (the phase in which a container reads the bindings of an
//! element of a template instance, through [`IDataTemplateComponent`](ferroui_controls::templates::IDataTemplateComponent),
//! which [`XBindings`] implements), `x:Load` and `x:DeferLoadStrategy` (an element created when
//! it is loaded, in the place of a [`DeferredElement`]), and the roots of control templates (the
//! templated parent, of the `TargetType`) and of windows (initialized when activated).

mod binding;
mod bindings;
mod deferred_element;
mod event;
mod extension;
mod root_node;

pub use binding::XBinding;
pub use bindings::{XBindDiagnostic, XBindErrors, XBindings};
pub use deferred_element::DeferredElement;
pub use extension::XBindExtension;
