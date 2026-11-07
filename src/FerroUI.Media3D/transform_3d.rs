//! WPF's 3D transforms: `Transform3D` and its classes `TranslateTransform3D`,
//! `ScaleTransform3D`, `RotateTransform3D`, `MatrixTransform3D` and `Transform3DGroup`.
//!
//! A transform is a [`Matrix3D`] ([`Transform3D::value`]) in WPF's row-vector convention: a
//! point is transformed as `p * value`. A change of a transform is a change of a record of the
//! scene ([`Scene3DChange::Record`]).

use crate::change::{same, ListLink, Link, Notifier, Notifying, Scene3DChange};
use crate::{Matrix3D, Point3D, Rect3D, Rotation3D, Transform3DCollection, Vector3D};
use ferroui_base::animation::Animatable;
use ferroui_base::reactive::Subscription;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use std::cell::Cell;
use std::rc::Rc;

/// The bounds of `rect` transformed by `matrix`: the box of its eight corners transformed.
pub(crate) fn transform_rect(rect: Rect3D, matrix: &Matrix3D) -> Rect3D {
    if rect.is_empty() || matrix.is_identity() {
        return rect;
    }
    let (x0, y0, z0) = (rect.x(), rect.y(), rect.z());
    let (x1, y1, z1) = (x0 + rect.size_x(), y0 + rect.size_y(), z0 + rect.size_z());
    let corners = [
        Point3D::new(x0, y0, z0),
        Point3D::new(x1, y0, z0),
        Point3D::new(x0, y1, z0),
        Point3D::new(x1, y1, z0),
        Point3D::new(x0, y0, z1),
        Point3D::new(x1, y0, z1),
        Point3D::new(x0, y1, z1),
        Point3D::new(x1, y1, z1),
    ];
    let first = matrix.transform(corners[0]);
    let mut bounds = Rect3D::from_points(first, first);
    for corner in &corners[1..] {
        bounds.union_point(matrix.transform(*corner));
    }
    bounds
}

/// WPF's `Transform3D`: the base of the 3D transforms.
#[repr(C)]
pub struct Transform3D {
    base: Animatable,
    notifier: Rc<Notifier>,
}

ferro_class! {
    Transform3D: Animatable, virtuals Transform3DImpl: FerroObjectImpl {
        /// The matrix of the transform (WPF's `Value`).
        fn value(this) -> Matrix3D;
    }
}

ferro_impl_classes!(Transform3D: FerroObjectImpl);

impl Transform3DImpl for Transform3D {
    fn value(_this: &Self) -> Matrix3D {
        Matrix3D::IDENTITY
    }
}

impl Notifying for Transform3D {
    fn notifier(&self) -> &Rc<Notifier> {
        &self.notifier
    }
}

impl Transform3D {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Animatable::construct(), notifier: Notifier::new() }
    }

    /// WPF's `Transform3D.Identity`: a new transform that changes nothing.
    pub fn identity() -> Ref<Transform3D> {
        MatrixTransform3D::new().upcast()
    }

    /// Whether the transform is affine (WPF's `IsAffine`): its matrix has no projective part.
    pub fn is_affine(&self) -> bool {
        self.value().is_affine()
    }

    /// The inverse transform (WPF's `Inverse`); `None` when the matrix has no inverse.
    pub fn inverse(&self) -> Option<Ref<Transform3D>> {
        let mut matrix = self.value();
        matrix.invert().ok()?;
        Some(MatrixTransform3D::with_matrix(matrix).upcast())
    }

    /// The point transformed (WPF's `TryTransform`); `None` when the result is not finite (a
    /// projective transform that sends the point to infinity).
    pub fn try_transform(&self, point: Point3D) -> Option<Point3D> {
        let result = self.value().transform(point);
        (result.x.is_finite() && result.y.is_finite() && result.z.is_finite()).then_some(result)
    }

    /// The point transformed (WPF's `Transform`).
    pub fn transform(&self, point: Point3D) -> Point3D {
        self.value().transform(point)
    }

    /// The axis-aligned bounds of the box transformed (WPF's `TransformBounds`).
    pub fn transform_bounds(&self, rect: Rect3D) -> Rect3D {
        transform_rect(rect, &self.value())
    }

    /// Calls `handler` on every change of the transform, until the returned guard is dropped.
    pub fn changed(&self, handler: impl Fn(Scene3DChange) + 'static) -> Subscription {
        self.notifier.subscribe(Rc::new(handler))
    }

    /// Reports a change of the matrix to the owners of the transform.
    fn raise_record(&self) {
        self.notifier.raise(Scene3DChange::Record);
    }
}

