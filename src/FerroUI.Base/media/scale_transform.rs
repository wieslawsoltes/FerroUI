use crate::media::{Transform, TransformImpl};
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Matrix,
    Ref, StyledProperty,
};

/// Scale a visual.
#[repr(C)]
pub struct ScaleTransform {
    base: Transform,
}

ferro_class!(ScaleTransform: Transform);
crate::ferro_class_info!(ScaleTransform { new: ScaleTransform::new });

impl FerroObjectImpl for ScaleTransform {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::scale_x_property().as_property() || change.property() == Self::scale_y_property().as_property() {
            this.raise_changed();
        }
    }
}

impl TransformImpl for ScaleTransform {
    fn value(this: &Self) -> Matrix {
        Matrix::create_scale(this.scale_x(), this.scale_y())
    }
}

crate::ferro_properties! { impl ScaleTransform {
    ferro_property!(pub fn scale_x_property() -> StyledProperty<f64> {
        FerroProperty::register::<ScaleTransform, _>("ScaleX", 1.0)
    });

    ferro_property!(pub fn scale_y_property() -> StyledProperty<f64> {
        FerroProperty::register::<ScaleTransform, _>("ScaleY", 1.0)
    });
} }

impl ScaleTransform {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Transform::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn with_scale(scale_x: f64, scale_y: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_scale_x(scale_x);
        result.set_scale_y(scale_y);
        result
    }

    /// The horizontal scale.
    pub fn scale_x(&self) -> f64 {
        self.get_value(Self::scale_x_property())
    }

    pub fn set_scale_x(&self, value: f64) {
        self.set_value(Self::scale_x_property(), value)
    }

    /// The vertical scale.
    pub fn scale_y(&self) -> f64 {
        self.get_value(Self::scale_y_property())
    }

    pub fn set_scale_y(&self, value: f64) {
        self.set_value(Self::scale_y_property(), value)
    }
}
