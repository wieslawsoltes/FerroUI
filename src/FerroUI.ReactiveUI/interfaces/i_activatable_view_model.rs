//! `IActivatableViewModel` (`ReactiveUI.Shared/Interfaces/IActivatableViewModel.cs`).

use crate::ViewModelActivator;

/// A view model that is interested in its activation (`IActivatableViewModel`): it holds a
/// [`ViewModelActivator`] and registers, with
/// [`when_activated`](crate::ViewModelActivationMixins::when_activated), what to do while the
/// view that shows it is active. See [`ViewModelActivator`].
///
/// ```
/// use ferroui_reactive::disposables::Disposable;
/// use ferroui_reactiveui::{IActivatableViewModel, ViewModelActivationMixins, ViewModelActivator};
/// use ferroui_system::IDisposable;
/// use std::cell::Cell;
/// use std::rc::Rc;
///
/// struct DetailsViewModel {
///     activator: ViewModelActivator,
///     active: Rc<Cell<bool>>,
/// }
///
/// impl IActivatableViewModel for DetailsViewModel {
///     fn activator(&self) -> &ViewModelActivator {
///         &self.activator
///     }
/// }
///
/// let view_model = DetailsViewModel { activator: ViewModelActivator::new(), active: Rc::new(Cell::new(false)) };
/// let active = view_model.active.clone();
/// view_model.when_activated(move || {
///     active.set(true);
///     let active = active.clone();
///     vec![Disposable::create(move || active.set(false))]
/// });
///
/// let activation = view_model.activator().activate();
/// assert!(view_model.active.get());
/// activation.dispose();
/// assert!(!view_model.active.get());
/// ```
pub trait IActivatableViewModel: 'static {
    /// The activator the view uses when it is activated and deactivated (`Activator`).
    fn activator(&self) -> &ViewModelActivator;
}
