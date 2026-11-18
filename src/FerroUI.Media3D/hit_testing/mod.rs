//! The 3D hit testing of WPF (`docs/porting/media3d.md`, section 3.8): which models of a scene a
//! ray passes through, nearest first.
//!
//! [`VisualTreeHelper`] has the overloads of WPF's `VisualTreeHelper.HitTest` for 3D:
//!
//! - [`hit_test`](VisualTreeHelper::hit_test): the nearest model under a 2D point of a
//!   [`Viewport3DVisual`] or a [`Viewport3D`](crate::Viewport3D) (WPF's `HitTest(Visual, Point)`);
//! - [`hit_test_point`](VisualTreeHelper::hit_test_point): every model under such a point, with
//!   a filter and a result callback (WPF's `HitTest(Visual, HitTestFilterCallback,
//!   HitTestResultCallback, HitTestParameters)` with a `PointHitTestParameters`);
//! - [`hit_test_ray`](VisualTreeHelper::hit_test_ray): every model a [`RayHitTestParameters`]
//!   ray passes through under a [`Visual3D`], with the callbacks (WPF's `HitTest(Visual3D,
//!   HitTestFilterCallback, HitTestResultCallback, HitTestParameters3D)`).
//!
//! The ray of a 2D point goes from the camera of the scene through the point, with the near and
//! far planes the snapshot fitted to the scene: what lies outside them, or outside the viewport,
//! is not hit, as it is not drawn. The ray of a `Visual3D` is in the coordinates of its parent
//! (the visual's own transform applies). Each `GeometryModel3D` with a `MeshGeometry3D` is
//! tested in its own coordinates (the ray through the inverse of the transforms of its models
//! and visuals) against a bounding-volume hierarchy of its mesh made once per version of the mesh.
//! A triangle is hit on its front (its vertices counterclockwise toward the ray's origin) when
//! the model has a `Material`, on its back when it has a `BackMaterial`, as in WPF; a transform
//! that mirrors (a negative determinant) turns the faces over, as it does on the screen. The hits
//! are reported as [`RayMeshGeometry3DHitTestResult`]s sorted by their distance to the origin of
//! the ray, until the result callback returns [`HitTestResultBehavior::Stop`].
//!
//! Hit testing runs on the CPU and the UI thread, as in WPF.

mod bvh;
mod ray;

pub(crate) use bvh::{hierarchies_built, MeshBvh};

use crate::transform_3d::value_of;
use crate::{GeometryModel3D, Matrix3D, MeshGeometry3D, Model3D, Model3DGroup, ModelVisual3D, Point3D, Vector3D};
use crate::{Viewport3D, Viewport3DVisual, Visual3D};
use ferroui_base::{Point, Ref};

/// How deep the tree is followed (the depth of the snapshot compiler).
const MAX_DEPTH: usize = 256;

/// WPF's `RayHitTestParameters`: a ray from `origin` along `direction`, in the coordinates of
/// the parent of the visual it is tested against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHitTestParameters {
    origin: Point3D,
    direction: Vector3D,
}

impl RayHitTestParameters {
    /// The ray from `origin` along `direction` (of any length but zero, which hits nothing).
    pub fn new(origin: Point3D, direction: Vector3D) -> Self {
        Self { origin, direction }
    }

    /// Where the ray starts.
    pub fn origin(&self) -> Point3D {
        self.origin
    }

    /// The direction of the ray.
    pub fn direction(&self) -> Vector3D {
        self.direction
    }
}

/// WPF's `HitTestResultBehavior`: what a result callback asks for after a hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HitTestResultBehavior {
    /// No further results.
    Stop,
    /// The next result, if any.
    Continue,
}

/// WPF's `HitTestFilterBehavior`: what a filter callback says of a visual before it is tested.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HitTestFilterBehavior {
    /// Neither the visual nor its descendants are tested.
    ContinueSkipSelfAndChildren,
    /// The visual is tested, its descendants are not.
    ContinueSkipChildren,
    /// The descendants are tested, the visual's own content is not.
    ContinueSkipSelf,
    /// The visual and its descendants are tested.
    Continue,
    /// The test ends here; the hits found so far are reported.
    Stop,
}

/// WPF's `HitTestResult`: what a hit test found.
pub trait HitTestResult {
    /// The visual hit (WPF's `VisualHit`).
    fn visual_hit(&self) -> Ref<Visual3D>;
}

