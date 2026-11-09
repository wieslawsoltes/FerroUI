//! WPF's cameras: `Camera`, `ProjectionCamera`, `PerspectiveCamera`, `OrthographicCamera` and
//! `MatrixCamera`, with WPF's defaults and matrices.
//!
//! A camera gives a view matrix (from the world to its coordinates, where it looks down the
//! negative z axis) and a projection matrix for an aspect ratio and a pair of planes. Both are
//! WPF's: right-handed, row vectors, a clip depth between 0 and `w`. Any change of a camera,
//! its transform included, is a change of the camera ([`Scene3DChange::Camera`]).

use crate::change::{camera, Link, Notifier, Notifying, Scene3DChange};
use crate::{Matrix3D, Point3D, Transform3D, Vector3D};
use ferroui_base::animation::Animatable;
use ferroui_base::reactive::Subscription;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use std::rc::Rc;

/// WPF's `Camera`: the base of the cameras, with a `Transform` that places the camera in the
/// world.
#[repr(C)]
pub struct Camera {
    base: Animatable,
    notifier: Rc<Notifier>,
    transform_link: Link,
}

ferro_class! {
    Camera: Animatable, virtuals CameraImpl: FerroObjectImpl {
        /// The matrix from the world to the coordinates of the camera, its transform included
        /// (WPF's `GetViewMatrix`). Not finite when the transform has no inverse: the camera
        /// then shows nothing.
        fn get_view_matrix(this) -> Matrix3D;
        /// The matrix from the coordinates of the camera to the clip space for the aspect ratio
        /// (width over height) and the near and far planes (WPF's `GetProjectionMatrix`, with
        /// the planes given, as the snapshot fits them to the scene). A matrix camera ignores
        /// the planes.
        fn get_projection_matrix(this, aspect_ratio: f64, near_plane_distance: f64, far_plane_distance: f64) -> Matrix3D;
    }
}

impl FerroObjectImpl for Camera {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::transform_property().as_property() {
            this.transform_link.set(this.transform().as_ref(), &this.notifier, camera);
        }
        // Every property of every camera is a property of the camera.
        this.notifier.raise(Scene3DChange::Camera);
    }
}

impl CameraImpl for Camera {
    fn get_view_matrix(this: &Self) -> Matrix3D {
        let mut matrix = Matrix3D::IDENTITY;
        prepend_inverse_transform(this.transform().as_ref(), &mut matrix);
        matrix
    }

    fn get_projection_matrix(_this: &Self, _aspect_ratio: f64, _near: f64, _far: f64) -> Matrix3D {
        Matrix3D::IDENTITY
    }
}

impl Notifying for Camera {
    fn notifier(&self) -> &Rc<Notifier> {
        &self.notifier
    }
}

ferro_properties! { impl Camera {
    pub fn transform_property() -> StyledProperty<Option<Ref<Transform3D>>> {
        FerroProperty::register::<Camera, _>("Transform", None)
    }
} }

impl Camera {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Animatable::construct(), notifier: Notifier::new(), transform_link: Link::default() }
    }

    /// The transform that places the camera in the world; none is the identity.
    pub fn transform(&self) -> Option<Ref<Transform3D>> {
        self.get_value(Self::transform_property())
    }

    pub fn set_transform(&self, value: Option<Ref<Transform3D>>) {
        self.set_value(Self::transform_property(), value)
    }

    /// Calls `handler` on every change of the camera, until the returned guard is dropped.
    pub fn changed(&self, handler: impl Fn(Scene3DChange) + 'static) -> Subscription {
        self.notifier.subscribe(Rc::new(handler))
    }

    /// The position of the camera in the world: the origin of its coordinates.
    pub fn world_position(&self) -> Option<Point3D> {
        let mut inverse_view = self.get_view_matrix();
        inverse_view.invert().ok()?;
        Some(inverse_view.transform(Point3D::default()))
    }
}

