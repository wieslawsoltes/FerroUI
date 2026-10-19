//! `ReactiveUI.Shared/Activation` and `Shared/ActivationSignals.cs`: the activation of views and
//! view models.
//!
//! A view is activated when it enters the visual tree and deactivated when it leaves it; a
//! registered [`IActivationForViewFetcher`] says when that happens for the views of a toolkit.
//! What should only live while a view is shown is set up with
//! [`ViewForMixins::when_activated`], and the view model it shows is activated with it through
//! its [`ViewModelActivator`].

mod activation_signals;
mod can_activate_view_fetcher;
mod i_activation_for_view_fetcher;
mod view_for_mixins;
mod view_model_activator;

pub use activation_signals::ActivationSignals;
pub use can_activate_view_fetcher::CanActivateViewFetcher;
pub use i_activation_for_view_fetcher::IActivationForViewFetcher;
pub use view_for_mixins::{register_activatable_view_model, ActivationDisposables, ViewForMixins, ViewModelActivationMixins, ViewModelValue};
pub use view_model_activator::ViewModelActivator;
