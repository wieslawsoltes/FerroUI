//! The property value store: per-object storage and priority resolution of
//! styled property values.

mod binding_entry;
mod effective_value;
mod frame_priority;
mod immediate_value_frame;
mod local_value_binding_observer;
mod value_entry;
mod value_frame;
mod value_store;

pub(crate) use binding_entry::{BindingEntry, BindingSource};
pub(crate) use effective_value::{EffectiveValue, EffectiveValueDyn};
pub(crate) use frame_priority::{FramePriority, FrameType};
pub use immediate_value_frame::ImmediateValueFrame;
pub(crate) use local_value_binding_observer::{DirectBindingObserver, LocalValueBindingObserver};
pub(crate) use value_entry::entry_ptr_eq;
pub use value_entry::IValueEntry;
pub(crate) use value_frame::{frame_ptr_eq, ValueFrame, ValueFrameBase};
pub(crate) use value_store::ValueStore;
