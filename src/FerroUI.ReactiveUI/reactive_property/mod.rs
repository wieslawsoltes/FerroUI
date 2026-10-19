//! `ReactiveProperty` (`ReactiveUI.Core/ReactiveProperty`, `ReactiveUI.Shared/ReactiveProperty`):
//! a property that is its own observable, with validation through `INotifyDataErrorInfo`, and the
//! shared arguments of its notifications.

mod i_reactive_property;
#[allow(clippy::module_inception)]
mod reactive_property;
mod reactive_property_mixins;
mod singleton_data_errors_changed_event_args;
mod singleton_property_changed_event_args;

pub use i_reactive_property::IReactiveProperty;
pub use reactive_property::{DataErrors, IntoValidationErrors, ReactiveProperty, ValidationErrors};
pub use reactive_property_mixins::ReactivePropertyMixins;
pub use singleton_data_errors_changed_event_args::SingletonDataErrorsChangedEventArgs;
pub use singleton_property_changed_event_args::SingletonPropertyChangedEventArgs;
