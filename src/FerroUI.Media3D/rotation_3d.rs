//! WPF's 3D rotations: `Rotation3D`, `AxisAngleRotation3D` and `QuaternionRotation3D`, the
//! rotation of a `RotateTransform3D`.

use crate::change::{Notifier, Notifying, Scene3DChange};
use crate::{Quaternion, Vector3D};
use ferroui_base::animation::Animatable;
use ferroui_base::reactive::Subscription;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use std::rc::Rc;

/// WPF's `Rotation3D`: the base of the rotations.
#[repr(C)]
pub struct Rotation3D {
    base: Animatable,
    notifier: Rc<Notifier>,
}

ferro_class! {
    Rotation3D: Animatable, virtuals Rotation3DImpl: FerroObjectImpl {
        /// The rotation as a quaternion (WPF's `InternalQuaternion`).
        fn internal_quaternion(this) -> Quaternion;
    }
}

ferro_impl_classes!(Rotation3D: FerroObjectImpl);

impl Rotation3DImpl for Rotation3D {
    fn internal_quaternion(_this: &Self) -> Quaternion {
        Quaternion::IDENTITY
    }
}

impl Notifying for Rotation3D {
    fn notifier(&self) -> &Rc<Notifier> {
        &self.notifier
    }
}

impl Rotation3D {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Animatable::construct(), notifier: Notifier::new() }
    }

    /// WPF's `Rotation3D.Identity`: a new rotation by nothing.
    pub fn identity() -> Ref<Rotation3D> {
        QuaternionRotation3D::new().upcast()
    }

    /// Calls `handler` on every change of the rotation, until the returned guard is dropped.
    pub fn changed(&self, handler: impl Fn(Scene3DChange) + 'static) -> Subscription {
        self.notifier.subscribe(Rc::new(handler))
    }
}

/// WPF's `AxisAngleRotation3D`: a rotation by `Angle` degrees about `Axis`, counterclockwise
/// when the axis points at the viewer. An axis of (nearly) zero length is no rotation.
#[repr(C)]
pub struct AxisAngleRotation3D {
    base: Rotation3D,
}

ferro_class!(AxisAngleRotation3D: Rotation3D);
ferro_class_info!(AxisAngleRotation3D {
    new: AxisAngleRotation3D::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Vector3D, f64) => AxisAngleRotation3D::with_axis_angle,
        ],
    },
});

impl FerroObjectImpl for AxisAngleRotation3D {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        this.notifier().raise(Scene3DChange::Record);
    }
}

impl Rotation3DImpl for AxisAngleRotation3D {
    fn internal_quaternion(this: &Self) -> Quaternion {
        let axis = this.axis();
        // WPF compares the squared length with the smallest normal single.
        if axis.length_squared() > f32::MIN_POSITIVE as f64 {
            Quaternion::from_axis_angle(axis, this.angle()).unwrap_or(Quaternion::IDENTITY)
        } else {
            Quaternion::IDENTITY
        }
    }
}

ferro_properties! { impl AxisAngleRotation3D {
    pub fn axis_property() -> StyledProperty<Vector3D> {
        FerroProperty::register::<AxisAngleRotation3D, _>("Axis", Vector3D::new(0.0, 1.0, 0.0))
    }
    pub fn angle_property() -> StyledProperty<f64> {
        FerroProperty::register::<AxisAngleRotation3D, _>("Angle", 0.0)
    }
} }

impl AxisAngleRotation3D {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Rotation3D::construct() }
    }

    /// A rotation by 0 degrees about the y axis.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A rotation by `angle` degrees about `axis`.
    pub fn with_axis_angle(axis: Vector3D, angle: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_axis(axis);
        result.set_angle(angle);
        result
    }

    /// The axis of the rotation.
    pub fn axis(&self) -> Vector3D {
        self.get_value(Self::axis_property())
    }

    pub fn set_axis(&self, value: Vector3D) {
        self.set_value(Self::axis_property(), value)
    }

    /// The angle of the rotation, in degrees.
    pub fn angle(&self) -> f64 {
        self.get_value(Self::angle_property())
    }

    pub fn set_angle(&self, value: f64) {
        self.set_value(Self::angle_property(), value)
    }
}

/// WPF's `QuaternionRotation3D`: the rotation of its `Quaternion`.
#[repr(C)]
pub struct QuaternionRotation3D {
    base: Rotation3D,
}

ferro_class!(QuaternionRotation3D: Rotation3D);
ferro_class_info!(QuaternionRotation3D {
    new: QuaternionRotation3D::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Quaternion) => QuaternionRotation3D::with_quaternion,
        ],
    },
});

impl FerroObjectImpl for QuaternionRotation3D {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        this.notifier().raise(Scene3DChange::Record);
    }
}

impl Rotation3DImpl for QuaternionRotation3D {
    fn internal_quaternion(this: &Self) -> Quaternion {
        this.quaternion()
    }
}

ferro_properties! { impl QuaternionRotation3D {
    pub fn quaternion_property() -> StyledProperty<Quaternion> {
        FerroProperty::register::<QuaternionRotation3D, _>("Quaternion", Quaternion::IDENTITY)
    }
} }

impl QuaternionRotation3D {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Rotation3D::construct() }
    }

    /// The identity rotation.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The rotation of `quaternion`.
    pub fn with_quaternion(quaternion: Quaternion) -> Ref<Self> {
        let result = Self::new();
        result.set_quaternion(quaternion);
        result
    }

    /// The quaternion of the rotation.
    pub fn quaternion(&self) -> Quaternion {
        self.get_value(Self::quaternion_property())
    }

    pub fn set_quaternion(&self, value: Quaternion) {
        self.set_value(Self::quaternion_property(), value)
    }
}
