//! `RegisterViewForExt`: the registration of a view in the service locator as
//! `IViewFor<TViewModel>`, recorded so that a view model held as `dyn Any` finds it.
//!
//! Not an upstream file: upstream registers `IViewFor<TViewModel>` with Splat's `Register`, and
//! [`IViewLocator::resolve_view_unsafe`](crate::IViewLocator::resolve_view_unsafe) closes
//! `IViewFor<>` over the run-time type of a view model by reflection.

use super::record_view_model_type;
use crate::binding::interfaces::{IViewForOf, ViewModelType};
use ferroui_splat::{IMutableDependencyResolver, MutableDependencyResolverExt};
use std::rc::Rc;

/// Registers the views of a view model type in a resolver.
pub trait RegisterViewForExt {
    /// Registers `factory` as `dyn IViewForOf<VM>` under `contract`
    /// (`Register<IViewFor<TViewModel>>(Func<...>, string?)`), and records the view model type
    /// `VM`, so that [`resolve_view_unsafe`](crate::IViewLocator::resolve_view_unsafe) of a
    /// [`DefaultViewLocator`](crate::DefaultViewLocator) asks for it when a view model of that type
    /// is held as `dyn Any`.
    fn register_view_for<VM: 'static>(&self, factory: impl Fn() -> Rc<dyn IViewForOf<VM>> + 'static, contract: Option<&str>);
}

impl<R: IMutableDependencyResolver + ?Sized> RegisterViewForExt for R {
    fn register_view_for<VM: 'static>(&self, factory: impl Fn() -> Rc<dyn IViewForOf<VM>> + 'static, contract: Option<&str>) {
        record_view_model_type(ViewModelType::of::<VM>());
        self.register_with_contract::<dyn IViewForOf<VM>>(move || Some(factory()), contract);
    }
}
