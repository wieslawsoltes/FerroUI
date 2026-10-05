use crate::data::BindingExpressionBase;
use crate::{FerroObject, FerroProperty, Ref};
use std::rc::Rc;

/// The base of bindings: a description from which binding expressions are
/// instantiated on targets.
pub trait BindingBase {
    /// Creates a binding expression from the binding.
    ///
    /// `anchor` provides, if `target` is not an element, an object from which
    /// to locate a data context or other elements.
    ///
    /// This is a low-level method which returns a binding expression that is
    /// not yet connected to a binding sink, and so is inactive.
    fn create_instance(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase>;

    /// The implementing value, for casting to its concrete type
    /// (`as_any()?.downcast_ref::<T>()`): the equivalent of `is`/`as` on the
    /// contract. `None` for an implementation that does not expose itself.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }

    /// The identity of the binding, used for reference equality.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn BindingBase {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

/// A shared handle of a binding is itself usable where a binding is expected
/// (`target.bind_binding(property, &binding)` with `binding: Rc<Binding>`).
/// It has the identity of the binding it refers to.
impl<T: BindingBase + ?Sized> BindingBase for Rc<T> {
    fn create_instance(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        (**self).create_instance(target, target_property, anchor)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        (**self).as_any()
    }

    fn reference_id(&self) -> *const () {
        (**self).reference_id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{CompiledBinding, CompiledBindingPath, ReflectionBinding};

    #[test]
    fn a_binding_is_cast_to_its_concrete_class_through_any_handle() {
        let reflection = ReflectionBinding::new("Name");
        let contract: Rc<dyn BindingBase> = reflection.clone();
        assert!(contract.as_any().is_some_and(|any| any.is::<ReflectionBinding>()));
        // The handle of the concrete class, used as a binding itself.
        let handle: &dyn BindingBase = &reflection;
        let concrete = handle.as_any().and_then(|any| any.downcast_ref::<ReflectionBinding>());
        assert!(concrete.is_some_and(|binding| std::ptr::eq(binding, &*reflection)));
        // And the handle of the contract.
        let handle: &dyn BindingBase = &contract;
        assert!(handle.as_any().is_some_and(|any| any.is::<ReflectionBinding>()));

        let compiled: Rc<dyn BindingBase> = CompiledBinding::new(CompiledBindingPath::new());
        assert!(compiled.as_any().is_some_and(|any| any.is::<CompiledBinding>()));
        assert!(!compiled.as_any().is_some_and(|any| any.is::<ReflectionBinding>()));
    }
}
