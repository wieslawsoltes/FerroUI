use crate::metadata::IServiceProvider;
use crate::BoxedValue;
use std::rc::Rc;

/// Represents a deferred content: content that is built on demand, the first
/// time it is requested.
pub trait IDeferredContent {
    /// Builds the deferred content. `service_provider` is the service
    /// provider to build it with, if any; deferred resources are built with
    /// none.
    fn build(&self, service_provider: Option<&Rc<dyn IServiceProvider>>) -> Option<BoxedValue>;
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn IDeferredContent {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
