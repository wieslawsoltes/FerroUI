//! `System.ComponentModel`: property change notification (`INotifyPropertyChanged`,
//! `INotifyPropertyChanging`) and the backing fields of notifying properties.

mod backing_field;
mod i_notify_property_changed;
mod i_notify_property_changing;
mod property_path;

pub use backing_field::BackingField;
pub use i_notify_property_changed::INotifyPropertyChanged;
pub use i_notify_property_changing::INotifyPropertyChanging;
pub use property_path::{property_name_of, PathStep, PropertyOwner, PropertyPath};
mod data_errors_changed_event_args;
mod i_notify_data_error_info;
pub use data_errors_changed_event_args::DataErrorsChangedEventArgs;
pub use i_notify_data_error_info::INotifyDataErrorInfo;
