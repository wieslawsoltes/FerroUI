//! `RxAppBuilderMixins` (`ReactiveUI.Shared/Builder/RxAppBuilderMixins.cs`).

use super::ReactiveUIBuilder;
use ferroui_splat::{AppLocator, IDependencyResolver, IMutableDependencyResolver, IReadonlyDependencyResolver};
use std::rc::Rc;

/// A ReactiveUI builder over a resolver (`RxAppBuilderMixins`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream uses the resolver as the resolver of the
/// application when it is also an `IReadonlyDependencyResolver` (`resolver as
/// IReadonlyDependencyResolver`), and the current one of the locator otherwise. A trait object
/// cannot be cast to another trait, so the handles of resolvers whose type says what they are
/// implement this trait: a resolver of a concrete type that reads and registers, and a
/// `dyn IDependencyResolver`, are the resolvers of their applications; a
/// `dyn IMutableDependencyResolver` uses the current one of the locator.
pub trait RxAppBuilderMixins {
    /// A builder over the resolver (`CreateReactiveUIBuilder()`).
    fn create_reactive_ui_builder(&self) -> ReactiveUIBuilder;
}

impl<R: IMutableDependencyResolver + IReadonlyDependencyResolver + 'static> RxAppBuilderMixins for Rc<R> {
    fn create_reactive_ui_builder(&self) -> ReactiveUIBuilder {
        ReactiveUIBuilder::new(self.clone(), Some(self.clone()))
    }
}

impl RxAppBuilderMixins for Rc<dyn IDependencyResolver> {
    fn create_reactive_ui_builder(&self) -> ReactiveUIBuilder {
        let resolver: Rc<dyn IMutableDependencyResolver> = self.clone();
        let current: Rc<dyn IReadonlyDependencyResolver> = self.clone();
        ReactiveUIBuilder::new(resolver, Some(current))
    }
}

impl RxAppBuilderMixins for Rc<dyn IMutableDependencyResolver> {
    fn create_reactive_ui_builder(&self) -> ReactiveUIBuilder {
        ReactiveUIBuilder::new(self.clone(), Some(AppLocator::current()))
    }
}