/// The matrix of an optional transform: the identity for none.
pub(crate) fn value_of(transform: Option<&Ref<Transform3D>>) -> Matrix3D {
    transform.map_or(Matrix3D::IDENTITY, |transform| transform.value())
}

// --- TranslateTransform3D ----------------------------------------------------------------------

/// WPF's `TranslateTransform3D`: a translation by `OffsetX`, `OffsetY` and `OffsetZ`.
#[repr(C)]
pub struct TranslateTransform3D {
    base: Transform3D,
}

ferro_class!(TranslateTransform3D: Transform3D);
ferro_class_info!(TranslateTransform3D {
    new: TranslateTransform3D::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Vector3D) => TranslateTransform3D::with_offset,
            (f64, f64, f64) => TranslateTransform3D::with_offset_xyz,
        ],
    },
});

impl FerroObjectImpl for TranslateTransform3D {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        this.raise_record();
    }
}

impl Transform3DImpl for TranslateTransform3D {
    fn value(this: &Self) -> Matrix3D {
        let mut matrix = Matrix3D::IDENTITY;
        matrix.translate(Vector3D::new(this.offset_x(), this.offset_y(), this.offset_z()));
        matrix
    }
}

ferro_properties! { impl TranslateTransform3D {
    pub fn offset_x_property() -> StyledProperty<f64> {
        FerroProperty::register::<TranslateTransform3D, _>("OffsetX", 0.0)
    }
    pub fn offset_y_property() -> StyledProperty<f64> {
        FerroProperty::register::<TranslateTransform3D, _>("OffsetY", 0.0)
    }
    pub fn offset_z_property() -> StyledProperty<f64> {
        FerroProperty::register::<TranslateTransform3D, _>("OffsetZ", 0.0)
    }
} }

impl TranslateTransform3D {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Transform3D::construct() }
    }

    /// A translation by nothing.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A translation by `offset`.
    pub fn with_offset(offset: Vector3D) -> Ref<Self> {
        Self::with_offset_xyz(offset.x, offset.y, offset.z)
    }

    /// A translation by the three offsets.
    pub fn with_offset_xyz(offset_x: f64, offset_y: f64, offset_z: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_offset_x(offset_x);
        result.set_offset_y(offset_y);
        result.set_offset_z(offset_z);
        result
    }

    /// The translation along the x axis.
    pub fn offset_x(&self) -> f64 {
        self.get_value(Self::offset_x_property())
    }

    pub fn set_offset_x(&self, value: f64) {
        self.set_value(Self::offset_x_property(), value)
    }

    /// The translation along the y axis.
    pub fn offset_y(&self) -> f64 {
        self.get_value(Self::offset_y_property())
    }

    pub fn set_offset_y(&self, value: f64) {
        self.set_value(Self::offset_y_property(), value)
    }

    /// The translation along the z axis.
    pub fn offset_z(&self) -> f64 {
        self.get_value(Self::offset_z_property())
    }

    pub fn set_offset_z(&self, value: f64) {
        self.set_value(Self::offset_z_property(), value)
    }
}

// --- ScaleTransform3D --------------------------------------------------------------------------

/// WPF's `ScaleTransform3D`: a scale by `ScaleX`, `ScaleY` and `ScaleZ` about the center
/// (`CenterX`, `CenterY`, `CenterZ`).
#[repr(C)]
pub struct ScaleTransform3D {
    base: Transform3D,
}

