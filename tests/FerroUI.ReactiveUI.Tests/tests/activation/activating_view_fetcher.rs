//! `ActivatingViewFetcher` (`ReactiveUI.Tests/Activation/ActivatingViewFetcher.cs`).

use super::ActivatingView;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::binding::interfaces::IActivatableView;
use ferroui_reactiveui::*;
use ferroui_splat::AppLocator;
use ferroui_system::IObservable;
use std::any::Any;
use std::rc::Rc;

/// The affinity of the fetcher for an [`ActivatingView`] (`LargeAffinity`).
const LARGE_AFFINITY: i32 = 100;

/// Activates an [`ActivatingView`] with its `Loaded` and `Unloaded` subjects
/// (`ActivatingViewFetcher`).
pub(crate) struct ActivatingViewFetcher;

impl IActivationForViewFetcher for ActivatingViewFetcher {
    fn get_affinity_for_view(&self, view: &dyn IActivatableView) -> i32 {
        let view: &dyn Any = view;
        if view.is::<ActivatingView>() {
            LARGE_AFFINITY
        } else {
            0
        }
    }

    /// Panics for another view (upstream's `ArgumentNullException`).
    fn get_activation_for_view(&self, view: &Rc<dyn IActivatableView>) -> Rc<dyn IObservable<bool>> {
        let view: Rc<dyn Any> = view.clone();
        let Ok(view) = view.downcast::<ActivatingView>() else {
            panic!("ArgumentNullException: view");
        };
        let loaded = view.loaded().as_observable().select(|_: ()| true);
        let unloaded = view.unloaded().as_observable().select(|_: ()| false);
        loaded.merge(unloaded)
    }
}

/// Registers an [`ActivatingViewFetcher`] in the locator of the thread: the part of upstream's
/// `CreateReactiveUIBuilder().WithCoreServices().WithCustomRegistration(...)` that the tests of
/// activation need (the core services of the observation of properties are the defaults while
/// none is registered).
pub(crate) fn register_activating_view_fetcher() {
    AppLocator::register::<dyn IActivationForViewFetcher>(|| Some(Rc::new(ActivatingViewFetcher) as Rc<dyn IActivationForViewFetcher>));
}
