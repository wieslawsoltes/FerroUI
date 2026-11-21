//! The comparison scenes of 3D (#608, steps D3 and D6 of `docs/porting/media3d.md`): snapshots of
//! `Viewport3D` scenes, made by hand with WPF's camera matrices, drawn through the contracts as
//! the render walk draws them (`rendering::scene3d::draw_scene_3d` on a drawing context), at 96
//! and 192 DPI on a square of [`SIDE`] logical pixels: the eleven scenes of D3, and the six of the
//! materials from brushes of D6 (gradients, the lookup texture of many stops, absolute mapping,
//! image brushes with tiles, viewbox and stretch, and the specular and emissive layers of a
//! `MaterialGroup` painting with brushes).
//!
//! Each scene states its golden properties as pixel probes with analytic expectations: the
//! colours WPF's lighting gives at the probed point (computed here from the formulas, in double
//! precision, independently of the renderer), the transparent background, culled faces, clipped
//! parts. `scene3d_tests.rs` holds every backend and mode to them and to the CPU reference of
//! `ferroui-render3d`, and compares the backends with each other.

use crate::custom_effect_scenes::{Tolerance, SIDE};
use crate::Pixels;
use ferroui_base::media::imaging::{Bitmap, BitmapEncoderOptions};
use ferroui_base::media::immutable::{
    ImmutableGradientStop, ImmutableImageBrush, ImmutableLinearGradientBrush, ImmutableRadialGradientBrush,
    ImmutableSolidColorBrush,
};
use ferroui_base::media::{AlignmentX, AlignmentY, Color, GradientSpreadMethod, IBrush, SharedBrush, Stretch, TileMode};
use ferroui_base::numerics::{Matrix4x4, Vector2, Vector3};
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, ILockedFramebuffer, IReadableBitmapImpl, PixelFormat,
    SharedBitmapImpl,
};
use ferroui_base::rendering::scene3d::{
    draw_scene_3d, MeshData, Scene3DBounds, Scene3DCamera, Scene3DCameraKind, Scene3DLight, Scene3DMaterialKind,
    Scene3DMaterialLayer, Scene3DModel, Scene3DSnapshot,
};
use ferroui_base::{Matrix, PixelSize, Point, Rect, RelativePoint, RelativeRect, RelativeUnit, Vector};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// A 3D comparison scene.
pub struct Scene3DCase {
    pub name: &'static str,
    /// The snapshot.
    pub scene: fn() -> Arc<Scene3DSnapshot>,
    /// How the scene is drawn on a context of [`SIDE`] logical pixels.
    pub draw: fn(&mut dyn IDrawingContextImpl, &Arc<Scene3DSnapshot>),
    /// Whether the scene is drawn over the whole square without a transform, clip or opacity:
    /// its pixels are then the CPU reference at the size of the target, drawn one to one.
    pub plain: bool,
    /// The probes of the scene in an image of `side` device pixels.
    pub probes: fn(side: usize) -> Vec<Probe>,
    /// How far a backend may be from the CPU reference (plain scenes).
    pub tolerance: Tolerance,
    /// The share of the pixels (in percent) by which Skia and Vello may differ by more than the
    /// tolerance of the harness: none for a scene drawn one to one, the antialiasing of the edges
    /// of a rotated image for the others.
    pub bound: f64,
    /// The layers the scene draws over the same silhouettes (a `MaterialGroup` with a diffuse, a
    /// specular and an emissive layer draws three): each layer's edges are sampled by 4x MSAA on
    /// the device and by the ordered grid of the reference, so the edge bounds of a GPU executor
    /// grow with it. The interior bound does not.
    pub layers: u32,
}

/// A pixel with the colour it has (premultiplied RGBA), within `levels`.
#[derive(Clone, Debug)]
pub struct Probe {
    pub what: &'static str,
    pub x: usize,
    pub y: usize,
    pub expected: [u8; 4],
    pub levels: u8,
}

impl Probe {
    /// The probe of the pixel at the fractions `fx`, `fy` of an image of `side` pixels.
    fn at(what: &'static str, side: usize, fx: f64, fy: f64, expected: [u8; 4], levels: u8) -> Probe {
        let pixel = |fraction: f64| ((fraction * side as f64).floor() as usize).min(side - 1);
        Probe { what, x: pixel(fx), y: pixel(fy), expected, levels }
    }

    /// The pixel of `pixels` the probe looks at.
    pub fn read(&self, pixels: &Pixels) -> [u8; 4] {
        let index = (self.y * pixels.width + self.x) * 4;
        [pixels.rgba[index], pixels.rgba[index + 1], pixels.rgba[index + 2], pixels.rgba[index + 3]]
    }

    /// Whether `actual` is the expected colour within the levels of the probe.
    pub fn holds(&self, actual: [u8; 4]) -> bool {
        self.expected.iter().zip(actual).all(|(expected, actual)| expected.abs_diff(actual) <= self.levels)
    }
}

// Colours.
const RED: Color = Color::from_argb(255, 255, 0, 0);
const GREEN: Color = Color::from_argb(255, 0, 255, 0);
const BLUE: Color = Color::from_argb(255, 0, 0, 255);
const WHITE: Color = Color::from_argb(255, 255, 255, 255);
const ORANGE: Color = Color::from_argb(255, 255, 128, 0);
const TRANSLUCENT_RED: Color = Color::from_argb(128, 255, 0, 0);
const TRANSPARENT: [u8; 4] = [0, 0, 0, 0];

fn gray(level: u8) -> Color {
    Color::from_argb(255, level, level, level)
}

// Materials.

fn solid_layer(kind: Scene3DMaterialKind, color: Color, specular_power: f32) -> Scene3DMaterialLayer {
    Scene3DMaterialLayer {
        kind,
        color: WHITE,
        ambient_color: WHITE,
        specular_power,
        brush: SharedBrush::from_brush(&ImmutableSolidColorBrush::new(color)),
    }
}

fn diffuse(color: Color) -> Scene3DMaterialLayer {
    solid_layer(Scene3DMaterialKind::Diffuse, color, 0.0)
}

fn specular(color: Color, power: f32) -> Scene3DMaterialLayer {
    solid_layer(Scene3DMaterialKind::Specular, color, power)
}

fn emissive(color: Color) -> Scene3DMaterialLayer {
    solid_layer(Scene3DMaterialKind::Emissive, color, 0.0)
}

// Meshes.

fn mesh(id: u64, positions: Vec<Vector3>, normals: Vec<Vector3>, indices: Vec<u32>) -> Arc<MeshData> {
    let texture_coordinates = vec![Vector2::new(0.0, 0.0); positions.len()];
    let bounds = Scene3DBounds::of_points(positions.iter().copied());
    Arc::new(MeshData { id, version: 1, positions, normals, normals_generated: false, texture_coordinates, indices, bounds })
}

/// A quad with the corners `corners` in counterclockwise order seen from its front, and the
/// normal `normal`.
fn polygon(id: u64, corners: [[f32; 3]; 4], normal: [f32; 3]) -> Arc<MeshData> {
    mesh(
        id,
        corners.iter().map(|c| Vector3::new(c[0], c[1], c[2])).collect(),
        vec![Vector3::new(normal[0], normal[1], normal[2]); 4],
        vec![0, 1, 2, 0, 2, 3],
    )
}

/// A quad in the plane `z`, its front toward +z.
fn quad(id: u64, x0: f32, y0: f32, x1: f32, y1: f32, z: f32) -> Arc<MeshData> {
    polygon(id, [[x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z]], [0.0, 0.0, 1.0])
}

/// A quad in the plane `z`, its front toward -z: the camera sees its back.
fn reversed_quad(id: u64, x0: f32, y0: f32, x1: f32, y1: f32, z: f32) -> Arc<MeshData> {
    polygon(id, [[x0, y0, z], [x0, y1, z], [x1, y1, z], [x1, y0, z]], [0.0, 0.0, -1.0])
}

