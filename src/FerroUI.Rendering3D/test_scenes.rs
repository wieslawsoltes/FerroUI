//! Snapshots for the tests of the crate, made by hand: the matrices of WPF's cameras, quads and
//! layers of solid colours.

use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, SharedBrush};
use ferroui_base::numerics::{Matrix4x4, Vector2, Vector3};
use ferroui_base::rendering::scene3d::{
    MeshData, Scene3DBounds, Scene3DCamera, Scene3DCameraKind, Scene3DLight, Scene3DMaterialKind,
    Scene3DMaterialLayer, Scene3DModel, Scene3DSnapshot,
};
use ferroui_base::{PixelSize, Rect};
use std::sync::Arc;

/// A layer of `kind` with a solid colour brush of `color` (the material's `Color` white).
pub(crate) fn layer(kind: Scene3DMaterialKind, color: Color) -> Scene3DMaterialLayer {
    Scene3DMaterialLayer {
        kind,
        color: Color::from_argb(255, 255, 255, 255),
        ambient_color: Color::from_argb(255, 255, 255, 255),
        specular_power: if kind == Scene3DMaterialKind::Specular { 40.0 } else { 0.0 },
        brush: SharedBrush::from_brush(&ImmutableSolidColorBrush::new(color)),
    }
}

/// A diffuse layer of `color`.
pub(crate) fn diffuse(color: Color) -> Scene3DMaterialLayer {
    layer(Scene3DMaterialKind::Diffuse, color)
}

/// An emissive layer of `color`.
pub(crate) fn emissive(color: Color) -> Scene3DMaterialLayer {
    layer(Scene3DMaterialKind::Emissive, color)
}

/// A quad in the plane `z` from `(x0, y0)` to `(x1, y1)`, counterclockwise seen from +z (its
/// front faces the camera of [`orthographic`]), with normals along +z.
pub(crate) fn quad(id: u64, x0: f32, y0: f32, x1: f32, y1: f32, z: f32) -> Arc<MeshData> {
    let positions = vec![Vector3::new(x0, y0, z), Vector3::new(x1, y0, z), Vector3::new(x1, y1, z), Vector3::new(x0, y1, z)];
    let bounds = Scene3DBounds::of_points(positions.iter().copied());
    Arc::new(MeshData {
        id,
        version: 1,
        positions,
        normals: vec![Vector3::new(0.0, 0.0, 1.0); 4],
        normals_generated: false,
        texture_coordinates: vec![Vector2::new(0.0, 1.0), Vector2::new(1.0, 1.0), Vector2::new(1.0, 0.0), Vector2::new(0.0, 0.0)],
        indices: vec![0, 1, 2, 0, 2, 3],
        bounds,
    })
}

/// The same quad wound the other way: its front faces away from +z, so the camera sees its back.
pub(crate) fn reversed_quad(id: u64, x0: f32, y0: f32, x1: f32, y1: f32, z: f32) -> Arc<MeshData> {
    let mut mesh = (*quad(id, x0, y0, x1, y1, z)).clone();
    mesh.indices = vec![0, 2, 1, 0, 3, 2];
    mesh.normals = vec![Vector3::new(0.0, 0.0, -1.0); 4];
    Arc::new(mesh)
}

/// A model of `mesh` at the identity with `front` and `back` layers.
pub(crate) fn model(mesh: Arc<MeshData>, front: Vec<Scene3DMaterialLayer>, back: Vec<Scene3DMaterialLayer>) -> Scene3DModel {
    Scene3DModel { world: Matrix4x4::IDENTITY, world_bounds: mesh.bounds, mesh, front, back }
}

/// WPF's orthographic camera at (0, 0, 5) looking down -z with `Width` 2 (x and y from -1 to 1
/// cover the viewport of a square image), planes 1 and 10.
pub(crate) fn orthographic() -> Scene3DCamera {
    let (near, far, width, aspect) = (1.0f32, 10.0f32, 2.0f32, 1.0f32);
    let view = Matrix4x4::create_look_at(Vector3::new(0.0, 0.0, 5.0), Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0));
    let m11 = 2.0 / width;
    let m33 = 1.0 / (near - far);
    let projection = Matrix4x4::new(m11, 0.0, 0.0, 0.0, 0.0, aspect * m11, 0.0, 0.0, 0.0, 0.0, m33, 0.0, 0.0, 0.0, near * m33, 1.0);
    Scene3DCamera {
        kind: Scene3DCameraKind::Orthographic,
        view,
        projection,
        position: Vector3::new(0.0, 0.0, 5.0),
        near_plane_distance: near,
        far_plane_distance: far,
    }
}

/// WPF's perspective camera at `position` looking at the origin, with the horizontal field of
/// view `fov` in degrees and finite planes.
pub(crate) fn perspective(position: Vector3, fov: f32, near: f32, far: f32) -> Scene3DCamera {
    let view = Matrix4x4::create_look_at(position, Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0));
    let m11 = 1.0 / (fov.to_radians() / 2.0).tan();
    let m33 = far / (near - far);
    let projection = Matrix4x4::new(m11, 0.0, 0.0, 0.0, 0.0, m11, 0.0, 0.0, 0.0, 0.0, m33, -1.0, 0.0, 0.0, near * m33, 0.0);
    Scene3DCamera {
        kind: Scene3DCameraKind::Perspective,
        view,
        projection,
        position,
        near_plane_distance: near,
        far_plane_distance: far,
    }
}

/// A snapshot of `models` lit by `lights` through `camera`.
pub(crate) fn snapshot(camera: Scene3DCamera, models: Vec<Scene3DModel>, lights: Vec<Scene3DLight>) -> Arc<Scene3DSnapshot> {
    Arc::new(Scene3DSnapshot {
        structure_generation: 1,
        record_generation: 1,
        camera_generation: 1,
        viewport: Rect::new(0.0, 0.0, 100.0, 100.0),
        camera: Some(camera),
        models,
        lights,
        bounds: None,
    })
}

/// A white ambient light: a diffuse layer is then drawn in its own colour.
pub(crate) fn white_ambient() -> Scene3DLight {
    Scene3DLight::Ambient { color: Color::from_argb(255, 255, 255, 255) }
}

/// The square size of the tests.
pub(crate) const SIZE: PixelSize = PixelSize::new(100, 100);
