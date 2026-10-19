//! `DynamicData.Binding`: the objects that notify the changes of their properties, and the
//! operators that observe them and bind change sets to collections.

mod abstract_notify_property_changed;

pub use abstract_notify_property_changed::{AbstractNotifyPropertyChanged, BackingField};