/// WPF's `RayHitTestResult`: a hit of a ray on a model.
pub trait RayHitTestResult: HitTestResult {
    /// The model hit (WPF's `ModelHit`).
    fn model_hit(&self) -> Ref<Model3D>;

    /// The point hit, in the coordinates of the model (WPF's `PointHit`).
    fn point_hit(&self) -> Point3D;

    /// The distance from the origin of the ray to the point hit, in the coordinates of the ray
    /// (WPF's `DistanceToRayOrigin`).
    fn distance_to_ray_origin(&self) -> f64;
}

/// WPF's `RayMeshGeometry3DHitTestResult`: a hit of a ray on a triangle of a `MeshGeometry3D`,
/// with the indices of its vertices and their barycentric weights (`point_hit` is the sum of
/// the positions times the weights).
#[derive(Clone)]
pub struct RayMeshGeometry3DHitTestResult {
    visual_hit: Ref<Visual3D>,
    model_hit: Ref<GeometryModel3D>,
    mesh_hit: Ref<MeshGeometry3D>,
    point_hit: Point3D,
    distance_to_ray_origin: f64,
    vertex_indices: [u32; 3],
    vertex_weights: [f64; 3],
    front_face: bool,
}

impl RayMeshGeometry3DHitTestResult {
    /// The visual of the model hit (WPF's `VisualHit`).
    pub fn visual_hit(&self) -> &Ref<Visual3D> {
        &self.visual_hit
    }

    /// The model hit (WPF's `ModelHit`, here with its class).
    pub fn model_hit(&self) -> &Ref<GeometryModel3D> {
        &self.model_hit
    }

    /// The mesh hit (WPF's `MeshHit`).
    pub fn mesh_hit(&self) -> &Ref<MeshGeometry3D> {
        &self.mesh_hit
    }

    /// The point hit, in the coordinates of the mesh (WPF's `PointHit`).
    pub fn point_hit(&self) -> Point3D {
        self.point_hit
    }

    /// The distance from the origin of the ray to the point hit (WPF's `DistanceToRayOrigin`).
    pub fn distance_to_ray_origin(&self) -> f64 {
        self.distance_to_ray_origin
    }

    /// The index in `Positions` of the first vertex of the triangle hit (WPF's `VertexIndex1`).
    pub fn vertex_index_1(&self) -> i32 {
        self.vertex_indices[0] as i32
    }

    /// The index of the second vertex (WPF's `VertexIndex2`).
    pub fn vertex_index_2(&self) -> i32 {
        self.vertex_indices[1] as i32
    }

    /// The index of the third vertex (WPF's `VertexIndex3`).
    pub fn vertex_index_3(&self) -> i32 {
        self.vertex_indices[2] as i32
    }

    /// The weight of the first vertex in the point hit (WPF's `VertexWeight1`).
    pub fn vertex_weight_1(&self) -> f64 {
        self.vertex_weights[0]
    }

    /// The weight of the second vertex (WPF's `VertexWeight2`).
    pub fn vertex_weight_2(&self) -> f64 {
        self.vertex_weights[1]
    }

    /// The weight of the third vertex (WPF's `VertexWeight3`).
    pub fn vertex_weight_3(&self) -> f64 {
        self.vertex_weights[2]
    }

    /// Whether the front face of the triangle was hit (its `Material`), not the back face (its
    /// `BackMaterial`). WPF does not report the side.
    pub fn is_front_face(&self) -> bool {
        self.front_face
    }
}

impl HitTestResult for RayMeshGeometry3DHitTestResult {
    fn visual_hit(&self) -> Ref<Visual3D> {
        self.visual_hit.clone()
    }
}

impl RayHitTestResult for RayMeshGeometry3DHitTestResult {
    fn model_hit(&self) -> Ref<Model3D> {
        self.model_hit.clone().upcast()
    }

    fn point_hit(&self) -> Point3D {
        self.point_hit
    }

    fn distance_to_ray_origin(&self) -> f64 {
        self.distance_to_ray_origin
    }
}

/// WPF's `HitTestFilterCallback` for 3D: called with each visual before it is tested.
pub type HitTestFilterCallback<'a> = &'a mut dyn FnMut(&Ref<Visual3D>) -> HitTestFilterBehavior;

/// WPF's `HitTestResultCallback` for 3D: called with each hit, nearest first.
pub type HitTestResultCallback<'a> = &'a mut dyn FnMut(&RayMeshGeometry3DHitTestResult) -> HitTestResultBehavior;

