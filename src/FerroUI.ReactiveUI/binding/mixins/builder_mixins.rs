//! `BuilderMixins` (ReactiveUI.Binding, `Mixins/BuilderMixins.cs`).

use crate::binding::builder::{IReactiveUIBindingInstance, ReactiveUIBindingBuilder};
use crate::binding::view::ViewMappingBuilder;
use crate::{IBindingFallbackConverter, IBindingTypeConverter, ISetMethodBindingConverter};
use ferroui_splat::{IAppBuilder, IModule, IMutableDependencyResolver};
use std::rc::Rc;

/// The message of the panic when an `IAppBuilder` is not a builder of ReactiveUI.Binding
/// (`NotBindingBuilderMessage`).
const NOT_BINDING_BUILDER_MESSAGE: &str =
    "InvalidOperationException: The provided IAppBuilder is not an IReactiveUIBindingBuilder. Ensure you are using the ReactiveUI.Binding builder pattern.";

/// The builder of ReactiveUI.Binding behind `app_builder`; it panics with an
/// `InvalidOperationException` message when there is none.
fn binding_builder(app_builder: &dyn IAppBuilder) -> Rc<ReactiveUIBindingBuilder> {
    match ReactiveUIBindingBuilder::find(app_builder) {
        Some(builder) => builder,
        None => panic!("{NOT_BINDING_BUILDER_MESSAGE}"),
    }
}

/// Continues a chain of a builder of ReactiveUI.Binding from a `&dyn IAppBuilder`
/// (`BuilderMixins`), such as the one [`IAppBuilder::with_core_services`] returns. Each method
/// panics with an `InvalidOperationException` message when the builder is not a
/// [`ReactiveUIBindingBuilder`].
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream casts the `IAppBuilder` to
/// `IReactiveUIBindingBuilder`; Rust cannot cast a trait object to another trait, so the builder is
/// found among the builders of the thread by its address
/// ([`ReactiveUIBindingBuilder::find`]). The methods return the builder (`Rc`).
pub trait BuilderMixins {
    /// Builds the application (`BuildApp`).
    fn build_app(&self) -> Rc<dyn IReactiveUIBindingInstance>;

    /// Adds the module of a platform (`WithPlatformModule<T>`).
    fn with_platform_module<M: IModule + 'static>(&self, module: M) -> Rc<ReactiveUIBindingBuilder>;

    /// Runs `configure_action` on the resolver now (`WithRegistration`).
    fn with_registration(&self, configure_action: impl FnOnce(&dyn IMutableDependencyResolver)) -> Rc<ReactiveUIBindingBuilder>;

    /// Registers a converter of a pair of types (`WithConverter`).
    fn with_converter(&self, converter: Rc<dyn IBindingTypeConverter>) -> Rc<ReactiveUIBindingBuilder>;

    /// Registers a fallback converter (`WithFallbackConverter`).
    fn with_fallback_converter(&self, converter: Rc<dyn IBindingFallbackConverter>) -> Rc<ReactiveUIBindingBuilder>;

    /// Registers a converter of set methods (`WithSetMethodConverter`).
    fn with_set_method_converter(&self, converter: Rc<dyn ISetMethodBindingConverter>) -> Rc<ReactiveUIBindingBuilder>;

    /// Registers a view locator with the mappings `configure` adds (`ConfigureViewLocator`).
    fn configure_view_locator(&self, configure: impl FnOnce(&ViewMappingBuilder<'_>)) -> Rc<ReactiveUIBindingBuilder>;
}

impl BuilderMixins for dyn IAppBuilder + '_ {
    fn build_app(&self) -> Rc<dyn IReactiveUIBindingInstance> {
        binding_builder(self).build_app()
    }

    fn with_platform_module<M: IModule + 'static>(&self, module: M) -> Rc<ReactiveUIBindingBuilder> {
        binding_builder(self).with_platform_module(module)
    }

    fn with_registration(&self, configure_action: impl FnOnce(&dyn IMutableDependencyResolver)) -> Rc<ReactiveUIBindingBuilder> {
        binding_builder(self).with_registration(configure_action)
    }

    fn with_converter(&self, converter: Rc<dyn IBindingTypeConverter>) -> Rc<ReactiveUIBindingBuilder> {
        binding_builder(self).with_converter(converter)
    }

    fn with_fallback_converter(&self, converter: Rc<dyn IBindingFallbackConverter>) -> Rc<ReactiveUIBindingBuilder> {
        binding_builder(self).with_fallback_converter(converter)
    }

    fn with_set_method_converter(&self, converter: Rc<dyn ISetMethodBindingConverter>) -> Rc<ReactiveUIBindingBuilder> {
        binding_builder(self).with_set_method_converter(converter)
    }

    fn configure_view_locator(&self, configure: impl FnOnce(&ViewMappingBuilder<'_>)) -> Rc<ReactiveUIBindingBuilder> {
        binding_builder(self).configure_view_locator(configure)
    }
}
