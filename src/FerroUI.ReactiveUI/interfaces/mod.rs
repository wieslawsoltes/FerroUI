//! The interfaces of ReactiveUI.Core and ReactiveUI.Shared (`Interfaces/`) that are not those of
//! commands, routing, suspension or activation: the change notifications of reactive objects and
//! the errors of their pipelines.

mod i_handle_observable_errors;
mod i_reactive_notify_property_changed;
mod i_reactive_property_changed_event_args;
mod reactive_property_changed_event_args;
mod reactive_property_changing_event_args;

pub use i_handle_observable_errors::IHandleObservableErrors;
pub use i_reactive_notify_property_changed::IReactiveNotifyPropertyChanged;
pub use i_reactive_property_changed_event_args::IReactivePropertyChangedEventArgs;
pub use reactive_property_changed_event_args::ReactivePropertyChangedEventArgs;
pub use reactive_property_changing_event_args::ReactivePropertyChangingEventArgs;
mod i_activatable_view_model;
mod i_can_activate;
mod i_can_force_manual_activation;
mod i_screen;
mod i_routable_view_model;
mod i_message_bus;
mod i_suspension_driver;
mod i_suspension_host;
mod i_suspension_host_t_app_state;
pub use i_activatable_view_model::IActivatableViewModel;
pub use i_can_activate::ICanActivate;
pub use i_can_force_manual_activation::ICanForceManualActivation;
pub use i_message_bus::{IMessageBus, MessageKey};
pub use i_routable_view_model::IRoutableViewModel;
pub use i_screen::IScreen;
pub use i_suspension_driver::{ISuspensionDriver, ISuspensionDriverOf};
pub use i_suspension_host::{AppStateFactory, ISuspensionHost};
pub use i_suspension_host_t_app_state::{ISuspensionHostOf, TypedAppStateFactory};