/// A cube of half side `half` at the origin: 24 vertices with the normals of its faces.
fn cube(id: u64, half: f32) -> Arc<MeshData> {
    // (normal, u, v) with u x v = normal: each face counterclockwise seen from outside.
    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
    ];
    let (mut positions, mut normals, mut indices) = (Vec::new(), Vec::new(), Vec::new());
    for (n, u, v) in faces {
        let base = positions.len() as u32;
        for (su, sv) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            positions.push(Vector3::new(
                (n[0] + su * u[0] + sv * v[0]) * half,
                (n[1] + su * u[1] + sv * v[1]) * half,
                (n[2] + su * u[2] + sv * v[2]) * half,
            ));
            normals.push(Vector3::new(n[0], n[1], n[2]));
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    mesh(id, positions, normals, indices)
}

fn model(mesh: Arc<MeshData>, world: Matrix4x4, front: Vec<Scene3DMaterialLayer>, back: Vec<Scene3DMaterialLayer>) -> Scene3DModel {
    Scene3DModel { world, world_bounds: mesh.bounds, mesh, front, back }
}

// Cameras: WPF's matrices for a square viewport (aspect ratio 1).

fn look_at(position: [f32; 3]) -> Matrix4x4 {
    Matrix4x4::create_look_at(
        Vector3::new(position[0], position[1], position[2]),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    )
}

/// WPF's `PerspectiveCamera.GetProjectionMatrix`: the horizontal field of view in degrees.
fn perspective_projection(field_of_view: f32, near: f32, far: f32) -> Matrix4x4 {
    let m11 = 1.0 / (field_of_view.to_radians() / 2.0).tan();
    let m33 = far / (near - far);
    Matrix4x4::new(m11, 0.0, 0.0, 0.0, 0.0, m11, 0.0, 0.0, 0.0, 0.0, m33, -1.0, 0.0, 0.0, near * m33, 0.0)
}

fn perspective(position: [f32; 3], field_of_view: f32, near: f32, far: f32) -> Scene3DCamera {
    Scene3DCamera {
        kind: Scene3DCameraKind::Perspective,
        view: look_at(position),
        projection: perspective_projection(field_of_view, near, far),
        position: Vector3::new(position[0], position[1], position[2]),
        near_plane_distance: near,
        far_plane_distance: far,
    }
}

/// WPF's `OrthographicCamera.GetProjectionMatrix`.
fn orthographic(position: [f32; 3], width: f32, near: f32, far: f32) -> Scene3DCamera {
    let m11 = 2.0 / width;
    let m33 = 1.0 / (near - far);
    Scene3DCamera {
        kind: Scene3DCameraKind::Orthographic,
        view: look_at(position),
        projection: Matrix4x4::new(m11, 0.0, 0.0, 0.0, 0.0, m11, 0.0, 0.0, 0.0, 0.0, m33, 0.0, 0.0, 0.0, near * m33, 1.0),
        position: Vector3::new(position[0], position[1], position[2]),
        near_plane_distance: near,
        far_plane_distance: far,
    }
}

fn snapshot(camera: Option<Scene3DCamera>, models: Vec<Scene3DModel>, lights: Vec<Scene3DLight>) -> Arc<Scene3DSnapshot> {
    let bounds = Scene3DBounds::of_points(models.iter().filter_map(|model| model.world_bounds).flat_map(|b| [b.min, b.max]));
    Arc::new(Scene3DSnapshot {
        structure_generation: 1,
        record_generation: 1,
        camera_generation: 1,
        viewport: Rect::new(0.0, 0.0, f64::from(SIDE), f64::from(SIDE)),
        camera,
        models,
        lights,
        bounds,
    })
}

fn directional(color: Color, direction: [f32; 3]) -> Scene3DLight {
    Scene3DLight::Directional { color, direction: Vector3::new(direction[0], direction[1], direction[2]) }
}

// The analytic side: projections and the expected pixels, in double precision.

type F3 = [f64; 3];

fn sub(a: F3, b: F3) -> F3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: F3, b: F3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(a: F3) -> F3 {
    let length = dot(a, a).sqrt();
    [a[0] / length, a[1] / length, a[2] / length]
}

fn matrix(m: &Matrix4x4) -> [[f64; 4]; 4] {
    [
        [m.m11, m.m12, m.m13, m.m14],
        [m.m21, m.m22, m.m23, m.m24],
        [m.m31, m.m32, m.m33, m.m34],
        [m.m41, m.m42, m.m43, m.m44],
    ]
    .map(|row| row.map(f64::from))
}

