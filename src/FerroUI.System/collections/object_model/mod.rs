//! `System.Collections.ObjectModel`: collections that notify their changes.

mod observable_collection;

pub use observable_collection::{CollectionChangedHandler, ObservableCollection};
