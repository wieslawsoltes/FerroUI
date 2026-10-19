//! `ViewMappingBuilder` (ReactiveUI.Binding, `View/ViewMappingBuilder.cs`).

use super::DefaultViewLocator;
use crate::binding::interfaces::{IntoViewFor, IViewFor};
use ferroui_splat::{AppLocator, ReadonlyDependencyResolverExt};
use std::fmt;
use std::rc::Rc;

/// Registers the mappings of view models to views on a [`DefaultViewLocator`], each method
/// returning the builder to chain the calls (`ViewMappingBuilder`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the builder borrows its locator. The overloads without a
/// contract are the same methods with `None`.
pub struct ViewMappingBuilder<'a> {
    /// `_locator`.
    locator: &'a DefaultViewLocator,
}

impl fmt::Debug for ViewMappingBuilder<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewMappingBuilder").field("locator", self.locator).finish()
    }
}

impl<'a> ViewMappingBuilder<'a> {
    /// A builder that registers on `locator` (`ViewMappingBuilder(DefaultViewLocator)`).
    pub fn new(locator: &'a DefaultViewLocator) -> Self {
        Self { locator }
    }

    /// Maps the view model type `VM`, under `contract` (`None` for the default), to a new view of
    /// type `V`, replacing an existing mapping (`Map<TViewModel, TView>(string?)`).
    pub fn map<VM: 'static, V: IViewFor + Default>(&self, contract: Option<&str>) -> &Self {
        self.locator.map::<VM, V>(contract);
        self
    }

    /// Maps the view model type `VM`, under `contract`, to the view `factory` creates
    /// (`Map<TViewModel>(Func<IViewFor>, string?)`).
    pub fn map_factory<VM: 'static>(&self, factory: impl Fn() -> Rc<dyn IViewFor> + 'static, contract: Option<&str>) -> &Self {
        self.locator.map_factory::<VM>(factory, contract);
        self
    }

    /// Maps the view model type `VM`, under `contract`, to the view the service locator gives for
    /// the service type `V` under `service_contract` (`None` for the registration without a
    /// contract), asked each time the mapping resolves
    /// (`MapFromServiceLocator<TViewModel, TView>(string?, string?)`). `V` is a view type or a
    /// trait object of one (`dyn IViewForOf<VM>`).
    ///
    /// A view that is not registered panics with an `InvalidOperationException` message when the
    /// mapping resolves, rather than resolving to nothing or to a view registered under another
    /// contract.
    pub fn map_from_service_locator<VM: 'static, V: IntoViewFor + ?Sized + 'static>(&self, contract: Option<&str>, service_contract: Option<&str>) -> &Self {
        let service_contract: Option<String> = service_contract.map(str::to_owned);
        self.locator.map_factory::<VM>(
            move || match AppLocator::current().get_service_with_contract::<V>(service_contract.as_deref()) {
                Some(view) => view.into_view_for(),
                None => match &service_contract {
                    None => panic!("InvalidOperationException: View {} is not registered in the service locator.", std::any::type_name::<V>()),
                    Some(service_contract) => panic!(
                        "InvalidOperationException: View {} is not registered in the service locator under contract '{service_contract}'.",
                        std::any::type_name::<V>()
                    ),
                },
            },
            contract,
        );
        self
    }
}
