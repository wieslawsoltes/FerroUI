use super::IDataTemplate;
use std::any::TypeId;

/// A data template that declares the type of the data it applies to.
pub trait ITypedDataTemplate: IDataTemplate {
    /// The type of the data the template applies to.
    fn data_type(&self) -> Option<TypeId>;
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn ITypedDataTemplate {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
