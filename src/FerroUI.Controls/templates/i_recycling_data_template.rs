use super::IDataTemplate;
use crate::Control;
use ferroui_base::{BoxedValue, Ref};

/// An [`IDataTemplate`] that supports recycling existing elements.
pub trait IRecyclingDataTemplate: IDataTemplate {
    /// Creates or recycles a control to display the specified data.
    ///
    /// `existing` is an optional control that was previously created by this
    /// template that may be recycled. The recycled control should be
    /// returned unchanged; its data context is set by the caller.
    fn build_with_existing(&self, data: Option<&BoxedValue>, existing: Option<Ref<Control>>) -> Option<Ref<Control>>;
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn IRecyclingDataTemplate {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
