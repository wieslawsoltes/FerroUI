use crate::media::{Transform, TransformImpl};
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Matrix,
    Ref, StyledProperty,
};

/// Skews a visual.
#[repr(C)]
pub struct SkewTransform {
    base: Transform,
}

ferro_class!(SkewTransform: Transform);
crate::ferro_class_info!(SkewTransform { new: SkewTransform::new });

impl FerroObjectImpl for SkewTransform {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::angle_x_property().as_property() || change.property() == Self::angle_y_property().as_property() {
            this.raise_changed();
        }
    }
}

impl TransformImpl for SkewTransform {
    fn value(this: &Self) -> Matrix {
        Matrix::create_skew(Matrix::to_radians(this.angle_x()), Matrix::to_radians(this.angle_y()))
    }
}

crate::ferro_properties! { impl SkewTransform {
    ferro_property!(pub fn angle_x_property() -> StyledProperty<f64> {
        FerroProperty::register::<SkewTransform, _>("AngleX", 0.0)
    });

    ferro_property!(pub fn angle_y_property() -> StyledProperty<f64> {
        FerroProperty::register::<SkewTransform, _>("AngleY", 0.0)
    });
} }

impl SkewTransform {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Transform::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn with_angles(angle_x: f64, angle_y: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_angle_x(angle_x);
        result.set_angle_y(angle_y);
        result
    }

    /// The skew angle on the x axis, in degrees.
    pub fn angle_x(&self) -> f64 {
        self.get_value(Self::angle_x_property())
    }

    pub fn set_angle_x(&self, value: f64) {
        self.set_value(Self::angle_x_property(), value)
    }

    /// The skew angle on the y axis, in degrees.
    pub fn angle_y(&self) -> f64 {
        self.get_value(Self::angle_y_property())
    }

    pub fn set_angle_y(&self, value: f64) {
        self.set_value(Self::angle_y_property(), value)
    }
}
