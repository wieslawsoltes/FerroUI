//! `IActivationForViewFetcher` (`ReactiveUI.Shared/Activation/IActivationForViewFetcher.cs`).

use crate::IActivatableView;
use ferroui_system::IObservable;
use std::rc::Rc;

/// Tells when a view is activated and deactivated (`IActivationForViewFetcher`); usually only
/// the integration of a user-interface toolkit implements it.
///
/// A fetcher translates the signals of a toolkit (the view entering or leaving the visual tree,
/// navigation, visibility) into activation. Several fetchers are registered in `AppLocator` as
/// `dyn IActivationForViewFetcher`; for a view, the one with the highest affinity is used.
///
/// The integration with FerroUI registers the fetcher of its controls; a test registers a fetcher
/// of its own, as upstream's tests do.
// Deviation (DEVIATIONS.md, ReactiveUI): upstream computes the affinity on the `Type` of the
// view; without reflection it is computed on the view itself, as for
// `ICreatesObservableForProperty`.
pub trait IActivationForViewFetcher {
    /// How well this fetcher can activate `view`: zero when it cannot, a positive score when it
    /// can, where the highest score wins (`GetAffinityForView`).
    fn get_affinity_for_view(&self, view: &dyn IActivatableView) -> i32;

    /// The activation of `view`: `true` when it is activated, `false` when it is deactivated
    /// (`GetActivationForView`). Called after [`get_affinity_for_view`](Self::get_affinity_for_view).
    fn get_activation_for_view(&self, view: &Rc<dyn IActivatableView>) -> Rc<dyn IObservable<bool>>;
}
