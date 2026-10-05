use crate::media::{Transform, TransformImpl};
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Matrix,
    Ref, StyledProperty,
};

/// Translates (moves) a visual.
#[repr(C)]
pub struct TranslateTransform {
    base: Transform,
}

ferro_class!(TranslateTransform: Transform);
crate::ferro_class_info!(TranslateTransform { new: TranslateTransform::new });

impl FerroObjectImpl for TranslateTransform {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::x_property().as_property() || change.property() == Self::y_property().as_property() {
            this.raise_changed();
        }
    }
}

impl TransformImpl for TranslateTransform {
    fn value(this: &Self) -> Matrix {
        Matrix::create_translation(this.x(), this.y())
    }
}

crate::ferro_properties! { impl TranslateTransform {
    ferro_property!(pub fn x_property() -> StyledProperty<f64> {
        FerroProperty::register::<TranslateTransform, _>("X", 0.0)
    });

    ferro_property!(pub fn y_property() -> StyledProperty<f64> {
        FerroProperty::register::<TranslateTransform, _>("Y", 0.0)
    });
} }

impl TranslateTransform {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Transform::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn with_offset(x: f64, y: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_x(x);
        result.set_y(y);
        result
    }

    /// The horizontal offset.
    pub fn x(&self) -> f64 {
        self.get_value(Self::x_property())
    }

    pub fn set_x(&self, value: f64) {
        self.set_value(Self::x_property(), value)
    }

    /// The vertical offset.
    pub fn y(&self) -> f64 {
        self.get_value(Self::y_property())
    }

    pub fn set_y(&self, value: f64) {
        self.set_value(Self::y_property(), value)
    }
}
