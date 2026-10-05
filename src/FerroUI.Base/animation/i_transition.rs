use crate::animation::{Animatable, IClock, TransitionBase};
use crate::reactive::IDisposable;
use crate::{BoxedValue, FerroObject, FerroProperty, ObjectType, Ref, Upcast};
use std::rc::Rc;

/// Interface for transition objects: animates a property from its old value
/// to its new value whenever the value changes.
pub trait ITransition: 'static {
    /// Applies the transition to the specified control: animates the
    /// transition's property from `old_value` to `new_value`, which hold
    /// exactly the value type of the property. Disposing the result ends the
    /// transition.
    fn apply(
        &self,
        control: &Animatable,
        clock: Rc<dyn IClock>,
        old_value: &BoxedValue,
        new_value: &BoxedValue,
    ) -> Rc<dyn IDisposable>;

    /// The property to be animated. Panics when the transition has none.
    fn property(&self) -> &'static FerroProperty;

    fn set_property(&self, value: &'static FerroProperty);

    /// A description of the transition for diagnostics.
    fn debug_display(&self) -> String {
        "ITransition".to_string()
    }

    /// The identity of the transition: handles to the same transition
    /// object compare equal.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }

    /// The object of the object model that implements the contract, if one
    /// does: the equivalent of `is`/`as` on the interface. Cast the object to
    /// the class looked for (`as_object()?.cast::<T>()`).
    fn as_object(&self) -> Option<crate::Ref<crate::FerroObject>> {
        None
    }
}

impl PartialEq for dyn ITransition {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

impl std::fmt::Debug for dyn ITransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.debug_display())
    }
}

/// The interface handle of a transition object.
struct TransitionRef(Ref<TransitionBase>);

impl ITransition for TransitionRef {
    fn apply(
        &self,
        control: &Animatable,
        clock: Rc<dyn IClock>,
        old_value: &BoxedValue,
        new_value: &BoxedValue,
    ) -> Rc<dyn IDisposable> {
        self.0.apply(control, clock, old_value, new_value)
    }

    fn property(&self) -> &'static FerroProperty {
        match self.0.property() {
            Some(property) => property,
            None => panic!("Transition has no property specified."),
        }
    }

    fn set_property(&self, value: &'static FerroProperty) {
        self.0.set_property(Some(value))
    }

    fn debug_display(&self) -> String {
        self.0.debug_display()
    }

    fn reference_id(&self) -> *const () {
        let object: &FerroObject = &self.0;
        object as *const FerroObject as *const ()
    }

    fn as_object(&self) -> Option<Ref<FerroObject>> {
        Some(self.0.clone().upcast())
    }
}

impl<T: ObjectType + Upcast<TransitionBase>> From<Ref<T>> for Rc<dyn ITransition> {
    #[inline]
    fn from(value: Ref<T>) -> Self {
        Rc::new(TransitionRef(value.upcast()))
    }
}

impl<T: ObjectType + Upcast<TransitionBase>> From<&Ref<T>> for Rc<dyn ITransition> {
    #[inline]
    fn from(value: &Ref<T>) -> Self {
        Rc::new(TransitionRef(value.clone().upcast()))
    }
}
