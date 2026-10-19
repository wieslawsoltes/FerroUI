//! `BuilderMixins` (`ReactiveUI.Shared/Mixins/BuilderMixins.cs`): the view registration, view
//! modules and converters of a builder.
//!
//! Not ported: `BuildApp()` of an `IAppBuilder`, which casts the builder to `IReactiveUIBuilder`; a
//! Splat builder cannot be cast to another trait. A ReactiveUI builder has
//! [`build_app`](super::IReactiveUIBuilder::build_app) itself. `UsingSplatBuilder(Action<IAppBuilder>?)`
//! is [`IReactiveUIBuilder::using_splat_builder`](super::IReactiveUIBuilder::using_splat_builder),
//! which takes an optional action as this mixin does.

use super::{IReactiveUIBuilder, IViewModule, ReactiveUIBuilder};
use crate::{DefaultViewLocator, IBindingTypeConverter, IViewLocator, ViewMappingBuilder};
use ferroui_splat::{AppLocator, ReadonlyDependencyResolverExt};
use std::any::Any;
use std::rc::Rc;

/// The registered [`DefaultViewLocator`], for `RegisterViews` and `WithViewModule`.
///
/// # Panics
/// When the view locator is not a [`DefaultViewLocator`] (`InvalidOperationException`).
fn registered_default_view_locator(method: &str) -> Rc<DefaultViewLocator> {
    AppLocator::current()
        .get_service::<dyn IViewLocator>()
        .and_then(|locator| (locator as Rc<dyn Any>).downcast::<DefaultViewLocator>().ok())
        .unwrap_or_else(|| {
            panic!(
                "InvalidOperationException: DefaultViewLocator must be registered before calling {method}. \
                 Ensure you've called WithPlatformModule() or manually registered DefaultViewLocator."
            )
        })
}

/// The view registration, view modules and converters of a builder (`BuilderMixins`).
pub trait ReactiveUIBuilderMixins {
    /// Maps views to view models on the registered [`DefaultViewLocator`], through its mapping
    /// builder (`RegisterViews(Action<ViewMappingBuilder>)`).
    ///
    /// # Panics
    /// When the registered view locator is not a [`DefaultViewLocator`]
    /// (`InvalidOperationException`).
    fn register_views(&self, configure: impl FnOnce(ViewMappingBuilder<'_>)) -> &Self;

    /// Maps the views of a new `M` on the registered [`DefaultViewLocator`]
    /// (`WithViewModule<TModule>()`).
    ///
    /// # Panics
    /// When the registered view locator is not a [`DefaultViewLocator`]
    /// (`InvalidOperationException`).
    fn with_view_module<M: IViewModule + Default>(&self) -> &Self;

    /// Registers each of `converters` (`WithConverters(params IBindingTypeConverter[])`).
    fn with_converters(&self, converters: &[Rc<dyn IBindingTypeConverter>]) -> &Self;
}

impl ReactiveUIBuilderMixins for ReactiveUIBuilder {
    fn register_views(&self, configure: impl FnOnce(ViewMappingBuilder<'_>)) -> &Self {
        let view_locator = registered_default_view_locator("RegisterViews");
        configure(view_locator.create_mapping_builder());
        self
    }

    fn with_view_module<M: IViewModule + Default>(&self) -> &Self {
        let view_locator = registered_default_view_locator("WithViewModule");
        M::default().register_views(&view_locator);
        self
    }

    fn with_converters(&self, converters: &[Rc<dyn IBindingTypeConverter>]) -> &Self {
        for converter in converters {
            let _ = self.with_converter(converter.clone());
        }
        self
    }
}

impl ReactiveUIBuilderMixins for dyn IReactiveUIBuilder + '_ {
    fn register_views(&self, configure: impl FnOnce(ViewMappingBuilder<'_>)) -> &Self {
        let view_locator = registered_default_view_locator("RegisterViews");
        configure(view_locator.create_mapping_builder());
        self
    }

    fn with_view_module<M: IViewModule + Default>(&self) -> &Self {
        let view_locator = registered_default_view_locator("WithViewModule");
        M::default().register_views(&view_locator);
        self
    }

    fn with_converters(&self, converters: &[Rc<dyn IBindingTypeConverter>]) -> &Self {
        for converter in converters {
            let _ = self.with_converter(converter.clone());
        }
        self
    }
}
