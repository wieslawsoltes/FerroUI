//! `RxBindingBuilderMixins` (ReactiveUI.Binding, `Builder/RxBindingBuilderMixins.cs`).

use super::ReactiveUIBindingBuilder;
use ferroui_splat::{AppLocator, IDependencyResolver, IMutableDependencyResolver, IReadonlyDependencyResolver};
use std::rc::Rc;

/// Creates a [`ReactiveUIBindingBuilder`] over a resolver (`RxBindingBuilderMixins`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream asks at run time whether the resolver also
/// resolves (`resolver as IReadonlyDependencyResolver`); here the type answers. A resolver that
/// both registers and resolves reads its services from itself; an
/// `Rc<dyn IMutableDependencyResolver>` reads them from the current resolver of [`AppLocator`].
pub trait RxBindingBuilderMixins {
    /// A builder that registers into the resolver (`CreateReactiveUIBindingBuilder`).
    fn create_reactive_ui_binding_builder(&self) -> Rc<ReactiveUIBindingBuilder>;
}

impl<R: IMutableDependencyResolver + IReadonlyDependencyResolver + 'static> RxBindingBuilderMixins for Rc<R> {
    fn create_reactive_ui_binding_builder(&self) -> Rc<ReactiveUIBindingBuilder> {
        ReactiveUIBindingBuilder::new(self.clone(), Some(self.clone()))
    }
}

impl RxBindingBuilderMixins for Rc<dyn IDependencyResolver> {
    fn create_reactive_ui_binding_builder(&self) -> Rc<ReactiveUIBindingBuilder> {
        ReactiveUIBindingBuilder::new(self.clone(), Some(self.clone()))
    }
}

impl RxBindingBuilderMixins for Rc<dyn IMutableDependencyResolver> {
    fn create_reactive_ui_binding_builder(&self) -> Rc<ReactiveUIBindingBuilder> {
        ReactiveUIBindingBuilder::new(self.clone(), Some(AppLocator::current()))
    }
}
