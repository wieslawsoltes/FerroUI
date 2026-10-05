//! The bridge between class handles and the media interfaces.
//!
//! The media interfaces (`IBrush`, `IPen`, `ITransform`, ...) are implemented
//! by immutable value types and by mutable classes. A handle to an interface
//! is an `Rc<dyn IFoo>`; for a mutable class the `Rc` holds a [`RefAdapter`]
//! around the object's [`Ref`]. The interfaces are deliberately not
//! implemented on the class structs or on `Ref<T>` itself, so that calling a
//! member on an object handle always resolves to the class's own member, even
//! where the interface declares a member of the same name with another type
//! (`DashStyle::dashes`, `GradientBrush::gradient_stops`).

use crate::{FerroObject, ObjectType, Ref};

/// Implements the media interfaces for the object behind a class handle.
pub(crate) struct RefAdapter<T: ObjectType>(pub(crate) Ref<T>);

impl<T: ObjectType> RefAdapter<T> {
    /// The object behind the handle.
    #[inline]
    pub(crate) fn object(&self) -> &FerroObject {
        (*self.0).upcast()
    }

    /// The object viewed as class `C`. Panics if it is not one; callers reach
    /// this only through an interface cast that checked the class.
    #[inline]
    pub(crate) fn class<C: ObjectType>(&self) -> &C {
        self.object().downcast_ref::<C>().expect("the object implements the interface")
    }

    /// The address of the object, used for reference equality.
    #[inline]
    pub(crate) fn reference_id(&self) -> *const () {
        self.object() as *const FerroObject as *const ()
    }
}
