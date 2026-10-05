use crate::media::{Transform, TransformImpl};
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Matrix,
    Ref, StyledProperty,
};

/// Transforms a visual according to a [`Matrix`].
#[repr(C)]
pub struct MatrixTransform {
    base: Transform,
}

ferro_class!(MatrixTransform: Transform);
crate::ferro_class_info!(MatrixTransform { new: MatrixTransform::new });

impl FerroObjectImpl for MatrixTransform {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::matrix_property().as_property() {
            this.raise_changed();
        }
    }
}

impl TransformImpl for MatrixTransform {
    fn value(this: &Self) -> Matrix {
        this.matrix()
    }
}

crate::ferro_properties! { impl MatrixTransform {
    ferro_property!(pub fn matrix_property() -> StyledProperty<Matrix> {
        FerroProperty::register::<MatrixTransform, _>("Matrix", Matrix::IDENTITY)
    });
} }

impl MatrixTransform {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Transform::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a transform with the given matrix.
    pub fn with_matrix(matrix: Matrix) -> Ref<Self> {
        let result = Self::new();
        result.set_matrix(matrix);
        result
    }

    /// The matrix.
    pub fn matrix(&self) -> Matrix {
        self.get_value(Self::matrix_property())
    }

    pub fn set_matrix(&self, value: Matrix) {
        self.set_value(Self::matrix_property(), value)
    }
}
