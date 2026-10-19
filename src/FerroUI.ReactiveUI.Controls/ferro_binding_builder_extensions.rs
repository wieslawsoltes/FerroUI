//! `FerroBindingBuilderExtensions` (ReactiveUI.Binding's integration,
//! `Builder/...BindingBuilderExtensions.cs`).

use crate::FerroBindingModule;
use ferroui_reactiveui::{IReactiveUIBindingBuilder, ReactiveUIBindingBuilder};
use ferroui_splat::IAppBuilder;
use std::rc::Rc;

/// The message of the panic when an `IAppBuilder` is not a builder of ReactiveUI.Binding (the cast
/// of upstream's `WithFerro` on an `IAppBuilder`).
const NOT_BINDING_BUILDER_MESSAGE: &str = "InvalidCastException: The provided IAppBuilder is not an IReactiveUIBindingBuilder.";

/// The FerroUI members of the builder of ReactiveUI.Binding (`FerroBindingBuilderExtensions`):
/// `with_ferro` registers the [`FerroBindingModule`], which adds the property observation of
/// FerroUI objects, the command binding of FerroUI controls and the view thread invoker.
///
/// Implemented for the object-safe builder (`&dyn IReactiveUIBindingBuilder`), which it returns,
/// and for the builder itself.
pub trait FerroBindingBuilderExtensions {
    /// Registers the [`FerroBindingModule`] (`WithFerro`); returns the builder.
    fn with_ferro(&self) -> &dyn IReactiveUIBindingBuilder;
}

impl FerroBindingBuilderExtensions for dyn IReactiveUIBindingBuilder + '_ {
    fn with_ferro(&self) -> &dyn IReactiveUIBindingBuilder {
        self.with_platform_module(Rc::new(FerroBindingModule::new()))
    }
}

impl FerroBindingBuilderExtensions for ReactiveUIBindingBuilder {
    fn with_ferro(&self) -> &dyn IReactiveUIBindingBuilder {
        IReactiveUIBindingBuilder::with_platform_module(self, Rc::new(FerroBindingModule::new()))
    }
}

/// `with_ferro` on an application builder that is a builder of ReactiveUI.Binding
/// (`WithFerro` on `IAppBuilder`, which casts the builder).
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): Rust cannot cast a trait object to another
/// trait; the builder is found by its address among the builders of the thread
/// ([`ReactiveUIBindingBuilder::find`]) and returned as its `Rc`.
pub trait FerroAppBuilderExtensions {
    /// Registers the [`FerroBindingModule`] with the builder of ReactiveUI.Binding behind the
    /// application builder (`WithFerro`); returns that builder.
    ///
    /// # Panics
    /// Panics when the application builder is not a builder of ReactiveUI.Binding.
    fn with_ferro(&self) -> Rc<ReactiveUIBindingBuilder>;
}

impl FerroAppBuilderExtensions for dyn IAppBuilder + '_ {
    fn with_ferro(&self) -> Rc<ReactiveUIBindingBuilder> {
        let Some(builder) = ReactiveUIBindingBuilder::find(self) else {
            panic!("{NOT_BINDING_BUILDER_MESSAGE}");
        };
        builder.with_platform_module(FerroBindingModule::new())
    }
}
