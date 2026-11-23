//! The ray through a point of a viewport: from the camera of the scene's snapshot, through the
//! inverse of its view and projection, with the planes the snapshot fitted to the scene (the
//! matrices rendering draws with, so a pick hits what is drawn under the point).

use crate::{Matrix3D, Point3D, Point4D, Vector3D};
use ferroui_base::numerics::Matrix4x4;
use ferroui_base::rendering::scene3d::{Scene3DCameraKind, Scene3DSnapshot};
use ferroui_base::Point;

/// A ray of the world through a point of a viewport, and the matrix that keeps a hit to the
/// visible depths.
pub(crate) struct ViewportRay {
    pub origin: Point3D,
    /// Unit length.
    pub direction: Vector3D,
    /// The matrix from the world to the clip space of the camera: a hit counts when it lies
    /// between the near and the far plane (`0 <= z <= w`), as rendering clips.
    pub world_to_clip: Matrix3D,
}

/// The ray through `point` (in the coordinates of the viewport's element, like
/// `Viewport3DVisual::viewport`) for the camera of `snapshot`; `None` without a camera, outside
/// the viewport, for an empty viewport or for matrices without an inverse.
///
/// The origin is the camera's position for a perspective camera; for an orthographic camera the
/// point of the plane through its position (or of the near plane, when that is behind the
/// camera); for a matrix camera, whose position is not known, the point of the near plane.
pub(crate) fn viewport_ray(snapshot: &Scene3DSnapshot, point: Point) -> Option<ViewportRay> {
    let camera = snapshot.camera.as_ref()?;
    let viewport = snapshot.viewport;
    if !(viewport.width > 0.0 && viewport.height > 0.0) {
        return None;
    }
    let (u, v) = ((point.x - viewport.x) / viewport.width, (point.y - viewport.y) / viewport.height);
    if !((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v)) {
        return None;
    }
    // The coordinates of the device: x to the right, y up (rows go down, as the rasterizer maps
    // them).
    let (x, y) = (u * 2.0 - 1.0, 1.0 - v * 2.0);
    let world_to_clip = widen(&camera.view) * widen(&camera.projection);
    let mut clip_to_world = world_to_clip;
    clip_to_world.invert().ok()?;
    let near = unproject(&clip_to_world, x, y, 0.0)?;
    // Half way to the far plane: finite also for a perspective projection with an infinite far
    // plane.
    let farther = unproject(&clip_to_world, x, y, 0.5)?;
    let mut direction = farther - near;
    if !(direction.length_squared() > 0.0) {
        return None;
    }
    direction.normalize();
    let position = Point3D::new(camera.position.x as f64, camera.position.y as f64, camera.position.z as f64);
    let origin = match camera.kind {
        Scene3DCameraKind::Perspective => position,
        // The point of the plane of the camera under `point`; the near plane when it is behind
        // the camera (a negative `NearPlaneDistance`), which an orthographic camera sees.
        Scene3DCameraKind::Orthographic if camera.near_plane_distance >= 0.0 => {
            near + direction * Vector3D::dot_product(position - near, direction)
        }
        Scene3DCameraKind::Orthographic | Scene3DCameraKind::Matrix => near,
    };
    Some(ViewportRay { origin, direction, world_to_clip })
}

/// Whether a point of the world lies between the near and the far plane (and in front of the
/// camera), with a small tolerance for the rounding of single precision.
pub(crate) fn is_visible_depth(world_to_clip: &Matrix3D, point: Point3D) -> bool {
    let clip = world_to_clip.transform_point4d(Point4D::new(point.x, point.y, point.z, 1.0));
    let tolerance = clip.w.abs() * 1e-6;
    clip.w > 0.0 && clip.z >= -tolerance && clip.z <= clip.w + tolerance
}

/// The point of the world at the device coordinates `x`, `y` and the clip depth `z`.
fn unproject(clip_to_world: &Matrix3D, x: f64, y: f64, z: f64) -> Option<Point3D> {
    let point = clip_to_world.transform_point4d(Point4D::new(x, y, z, 1.0));
    if !(point.w.abs() > 0.0) || !point.w.is_finite() {
        return None;
    }
    let result = Point3D::new(point.x / point.w, point.y / point.w, point.z / point.w);
    (result.x.is_finite() && result.y.is_finite() && result.z.is_finite()).then_some(result)
}

/// The single-precision matrix of a snapshot in double precision, element by element.
fn widen(matrix: &Matrix4x4) -> Matrix3D {
    Matrix3D::new(
        matrix.m11 as f64,
        matrix.m12 as f64,
        matrix.m13 as f64,
        matrix.m14 as f64,
        matrix.m21 as f64,
        matrix.m22 as f64,
        matrix.m23 as f64,
        matrix.m24 as f64,
        matrix.m31 as f64,
        matrix.m32 as f64,
        matrix.m33 as f64,
        matrix.m34 as f64,
        matrix.m41 as f64,
        matrix.m42 as f64,
        matrix.m43 as f64,
        matrix.m44 as f64,
    )
}
