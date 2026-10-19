//! `IActivatableView` (ReactiveUI.Binding, `Interfaces/IActivatableView.cs`).

use super::IViewFor;
use ferroui_system::component_model::INotifyPropertyChanged;
use std::any::Any;
use std::rc::Rc;

/// The marker of a view that can be activated (`IActivatableView`); every
/// [`IViewFor`] is one. Upstream declares no members.
///
/// It is `Any`, so that the code that activates a view (the activation fetchers) can find the
/// concrete type of the view behind a `&dyn IActivatableView`.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream asks a view at run time whether it is an
/// `IViewFor` and whether it raises `PropertyChanged` (`view is IViewFor`, `view is
/// INotifyPropertyChanged`), which Rust cannot ask of a trait object. The two provided methods
/// answer it; a view overrides them, and [`view_for!`](crate::view_for) writes them.
pub trait IActivatableView: Any {
    /// The view as an [`IViewFor`], when it is one (`view as IViewFor`); `None` by default.
    fn as_view_for(&self) -> Option<&dyn IViewFor> {
        None
    }

    /// The change notification of the view, when it raises `PropertyChanged`
    /// (`view as INotifyPropertyChanged`); `None` by default. The property observation of the
    /// view's `ViewModel` uses it.
    fn as_notify_property_changed(self: Rc<Self>) -> Option<Rc<dyn INotifyPropertyChanged>> {
        None
    }

    /// The view as an [`ICanActivate`](crate::ICanActivate), when it is one
    /// (`view as ICanActivate`); `None` by default. `CanActivateViewFetcher` uses it.
    fn as_can_activate(&self) -> Option<&dyn crate::ICanActivate> {
        None
    }
}