ferro_class!(ScaleTransform3D: Transform3D);
ferro_class_info!(ScaleTransform3D {
    new: ScaleTransform3D::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Vector3D) => ScaleTransform3D::with_scale,
            (f64, f64, f64) => ScaleTransform3D::with_scale_xyz,
            (Vector3D, Point3D) => ScaleTransform3D::with_scale_and_center,
        ],
    },
});

impl FerroObjectImpl for ScaleTransform3D {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        this.raise_record();
    }
}

impl Transform3DImpl for ScaleTransform3D {
    fn value(this: &Self) -> Matrix3D {
        let mut matrix = Matrix3D::IDENTITY;
        matrix.scale_at(
            Vector3D::new(this.scale_x(), this.scale_y(), this.scale_z()),
            Point3D::new(this.center_x(), this.center_y(), this.center_z()),
        );
        matrix
    }
}

ferro_properties! { impl ScaleTransform3D {
    pub fn scale_x_property() -> StyledProperty<f64> {
        FerroProperty::register::<ScaleTransform3D, _>("ScaleX", 1.0)
    }
    pub fn scale_y_property() -> StyledProperty<f64> {
        FerroProperty::register::<ScaleTransform3D, _>("ScaleY", 1.0)
    }
    pub fn scale_z_property() -> StyledProperty<f64> {
        FerroProperty::register::<ScaleTransform3D, _>("ScaleZ", 1.0)
    }
    pub fn center_x_property() -> StyledProperty<f64> {
        FerroProperty::register::<ScaleTransform3D, _>("CenterX", 0.0)
    }
    pub fn center_y_property() -> StyledProperty<f64> {
        FerroProperty::register::<ScaleTransform3D, _>("CenterY", 0.0)
    }
    pub fn center_z_property() -> StyledProperty<f64> {
        FerroProperty::register::<ScaleTransform3D, _>("CenterZ", 0.0)
    }
} }

impl ScaleTransform3D {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Transform3D::construct() }
    }

    /// A scale by one.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A scale by the components of `scale` about the origin.
    pub fn with_scale(scale: Vector3D) -> Ref<Self> {
        Self::with_scale_xyz(scale.x, scale.y, scale.z)
    }

    /// A scale by the three factors about the origin.
    pub fn with_scale_xyz(scale_x: f64, scale_y: f64, scale_z: f64) -> Ref<Self> {
        let result = Self::new();
        result.set_scale_x(scale_x);
        result.set_scale_y(scale_y);
        result.set_scale_z(scale_z);
        result
    }

    /// A scale by the components of `scale` about `center`.
    pub fn with_scale_and_center(scale: Vector3D, center: Point3D) -> Ref<Self> {
        let result = Self::with_scale(scale);
        result.set_center_x(center.x);
        result.set_center_y(center.y);
        result.set_center_z(center.z);
        result
    }

    /// The scale along the x axis.
    pub fn scale_x(&self) -> f64 {
        self.get_value(Self::scale_x_property())
    }

    pub fn set_scale_x(&self, value: f64) {
        self.set_value(Self::scale_x_property(), value)
    }

    /// The scale along the y axis.
    pub fn scale_y(&self) -> f64 {
        self.get_value(Self::scale_y_property())
    }

    pub fn set_scale_y(&self, value: f64) {
        self.set_value(Self::scale_y_property(), value)
    }

    /// The scale along the z axis.
    pub fn scale_z(&self) -> f64 {
        self.get_value(Self::scale_z_property())
    }

    pub fn set_scale_z(&self, value: f64) {
        self.set_value(Self::scale_z_property(), value)
    }

    /// The x coordinate of the center of the scale.
    pub fn center_x(&self) -> f64 {
        self.get_value(Self::center_x_property())
    }

    pub fn set_center_x(&self, value: f64) {
        self.set_value(Self::center_x_property(), value)
    }

    /// The y coordinate of the center of the scale.
    pub fn center_y(&self) -> f64 {
        self.get_value(Self::center_y_property())
    }

    pub fn set_center_y(&self, value: f64) {
        self.set_value(Self::center_y_property(), value)
    }

    /// The z coordinate of the center of the scale.
    pub fn center_z(&self) -> f64 {
        self.get_value(Self::center_z_property())
    }

    pub fn set_center_z(&self, value: f64) {
        self.set_value(Self::center_z_property(), value)
    }
}