/// WPF's `Camera.PrependInverseTransform`: the inverse of the camera's transform before `view`;
/// a matrix of NaN when the transform has no inverse.
fn prepend_inverse_transform(transform: Option<&Ref<Transform3D>>, view: &mut Matrix3D) {
    let Some(transform) = transform else { return };
    let mut matrix = transform.value();
    if matrix.is_identity() {
        return;
    }
    if matrix.invert().is_ok() {
        view.prepend(matrix);
    } else {
        let nan = f64::NAN;
        *view = Matrix3D::new(nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan, nan);
    }
}

// --- ProjectionCamera --------------------------------------------------------------------------

/// WPF's `ProjectionCamera`: a camera at `Position` looking along `LookDirection` with
/// `UpDirection` up, between `NearPlaneDistance` and `FarPlaneDistance`.
#[repr(C)]
pub struct ProjectionCamera {
    base: Camera,
}

ferro_class!(ProjectionCamera: Camera);
ferro_impl_classes!(ProjectionCamera: FerroObjectImpl);

impl CameraImpl for ProjectionCamera {
    fn get_view_matrix(this: &Self) -> Matrix3D {
        let mut matrix = create_view_matrix(this.position(), this.look_direction(), this.up_direction());
        prepend_inverse_transform(this.transform().as_ref(), &mut matrix);
        matrix
    }
}

/// WPF's `ProjectionCamera.CreateViewMatrix`: the look-at matrix of a right-handed camera.
fn create_view_matrix(position: Point3D, look_direction: Vector3D, up_direction: Vector3D) -> Matrix3D {
    let mut z_axis = -look_direction;
    z_axis.normalize();
    let mut x_axis = Vector3D::cross_product(up_direction, z_axis);
    x_axis.normalize();
    let y_axis = Vector3D::cross_product(z_axis, x_axis);
    let position = Vector3D::from(position);
    let cx = -Vector3D::dot_product(x_axis, position);
    let cy = -Vector3D::dot_product(y_axis, position);
    let cz = -Vector3D::dot_product(z_axis, position);
    Matrix3D::new(
        x_axis.x, y_axis.x, z_axis.x, 0.0, x_axis.y, y_axis.y, z_axis.y, 0.0, x_axis.z, y_axis.z, z_axis.z, 0.0, cx, cy,
        cz, 1.0,
    )
}

ferro_properties! { impl ProjectionCamera {
    pub fn position_property() -> StyledProperty<Point3D> {
        FerroProperty::register::<ProjectionCamera, _>("Position", Point3D::new(0.0, 0.0, 0.0))
    }
    pub fn look_direction_property() -> StyledProperty<Vector3D> {
        FerroProperty::register::<ProjectionCamera, _>("LookDirection", Vector3D::new(0.0, 0.0, -1.0))
    }
    pub fn up_direction_property() -> StyledProperty<Vector3D> {
        FerroProperty::register::<ProjectionCamera, _>("UpDirection", Vector3D::new(0.0, 1.0, 0.0))
    }
    pub fn near_plane_distance_property() -> StyledProperty<f64> {
        FerroProperty::register::<ProjectionCamera, _>("NearPlaneDistance", 0.125)
    }
    pub fn far_plane_distance_property() -> StyledProperty<f64> {
        FerroProperty::register::<ProjectionCamera, _>("FarPlaneDistance", f64::INFINITY)
    }
} }

