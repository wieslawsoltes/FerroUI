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