// --- RotateTransform3D -------------------------------------------------------------------------

/// WPF's `RotateTransform3D`: the rotation of its `Rotation` about the center (`CenterX`,
/// `CenterY`, `CenterZ`). Without a rotation it is the identity.
#[repr(C)]
pub struct RotateTransform3D {
    base: Transform3D,
    rotation_link: Link,
}

ferro_class!(RotateTransform3D: Transform3D);
ferro_class_info!(RotateTransform3D {
    new: RotateTransform3D::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Ref<Rotation3D>) => RotateTransform3D::with_rotation,
            (Ref<Rotation3D>, Point3D) => RotateTransform3D::with_rotation_and_center,
        ],
    },
});

impl FerroObjectImpl for RotateTransform3D {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::rotation_property().as_property() {
            this.rotation_link.set(this.rotation().as_ref(), this.notifier(), same);
        }
        this.raise_record();
    }
}

impl Transform3DImpl for RotateTransform3D {
    fn value(this: &Self) -> Matrix3D {
        let Some(rotation) = this.rotation() else {
            return Matrix3D::IDENTITY;
        };
        let mut matrix = Matrix3D::IDENTITY;
        matrix.rotate_at(
            rotation.internal_quaternion(),
            Point3D::new(this.center_x(), this.center_y(), this.center_z()),
        );
        matrix
    }
}

ferro_properties! { impl RotateTransform3D {
    pub fn rotation_property() -> StyledProperty<Option<Ref<Rotation3D>>> {
        FerroProperty::register::<RotateTransform3D, _>("Rotation", None)
    }
    pub fn center_x_property() -> StyledProperty<f64> {
        FerroProperty::register::<RotateTransform3D, _>("CenterX", 0.0)
    }
    pub fn center_y_property() -> StyledProperty<f64> {
        FerroProperty::register::<RotateTransform3D, _>("CenterY", 0.0)
    }
    pub fn center_z_property() -> StyledProperty<f64> {
        FerroProperty::register::<RotateTransform3D, _>("CenterZ", 0.0)
    }
} }

impl RotateTransform3D {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Transform3D::construct(), rotation_link: Link::default() }
    }

    /// A transform without a rotation.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The rotation about the origin.
    pub fn with_rotation(rotation: Ref<Rotation3D>) -> Ref<Self> {
        let result = Self::new();
        result.set_rotation(Some(rotation));
        result
    }

    /// The rotation about `center`.
    pub fn with_rotation_and_center(rotation: Ref<Rotation3D>, center: Point3D) -> Ref<Self> {
        let result = Self::with_rotation(rotation);
        result.set_center_x(center.x);
        result.set_center_y(center.y);
        result.set_center_z(center.z);
        result
    }

    /// The rotation.
    pub fn rotation(&self) -> Option<Ref<Rotation3D>> {
        self.get_value(Self::rotation_property())
    }

    pub fn set_rotation(&self, value: Option<Ref<Rotation3D>>) {
        self.set_value(Self::rotation_property(), value)
    }

    /// The x coordinate of the center of the rotation.
    pub fn center_x(&self) -> f64 {
        self.get_value(Self::center_x_property())
    }

    pub fn set_center_x(&self, value: f64) {
        self.set_value(Self::center_x_property(), value)
    }

    /// The y coordinate of the center of the rotation.
    pub fn center_y(&self) -> f64 {
        self.get_value(Self::center_y_property())
    }

    pub fn set_center_y(&self, value: f64) {
        self.set_value(Self::center_y_property(), value)
    }

    /// The z coordinate of the center of the rotation.
    pub fn center_z(&self) -> f64 {
        self.get_value(Self::center_z_property())
    }

    pub fn set_center_z(&self, value: f64) {
        self.set_value(Self::center_z_property(), value)
    }
}

