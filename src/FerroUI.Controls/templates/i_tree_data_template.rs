use super::IDataTemplate;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{BoxedValue, FerroObject, FerroProperty};
use std::rc::Rc;

/// Interface representing a template used to build hierarchical data.
pub trait ITreeDataTemplate: IDataTemplate {
    /// Binds the children of the specified item to a property on a target
    /// object.
    fn bind_children(
        &self,
        target: &FerroObject,
        target_property: &'static FerroProperty,
        item: &BoxedValue,
    ) -> Rc<dyn IDisposable>;
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn ITreeDataTemplate {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
