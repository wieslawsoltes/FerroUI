use super::PropertySetSnapshot;
use crate::media::Color;
use crate::numerics::{Matrix3x2, Matrix4x4, Quaternion, Vector2, Vector3, Vector4};
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::{AsCompositionObject, CompositionObject, CompositionPropertySet, Compositor};
use std::cell::RefCell;
use std::rc::Rc;

/// This is the base class for ExpressionAnimation and KeyFrameAnimation.
///
/// Use the `start_animation` method of a composition object to start the
/// animation. Value parameters (as opposed to reference parameters which
/// are set using [`set_reference_parameter`](Self::set_reference_parameter))
/// are copied and "embedded" into an expression at the time
/// `start_animation` is called. Changing the value of the variable after
/// `start_animation` is called will not affect the value of the
/// ExpressionAnimation. See the remarks of
/// [`ExpressionAnimation`](super::ExpressionAnimation) for additional
/// information.
///
/// A class derives from it by embedding it (`base`) and dereferencing to
/// it; the class implements [`ICompositionAnimation`](super::ICompositionAnimation),
/// whose `Rc<dyn ..>` is the handle of an animation of any class.
pub struct CompositionAnimation {
    object: CompositionObject,
    property_set: Rc<CompositionPropertySet>,
    target: RefCell<Option<String>>,
}

impl CompositionAnimation {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> Self {
        Self {
            object: CompositionObject::new(compositor, None),
            property_set: CompositionPropertySet::new(compositor),
            target: RefCell::new(None),
        }
    }

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The associated compositor.
    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// Clears all of the parameters of the animation.
    pub fn clear_all_parameters(&self) {
        self.property_set.clear_all()
    }

    /// Clears a parameter from the animation.
    pub fn clear_parameter(&self, key: &str) {
        self.property_set.clear(key)
    }

    fn set_variant(&self, key: &str, value: ExpressionVariant) {
        self.property_set.set(key, value)
    }

    pub fn set_color_parameter(&self, key: &str, value: Color) {
        self.set_variant(key, value.into())
    }

    pub fn set_matrix3x2_parameter(&self, key: &str, value: Matrix3x2) {
        self.set_variant(key, value.into())
    }

    pub fn set_matrix4x4_parameter(&self, key: &str, value: Matrix4x4) {
        self.set_variant(key, value.into())
    }

    pub fn set_quaternion_parameter(&self, key: &str, value: Quaternion) {
        self.set_variant(key, value.into())
    }

    pub fn set_reference_parameter(&self, key: &str, composition_object: Rc<dyn AsCompositionObject>) {
        self.property_set.set_object(key, composition_object)
    }

    pub fn set_scalar_parameter(&self, key: &str, value: f32) {
        self.set_variant(key, value.into())
    }

    pub fn set_vector2_parameter(&self, key: &str, value: Vector2) {
        self.set_variant(key, value.into())
    }

    pub fn set_vector3_parameter(&self, key: &str, value: Vector3) {
        self.set_variant(key, value.into())
    }

    pub fn set_vector4_parameter(&self, key: &str, value: Vector4) {
        self.set_variant(key, value.into())
    }

    /// The name of the property the animation targets.
    pub fn target(&self) -> Option<String> {
        self.target.borrow().clone()
    }

    pub fn set_target(&self, value: Option<String>) {
        *self.target.borrow_mut() = value;
    }

    pub(crate) fn create_snapshot(&self) -> Rc<PropertySetSnapshot> {
        Rc::new(self.property_set.snapshot())
    }
}