// --- MatrixTransform3D -------------------------------------------------------------------------

/// WPF's `MatrixTransform3D`: the transform of its `Matrix`.
#[repr(C)]
pub struct MatrixTransform3D {
    base: Transform3D,
}

ferro_class!(MatrixTransform3D: Transform3D);
ferro_class_info!(MatrixTransform3D {
    new: MatrixTransform3D::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        constructors: [
            (Matrix3D) => MatrixTransform3D::with_matrix,
        ],
    },
});

impl FerroObjectImpl for MatrixTransform3D {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        this.raise_record();
    }
}

impl Transform3DImpl for MatrixTransform3D {
    fn value(this: &Self) -> Matrix3D {
        this.matrix()
    }
}

ferro_properties! { impl MatrixTransform3D {
    pub fn matrix_property() -> StyledProperty<Matrix3D> {
        FerroProperty::register::<MatrixTransform3D, _>("Matrix", Matrix3D::IDENTITY)
    }
} }

impl MatrixTransform3D {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Transform3D::construct() }
    }

    /// The identity.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The transform of `matrix`.
    pub fn with_matrix(matrix: Matrix3D) -> Ref<Self> {
        let result = Self::new();
        result.set_matrix(matrix);
        result
    }

    /// The matrix.
    pub fn matrix(&self) -> Matrix3D {
        self.get_value(Self::matrix_property())
    }

    pub fn set_matrix(&self, value: Matrix3D) {
        self.set_value(Self::matrix_property(), value)
    }
}

// --- Transform3DGroup --------------------------------------------------------------------------

/// WPF's `Transform3DGroup`: its `Children` applied in order (the first one first).
#[repr(C)]
pub struct Transform3DGroup {
    base: Transform3D,
    children_link: ListLink<Transform3D>,
    cached: Cell<Option<Matrix3D>>,
    cache_subscription: std::cell::RefCell<Option<Subscription>>,
}

ferro_class!(Transform3DGroup: Transform3D);
ferro_class_info!(Transform3DGroup {
    new: Transform3DGroup::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
        content: Children,
    },
});

impl FerroObjectImpl for Transform3DGroup {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        // The cached product is dropped on every change, before the owners hear of it.
        let weak = this.to_ref().downgrade();
        let subscription = this.notifier().subscribe(Rc::new(move |_: Scene3DChange| {
            if let Some(this) = weak.upgrade() {
                this.cached.set(None);
            }
        }));
        *this.cache_subscription.borrow_mut() = Some(subscription);
        this.set_children(Transform3DCollection::new());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::children_property().as_property() {
            let children = this.get_value(Self::children_property());
            this.children_link.set(children.as_deref(), this.notifier(), Scene3DChange::Record);
            this.raise_record();
        }
    }
}

impl Transform3DImpl for Transform3DGroup {
    fn value(this: &Self) -> Matrix3D {
        if let Some(matrix) = this.cached.get() {
            return matrix;
        }
        let mut matrix = Matrix3D::IDENTITY;
        if let Some(children) = this.get_value(Self::children_property()) {
            for child in children.iter() {
                matrix.append(child.value());
            }
        }
        this.cached.set(Some(matrix));
        matrix
    }
}

ferro_properties! { impl Transform3DGroup {
    pub fn children_property() -> StyledProperty<Option<Transform3DCollection>> {
        FerroProperty::register::<Transform3DGroup, _>("Children", None)
    }
} }

impl Transform3DGroup {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: Transform3D::construct(),
            children_link: ListLink::default(),
            cached: Cell::new(None),
            cache_subscription: std::cell::RefCell::new(None),
        }
    }

    /// An empty group (the identity).
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The children. Panics if the property has been cleared.
    pub fn children(&self) -> Transform3DCollection {
        self.get_value(Self::children_property()).expect("Children is not set")
    }

    pub fn set_children(&self, value: Transform3DCollection) {
        self.set_value(Self::children_property(), Some(value))
    }
}
