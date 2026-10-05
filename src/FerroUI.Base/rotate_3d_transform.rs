use crate::media::{Transform, TransformImpl};
use crate::{
    ferro_class, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Matrix, Ref, StyledProperty,
};
use std::cell::Cell;

/// A row-major 4x4 single-precision matrix using the row-vector convention.
type Matrix4x4 = [[f32; 4]; 4];

const IDENTITY: Matrix4x4 =
    [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

fn multiply(a: &Matrix4x4, b: &Matrix4x4) -> Matrix4x4 {
    let mut result = [[0.0f32; 4]; 4];
    for (i, row) in result.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j] + a[i][3] * b[3][j];
        }
    }
    result
}

fn create_translation(x: f32, y: f32, z: f32) -> Matrix4x4 {
    let mut result = IDENTITY;
    result[3][0] = x;
    result[3][1] = y;
    result[3][2] = z;
    result
}

fn create_rotation_x(radians: f32) -> Matrix4x4 {
    let (s, c) = radians.sin_cos();
    let mut result = IDENTITY;
    result[1][1] = c;
    result[1][2] = s;
    result[2][1] = -s;
    result[2][2] = c;
    result
}

fn create_rotation_y(radians: f32) -> Matrix4x4 {
    let (s, c) = radians.sin_cos();
    let mut result = IDENTITY;
    result[0][0] = c;
    result[0][2] = -s;
    result[2][0] = s;
    result[2][2] = c;
    result
}

fn create_rotation_z(radians: f32) -> Matrix4x4 {
    let (s, c) = radians.sin_cos();
    let mut result = IDENTITY;
    result[0][0] = c;
    result[0][1] = s;
    result[1][0] = -s;
    result[1][1] = c;
    result
}

/// Non-affine 3D transformation for rotating a visual around a definable
/// axis.
#[repr(C)]
pub struct Rotate3DTransform {
    base: Transform,
    is_initializing: Cell<bool>,
}

ferro_class!(Rotate3DTransform: Transform);
crate::ferro_class_info!(Rotate3DTransform { new: Rotate3DTransform::new });

impl FerroObjectImpl for Rotate3DTransform {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if !this.is_initializing.get() {
            this.raise_changed();
        }
    }
}

impl TransformImpl for Rotate3DTransform {
    fn value(this: &Self) -> Matrix {
        let mut matrix44 = IDENTITY;
        // Copy values first, because it's not guaranteed that values will not
        // change during calculation.
        let (copy_center_x, copy_center_y, copy_center_z, copy_angle_x, copy_angle_y, copy_angle_z, copy_depth) = (
            this.center_x(),
            this.center_y(),
            this.center_z(),
            this.angle_x(),
            this.angle_y(),
            this.angle_z(),
            this.depth(),
        );

        // The smallest positive subnormal double.
        const EPSILON: f64 = f64::from_bits(1);

        let center_sum = copy_center_x + copy_center_y + copy_center_z;

        if center_sum.abs() > EPSILON {
            matrix44 = multiply(
                &matrix44,
                &create_translation(-(copy_center_x as f32), -(copy_center_y as f32), -(copy_center_z as f32)),
            );
        }

        if copy_angle_x != 0.0 {
            matrix44 = multiply(&matrix44, &create_rotation_x(Matrix::to_radians(copy_angle_x) as f32));
        }
        if copy_angle_y != 0.0 {
            matrix44 = multiply(&matrix44, &create_rotation_y(Matrix::to_radians(copy_angle_y) as f32));
        }
        if copy_angle_z != 0.0 {
            matrix44 = multiply(&matrix44, &create_rotation_z(Matrix::to_radians(copy_angle_z) as f32));
        }

        if center_sum.abs() > EPSILON {
            matrix44 = multiply(
                &matrix44,
                &create_translation(copy_center_x as f32, copy_center_y as f32, copy_center_z as f32),
            );
        }

        if copy_depth != 0.0 {
            let mut perspective_matrix = IDENTITY;
            perspective_matrix[2][3] = -1.0 / copy_depth as f32;
            matrix44 = multiply(&matrix44, &perspective_matrix);
        }

        Matrix::new_3x3(
            matrix44[0][0] as f64,
            matrix44[0][1] as f64,
            matrix44[0][3] as f64,
            matrix44[1][0] as f64,
            matrix44[1][1] as f64,
            matrix44[1][3] as f64,
            matrix44[3][0] as f64,
            matrix44[3][1] as f64,
            matrix44[3][3] as f64,
        )
    }
}

