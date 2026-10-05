use crate::media::ref_adapter::RefAdapter;
use crate::media::{IMutableTransform, Transform};
use crate::reactive::IDisposable;
use crate::{FerroObject, Matrix, ObjectType, Ref, Upcast};
use std::any::Any;
use std::rc::Rc;

/// Represents a transform on a visual or brush.
///
/// Implemented by the immutable transform types and, through an adapter, by
/// every class deriving from [`Transform`]: a handle of such a class converts
/// to `Rc<dyn ITransform>` with `into()`.
pub trait ITransform: 'static {
    /// The transform's matrix.
    fn value(&self) -> Matrix;

    /// The implementing value, for downcasts to immutable transform types.
    fn as_any(&self) -> &dyn Any;

    /// The object behind the transform when it is a mutable [`Transform`].
    fn as_object(&self) -> Option<&FerroObject> {
        None
    }

    /// The transform viewed as [`IMutableTransform`], when it is one.
    fn as_mutable_transform(&self) -> Option<&dyn IMutableTransform> {
        None
    }

    /// The composition render resource behind the object, if it is a
    /// mutable object with server-side counterparts on the compositors it
    /// is used with.
    fn as_composition_render_resource(
        &self,
    ) -> Option<&dyn crate::rendering::composition::drawing::ICompositionRenderResource> {
        None
    }

    /// The identity of the transform, used for reference equality.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }
}

/// Transforms compare by reference.
impl PartialEq for dyn ITransform {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

impl std::fmt::Debug for dyn ITransform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value())
    }
}

impl<T: ObjectType + Upcast<Transform>> RefAdapter<T> {
    #[inline]
    fn transform(&self) -> &Transform {
        (*self.0).upcast()
    }
}

impl<T: ObjectType + Upcast<Transform>> ITransform for RefAdapter<T> {
    #[inline]
    fn value(&self) -> Matrix {
        self.transform().value()
    }

    fn as_any(&self) -> &dyn Any {
        self.transform()
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some(self.object())
    }

    fn as_mutable_transform(&self) -> Option<&dyn IMutableTransform> {
        Some(self)
    }

    fn as_composition_render_resource(
        &self,
    ) -> Option<&dyn crate::rendering::composition::drawing::ICompositionRenderResource> {
        Some(self.transform())
    }

    fn reference_id(&self) -> *const () {
        RefAdapter::reference_id(self)
    }
}

impl<T: ObjectType + Upcast<Transform>> IMutableTransform for RefAdapter<T> {
    fn changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.transform().changed(move || handler())
    }
}

impl<T: ObjectType + Upcast<Transform>> From<Ref<T>> for Rc<dyn ITransform> {
    #[inline]
    fn from(value: Ref<T>) -> Self {
        Rc::new(RefAdapter(value))
    }
}

impl<T: ObjectType + Upcast<Transform>> From<&Ref<T>> for Rc<dyn ITransform> {
    #[inline]
    fn from(value: &Ref<T>) -> Self {
        Rc::new(RefAdapter(value.clone()))
    }
}
