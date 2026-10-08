//! Port of `StyledElementExtensions.cs`: the extension methods are members
//! of [`StyledElement`], the type they extend.
//!
//! The internal `IsClassesBindingProperty` is not repeated here: the one
//! caller in the framework (the setter) asks
//! [`ClassBindingManager::is_classes_binding_property`] directly.

use crate::data::{BindingBase, BindingExpressionBase};
use crate::{ClassBindingManager, FerroObject, FerroProperty, Ref, StyledElement};
use std::rc::Rc;

impl StyledElement {
    /// Binds the presence of the class `class_name` on this element to
    /// `source`. `anchor` provides the context the binding locates its source
    /// with when the element cannot.
    pub fn bind_class(
        &self,
        class_name: &str,
        source: &dyn BindingBase,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        ClassBindingManager::bind(self, class_name, source, anchor)
    }

    /// The property a binding to the class `class_name` is set on.
    pub fn get_class_property(class_name: &str) -> &'static FerroProperty {
        ClassBindingManager::get_class_property(class_name)
    }
}
