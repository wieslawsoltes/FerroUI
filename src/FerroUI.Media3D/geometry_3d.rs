//! WPF's `Geometry3D` and `MeshGeometry3D`: the triangles of a model.
//!
//! A mesh has an identity for the life of the process and a version that every change bumps; the
//! snapshot shares its [`MeshData`] (single precision, made once per version), and an executor
//! keeps the buffers of a mesh by the pair. Any change of a mesh is a change of the structure
//! ([`Scene3DChange::Structure`]).

use crate::change::{Notifier, Notifying, Scene3DChange, ValueListLink};
use crate::{Int32Collection, Point3D, Point3DCollection, Rect3D, Vector3D, Vector3DCollection};
use ferroui_base::animation::Animatable;
use ferroui_base::media::Points;
use ferroui_base::numerics::{Vector2, Vector3};
use ferroui_base::reactive::Subscription;
use ferroui_base::rendering::scene3d::{MeshData, Scene3DBounds};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// WPF's `Geometry3D`: the base of the 3D geometries.
#[repr(C)]
pub struct Geometry3D {
    base: Animatable,
    notifier: Rc<Notifier>,
}

ferro_class! {
    Geometry3D: Animatable, virtuals Geometry3DImpl: FerroObjectImpl {
        /// The axis-aligned bounds of the geometry (WPF's `Bounds`); [`Rect3D::EMPTY`] when it
        /// has no points.
        fn bounds(this) -> Rect3D;
    }
}

ferro_impl_classes!(Geometry3D: FerroObjectImpl);

impl Geometry3DImpl for Geometry3D {
    fn bounds(_this: &Self) -> Rect3D {
        Rect3D::EMPTY
    }
}

impl Notifying for Geometry3D {
    fn notifier(&self) -> &Rc<Notifier> {
        &self.notifier
    }
}

impl Geometry3D {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Animatable::construct(), notifier: Notifier::new() }
    }

    /// Calls `handler` on every change of the geometry, until the returned guard is dropped.
    pub fn changed(&self, handler: impl Fn(Scene3DChange) + 'static) -> Subscription {
        self.notifier.subscribe(Rc::new(handler))
    }
}

/// The identities of the meshes, for the process.
static NEXT_MESH_ID: AtomicU64 = AtomicU64::new(1);

/// WPF's `MeshGeometry3D`: `Positions`, `Normals`, `TextureCoordinates` and
/// `TriangleIndices`.
///
/// When `TriangleIndices` is empty the positions are taken three at a time; a triangle with an
/// index out of range is left out; normals are generated from the faces when there is not one
/// per position, as WPF does.
#[repr(C)]
pub struct MeshGeometry3D {
    base: Geometry3D,
    id: u64,
    version: Cell<u64>,
    data: RefCell<Option<Arc<MeshData>>>,
    positions_link: ValueListLink,
    normals_link: ValueListLink,
    texture_coordinates_link: ValueListLink,
    triangle_indices_link: ValueListLink,
}

ferro_class!(MeshGeometry3D: Geometry3D);
ferro_class_info!(MeshGeometry3D {
    new: MeshGeometry3D::new,
    markup: {
        namespace: "FerroUI.Media.Media3D",
    },
});

impl FerroObjectImpl for MeshGeometry3D {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_positions(Point3DCollection::new());
        this.set_normals(Vector3DCollection::new());
        this.set_texture_coordinates(Points::new());
        this.set_triangle_indices(Int32Collection::new());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        let weak = this.to_ref().downgrade();
        let changed = move || {
            if let Some(mesh) = weak.upgrade() {
                mesh.content_changed();
            }
        };
        if property == Self::positions_property().as_property() {
            let positions = this.get_value(Self::positions_property());
            this.positions_link.set(positions.as_deref(), changed);
        } else if property == Self::normals_property().as_property() {
            let normals = this.get_value(Self::normals_property());
            this.normals_link.set(normals.as_deref(), changed);
        } else if property == Self::texture_coordinates_property().as_property() {
            let texture_coordinates = this.get_value(Self::texture_coordinates_property());
            this.texture_coordinates_link.set(texture_coordinates.as_deref(), changed);
        } else if property == Self::triangle_indices_property().as_property() {
            let triangle_indices = this.get_value(Self::triangle_indices_property());
            this.triangle_indices_link.set(triangle_indices.as_deref(), changed);
        } else {
            return;
        }
        this.content_changed();
    }
}

