//! `IReactiveUIBindingBuilder` (ReactiveUI.Binding, `Builder/IReactiveUIBindingBuilder.cs`).

use super::IReactiveUIBindingInstance;
use crate::binding::interfaces::ICreatesCommandBinding;
use crate::binding::view::ViewMappingBuilder;
use crate::{IBindingFallbackConverter, IBindingTypeConverter, ISetMethodBindingConverter};
use ferroui_splat::{IAppBuilder, IModule, IMutableDependencyResolver};
use std::rc::Rc;

/// Configures the services, converters and platform modules of ReactiveUI.Binding, then builds
/// the application (`IReactiveUIBindingBuilder`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the trait is object safe, so its methods take trait
/// objects and return `&dyn IReactiveUIBindingBuilder`. `WithCoreServices`, which upstream
/// redeclares to return this builder, is [`IAppBuilder::with_core_services`]; the inherent methods
/// of [`ReactiveUIBindingBuilder`](super::ReactiveUIBindingBuilder) return the builder itself, and
/// [`BuilderMixins`](crate::BuilderMixins) continues a chain from a `&dyn IAppBuilder`.
pub trait IReactiveUIBindingBuilder: IAppBuilder {
    /// Adds the module of a platform, applied at build time (`WithPlatformModule<T>(T)`).
    fn with_platform_module(&self, module: Rc<dyn IModule>) -> &dyn IReactiveUIBindingBuilder;

    /// Runs `configure_action` on the resolver now (`WithRegistration`).
    fn with_registration(&self, configure_action: &dyn Fn(&dyn IMutableDependencyResolver)) -> &dyn IReactiveUIBindingBuilder;

    /// Registers a converter of a pair of types (`WithConverter`).
    fn with_converter(&self, converter: Rc<dyn IBindingTypeConverter>) -> &dyn IReactiveUIBindingBuilder;

    /// Registers a fallback converter (`WithFallbackConverter`).
    fn with_fallback_converter(&self, converter: Rc<dyn IBindingFallbackConverter>) -> &dyn IReactiveUIBindingBuilder;

    /// Registers a converter of set methods (`WithSetMethodConverter`).
    fn with_set_method_converter(&self, converter: Rc<dyn ISetMethodBindingConverter>) -> &dyn IReactiveUIBindingBuilder;

    /// Registers a binder of commands to controls (`WithCommandBinder`).
    fn with_command_binder(&self, binder: Rc<dyn ICreatesCommandBinding>) -> &dyn IReactiveUIBindingBuilder;

    /// Registers a [`DefaultViewLocator`](crate::DefaultViewLocator) with the mappings `configure`
    /// adds (`ConfigureViewLocator`).
    fn configure_view_locator(&self, configure: &dyn Fn(&ViewMappingBuilder<'_>)) -> &dyn IReactiveUIBindingBuilder;

    /// Builds the application, publishes the converter service and marks ReactiveUI.Binding
    /// initialized (`BuildApp`).
    fn build_app(&self) -> &dyn IReactiveUIBindingInstance;
}
