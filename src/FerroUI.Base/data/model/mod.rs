//! The model layer: what bindings need from data that is not part of the
//! class hierarchy (view models, plain data, collections, commands).
//!
//! The managed runtime discovers these capabilities through reflection and
//! interface casts on arbitrary objects. Here a model type states them once:
//! it implements the notification traits it supports and declares its
//! bindable properties and methods (see [`ferro_model!`](crate::ferro_model)
//! and [`ModelTypeBuilder`]). Compiled bindings do not need the declaration:
//! they are built from typed accessor closures.

mod event;
mod i_bindable_indexer;
mod i_notify_collection_changed;
mod i_notify_data_error_info;
mod i_notify_property_changed;
mod model_type;

pub use event::Event;
pub use crate::input::ICommand;
pub use i_bindable_indexer::{BindableArray, BindableDictionary, IBindableArray, IBindableIndexer};
pub use i_notify_collection_changed::{BindableList, CollectionChange, IBindableList, INotifyCollectionChanged};
pub use i_notify_data_error_info::INotifyDataErrorInfo;
pub use i_notify_property_changed::INotifyPropertyChanged;
pub use model_type::{Model, ModelMethod, ModelType, ModelTypeBuilder, ModelTypes};
