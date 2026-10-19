//! `System.ComponentModel`: property change notification.

mod i_notify_property_changed;
mod property_path;

pub use i_notify_property_changed::INotifyPropertyChanged;
pub use property_path::{property_name_of, PropertyPath};