crate::ferro_properties! { impl Rotate3DTransform {
    ferro_property!(pub fn angle_x_property() -> StyledProperty<f64> {
        FerroProperty::register::<Rotate3DTransform, _>("AngleX", 0.0)
    });

    ferro_property!(pub fn angle_y_property() -> StyledProperty<f64> {
        FerroProperty::register::<Rotate3DTransform, _>("AngleY", 0.0)
    });

    ferro_property!(pub fn angle_z_property() -> StyledProperty<f64> {
        FerroProperty::register::<Rotate3DTransform, _>("AngleZ", 0.0)
    });

    ferro_property!(pub fn center_x_property() -> StyledProperty<f64> {
        FerroProperty::register::<Rotate3DTransform, _>("CenterX", 0.0)
    });

    ferro_property!(pub fn center_y_property() -> StyledProperty<f64> {
        FerroProperty::register::<Rotate3DTransform, _>("CenterY", 0.0)
    });

    ferro_property!(pub fn center_z_property() -> StyledProperty<f64> {
        FerroProperty::register::<Rotate3DTransform, _>("CenterZ", 0.0)
    });

    ferro_property!(pub fn depth_property() -> StyledProperty<f64> {
        FerroProperty::register::<Rotate3DTransform, _>("Depth", 0.0)
    });
} }

impl Rotate3DTransform {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Transform::construct(), is_initializing: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a transform with the given rotation angles (in degrees),
    /// rotation center and depth.
    #[allow(clippy::too_many_arguments)]
    pub fn with_values(
        angle_x: f64,
        angle_y: f64,
        angle_z: f64,
        center_x: f64,
        center_y: f64,
        center_z: f64,
        depth: f64,
    ) -> Ref<Self> {
        let result = Self::new();
        result.is_initializing.set(true);
        result.set_angle_x(angle_x);
        result.set_angle_y(angle_y);
        result.set_angle_z(angle_z);
        result.set_center_x(center_x);
        result.set_center_y(center_y);
        result.set_center_z(center_z);
        result.set_depth(depth);
        result.is_initializing.set(false);
        result
    }

    /// The rotation around the X-axis, in degrees.
    pub fn angle_x(&self) -> f64 {
        self.get_value(Self::angle_x_property())
    }

    pub fn set_angle_x(&self, value: f64) {
        self.set_value(Self::angle_x_property(), value)
    }

    /// The rotation around the Y-axis, in degrees.
    pub fn angle_y(&self) -> f64 {
        self.get_value(Self::angle_y_property())
    }

    pub fn set_angle_y(&self, value: f64) {
        self.set_value(Self::angle_y_property(), value)
    }

    /// The rotation around the Z-axis, in degrees.
    pub fn angle_z(&self) -> f64 {
        self.get_value(Self::angle_z_property())
    }

    pub fn set_angle_z(&self, value: f64) {
        self.set_value(Self::angle_z_property(), value)
    }

    /// The X-coordinate of the rotation center.
    pub fn center_x(&self) -> f64 {
        self.get_value(Self::center_x_property())
    }

    pub fn set_center_x(&self, value: f64) {
        self.set_value(Self::center_x_property(), value)
    }

    /// The Y-coordinate of the rotation center.
    pub fn center_y(&self) -> f64 {
        self.get_value(Self::center_y_property())
    }

    pub fn set_center_y(&self, value: f64) {
        self.set_value(Self::center_y_property(), value)
    }

    /// The Z-coordinate of the rotation center.
    pub fn center_z(&self) -> f64 {
        self.get_value(Self::center_z_property())
    }

    pub fn set_center_z(&self, value: f64) {
        self.set_value(Self::center_z_property(), value)
    }

    /// The focal length of the camera.
    pub fn depth(&self) -> f64 {
        self.get_value(Self::depth_property())
    }

    pub fn set_depth(&self, value: f64) {
        self.set_value(Self::depth_property(), value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn default_value_is_identity() {
        assert_eq!(Matrix::IDENTITY, Rotate3DTransform::new().value());
    }

    #[test]
    fn rotation_around_z_matches_2d_rotation() {
        let target = Rotate3DTransform::with_values(0.0, 0.0, 90.0, 0.0, 0.0, 0.0, 0.0);
        let value = target.value();
        let expected = Matrix::create_rotation(Matrix::to_radians(90.0));
        assert!((value.m11 - expected.m11).abs() < 1e-6);
        assert!((value.m12 - expected.m12).abs() < 1e-6);
        assert!((value.m21 - expected.m21).abs() < 1e-6);
        assert!((value.m22 - expected.m22).abs() < 1e-6);
        assert!(!value.contains_perspective());
    }

    #[test]
    fn depth_and_x_rotation_produce_perspective() {
        let target = Rotate3DTransform::with_values(60.0, 0.0, 0.0, 0.0, 0.0, 0.0, 200.0);
        let value = target.value();
        // M22 = cos(60deg), M23 (perspective Y) = sin(60deg) * (-1 / depth).
        assert!((value.m22 - 0.5).abs() < 1e-6);
        assert!((value.m23 - (-(60f64.to_radians().sin()) / 200.0)).abs() < 1e-6);
        assert!(value.contains_perspective());
    }

    #[test]
    fn changed_is_raised_for_every_property_but_not_while_initializing() {
        let target = Rotate3DTransform::with_values(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0);
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        target.changed(move || c.set(c.get() + 1));
        target.set_angle_x(10.0);
        target.set_center_z(10.0);
        target.set_depth(10.0);
        assert_eq!(3, count.get());
    }
}
