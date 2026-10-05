use crate::media::{Transform, TransformImpl};
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Matrix,
    Ref, StyledProperty,
};

/// Rotates a visual.
#[repr(C)]
pub struct RotateTransform {
    base: Transform,
}

ferro_class!(RotateTransform: Transform);
crate::ferro_class_info!(RotateTransform { new: RotateTransform::new });

impl FerroObjectImpl for RotateTransform {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        // Only the angle raises the changed notification: the reference
        // implementation observes the angle property alone.
        if change.property() == Self::angle_property().as_property() {
            this.raise_changed();
        }
    }
}

impl TransformImpl for RotateTransform {
    fn value(this: &Self) -> Matrix {
        let (center_x, center_y) = (this.center_x(), this.center_y());
        Matrix::create_translation(-center_x, -center_y)
            * Matrix::create_rotation(Matrix::to_radians(this.angle()))
            * Matrix::create_translation(center_x, center_y)
    }
}

crate::ferro_properties! { impl RotateTransform {
    ferro_property!(pub fn angle_property() -> StyledProperty<f64> {
        FerroProperty::register::<RotateTransform, _>("Angle", 0.0)
    });

    ferro_property!(pub fn center_x_property() -> StyledProperty<f64> {
        FerroProperty::register::<RotateTransform, _>("CenterX", 0.0)
    });

    ferro_property!(pub fn center_y_property() -> StyledProperty<f64> {
        FerroProperty::register::<RotateTransform, _>("CenterY", 0.0)
    });
} }

impl RotateTransform {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Transform::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a rotation of `angle` degrees.
    pub fn with_angle(angle: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_angle(angle);
        result
    }

    /// Creates a rotation of `angle` degrees around the given center.
    pub fn with_angle_and_center(angle: f64, center_x: f64, center_y: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_angle(angle);
        result.set_center_x(center_x);
        result.set_center_y(center_y);
        result
    }

    /// The angle of rotation, in degrees.
    pub fn angle(&self) -> f64 {
        self.get_value(Self::angle_property())
    }

    pub fn set_angle(&self, value: f64) {
        self.set_value(Self::angle_property(), value)
    }

    /// The x-coordinate of the rotation center point. The default is 0.
    pub fn center_x(&self) -> f64 {
        self.get_value(Self::center_x_property())
    }

    pub fn set_center_x(&self, value: f64) {
        self.set_value(Self::center_x_property(), value)
    }

    /// The y-coordinate of the rotation center point. The default is 0.
    pub fn center_y(&self) -> f64 {
        self.get_value(Self::center_y_property())
    }

    pub fn set_center_y(&self, value: f64) {
        self.set_value(Self::center_y_property(), value)
    }
}
