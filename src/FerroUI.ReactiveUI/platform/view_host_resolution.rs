//! `ViewHostResolution` (`Shared/ViewHostResolution.cs`).

use crate::{IViewFor, IViewLocator, ViewLocator};
use std::any::Any;
use std::rc::Rc;

/// A lookup of the view of a view model under a contract (`Func<IViewLocator, object, string?, IViewFor?>`).
pub type ResolveView = dyn Fn(&Rc<dyn IViewLocator>, Rc<dyn Any>, Option<&str>) -> Option<Rc<dyn IViewFor>>;

/// The view lookup that the view hosts of the platforms share (`ViewHostResolution`).
pub struct ViewHostResolution;

impl ViewHostResolution {
    /// The view of `view_model` under `contract`, found by the run-time type of the view model
    /// without building any type at run time (`ResolveViewWithoutReflection`).
    pub fn resolve_view_without_reflection(
        view_locator: &Rc<dyn IViewLocator>,
        view_model: Rc<dyn Any>,
        contract: Option<&str>,
    ) -> Option<Rc<dyn IViewFor>> {
        view_locator.resolve_view(Some(view_model), contract)
    }

    /// The view of `view_model` under `contract`, then under the default contract unless
    /// `contract_fallback_by_pass` (`ResolveViewWithFallback`); `view_locator` is the host's
    /// locator, or the current one when it is `None`.
    pub fn resolve_view_with_fallback(
        resolve_view: &ResolveView,
        view_locator: Option<Rc<dyn IViewLocator>>,
        view_model: Rc<dyn Any>,
        contract: Option<&str>,
        contract_fallback_by_pass: bool,
    ) -> Option<Rc<dyn IViewFor>> {
        let locator = view_locator.unwrap_or_else(ViewLocator::current);
        let view = resolve_view(&locator, view_model.clone(), contract);
        if view.is_none() && !contract_fallback_by_pass {
            return resolve_view(&locator, view_model, None);
        }
        view
    }

    /// The view of `view_model`, with the view model set on it (`ResolveAttachedView`); `None`
    /// when there is no view model or no view.
    pub fn resolve_attached_view(
        resolve_view: &ResolveView,
        view_locator: Option<Rc<dyn IViewLocator>>,
        view_model: Option<Rc<dyn Any>>,
        contract: Option<&str>,
        contract_fallback_by_pass: bool,
    ) -> Option<Rc<dyn IViewFor>> {
        let view_model = view_model?;
        let view = Self::resolve_view_with_fallback(resolve_view, view_locator, view_model.clone(), contract, contract_fallback_by_pass);
        if let Some(view) = &view {
            view.set_view_model(Some(view_model));
        }
        view
    }

    /// The warning a host logs when it finds no view for a view model (`NoViewFoundWarning`).
    pub fn no_view_found_warning(host_name: &str, view_model_type: &str, view_model: &dyn std::fmt::Debug, unsafe_host_name: &str) -> String {
        format!(
            "The {host_name} could not find a valid view for the view model of type {view_model_type} and value {view_model:?}. \
             The view locator checked the generated view lookup and its Map registrations; \
             use {unsafe_host_name} to also resolve a view registered only with the service locator."
        )
    }
}