impl ProjectionCamera {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Camera::construct() }
    }

    /// The position of the camera, before its transform.
    pub fn position(&self) -> Point3D {
        self.get_value(Self::position_property())
    }

    pub fn set_position(&self, value: Point3D) {
        self.set_value(Self::position_property(), value)
    }

    /// The direction the camera looks in, before its transform.
    pub fn look_direction(&self) -> Vector3D {
        self.get_value(Self::look_direction_property())
    }

    pub fn set_look_direction(&self, value: Vector3D) {
        self.set_value(Self::look_direction_property(), value)
    }

    /// The direction of the top of the camera, before its transform.
    pub fn up_direction(&self) -> Vector3D {
        self.get_value(Self::up_direction_property())
    }

    pub fn set_up_direction(&self, value: Vector3D) {
        self.set_value(Self::up_direction_property(), value)
    }

    /// The distance of the near clipping plane (0.125 by default).
    pub fn near_plane_distance(&self) -> f64 {
        self.get_value(Self::near_plane_distance_property())
    }

    pub fn set_near_plane_distance(&self, value: f64) {
        self.set_value(Self::near_plane_distance_property(), value)
    }

    /// The distance of the far clipping plane (infinite by default: the snapshot then fits it to
    /// the scene).
    pub fn far_plane_distance(&self) -> f64 {
        self.get_value(Self::far_plane_distance_property())
    }

    pub fn set_far_plane_distance(&self, value: f64) {
        self.set_value(Self::far_plane_distance_property(), value)
    }
}

// --- PerspectiveCamera -------------------------------------------------------------------------

/// WPF's `PerspectiveCamera`: a perspective projection with a horizontal `FieldOfView` in
/// degrees (45 by default).
#[repr(C)]
pub struct PerspectiveCamera {
    base: ProjectionCamera,
}

ferro_class!(PerspectiveCamera: ProjectionCamera);
ferro_class_info!(PerspectiveCamera {
    new: PerspectiveCamera::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Point3D, Vector3D, Vector3D, f64) => PerspectiveCamera::with_values,
        ],
    },
});
ferro_impl_classes!(PerspectiveCamera: FerroObjectImpl);

impl CameraImpl for PerspectiveCamera {
    fn get_projection_matrix(this: &Self, aspect_ratio: f64, near: f64, far: f64) -> Matrix3D {
        // The field of view is horizontal: the vertical scale is the horizontal one times the
        // aspect ratio.
        let half_width_depth_ratio = (this.field_of_view().to_radians() / 2.0).tan();
        let m11 = 1.0 / half_width_depth_ratio;
        let m22 = aspect_ratio * m11;
        let (m33, m43) = if far.is_infinite() {
            (-1.0, -near)
        } else {
            let m33 = far / (near - far);
            (m33, near * m33)
        };
        Matrix3D::new(m11, 0.0, 0.0, 0.0, 0.0, m22, 0.0, 0.0, 0.0, 0.0, m33, -1.0, 0.0, 0.0, m43, 0.0)
    }
}

ferro_properties! { impl PerspectiveCamera {
    pub fn field_of_view_property() -> StyledProperty<f64> {
        FerroProperty::register::<PerspectiveCamera, _>("FieldOfView", 45.0)
    }
} }

impl PerspectiveCamera {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: ProjectionCamera::construct() }
    }

    /// A camera with WPF's defaults: at the origin, looking down the negative z axis, 45 degrees.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A camera at `position` looking along `look_direction` with `up_direction` up and the
    /// horizontal `field_of_view` in degrees.
    pub fn with_values(
        position: Point3D,
        look_direction: Vector3D,
        up_direction: Vector3D,
        field_of_view: f64,
    ) -> Ref<Self> {
        let result = Self::new();
        result.set_position(position);
        result.set_look_direction(look_direction);
        result.set_up_direction(up_direction);
        result.set_field_of_view(field_of_view);
        result
    }

    /// The horizontal field of view, in degrees.
    pub fn field_of_view(&self) -> f64 {
        self.get_value(Self::field_of_view_property())
    }

    pub fn set_field_of_view(&self, value: f64) {
        self.set_value(Self::field_of_view_property(), value)
    }
}

// --- OrthographicCamera ------------------------------------------------------------------------

/// WPF's `OrthographicCamera`: a parallel projection `Width` wide (2 by default).
#[repr(C)]
pub struct OrthographicCamera {
    base: ProjectionCamera,
}

ferro_class!(OrthographicCamera: ProjectionCamera);
ferro_class_info!(OrthographicCamera {
    new: OrthographicCamera::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Point3D, Vector3D, Vector3D, f64) => OrthographicCamera::with_values,
        ],
    },
});
ferro_impl_classes!(OrthographicCamera: FerroObjectImpl);

