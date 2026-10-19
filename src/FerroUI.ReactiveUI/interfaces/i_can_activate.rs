//! `ICanActivate` (`ReactiveUI.Shared/Interfaces/ICanActivate.cs`).

use ferroui_system::IObservable;
use std::rc::Rc;

/// An object that says itself when it is activated and deactivated (`ICanActivate`); a view that
/// implements it is activated by [`CanActivateViewFetcher`](crate::CanActivateViewFetcher).
/// Usually only the integration of a user-interface toolkit implements it.
///
/// A view tells the activation fetchers that it implements this trait through
/// `IActivatableView::as_can_activate`.
pub trait ICanActivate {
    /// Ticks when the object is activated (`Activated`).
    fn activated(&self) -> Rc<dyn IObservable<()>>;

    /// Ticks when the object is deactivated (`Deactivated`).
    fn deactivated(&self) -> Rc<dyn IObservable<()>>;
}