impl Geometry3DImpl for MeshGeometry3D {
    fn bounds(this: &Self) -> Rect3D {
        let Some(positions) = this.get_value(Self::positions_property()) else {
            return Rect3D::EMPTY;
        };
        let mut points = positions.iter();
        let Some(first) = points.next() else {
            return Rect3D::EMPTY;
        };
        let mut bounds = Rect3D::from_points(first, first);
        for point in points {
            bounds.union_point(point);
        }
        bounds
    }
}

ferro_properties! { impl MeshGeometry3D {
    pub fn positions_property() -> StyledProperty<Option<Point3DCollection>> {
        FerroProperty::register::<MeshGeometry3D, _>("Positions", None)
    }
    pub fn normals_property() -> StyledProperty<Option<Vector3DCollection>> {
        FerroProperty::register::<MeshGeometry3D, _>("Normals", None)
    }
    pub fn texture_coordinates_property() -> StyledProperty<Option<Points>> {
        FerroProperty::register::<MeshGeometry3D, _>("TextureCoordinates", None)
    }
    pub fn triangle_indices_property() -> StyledProperty<Option<Int32Collection>> {
        FerroProperty::register::<MeshGeometry3D, _>("TriangleIndices", None)
    }
} }

impl MeshGeometry3D {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: Geometry3D::construct(),
            id: NEXT_MESH_ID.fetch_add(1, Ordering::Relaxed),
            version: Cell::new(1),
            data: RefCell::new(None),
            positions_link: ValueListLink::default(),
            normals_link: ValueListLink::default(),
            texture_coordinates_link: ValueListLink::default(),
            triangle_indices_link: ValueListLink::default(),
        }
    }

    /// An empty mesh.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The identity of the mesh, unique in the process.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The version of the content of the mesh: bumped by every change of a collection or of a
    /// property.
    pub fn version(&self) -> u64 {
        self.version.get()
    }

    /// The positions of the vertices. Panics if the property has been cleared.
    pub fn positions(&self) -> Point3DCollection {
        self.get_value(Self::positions_property()).expect("Positions is not set")
    }

    pub fn set_positions(&self, value: Point3DCollection) {
        self.set_value(Self::positions_property(), Some(value))
    }

    /// The normals of the vertices. Panics if the property has been cleared.
    pub fn normals(&self) -> Vector3DCollection {
        self.get_value(Self::normals_property()).expect("Normals is not set")
    }

    pub fn set_normals(&self, value: Vector3DCollection) {
        self.set_value(Self::normals_property(), Some(value))
    }

    /// The texture coordinates of the vertices. Panics if the property has been cleared.
    pub fn texture_coordinates(&self) -> Points {
        self.get_value(Self::texture_coordinates_property()).expect("TextureCoordinates is not set")
    }

    pub fn set_texture_coordinates(&self, value: Points) {
        self.set_value(Self::texture_coordinates_property(), Some(value))
    }

    /// The indices of the vertices of the triangles, three per triangle. Panics if the property
    /// has been cleared.
    pub fn triangle_indices(&self) -> Int32Collection {
        self.get_value(Self::triangle_indices_property()).expect("TriangleIndices is not set")
    }

    pub fn set_triangle_indices(&self, value: Int32Collection) {
        self.set_value(Self::triangle_indices_property(), Some(value))
    }

    /// A new version: the data made for the previous one is dropped and the owners hear of a
    /// change of the structure.
    fn content_changed(&self) {
        self.version.set(self.version.get() + 1);
        self.data.borrow_mut().take();
        self.notifier().raise(Scene3DChange::Structure);
    }

    /// The triangles in single precision, made once per version and shared.
    pub fn mesh_data(&self) -> Arc<MeshData> {
        if let Some(data) = self.data.borrow().as_ref() {
            return data.clone();
        }
        let data = Arc::new(self.build_mesh_data());
        *self.data.borrow_mut() = Some(data.clone());
        data
    }

    fn build_mesh_data(&self) -> MeshData {
        let narrow = |p: Point3D| Vector3::new(p.x as f32, p.y as f32, p.z as f32);
        let positions: Vec<Vector3> = self
            .get_value(Self::positions_property())
            .map(|positions| positions.iter().map(narrow).collect())
            .unwrap_or_default();
        let count = positions.len();
        let indices = triangle_indices(
            self.get_value(Self::triangle_indices_property()).map(|indices| indices.to_vec()).unwrap_or_default(),
            count,
        );
        let given_normals: Vec<Vector3D> =
            self.get_value(Self::normals_property()).map(|normals| normals.to_vec()).unwrap_or_default();
        let (normals, normals_generated) = if given_normals.len() == count && count > 0 {
            (given_normals.into_iter().map(|n| unit(Vector3::new(n.x as f32, n.y as f32, n.z as f32))).collect(), false)
        } else {
            (generate_normals(&positions, &indices), true)
        };
        let given_coordinates: Vec<ferroui_base::Point> = self
            .get_value(Self::texture_coordinates_property())
            .map(|coordinates| coordinates.to_vec())
            .unwrap_or_default();
        let texture_coordinates = if given_coordinates.is_empty() {
            Vec::new()
        } else {
            // One per position: the missing ones are (0, 0), the extra ones left out.
            (0..count)
                .map(|index| {
                    given_coordinates
                        .get(index)
                        .map_or(Vector2::new(0.0, 0.0), |point| Vector2::new(point.x as f32, point.y as f32))
                })
                .collect()
        };
        let bounds = Scene3DBounds::of_points(positions.iter().copied());
        MeshData {
            id: self.id,
            version: self.version.get(),
            positions,
            normals,
            normals_generated,
            texture_coordinates,
            indices,
            bounds,
        }
    }
}