/// The scene of a 2D hit test: a [`Viewport3DVisual`], or the [`Viewport3D`] that hosts one.
pub trait Viewport3DHitTestTarget {
    /// The scene.
    fn viewport_3d_visual(&self) -> &Viewport3DVisual;
}

impl Viewport3DHitTestTarget for Viewport3DVisual {
    fn viewport_3d_visual(&self) -> &Viewport3DVisual {
        self
    }
}

impl Viewport3DHitTestTarget for Viewport3D {
    fn viewport_3d_visual(&self) -> &Viewport3DVisual {
        Viewport3D::viewport_3d_visual(self)
    }
}

/// WPF's `VisualTreeHelper`, for its 3D hit testing; see the documentation of the module.
pub struct VisualTreeHelper;

impl VisualTreeHelper {
    /// The nearest model under `point` (in the coordinates of the viewport's element), or
    /// `None`: WPF's `VisualTreeHelper.HitTest(Visual, Point)` on a viewport.
    pub fn hit_test(
        reference: &(impl Viewport3DHitTestTarget + ?Sized),
        point: Point,
    ) -> Option<RayMeshGeometry3DHitTestResult> {
        let mut nearest = None;
        hit_test_point_core(
            reference.viewport_3d_visual(),
            None,
            &mut |result| {
                nearest = Some(result.clone());
                HitTestResultBehavior::Stop
            },
            point,
        );
        nearest
    }

    /// Every model under `point` (in the coordinates of the viewport's element), nearest first,
    /// handed to `result` until it returns [`HitTestResultBehavior::Stop`]; `filter` is asked
    /// of each visual before it is tested. WPF's `VisualTreeHelper.HitTest(Visual,
    /// HitTestFilterCallback, HitTestResultCallback, PointHitTestParameters)` on a viewport.
    pub fn hit_test_point(
        reference: &(impl Viewport3DHitTestTarget + ?Sized),
        filter: Option<HitTestFilterCallback<'_>>,
        result: HitTestResultCallback<'_>,
        point: Point,
    ) {
        hit_test_point_core(reference.viewport_3d_visual(), filter, result, point)
    }

    /// Every model the ray of `parameters` passes through under `reference` (the ray in the
    /// coordinates of the parent of `reference`), nearest first, handed to `result` until it
    /// returns [`HitTestResultBehavior::Stop`]; `filter` is asked of each visual before it is
    /// tested. WPF's `VisualTreeHelper.HitTest(Visual3D, HitTestFilterCallback,
    /// HitTestResultCallback, HitTestParameters3D)`.
    pub fn hit_test_ray(
        reference: &Visual3D,
        filter: Option<HitTestFilterCallback<'_>>,
        result: HitTestResultCallback<'_>,
        parameters: &RayHitTestParameters,
    ) {
        let mut walk = Walk::new(parameters.origin, parameters.direction, None, filter);
        walk.visit_visual(&reference.to_ref(), &Matrix3D::IDENTITY, 0);
        walk.report(result);
    }
}

fn hit_test_point_core(
    viewport: &Viewport3DVisual,
    filter: Option<HitTestFilterCallback<'_>>,
    result: HitTestResultCallback<'_>,
    point: Point,
) {
    let snapshot = viewport.snapshot();
    let Some(ray) = ray::viewport_ray(&snapshot, point) else {
        return;
    };
    let mut walk = Walk::new(ray.origin, ray.direction, Some(ray.world_to_clip), filter);
    for child in viewport.children().iter() {
        if !walk.visit_visual(&child, &Matrix3D::IDENTITY, 0) {
            break;
        }
    }
    walk.report(result);
}

/// The walk of a hit test over a tree of visuals and models, gathering the hits.
struct Walk<'a> {
    origin: Point3D,
    direction: Vector3D,
    /// For the ray of a viewport: the hits outside the planes of the camera are left out.
    world_to_clip: Option<Matrix3D>,
    filter: Option<HitTestFilterCallback<'a>>,
    results: Vec<RayMeshGeometry3DHitTestResult>,
}

impl<'a> Walk<'a> {
    fn new(
        origin: Point3D,
        direction: Vector3D,
        world_to_clip: Option<Matrix3D>,
        filter: Option<HitTestFilterCallback<'a>>,
    ) -> Self {
        Self { origin, direction, world_to_clip, filter, results: Vec::new() }
    }

