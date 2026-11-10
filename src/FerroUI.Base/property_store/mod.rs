//! The property value store: per-object storage and priority resolution of
//! styled property values.

mod binding_entry;
mod boxed_binding_entry;
mod boxed_local_value_binding_observer;
mod effective_value;
mod frame_priority;
mod hot_value;
mod immediate_value_frame;
mod local_value_binding_observer;
mod value_entry;
mod value_frame;
mod value_store;

pub(crate) use binding_entry::{BindingEntry, BindingSource};
pub(crate) use boxed_binding_entry::BoxedBindingEntry;
pub(crate) use boxed_local_value_binding_observer::BoxedLocalValueBindingObserver;
pub(crate) use effective_value::{EffectiveValue, EffectiveValueDyn, UncommonFields};
pub(crate) use frame_priority::{FramePriority, FrameType};
pub(crate) use hot_value::advance_metadata_generation;
#[doc(hidden)]
pub use hot_value::HotValue;
pub use immediate_value_frame::ImmediateValueFrame;
pub(crate) use local_value_binding_observer::{DirectBindingObserver, DirectBoxedBindingObserver, LocalValueBindingObserver};
pub(crate) use value_entry::entry_ptr_eq;
pub use value_entry::IValueEntry;
pub(crate) use value_frame::{frame_ptr_eq, ValueFrame, ValueFrameBase};
pub(crate) use value_store::ValueStore;