impl CameraImpl for OrthographicCamera {
    fn get_projection_matrix(this: &Self, aspect_ratio: f64, near: f64, far: f64) -> Matrix3D {
        let m11 = 2.0 / this.width();
        let m22 = aspect_ratio * m11;
        let m33 = 1.0 / (near - far);
        let m43 = near * m33;
        Matrix3D::new(m11, 0.0, 0.0, 0.0, 0.0, m22, 0.0, 0.0, 0.0, 0.0, m33, 0.0, 0.0, 0.0, m43, 1.0)
    }
}

ferro_properties! { impl OrthographicCamera {
    pub fn width_property() -> StyledProperty<f64> {
        FerroProperty::register::<OrthographicCamera, _>("Width", 2.0)
    }
} }

impl OrthographicCamera {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: ProjectionCamera::construct() }
    }

    /// A camera with WPF's defaults: at the origin, looking down the negative z axis, 2 wide.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A camera at `position` looking along `look_direction` with `up_direction` up, `width`
    /// wide.
    pub fn with_values(position: Point3D, look_direction: Vector3D, up_direction: Vector3D, width: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_position(position);
        result.set_look_direction(look_direction);
        result.set_up_direction(up_direction);
        result.set_width(width);
        result
    }

    /// The width of the view.
    pub fn width(&self) -> f64 {
        self.get_value(Self::width_property())
    }

    pub fn set_width(&self, value: f64) {
        self.set_value(Self::width_property(), value)
    }
}

// --- MatrixCamera ------------------------------------------------------------------------------

/// WPF's `MatrixCamera`: the camera of a `ViewMatrix` and a `ProjectionMatrix`.
#[repr(C)]
pub struct MatrixCamera {
    base: Camera,
}

ferro_class!(MatrixCamera: Camera);
ferro_class_info!(MatrixCamera {
    new: MatrixCamera::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Matrix3D, Matrix3D) => MatrixCamera::with_matrices,
        ],
    },
});
ferro_impl_classes!(MatrixCamera: FerroObjectImpl);

impl CameraImpl for MatrixCamera {
    fn get_view_matrix(this: &Self) -> Matrix3D {
        let mut matrix = this.view_matrix();
        prepend_inverse_transform(this.transform().as_ref(), &mut matrix);
        matrix
    }

    fn get_projection_matrix(this: &Self, _aspect_ratio: f64, _near: f64, _far: f64) -> Matrix3D {
        this.projection_matrix()
    }
}

ferro_properties! { impl MatrixCamera {
    pub fn view_matrix_property() -> StyledProperty<Matrix3D> {
        FerroProperty::register::<MatrixCamera, _>("ViewMatrix", Matrix3D::IDENTITY)
    }
    pub fn projection_matrix_property() -> StyledProperty<Matrix3D> {
        FerroProperty::register::<MatrixCamera, _>("ProjectionMatrix", Matrix3D::IDENTITY)
    }
} }

impl MatrixCamera {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Camera::construct() }
    }

    /// A camera of two identities.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The camera of the two matrices.
    pub fn with_matrices(view_matrix: Matrix3D, projection_matrix: Matrix3D) -> Ref<Self> {
        let result = Self::new();
        result.set_view_matrix(view_matrix);
        result.set_projection_matrix(projection_matrix);
        result
    }

    /// The view matrix, before the transform of the camera.
    pub fn view_matrix(&self) -> Matrix3D {
        self.get_value(Self::view_matrix_property())
    }

    pub fn set_view_matrix(&self, value: Matrix3D) {
        self.set_value(Self::view_matrix_property(), value)
    }

    /// The projection matrix.
    pub fn projection_matrix(&self) -> Matrix3D {
        self.get_value(Self::projection_matrix_property())
    }

    pub fn set_projection_matrix(&self, value: Matrix3D) {
        self.set_value(Self::projection_matrix_property(), value)
    }
}