    /// Tests a visual and its descendants; `false` when a filter stopped the test.
    fn visit_visual(&mut self, visual: &Ref<Visual3D>, parent: &Matrix3D, depth: usize) -> bool {
        if depth > MAX_DEPTH {
            return true;
        }
        let behavior = match self.filter.as_mut() {
            Some(filter) => filter(visual),
            None => HitTestFilterBehavior::Continue,
        };
        match behavior {
            HitTestFilterBehavior::Stop => return false,
            HitTestFilterBehavior::ContinueSkipSelfAndChildren => return true,
            _ => {}
        }
        let world = value_of(visual.transform().as_ref()) * *parent;
        let Some(model_visual) = visual.cast::<ModelVisual3D>() else {
            return true;
        };
        if behavior != HitTestFilterBehavior::ContinueSkipSelf {
            if let Some(content) = model_visual.content() {
                self.visit_model(&content, &world, visual, depth + 1);
            }
        }
        if behavior != HitTestFilterBehavior::ContinueSkipChildren {
            for child in model_visual.children().iter() {
                if !self.visit_visual(&child, &world, depth + 1) {
                    return false;
                }
            }
        }
        true
    }

    fn visit_model(&mut self, model: &Ref<Model3D>, parent: &Matrix3D, visual: &Ref<Visual3D>, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        let world = value_of(model.transform().as_ref()) * *parent;
        if let Some(group) = model.cast::<Model3DGroup>() {
            for child in group.children().iter() {
                self.visit_model(&child, &world, visual, depth + 1);
            }
        } else if let Some(geometry_model) = model.cast::<GeometryModel3D>() {
            self.test_geometry_model(&geometry_model, &world, visual);
        }
    }

    fn test_geometry_model(&mut self, model: &Ref<GeometryModel3D>, world: &Matrix3D, visual: &Ref<Visual3D>) {
        let Some(mesh) = model.geometry().and_then(|geometry| geometry.cast::<MeshGeometry3D>()) else {
            return;
        };
        // WPF: a side is hit only when the model has a material for it.
        let (front, back) = (model.material().is_some(), model.back_material().is_some());
        if !front && !back {
            return;
        }
        let mut inverse = *world;
        if inverse.invert().is_err() {
            // A transform that flattens the model: it covers nothing.
            return;
        }
        let local_origin = inverse.transform(self.origin);
        let local_direction = if inverse.is_affine() {
            inverse.transform_vector(self.direction)
        } else {
            inverse.transform(self.origin + self.direction) - local_origin
        };
        // A mirroring transform turns the faces over on the screen: the sides of the mesh's own
        // coordinates swap.
        let mirrored = linear_determinant(world) < 0.0;
        let sides = if mirrored { (back, front) } else { (front, back) };
        let hierarchy = mesh.hit_test_hierarchy();
        let (origin, world_to_clip) = (self.origin, self.world_to_clip);
        let results = &mut self.results;
        hierarchy.intersect(local_origin, local_direction, sides, &mut |hit| {
            let world_point = world.transform(hit.point);
            if let Some(world_to_clip) = world_to_clip.as_ref() {
                if !ray::is_visible_depth(world_to_clip, world_point) {
                    return;
                }
            }
            let distance = (world_point - origin).length();
            if !distance.is_finite() {
                return;
            }
            results.push(RayMeshGeometry3DHitTestResult {
                visual_hit: visual.clone(),
                model_hit: model.clone(),
                mesh_hit: mesh.clone(),
                point_hit: hit.point,
                distance_to_ray_origin: distance,
                vertex_indices: hit.vertices,
                vertex_weights: hit.weights,
                front_face: hit.front != mirrored,
            });
        });
    }

    /// Reports the hits, nearest first (in tree order at equal distances).
    fn report(mut self, result: HitTestResultCallback<'_>) {
        self.results.sort_by(|a, b| a.distance_to_ray_origin.total_cmp(&b.distance_to_ray_origin));
        for hit in &self.results {
            if result(hit) == HitTestResultBehavior::Stop {
                break;
            }
        }
    }
}

/// The determinant of the upper 3x3 block: negative for a transform that mirrors.
fn linear_determinant(m: &Matrix3D) -> f64 {
    m.m11 * (m.m22 * m.m33 - m.m23 * m.m32) - m.m12 * (m.m21 * m.m33 - m.m23 * m.m31)
        + m.m13 * (m.m21 * m.m32 - m.m22 * m.m31)
}

#[cfg(test)]
mod tests;
