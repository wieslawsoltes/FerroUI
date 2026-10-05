use crate::BoxedValue;

/// Creates an object on demand: the untyped view of a template.
pub trait ITemplate {
    /// Builds the object.
    ///
    /// When the template is the value of a [`Setter`](super::Setter), the
    /// result must hold exactly the value type of the setter's property.
    fn build(&self) -> BoxedValue;
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn ITemplate {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
