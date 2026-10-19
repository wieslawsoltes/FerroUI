//! `ViewLocatorMixins` (ReactiveUI.Binding, `Interfaces/ViewLocatorMixins.cs`).

use super::{IViewFor, IViewForOf, IViewLocator, ViewModelType};
use crate::binding::view::DefaultViewLocator;
use ferroui_splat::{AppLocator, ReadonlyDependencyResolverExt};
use std::any::Any;
use std::rc::Rc;

/// The generic resolutions of a view locator (`ViewLocatorMixins`), implemented for
/// `dyn IViewLocator` and every locator.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's overloads that supply the default contract
/// are the same methods with `None`; the untyped ones (`ResolveView(object?)`,
/// `ResolveViewUnsafe(object?)`) are [`IViewLocator::resolve_view`] and
/// [`IViewLocator::resolve_view_unsafe`] with `None`. The null checks of the locator have no
/// counterpart.
pub trait ViewLocatorMixins {
    /// Resolves the view of `view_model` through its compile-time type `VM`
    /// (`ResolveView<TViewModel>(TViewModel, string?)`); the view's view model is set.
    fn resolve_view_of<VM: 'static>(&self, view_model: &Rc<VM>, contract: Option<&str>) -> Option<Rc<dyn IViewFor>>;

    /// Resolves a view of the type `VM` without a view model (`ResolveView<TViewModel>(string?)`):
    /// a [`DefaultViewLocator`] asks its mappings, then the service locator; any other locator has
    /// no lookup by type, so the service locator is asked for `dyn IViewForOf<VM>` directly. The
    /// view's view model is left unset.
    fn resolve_view_for<VM: 'static>(&self, contract: Option<&str>) -> Option<Rc<dyn IViewForOf<VM>>>;
}

fn resolve_view_of<VM: 'static>(locator: &dyn IViewLocator, view_model: &Rc<VM>, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
    locator.resolve_view_typed(view_model.clone() as Rc<dyn Any>, &ViewModelType::of::<VM>(), contract)
}

fn resolve_view_for<VM: 'static>(locator: &dyn IViewLocator, contract: Option<&str>) -> Option<Rc<dyn IViewForOf<VM>>> {
    let any: &dyn Any = locator;
    match any.downcast_ref::<DefaultViewLocator>() {
        Some(default_locator) => default_locator.resolve_view_for::<VM>(contract),
        None => AppLocator::current().get_service_with_contract::<dyn IViewForOf<VM>>(contract.filter(|contract| !contract.is_empty())),
    }
}

impl ViewLocatorMixins for dyn IViewLocator {
    fn resolve_view_of<VM: 'static>(&self, view_model: &Rc<VM>, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        resolve_view_of(self, view_model, contract)
    }

    fn resolve_view_for<VM: 'static>(&self, contract: Option<&str>) -> Option<Rc<dyn IViewForOf<VM>>> {
        resolve_view_for::<VM>(self, contract)
    }
}

impl<L: IViewLocator> ViewLocatorMixins for L {
    fn resolve_view_of<VM: 'static>(&self, view_model: &Rc<VM>, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        resolve_view_of(self, view_model, contract)
    }

    fn resolve_view_for<VM: 'static>(&self, contract: Option<&str>) -> Option<Rc<dyn IViewForOf<VM>>> {
        resolve_view_for::<VM>(self, contract)
    }
}
