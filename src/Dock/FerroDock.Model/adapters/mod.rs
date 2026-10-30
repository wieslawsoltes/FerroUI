//! Adapters for model tracking, navigation, and window hosting.

use std::rc::{Rc, Weak};

// Standalone source adapters retain their target. A model-owned component uses
// the weak construction path to avoid the cycle that a GC collects upstream.
enum AdapterOwner<T: ?Sized> {
    Strong(Rc<T>),
    Weak(Weak<T>),
}
impl<T: ?Sized> AdapterOwner<T> {
    fn get(&self) -> Option<Rc<T>> {
        match self {
            Self::Strong(value) => Some(value.clone()),
            Self::Weak(value) => value.upgrade(),
        }
    }
}

mod host_adapter;
mod navigate_adapter;
mod tracking_adapter;

pub use host_adapter::HostAdapter;
pub use navigate_adapter::NavigateAdapter;

pub use tracking_adapter::TrackingAdapter;

#[cfg(test)]
mod navigate_adapter_tests;