fn multiply(a: &[[f64; 4]; 4], b: &[[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut result = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            result[i][j] = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    result
}

/// `v * m` (row vectors).
fn transform(v: [f64; 4], m: &[[f64; 4]; 4]) -> [f64; 4] {
    let mut result = [0.0; 4];
    for (j, value) in result.iter_mut().enumerate() {
        *value = (0..4).map(|i| v[i] * m[i][j]).sum();
    }
    result
}

/// The inverse by Gauss-Jordan elimination with partial pivoting.
fn invert(m: &[[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut a = *m;
    let mut inverse = [[0.0; 4]; 4];
    for (i, row) in inverse.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    for column in 0..4 {
        let pivot = (column..4).max_by(|&x, &y| a[x][column].abs().total_cmp(&a[y][column].abs())).unwrap_or(column);
        a.swap(column, pivot);
        inverse.swap(column, pivot);
        let scale = 1.0 / a[column][column];
        for j in 0..4 {
            a[column][j] *= scale;
            inverse[column][j] *= scale;
        }
        for row in 0..4 {
            if row != column {
                let factor = a[row][column];
                for j in 0..4 {
                    a[row][j] -= factor * a[column][j];
                    inverse[row][j] -= factor * inverse[column][j];
                }
            }
        }
    }
    inverse
}

fn view_projection(camera: &Scene3DCamera) -> [[f64; 4]; 4] {
    multiply(&matrix(&camera.view), &matrix(&camera.projection))
}

/// Where `point` of the world is drawn in an image of `side` pixels: x right, y down, in
/// pixels.
fn project(camera: &Scene3DCamera, side: usize, point: F3) -> (f64, f64) {
    let clip = transform([point[0], point[1], point[2], 1.0], &view_projection(camera));
    let (x, y) = (clip[0] / clip[3], clip[1] / clip[3]);
    ((x * 0.5 + 0.5) * side as f64, (0.5 - y * 0.5) * side as f64)
}

/// The point of the plane through `origin` with `normal` that is drawn at `x`, `y` (in pixels)
/// of an image of `side` pixels.
fn unproject(camera: &Scene3DCamera, side: usize, x: f64, y: f64, origin: F3, normal: F3) -> F3 {
    let inverse = invert(&view_projection(camera));
    let (ndc_x, ndc_y) = (x / side as f64 * 2.0 - 1.0, 1.0 - y / side as f64 * 2.0);
    let point = |z: f64| {
        let p = transform([ndc_x, ndc_y, z, 1.0], &inverse);
        [p[0] / p[3], p[1] / p[3], p[2] / p[3]]
    };
    let (near, far) = (point(0.0), point(0.5));
    let direction = sub(far, near);
    let t = dot(normal, sub(origin, near)) / dot(normal, direction);
    [near[0] + direction[0] * t, near[1] + direction[1] * t, near[2] + direction[2] * t]
}

/// The pixel the renderer resolves at `px`, `py` of a surface of `side` pixels with a plane
/// through `origin` with `normal` under all four samples, coloured by `shade` (the premultiplied
/// colour of a sample, already clamped): the mean of the samples at a quarter and three quarters
/// of the pixel, the alpha raised to the colour, rounded.
fn resolved(
    camera: &Scene3DCamera,
    side: usize,
    px: usize,
    py: usize,
    origin: F3,
    normal: F3,
    shade: &dyn Fn(F3) -> [f64; 4],
) -> [u8; 4] {
    let mut sum = [0.0; 4];
    for (sx, sy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
        let color = shade(unproject(camera, side, px as f64 + sx, py as f64 + sy, origin, normal));
        for i in 0..4 {
            sum[i] += color[i] / 4.0;
        }
    }
    sum[3] = sum[3].max(sum[0]).max(sum[1]).max(sum[2]);
    sum.map(|value| (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)
}

/// The probe of the pixel under `point` of the world.
fn probe_at(
    what: &'static str,
    camera: &Scene3DCamera,
    side: usize,
    point: F3,
    expected: impl FnOnce(usize, usize) -> [u8; 4],
    levels: u8,
) -> Probe {
    let (x, y) = project(camera, side, point);
    let (x, y) = (x.floor() as usize, y.floor() as usize);
    Probe { what, x, y, expected: expected(x, y), levels }
}

/// An opaque colour from levels of 0 to 1.
fn opaque(r: f64, g: f64, b: f64) -> [u8; 4] {
    let byte = |value: f64| (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    [byte(r), byte(g), byte(b), 255]
}

// The scenes.

const LEVEL_64: f64 = 64.0 / 255.0;
const LEVEL_128: f64 = 128.0 / 255.0;

/// The camera of the lit cube: 4 units in front of it, 45 degrees.
fn cube_camera() -> Scene3DCamera {
    perspective([0.0, 0.0, 4.0], 45.0, 1.0, 10.0)
}

/// The cube turned 30 degrees about y: its +z face toward +x, its -x face toward the camera
/// on the left.
fn turned_cube() -> Scene3DModel {
    model(cube(1, 0.5), Matrix4x4::create_rotation_y(30f32.to_radians()), vec![diffuse(RED)], vec![])
}

fn cube_lights() -> Vec<Scene3DLight> {
    vec![Scene3DLight::Ambient { color: gray(64) }, directional(gray(128), [0.0, 0.0, -1.0])]
}

fn lit_cube() -> Arc<Scene3DSnapshot> {
    snapshot(Some(cube_camera()), vec![turned_cube()], cube_lights())
}

/// The lit cube of the first scene, for the tests of the kept outputs.
pub fn lit_cube_scene() -> Arc<Scene3DSnapshot> {
    lit_cube()
}

/// The lit cube seen by the camera of [`cube_camera`] turned `step` degrees about the y axis at
/// the same distance: the snapshot of the `step`-th frame of one viewport whose camera moves (its
/// camera generation is `1 + step`). The mesh, the model and the lights are those of every step:
/// the same `(id, version)`, the same records.
pub fn orbiting_cube(step: u32) -> Arc<Scene3DSnapshot> {
    let angle = (step as f32).to_radians();
    let camera = perspective([4.0 * angle.sin(), 0.0, 4.0 * angle.cos()], 45.0, 1.0, 10.0);
    let mut scene = (*snapshot(Some(camera), vec![turned_cube()], cube_lights())).clone();
    scene.camera_generation = 1 + u64::from(step);
    Arc::new(scene)
}

/// The centres of the two visible faces of the turned cube and their red: the ambient 64 plus
/// 128 times the cosine to the light along -z (cos 30 for the +z face, cos 60 for the -x face).
fn cube_face_probes(camera: &Scene3DCamera, side: usize, shift: f64) -> Vec<Probe> {
    let angle = 30f64.to_radians();
    let shifted = |probe: Probe| Probe { x: probe.x + (shift * side as f64).round() as usize, ..probe };
    vec![
        shifted(probe_at(
            "the +z face, lit at 30 degrees",
            camera,
            side,
            [0.5 * angle.sin(), 0.0, 0.5 * angle.cos()],
            |_, _| opaque(LEVEL_64 + LEVEL_128 * angle.cos(), 0.0, 0.0),
            2,
        )),
        shifted(probe_at(
            "the -x face, lit at 60 degrees",
            camera,
            side,
            [-0.5 * angle.cos(), 0.0, 0.5 * angle.sin()],
            |_, _| opaque(LEVEL_64 + LEVEL_128 * 0.5, 0.0, 0.0),
            2,
        )),
        Probe::at("the background", side, 0.03, 0.03, TRANSPARENT, 0),
        Probe::at("above the cube", side, 0.5, 0.2, TRANSPARENT, 0),
    ]
}

fn lit_cube_probes(side: usize) -> Vec<Probe> {
    cube_face_probes(&cube_camera(), side, 0.0)
}

/// A plane facing a camera 4 units away.
fn floor_camera() -> Scene3DCamera {
    perspective([0.0, 0.0, 4.0], 45.0, 1.0, 10.0)
}

fn white_plane() -> Scene3DModel {
    model(quad(2, -3.0, -3.0, 3.0, 3.0, 0.0), Matrix4x4::IDENTITY, vec![diffuse(WHITE)], vec![])
}

fn point_light_scene() -> Arc<Scene3DSnapshot> {
    let light = Scene3DLight::Point {
        color: WHITE,
        position: Vector3::new(0.0, 0.0, 1.0),
        range: 1.5,
        attenuation: [1.0, 0.5, 0.0],
    };
    snapshot(Some(floor_camera()), vec![white_plane()], vec![light])
}

/// WPF's point light at (0, 0, 1) over the plane z = 0: Lambert times 1 / max(1 + 0.5 d, 1),
/// nothing beyond the range of 1.5.
fn point_light_at(p: F3) -> [f64; 4] {
    let to_light = sub([0.0, 0.0, 1.0], p);
    let distance = dot(to_light, to_light).sqrt();
    if distance > 1.5 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let lambert = to_light[2] / distance;
    let level = (lambert / (1.0 + 0.5 * distance).max(1.0)).clamp(0.0, 1.0);
    [level, level, level, 1.0]
}

fn plane_probes(side: usize, shade: fn(F3) -> [f64; 4], points: [(&'static str, F3); 3]) -> Vec<Probe> {
    let camera = floor_camera();
    points
        .into_iter()
        .map(|(what, point)| {
            probe_at(what, &camera, side, point, |x, y| resolved(&camera, side, x, y, [0.0; 3], [0.0, 0.0, 1.0], &shade), 2)
        })
        .collect()
}

fn point_light_probes(side: usize) -> Vec<Probe> {
    plane_probes(
        side,
        point_light_at,
        [
            ("under the light", [0.0, 0.0, 0.0]),
            ("attenuated, at 1.22 from the light", [0.7, 0.0, 0.0]),
            ("beyond the range: black", [1.4, 0.0, 0.0]),
        ],
    )
}

fn spot_light_scene() -> Arc<Scene3DSnapshot> {
    let light = Scene3DLight::Spot {
        color: WHITE,
        position: Vector3::new(0.0, 0.0, 2.0),
        direction: Vector3::new(0.0, 0.0, -1.0),
        range: f32::INFINITY,
        attenuation: [1.0, 0.0, 0.0],
        inner_cone_angle: 30.0,
        outer_cone_angle: 60.0,
    };
    snapshot(Some(floor_camera()), vec![white_plane()], vec![light])
}

/// WPF's spot light at (0, 0, 2) down -z with the full cone angles 30 and 60: full light within
/// 15 degrees of the axis, a linear falloff of the cosine to 30 degrees, none beyond.
fn spot_light_at(p: F3) -> [f64; 4] {
    let to_light = normalize(sub([0.0, 0.0, 2.0], p));
    let rho = to_light[2];
    let (inner, outer) = (15f64.to_radians().cos(), 30f64.to_radians().cos());
    let spot = ((rho - outer) / (inner - outer)).clamp(0.0, 1.0);
    let level = (to_light[2] * spot).clamp(0.0, 1.0);
    [level, level, level, 1.0]
}

fn spot_light_probes(side: usize) -> Vec<Probe> {
    let at_angle = |degrees: f64| [2.0 * degrees.to_radians().tan(), 0.0, 0.0];
    plane_probes(
        side,
        spot_light_at,
        [("on the axis", [0.0, 0.0, 0.0]), ("in the falloff, at 20 degrees", at_angle(20.0)), ("outside the cone, at 35 degrees", at_angle(35.0))],
    )
}

fn orthographic_scene() -> Arc<Scene3DSnapshot> {
    let camera = orthographic([0.0, 0.0, 5.0], 2.0, 1.0, 10.0);
    let near = model(quad(3, -0.75, -0.25, -0.25, 0.25, 0.0), Matrix4x4::IDENTITY, vec![diffuse(GREEN)], vec![]);
    let far = model(quad(4, 0.25, -0.25, 0.75, 0.25, -3.0), Matrix4x4::IDENTITY, vec![diffuse(GREEN)], vec![]);
    snapshot(Some(camera), vec![near, far], vec![directional(WHITE, [0.0, 0.0, -1.0])])
}

/// Two squares of the same size 3 units apart in depth are drawn at the same size: each covers
/// a quarter of the width (12.5 % to 37.5 % and 62.5 % to 87.5 %).
fn orthographic_probes(side: usize) -> Vec<Probe> {
    let green = [0, 255, 0, 255];
    let mut probes = Vec::new();
    for (fx, inside) in [(0.11, false), (0.14, true), (0.36, true), (0.39, false), (0.61, false), (0.64, true), (0.86, true), (0.89, false)] {
        probes.push(Probe::at(
            if inside { "inside a square" } else { "outside the squares" },
            side,
            fx,
            0.5,
            if inside { green } else { TRANSPARENT },
            1,
        ));
    }
    probes.push(Probe::at("above the far square", side, 0.75, 0.36, TRANSPARENT, 0));
    probes.push(Probe::at("in the far square", side, 0.75, 0.38, green, 1));
    probes
}

/// The camera of the lit cube as a `MatrixCamera`, its principal point moved by a tenth of the
/// width: `m31` of the projection adds 0.2 w to x (z of the view is -w).
fn shifted_camera() -> Scene3DCamera {
    let camera = cube_camera();
    let mut projection = camera.projection;
    projection.m31 = -0.2;
    Scene3DCamera {
        kind: Scene3DCameraKind::Matrix,
        projection,
        near_plane_distance: 0.0,
        far_plane_distance: f32::INFINITY,
        ..camera
    }
}

fn matrix_camera_scene() -> Arc<Scene3DSnapshot> {
    snapshot(Some(shifted_camera()), vec![turned_cube()], cube_lights())
}

/// The probes of the lit cube, a tenth of the width to the right.
fn matrix_camera_probes(side: usize) -> Vec<Probe> {
    cube_face_probes(&cube_camera(), side, 0.1)
}

/// The camera of the lit cube as a `MatrixCamera` with its own matrices: the same image.
pub fn unshifted_matrix_camera_scene() -> Arc<Scene3DSnapshot> {
    let camera = Scene3DCamera { kind: Scene3DCameraKind::Matrix, near_plane_distance: 0.0, far_plane_distance: f32::INFINITY, ..cube_camera() };
    snapshot(Some(camera), vec![turned_cube()], cube_lights())
}

fn culling_scene() -> Arc<Scene3DSnapshot> {
    let camera = orthographic([0.0, 0.0, 5.0], 2.0, 1.0, 10.0);
    let front = model(quad(5, -0.9, -0.4, -0.35, 0.4, 0.0), Matrix4x4::IDENTITY, vec![diffuse(GREEN)], vec![diffuse(RED)]);
    let back_with_material = model(reversed_quad(6, -0.25, -0.4, 0.25, 0.4, 0.0), Matrix4x4::IDENTITY, vec![diffuse(RED)], vec![diffuse(BLUE)]);
    let back_without = model(reversed_quad(7, 0.35, -0.4, 0.9, 0.4, 0.0), Matrix4x4::IDENTITY, vec![diffuse(RED)], vec![]);
    snapshot(Some(camera), vec![front, back_with_material, back_without], vec![directional(WHITE, [0.0, 0.0, -1.0])])
}

/// A front face draws its `Material`, a back face its `BackMaterial` (lit with its normal turned
/// toward the camera), and a back face without one is culled.
fn culling_probes(side: usize) -> Vec<Probe> {
    vec![
        Probe::at("a front face: Material", side, 0.1875, 0.5, [0, 255, 0, 255], 1),
        Probe::at("a back face: BackMaterial", side, 0.5, 0.5, [0, 0, 255, 255], 1),
        Probe::at("a back face without BackMaterial: culled", side, 0.8125, 0.5, TRANSPARENT, 0),
    ]
}

fn material_group_scene() -> Arc<Scene3DSnapshot> {
    let layers = vec![diffuse(gray(128)), specular(WHITE, 200.0), emissive(Color::from_argb(255, 0, 0, 64))];
    let plane = model(quad(8, -1.5, -1.5, 1.5, 1.5, 0.0), Matrix4x4::IDENTITY, layers, vec![]);
    snapshot(Some(floor_camera()), vec![plane], vec![directional(WHITE, [0.0, 0.0, -1.0])])
}

/// The layers in order: the diffuse grey (lit head on), the Blinn-Phong highlight of power 200
/// added, the emissive blue added, each clamped as the target stores it.
fn material_group_at(p: F3) -> [f64; 4] {
    let view = normalize(sub([0.0, 0.0, 4.0], p));
    let half = normalize([view[0], view[1], view[2] + 1.0]);
    let highlight = half[2].max(0.0).powf(200.0);
    let diffuse = LEVEL_128;
    let lit = (diffuse + highlight).min(1.0);
    [lit, lit, (lit + LEVEL_64).min(1.0), 1.0]
}

fn material_group_probes(side: usize) -> Vec<Probe> {
    plane_probes(
        side,
        material_group_at,
        [("the highlight", [0.0, 0.0, 0.0]), ("the edge of the highlight", [1.2, 0.0, 0.0]), ("away from it", [1.4, 1.4, 0.0])],
    )
}

fn transparency_scene() -> Arc<Scene3DSnapshot> {
    let camera = orthographic([0.0, 0.0, 5.0], 2.0, 1.0, 10.0);
    let layer = |color| vec![diffuse(color)];
    let models = vec![
        // On the left, the far opaque square first: the translucent one is blended over it.
        model(quad(9, -0.9, -0.5, -0.3, 0.5, -1.0), Matrix4x4::IDENTITY, layer(BLUE), vec![]),
        model(quad(10, -0.6, -0.5, -0.1, 0.5, 0.0), Matrix4x4::IDENTITY, layer(TRANSLUCENT_RED), vec![]),
        // On the right, the near translucent square first: it wrote depth, and the far square
        // behind it is not drawn there (no sorting, as in WPF).
        model(quad(11, 0.1, -0.5, 0.6, 0.5, 0.0), Matrix4x4::IDENTITY, layer(TRANSLUCENT_RED), vec![]),
        model(quad(12, 0.3, -0.5, 0.9, 0.5, -1.0), Matrix4x4::IDENTITY, layer(BLUE), vec![]),
    ];
    snapshot(Some(camera), models, vec![Scene3DLight::Ambient { color: WHITE }])
}

fn transparency_probes(side: usize) -> Vec<Probe> {
    let half_red = [128, 0, 0, 128];
    vec![
        Probe::at("blue alone", side, 0.125, 0.5, [0, 0, 255, 255], 1),
        Probe::at("translucent red over blue", side, 0.275, 0.5, [128, 0, 127, 255], 1),
        Probe::at("translucent red alone", side, 0.4, 0.5, half_red, 1),
        Probe::at("translucent red alone", side, 0.6, 0.5, half_red, 1),
        Probe::at("the far square hidden by the earlier translucent one", side, 0.725, 0.5, half_red, 1),
        Probe::at("blue alone", side, 0.875, 0.5, [0, 0, 255, 255], 1),
    ]
}

/// A camera 3 units away with its planes at 2.5 and 3.5.
fn clipping_camera() -> Scene3DCamera {
    perspective([0.0, 0.0, 3.0], 45.0, 2.5, 3.5)
}

fn clipping_scene() -> Arc<Scene3DSnapshot> {
    // The plane x + z = 0, from z = 2 on the left to z = -2 on the right, facing the camera.
    let normal = [std::f32::consts::FRAC_1_SQRT_2, 0.0, std::f32::consts::FRAC_1_SQRT_2];
    let plane = polygon(13, [[-2.0, -1.0, 2.0], [2.0, -1.0, -2.0], [2.0, 1.0, -2.0], [-2.0, 1.0, 2.0]], normal);
    let model = model(plane, Matrix4x4::IDENTITY, vec![diffuse(ORANGE)], vec![]);
    snapshot(Some(clipping_camera()), vec![model], vec![Scene3DLight::Ambient { color: WHITE }])
}

/// The part of the plane between depth 2.5 and 3.5 (z from 0.5 to -0.5) is drawn, the rest is
/// clipped by the near and the far plane.
fn clipping_probes(side: usize) -> Vec<Probe> {
    let camera = clipping_camera();
    let orange = |_, _| [255, 128, 0, 255];
    let none = |_, _| TRANSPARENT;
    vec![
        probe_at("between the planes", &camera, side, [0.0, 0.0, 0.0], orange, 1),
        probe_at("between the planes, near", &camera, side, [-0.3, 0.0, 0.3], orange, 1),
        probe_at("between the planes, far", &camera, side, [0.3, 0.0, -0.3], orange, 1),
        probe_at("before the near plane", &camera, side, [-0.75, 0.0, 0.75], none, 0),
        probe_at("beyond the far plane", &camera, side, [0.75, 0.0, -0.75], none, 0),
    ]
}

/// The bounds of the rotated viewport, its clip (the left half of its bounds) and its opacity.
const ROTATED_BOUNDS: Rect = Rect::new(10.0, 10.0, 80.0, 80.0);
const ROTATED_CLIP: Rect = Rect::new(10.0, 10.0, 40.0, 80.0);
const ROTATED_OPACITY: f64 = 0.5;

fn rotation() -> Matrix {
    Matrix::create_rotation_at(30f64.to_radians(), Point::new(50.0, 50.0))
}

fn rotated_scene() -> Arc<Scene3DSnapshot> {
    let mut scene = (*lit_cube()).clone();
    scene.viewport = ROTATED_BOUNDS;
    Arc::new(scene)
}

/// A `Viewport3D` rotated by 30 degrees, clipped to its bounds' left half (`ClipToBounds` of an
/// element in it) and drawn at half opacity.
fn draw_rotated(context: &mut dyn IDrawingContextImpl, scene: &Arc<Scene3DSnapshot>) {
    let saved = context.transform();
    context.set_transform(rotation() * saved);
    context.push_clip(ROTATED_CLIP);
    context.push_opacity(ROTATED_OPACITY, None);
    draw_scene_3d(context, scene, ROTATED_BOUNDS);
    context.pop_opacity();
    context.pop_clip();
    context.set_transform(saved);
}

/// The probes of the cube's faces, through the viewport, the rotation and the scale.
fn rotated_probes(side: usize) -> Vec<Probe> {
    let scale = side as f64 / f64::from(SIDE);
    let viewport_side = (ROTATED_BOUNDS.width * scale).round() as usize;
    let rotation = rotation();
    let to_device = |x: f64, y: f64| {
        // Pixels of the viewport's image to the coordinates of the context, then the rotation and
        // the scale to the DPI.
        let (lx, ly) = (ROTATED_BOUNDS.x + x / scale, ROTATED_BOUNDS.y + y / scale);
        let rx = lx * rotation.m11 + ly * rotation.m21 + rotation.m31;
        let ry = lx * rotation.m12 + ly * rotation.m22 + rotation.m32;
        ((rx * scale).floor() as usize, (ry * scale).floor() as usize)
    };
    let angle = 30f64.to_radians();
    let camera = cube_camera();
    let face = |point: F3| {
        let (x, y) = project(&camera, viewport_side, point);
        to_device(x, y)
    };
    // The -x face is on the left half (drawn at half opacity), the +z face on the right half
    // (clipped). Paint alpha is a byte: 127 of 255.
    let alpha = (255.0 * ROTATED_OPACITY) as u8;
    let red = ((LEVEL_64 + LEVEL_128 * 0.5) * 255.0 + 0.5).floor() * f64::from(alpha) / 255.0;
    let (lx, ly) = face([-0.5 * angle.cos(), 0.0, 0.5 * angle.sin()]);
    let (rx, ry) = face([0.5 * angle.sin(), 0.0, 0.5 * angle.cos()]);
    vec![
        Probe { what: "the -x face at half opacity", x: lx, y: ly, expected: [red.round() as u8, 0, 0, alpha], levels: 3 },
        Probe { what: "the +z face, clipped", x: rx, y: ry, expected: TRANSPARENT, levels: 0 },
        Probe::at("outside the rotated viewport", side, 0.02, 0.02, TRANSPARENT, 0),
    ]
}

fn empty_scene() -> Arc<Scene3DSnapshot> {
    snapshot(Some(cube_camera()), vec![], cube_lights())
}

fn empty_probes(side: usize) -> Vec<Probe> {
    vec![Probe::at("the centre", side, 0.5, 0.5, TRANSPARENT, 0), Probe::at("a corner", side, 0.0, 0.0, TRANSPARENT, 0)]
}

/// Draws a scene over the whole square.
pub fn draw_plain(context: &mut dyn IDrawingContextImpl, scene: &Arc<Scene3DSnapshot>) {
    draw_scene_3d(context, scene, Rect::new(0.0, 0.0, f64::from(SIDE), f64::from(SIDE)));
}

fn case(
    name: &'static str,
    scene: fn() -> Arc<Scene3DSnapshot>,
    probes: fn(usize) -> Vec<Probe>,
) -> Scene3DCase {
    Scene3DCase {
        name,
        scene,
        draw: draw_plain,
        plain: true,
        probes,
        tolerance: Tolerance::Largest(2),
        bound: 0.05,
        layers: 1,
    }
}

/// The eleven scenes of step D3.
pub fn scenes() -> Vec<Scene3DCase> {
    vec![
        case("lit-cube", lit_cube, lit_cube_probes),
        case("point-light", point_light_scene, point_light_probes),
        case("spot-light", spot_light_scene, spot_light_probes),
        case("orthographic-camera", orthographic_scene, orthographic_probes),
        case("matrix-camera", matrix_camera_scene, matrix_camera_probes),
        case("front-back-culling", culling_scene, culling_probes),
        Scene3DCase { layers: 3, ..case("material-group", material_group_scene, material_group_probes) },
        case("transparent-overlap", transparency_scene, transparency_probes),
        case("near-far-clipping", clipping_scene, clipping_probes),
        Scene3DCase {
            name: "clip-opacity-rotated",
            scene: rotated_scene,
            draw: draw_rotated,
            plain: false,
            probes: rotated_probes,
            tolerance: Tolerance::Moves,
            bound: 3.0,
            layers: 1,
        },
        case("empty", empty_scene, empty_probes),
        brush_case("linear-gradient", linear_gradient_scene, linear_gradient_probes),
        brush_case("radial-gradient", radial_gradient_scene, radial_gradient_probes),
        brush_case("gradient-lookup-absolute", gradient_lookup_scene, gradient_lookup_probes),
        brush_case("image-brush-tiles", image_tiles_scene, image_tiles_probes),
        brush_case("image-brush-viewbox", image_viewbox_scene, image_viewbox_probes),
        Scene3DCase { layers: 3, ..brush_case("brush-material-group", brush_group_scene, brush_group_probes) },
    ]
}

// Step D6: materials from brushes. Every scene is a quad over the whole square, seen by the
// orthographic camera of width 2 (x and y from -1 to 1), so the texture coordinate of a sample is
// a linear function of its position in the image.

/// The brush of a layer of `kind`: the material's `Color` white.
fn brush_layer(kind: Scene3DMaterialKind, brush: &dyn IBrush, specular_power: f32) -> Scene3DMaterialLayer {
    let brush = SharedBrush::from_brush(brush);
    Scene3DMaterialLayer { kind, color: WHITE, ambient_color: WHITE, specular_power, brush }
}

/// A quad over the square from (-1, -1) to (1, 1) with the texture coordinates `uv0` at its top
/// left and `uv1` at its bottom right (WPF's texture space has y down).
fn textured_square(id: u64, uv0: [f32; 2], uv1: [f32; 2]) -> Arc<MeshData> {
    let positions = vec![
        Vector3::new(-1.0, -1.0, 0.0),
        Vector3::new(1.0, -1.0, 0.0),
        Vector3::new(1.0, 1.0, 0.0),
        Vector3::new(-1.0, 1.0, 0.0),
    ];
    let texture_coordinates = vec![
        Vector2::new(uv0[0], uv1[1]),
        Vector2::new(uv1[0], uv1[1]),
        Vector2::new(uv1[0], uv0[1]),
        Vector2::new(uv0[0], uv0[1]),
    ];
    let bounds = Scene3DBounds::of_points(positions.iter().copied());
    Arc::new(MeshData {
        id,
        version: 1,
        positions,
        normals: vec![Vector3::new(0.0, 0.0, 1.0); 4],
        normals_generated: false,
        texture_coordinates,
        indices: vec![0, 1, 2, 0, 2, 3],
        bounds,
    })
}

fn brush_camera() -> Scene3DCamera {
    orthographic([0.0, 0.0, 5.0], 2.0, 1.0, 10.0)
}

/// A square of `layers` lit by a white ambient light: a diffuse layer is its brush.
fn brush_square(id: u64, uv0: [f32; 2], uv1: [f32; 2], layers: Vec<Scene3DMaterialLayer>) -> Arc<Scene3DSnapshot> {
    let square = model(textured_square(id, uv0, uv1), Matrix4x4::IDENTITY, layers, vec![]);
    snapshot(Some(brush_camera()), vec![square], vec![Scene3DLight::Ambient { color: WHITE }])
}

/// The texture coordinate of the point at `x`, `y` (in pixels) of an image of `side` pixels of a
/// square with `uv0` at its top left and `uv1` at its bottom right.
fn uv_at(side: usize, x: f64, y: f64, uv0: [f64; 2], uv1: [f64; 2]) -> [f64; 2] {
    let (fx, fy) = (x / side as f64, y / side as f64);
    [uv0[0] + (uv1[0] - uv0[0]) * fx, uv0[1] + (uv1[1] - uv0[1]) * fy]
}

/// The pixel at `px`, `py` resolved from its four samples, each the straight colour `color` of
/// the brush at its texture coordinate (opaque colours: premultiplied is straight), rounded.
fn resolved_brush(
    side: usize,
    px: usize,
    py: usize,
    uv0: [f64; 2],
    uv1: [f64; 2],
    color: &dyn Fn([f64; 2]) -> [f64; 4],
) -> [u8; 4] {
    let mut sum = [0.0; 4];
    for (sx, sy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
        let sample = color(uv_at(side, px as f64 + sx, py as f64 + sy, uv0, uv1));
        for i in 0..4 {
            sum[i] += sample[i].clamp(0.0, 1.0) / 4.0;
        }
    }
    sum.map(|value| (value.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)
}

/// The probe of the pixel at the fractions `fx`, `fy` with the colour of `color` there.
fn brush_probe(
    what: &'static str,
    side: usize,
    fx: f64,
    fy: f64,
    uv: ([f64; 2], [f64; 2]),
    color: &dyn Fn([f64; 2]) -> [f64; 4],
) -> Probe {
    let mut probe = Probe::at(what, side, fx, fy, TRANSPARENT, 2);
    probe.expected = resolved_brush(side, probe.x, probe.y, uv.0, uv.1, color);
    probe
}

fn stops_of(colors: &[(f64, Color)]) -> Vec<ImmutableGradientStop> {
    colors.iter().map(|(offset, color)| ImmutableGradientStop::new(*offset, *color)).collect()
}

/// Red to blue, straight.
fn red_to_blue(t: f64) -> [f64; 4] {
    let t = t.clamp(0.0, 1.0);
    [1.0 - t, 0.0, t, 1.0]
}

const UNIT_UV: ([f64; 2], [f64; 2]) = ([0.0, 0.0], [1.0, 1.0]);

fn linear_gradient_brush() -> ImmutableLinearGradientBrush {
    // Relative to the box of the texture coordinates, from its top left to its right edge.
    ImmutableLinearGradientBrush::new(
        &stops_of(&[(0.0, RED), (1.0, BLUE)]),
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative)),
        Some(RelativePoint::new(1.0, 0.0, RelativeUnit::Relative)),
        None,
    )
}

fn linear_gradient_scene() -> Arc<Scene3DSnapshot> {
    let layer = brush_layer(Scene3DMaterialKind::Diffuse, &linear_gradient_brush(), 0.0);
    brush_square(20, [0.0, 0.0], [1.0, 1.0], vec![layer])
}

/// A diffuse `LinearGradientBrush` relative to the box of the texture coordinates: red at the
/// left edge, blue at the right, linear between.
fn linear_gradient_probes(side: usize) -> Vec<Probe> {
    let color = |uv: [f64; 2]| red_to_blue(uv[0]);
    vec![
        brush_probe("the left edge: red", side, 0.0, 0.5, UNIT_UV, &color),
        brush_probe("a quarter across", side, 0.25, 0.3, UNIT_UV, &color),
        brush_probe("the middle", side, 0.5, 0.8, UNIT_UV, &color),
        brush_probe("the right edge: blue", side, 0.995, 0.5, UNIT_UV, &color),
    ]
}

/// The focus of the radial gradient: a quarter of the way from the left of the box.
const RADIAL_FOCUS: [f64; 2] = [0.25, 0.5];

fn radial_gradient_scene() -> Arc<Scene3DSnapshot> {
    let brush = ImmutableRadialGradientBrush::new(
        &stops_of(&[(0.0, RED), (1.0, BLUE)]),
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        None,
        Some(RelativePoint::new(RADIAL_FOCUS[0], RADIAL_FOCUS[1], RelativeUnit::Relative)),
        None,
        None,
        None,
    );
    brush_square(21, [0.0, 0.0], [1.0, 1.0], vec![brush_layer(Scene3DMaterialKind::Diffuse, &brush, 0.0)])
}

/// The parameter of WPF's radial gradient of centre (0.5, 0.5), radii 0.5 and the focus
/// [`RADIAL_FOCUS`] at `uv`: the distance from the focus over the distance from the focus to the
/// circle along the same ray.
fn radial_t(uv: [f64; 2]) -> f64 {
    let (fx, fy) = ((RADIAL_FOCUS[0] - 0.5) / 0.5, (RADIAL_FOCUS[1] - 0.5) / 0.5);
    let (qx, qy) = ((uv[0] - 0.5) / 0.5, (uv[1] - 0.5) / 0.5);
    let (dx, dy) = (qx - fx, qy - fy);
    let distance = dx.hypot(dy);
    if distance < 1e-9 {
        return 0.0;
    }
    let (ux, uy) = (dx / distance, dy / distance);
    // The ray from the focus along (ux, uy) meets the unit circle at s: |f + s u| = 1.
    let b = fx * ux + fy * uy;
    let s = -b + (b * b - (fx * fx + fy * fy - 1.0)).sqrt();
    distance / s
}

fn radial_gradient_probes(side: usize) -> Vec<Probe> {
    let color = |uv: [f64; 2]| red_to_blue(radial_t(uv));
    vec![
        brush_probe("at the focus: red", side, 0.25, 0.5, UNIT_UV, &color),
        brush_probe("between the focus and the circle", side, 0.6, 0.5, UNIT_UV, &color),
        brush_probe("above the focus", side, 0.3, 0.25, UNIT_UV, &color),
        brush_probe("outside the circle: blue", side, 0.03, 0.03, UNIT_UV, &color),
    ]
}

/// The texture coordinates of the square of the lookup scene: half of the absolute gradient.
const LOOKUP_UV: ([f64; 2], [f64; 2]) = ([0.0, 0.0], [0.5, 1.0]);

/// Twelve stops alternating red and blue at elevenths, sampled from the lookup texture.
fn lookup_stops() -> Vec<(f64, Color)> {
    (0..12).map(|index| (f64::from(index) / 11.0, if index % 2 == 0 { RED } else { BLUE })).collect()
}

fn gradient_lookup_scene() -> Arc<Scene3DSnapshot> {
    // Absolute: from (0, 0) to (1, 0) in texture coordinates, over a square whose texture
    // coordinates span only half of it.
    let brush = ImmutableLinearGradientBrush::new(
        &stops_of(&lookup_stops()),
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(0.0, 0.0, RelativeUnit::Absolute)),
        Some(RelativePoint::new(1.0, 0.0, RelativeUnit::Absolute)),
        None,
    );
    let uv1 = [LOOKUP_UV.1[0] as f32, LOOKUP_UV.1[1] as f32];
    brush_square(22, [0.0, 0.0], uv1, vec![brush_layer(Scene3DMaterialKind::Diffuse, &brush, 0.0)])
}

/// The colour of the twelve stops at `t`: red and blue alternating, linear between.
fn lookup_color(t: f64) -> [f64; 4] {
    let scaled = (t.clamp(0.0, 1.0) * 11.0).min(11.0);
    let index = scaled.floor().min(10.0);
    let f = scaled - index;
    let from_red = index as u32 % 2 == 0;
    red_to_blue(if from_red { f } else { 1.0 - f })
}

/// Half of the absolute gradient across the square: the stops at elevenths of the texture
/// coordinates, so one at 0.09, the next at 0.18. The probes avoid the texels around a stop,
/// where the linear filter of the lookup texture rounds the corner of the colours.
fn gradient_lookup_probes(side: usize) -> Vec<Probe> {
    let color = |uv: [f64; 2]| lookup_color(uv[0]);
    let mut probes = Vec::new();
    for fx in [0.05, 0.27, 0.6, 0.81] {
        let mut probe = brush_probe("between two stops", side, fx, 0.5, LOOKUP_UV, &color);
        probe.levels = 3;
        probes.push(probe);
    }
    probes
}

/// A bitmap of 4 by 4 opaque pixels in quadrants of 2 by 2: red, green / blue, white; the same
/// bitmap for every snapshot (one texture on a device).
fn quadrant_bitmap() -> Arc<SharedBitmapImpl> {
    static BITMAP: std::sync::OnceLock<Arc<SharedBitmapImpl>> = std::sync::OnceLock::new();
    BITMAP
        .get_or_init(|| {
            let mut pixels = Vec::new();
            for y in 0..4 {
                for x in 0..4 {
                    pixels.extend_from_slice(QUADRANT_COLORS[usize::from(y >= 2) * 2 + usize::from(x >= 2)].as_slice());
                }
            }
            Arc::new(PixelBitmap::new(4, 4, pixels))
        })
        .clone()
}

/// The colours of the quadrants of [`quadrant_bitmap`]: top left, top right, bottom left, bottom
/// right.
const QUADRANT_COLORS: [[u8; 4]; 4] = [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255], [255, 255, 255, 255]];

fn image_brush(
    stretch: Stretch,
    tile_mode: TileMode,
    destination: RelativeRect,
    source: RelativeRect,
) -> ImmutableImageBrush {
    let bitmap = Rc::new(Bitmap::from_impl(quadrant_bitmap()));
    ImmutableImageBrush::new(
        Some(bitmap),
        AlignmentX::Center,
        AlignmentY::Center,
        Some(destination),
        1.0,
        None,
        RelativePoint::new(0.0, 0.0, RelativeUnit::Relative),
        Some(source),
        stretch,
        tile_mode,
        None,
    )
}

fn image_tiles_scene() -> Arc<Scene3DSnapshot> {
    let half = RelativeRect::new(0.0, 0.0, 0.5, 0.5, RelativeUnit::Relative);
    let brush = image_brush(Stretch::Fill, TileMode::FlipX, half, RelativeRect::FILL);
    brush_square(23, [0.0, 0.0], [1.0, 1.0], vec![brush_layer(Scene3DMaterialKind::Diffuse, &brush, 0.0)])
}

fn quadrant(index: usize) -> [u8; 4] {
    QUADRANT_COLORS[index]
}

/// An `ImageBrush` tiled in four tiles of half the box, the odd columns mirrored (`FlipX`): the
/// probes sit in the middle of the quadrants of the bitmap in each tile, away from the texels
/// the linear filter mixes.
fn image_tiles_probes(side: usize) -> Vec<Probe> {
    let at = |what, fx, fy, color| Probe::at(what, side, fx, fy, color, 2);
    vec![
        at("the first tile, its top left: red", 0.125, 0.125, quadrant(0)),
        at("the first tile, its top right: green", 0.375, 0.125, quadrant(1)),
        at("the second tile, mirrored: green on its left", 0.625, 0.125, quadrant(1)),
        at("the second tile, mirrored: red on its right", 0.875, 0.125, quadrant(0)),
        at("the second row, not mirrored: blue", 0.125, 0.875, quadrant(2)),
        at("the last tile, mirrored: white on its left", 0.625, 0.875, quadrant(3)),
    ]
}

fn image_viewbox_scene() -> Arc<Scene3DSnapshot> {
    // The viewbox is the bottom half of the bitmap (blue, white), stretched uniformly into the box
    // of the texture coordinates, which is a little more than twice as high as wide: centred, with
    // transparent bands above and below. Not exactly twice: the edges of the viewbox would then be
    // at 37.5 % and 62.5 % of the image, the centres of two rows of pixels at 1x, where whether a
    // pixel is inside the viewbox is decided by the rounding of the interpolated texture
    // coordinates, which the rasterizer of a GPU and the CPU reference do not share.
    let source = RelativeRect::new(0.0, 0.5, 1.0, 0.5, RelativeUnit::Relative);
    let brush = image_brush(Stretch::Uniform, TileMode::None, RelativeRect::FILL, source);
    brush_square(24, [0.0, 0.0], [1.0, 2.1], vec![brush_layer(Scene3DMaterialKind::Diffuse, &brush, 0.0)])
}

/// The viewbox of 4 by 2 pixels placed uniformly in a box of 1 by 2.1: 1 by 0.5, from 0.8 to 1.3
/// of the height of the texture coordinates, so from 38.1 % to 61.9 % of the image (no edge on
/// the centre of a row of pixels at 1x or 2x).
fn image_viewbox_probes(side: usize) -> Vec<Probe> {
    vec![
        Probe::at("above the viewbox: transparent", side, 0.5, 0.2, TRANSPARENT, 0),
        Probe::at("the viewbox, its left half: blue", side, 0.2, 0.5, quadrant(2), 2),
        Probe::at("the viewbox, its right half: white", side, 0.8, 0.5, quadrant(3), 2),
        Probe::at("below the viewbox: transparent", side, 0.5, 0.8, TRANSPARENT, 0),
    ]
}

/// A dim green bitmap of 2 by 2 pixels for the emissive layer; the same for every snapshot.
fn dim_green_bitmap() -> Arc<SharedBitmapImpl> {
    static BITMAP: std::sync::OnceLock<Arc<SharedBitmapImpl>> = std::sync::OnceLock::new();
    BITMAP.get_or_init(|| Arc::new(PixelBitmap::new(2, 2, [0, 64, 0, 255].repeat(4)))).clone()
}

fn brush_group_scene() -> Arc<Scene3DSnapshot> {
    let emissive_brush = ImmutableImageBrush::new(
        Some(Rc::new(Bitmap::from_impl(dim_green_bitmap()))),
        AlignmentX::Center,
        AlignmentY::Center,
        None,
        1.0,
        None,
        RelativePoint::new(0.0, 0.0, RelativeUnit::Relative),
        None,
        Stretch::Fill,
        TileMode::None,
        None,
    );
    let layers = vec![
        brush_layer(Scene3DMaterialKind::Diffuse, &ImmutableSolidColorBrush::new(gray(64)), 0.0),
        brush_layer(Scene3DMaterialKind::Specular, &linear_gradient_brush(), 40.0),
        brush_layer(Scene3DMaterialKind::Emissive, &emissive_brush, 0.0),
    ];
    let square = model(textured_square(25, [0.0, 0.0], [1.0, 1.0]), Matrix4x4::IDENTITY, layers, vec![]);
    snapshot(Some(brush_camera()), vec![square], vec![directional(WHITE, [0.0, 0.0, -1.0])])
}

/// A `MaterialGroup` lit head on by a white directional light, seen by an orthographic camera
/// along the light: the grey diffuse layer at 64, the highlight of the specular layer (1
/// everywhere: N, L and H along +z) times its gradient brush, and the dim green image of the
/// emissive layer, added in order.
fn brush_group_probes(side: usize) -> Vec<Probe> {
    let color = |uv: [f64; 2]| {
        let gradient = red_to_blue(uv[0]);
        [LEVEL_64 + gradient[0], LEVEL_64 + LEVEL_64, LEVEL_64 + gradient[2], 1.0]
    };
    vec![
        brush_probe("a third across", side, 0.3, 0.5, UNIT_UV, &color),
        brush_probe("the middle", side, 0.5, 0.2, UNIT_UV, &color),
        brush_probe("two thirds across", side, 0.7, 0.7, UNIT_UV, &color),
    ]
}

/// A step D6 scene: plain, held to the CPU reference.
fn brush_case(name: &'static str, scene: fn() -> Arc<Scene3DSnapshot>, probes: fn(usize) -> Vec<Probe>) -> Scene3DCase {
    case(name, scene, probes)
}

/// The scene of `brush-material-group` seen by its camera moved `step` hundredths along x: the
/// snapshot of the `step`-th frame of one viewport whose camera moves (its camera generation is
/// `1 + step`), with the same meshes, brushes and bitmaps at every step.
pub fn moving_brush_group(step: u32) -> Arc<Scene3DSnapshot> {
    let mut scene = (*brush_group_scene()).clone();
    let offset = step as f32 * 0.01;
    let mut camera = orthographic([offset, 0.0, 5.0], 2.0, 1.0, 10.0);
    let (eye, target) = (Vector3::new(offset, 0.0, 5.0), Vector3::new(offset, 0.0, 0.0));
    camera.view = Matrix4x4::create_look_at(eye, target, Vector3::new(0.0, 1.0, 0.0));
    scene.camera = Some(camera);
    scene.camera_generation = 1 + u64::from(step);
    Arc::new(scene)
}

/// A square painted by an image brush of a bitmap of 2 by 2 pixels of `color` whose pixels the
/// test changes ([`VersionedBitmap`]), with the snapshot that draws it.
pub fn versioned_image_scene(color: [u8; 4]) -> (Arc<VersionedBitmap>, Arc<Scene3DSnapshot>) {
    let bitmap = Arc::new(VersionedBitmap(PixelBitmap::new(2, 2, color.repeat(4))));
    let shared: Arc<SharedBitmapImpl> = bitmap.clone();
    let brush = ImmutableImageBrush::new(
        Some(Rc::new(Bitmap::from_impl(shared))),
        AlignmentX::Center,
        AlignmentY::Center,
        None,
        1.0,
        None,
        RelativePoint::new(0.0, 0.0, RelativeUnit::Relative),
        None,
        Stretch::Fill,
        TileMode::None,
        None,
    );
    let layer = brush_layer(Scene3DMaterialKind::Diffuse, &brush, 0.0);
    (bitmap, brush_square(26, [0.0, 0.0], [1.0, 1.0], vec![layer]))
}

/// A bitmap whose pixels are replaced (a new version) by [`VersionedBitmap::set_pixels`].
pub struct VersionedBitmap(PixelBitmap);

impl VersionedBitmap {
    /// Replaces the pixels and bumps the version.
    pub fn set_pixels(&self, color: [u8; 4]) {
        let mut pixels = self.0.pixels.lock().expect("the pixels");
        let count = pixels.len() / 4;
        *pixels = color.repeat(count);
        self.0.version.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

impl IBitmapImpl for VersionedBitmap {
    fn dpi(&self) -> Vector {
        self.0.dpi()
    }

    fn pixel_size(&self) -> PixelSize {
        self.0.pixel_size()
    }

    fn version(&self) -> i32 {
        self.0.version()
    }

    fn save(&self, stream: &mut dyn std::io::Write, options: &BitmapEncoderOptions) -> std::io::Result<()> {
        self.0.save(stream, options)
    }

    fn dispose(&self) {}


    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        Some(&self.0)
    }
}

/// A bitmap of premultiplied RGBA pixels in memory at 96 DPI, read through its framebuffer: the
/// image of the image brushes of the scenes, the same for every backend.
struct PixelBitmap {
    width: i32,
    height: i32,
    pixels: std::sync::Mutex<Vec<u8>>,
    version: std::sync::atomic::AtomicI32,
}

impl PixelBitmap {
    fn new(width: i32, height: i32, pixels: Vec<u8>) -> PixelBitmap {
        let (pixels, version) = (std::sync::Mutex::new(pixels), std::sync::atomic::AtomicI32::new(0));
        PixelBitmap { width, height, pixels, version }
    }
}

impl IBitmapImpl for PixelBitmap {
    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }

    fn pixel_size(&self) -> PixelSize {
        PixelSize::new(self.width, self.height)
    }

    fn version(&self) -> i32 {
        self.version.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn save(&self, _stream: &mut dyn std::io::Write, _options: &BitmapEncoderOptions) -> std::io::Result<()> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "the bitmap of a scene is not saved"))
    }

    fn dispose(&self) {}


    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        Some(self)
    }
}

impl IReadableBitmapImpl for PixelBitmap {
    fn format(&self) -> Option<PixelFormat> {
        Some(PixelFormat::RGBA8888)
    }

    fn alpha_format(&self) -> Option<AlphaFormat> {
        Some(AlphaFormat::Premul)
    }

    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        Rc::new(PixelFramebuffer {
            data: RefCell::new(self.pixels.lock().expect("the pixels").clone()),
            size: PixelSize::new(self.width, self.height),
        })
    }
}

struct PixelFramebuffer {
    data: RefCell<Vec<u8>>,
    size: PixelSize,
}

impl ILockedFramebuffer for PixelFramebuffer {
    fn address(&self) -> *mut u8 {
        self.data.borrow_mut().as_mut_ptr()
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(&mut self.data.borrow_mut());
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn row_bytes(&self) -> i32 {
        self.size.width * 4
    }

    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }

    fn format(&self) -> PixelFormat {
        PixelFormat::RGBA8888
    }

    fn alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }

    fn dispose(&self) {}
}