/// The indices of the triangles that can be drawn: the given ones three at a time (an
/// incomplete last triangle and the triangles with an index out of range left out), or, without
/// any, the positions three at a time.
fn triangle_indices(given: Vec<i32>, vertex_count: usize) -> Vec<u32> {
    if given.is_empty() {
        let count = vertex_count - vertex_count % 3;
        return (0..count as u32).collect();
    }
    let valid = |index: i32| index >= 0 && (index as usize) < vertex_count;
    let mut indices = Vec::with_capacity(given.len());
    for triangle in given.chunks_exact(3) {
        if triangle.iter().all(|index| valid(*index)) {
            indices.extend(triangle.iter().map(|index| *index as u32));
        }
    }
    indices
}

/// The vector of unit length along `vector`; `vector` itself when it is zero.
fn unit(vector: Vector3) -> Vector3 {
    let length = (vector.x * vector.x + vector.y * vector.y + vector.z * vector.z).sqrt();
    if length > 0.0 && length.is_finite() {
        Vector3::new(vector.x / length, vector.y / length, vector.z / length)
    } else {
        vector
    }
}

/// The normals of the vertices: the sum of the normals of the faces that share each vertex,
/// weighted by their areas (the cross product of two edges, counterclockwise), at unit length.
/// A vertex of no face gets (0, 0, 1).
fn generate_normals(positions: &[Vector3], indices: &[u32]) -> Vec<Vector3> {
    let mut sums = vec![Vector3::new(0.0, 0.0, 0.0); positions.len()];
    for triangle in indices.chunks_exact(3) {
        let (a, b, c) = (triangle[0] as usize, triangle[1] as usize, triangle[2] as usize);
        let (p0, p1, p2) = (positions[a], positions[b], positions[c]);
        let (u, v) = (
            Vector3::new(p1.x - p0.x, p1.y - p0.y, p1.z - p0.z),
            Vector3::new(p2.x - p0.x, p2.y - p0.y, p2.z - p0.z),
        );
        let face = Vector3::new(u.y * v.z - u.z * v.y, u.z * v.x - u.x * v.z, u.x * v.y - u.y * v.x);
        for index in [a, b, c] {
            let sum = sums[index];
            sums[index] = Vector3::new(sum.x + face.x, sum.y + face.y, sum.z + face.z);
        }
    }
    sums.into_iter()
        .map(|sum| {
            let normal = unit(sum);
            if normal.x == 0.0 && normal.y == 0.0 && normal.z == 0.0 {
                Vector3::new(0.0, 0.0, 1.0)
            } else {
                normal
            }
        })
        .collect()
}
