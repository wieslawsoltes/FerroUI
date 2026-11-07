//! The compiled form of a 3D scene (#608, `docs/porting/media3d.md`, section 3.7): what a
//! `Viewport3D` of `ferroui-media3d` hands to the render thread, and how a drawing context is
//! asked to draw it.
//!
//! A [`Scene3DSnapshot`] is immutable and shared (`Arc`): the scene objects of the UI thread
//! (transforms, cameras, lights, materials, meshes, models and visuals) are flattened into it once
//! per change, in single precision, with world matrices, the meshes as shared [`MeshData`] with an
//! identity and a version, the material layers of each model, the lights in world space and the
//! camera. Its three generations say what changed since an earlier snapshot of the same viewport:
//!
//! - [`structure_generation`](Scene3DSnapshot::structure_generation): the meshes, the materials
//!   (which layers a model has and which brushes they paint with) and the objects of the tree;
//! - [`record_generation`](Scene3DSnapshot::record_generation): the transforms, the lights and the
//!   values of the materials (colours, specular power, the values of their brushes);
//! - [`camera_generation`](Scene3DSnapshot::camera_generation): the camera and the viewport.
//!
//! Each change bumps exactly one of them. An executor that kept the buffers of an earlier
//! snapshot writes only its uniforms on a new camera or record generation, and uploads only the
//! meshes whose `(id, version)` it has not seen on a new structure generation (a new structure
//! also rewrites the records).
//!
//! The snapshot is recorded into render data as the operation `DrawScene3D` (with the bounds of
//! the viewport in the coordinates of the visual) and replayed through
//! [`draw_scene_3d`]: a drawing context that implements
//! [`IDrawingContextImplWithScene3D`](crate::platform::IDrawingContextImplWithScene3D) draws it;
//! any other context draws nothing, and the scene is counted ([`scenes_not_drawn`]) and reported
//! once in the log, as the gaps of the GPU features are (`docs/porting/gpu-rendering.md`).
//!
//! Conventions: row vectors (a point is transformed as `p * M`), right-handed coordinates, the
//! camera looking down its negative z axis, and a projection to a clip depth between 0 and `w`
//! (Direct3D, Metal and WebGPU; a GL executor remaps it), as WPF's matrices are.

use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::media::{Color, SharedBrush};
use crate::numerics::{Matrix4x4, Vector2, Vector3};
use crate::platform::IDrawingContextImpl;
use crate::Rect;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// An axis-aligned box in single precision: the bounds of a mesh or of a scene.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Scene3DBounds {
    /// The corner with the smallest coordinates.
    pub min: Vector3,
    /// The corner with the largest coordinates.
    pub max: Vector3,
}

impl Scene3DBounds {
    /// The smallest box that holds every point; `None` without points.
    pub fn of_points(points: impl IntoIterator<Item = Vector3>) -> Option<Scene3DBounds> {
        let mut bounds: Option<Scene3DBounds> = None;
        for point in points {
            bounds = Some(match bounds {
                None => Scene3DBounds { min: point, max: point },
                Some(b) => Scene3DBounds {
                    min: Vector3::new(b.min.x.min(point.x), b.min.y.min(point.y), b.min.z.min(point.z)),
                    max: Vector3::new(b.max.x.max(point.x), b.max.y.max(point.y), b.max.z.max(point.z)),
                },
            });
        }
        bounds
    }

    /// The eight corners of the box.
    pub fn corners(&self) -> [Vector3; 8] {
        let (a, b) = (self.min, self.max);
        [
            Vector3::new(a.x, a.y, a.z),
            Vector3::new(b.x, a.y, a.z),
            Vector3::new(a.x, b.y, a.z),
            Vector3::new(b.x, b.y, a.z),
            Vector3::new(a.x, a.y, b.z),
            Vector3::new(b.x, a.y, b.z),
            Vector3::new(a.x, b.y, b.z),
            Vector3::new(b.x, b.y, b.z),
        ]
    }
}

/// The triangles of a mesh, in single precision, shared by every model and snapshot that draws
/// it.
///
/// `id` names the mesh for the lifetime of the process and `version` its content: an executor
/// caches the buffers of a mesh by the pair and uploads again only when it changes. The vertex
/// attributes have one entry per position (`normals` always, generated from the faces when the
/// mesh had none; `texture_coordinates` either none or one per position), and `indices` lists
/// three valid vertex indices per triangle.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MeshData {
    /// The identity of the mesh object this was made from.
    pub id: u64,
    /// The version of the content of that mesh.
    pub version: u64,
    /// The positions of the vertices, in the coordinates of the model.
    pub positions: Vec<Vector3>,
    /// The normals of the vertices, one per position, unit length.
    pub normals: Vec<Vector3>,
    /// Whether the normals were generated (the mesh named none, or not one per position).
    pub normals_generated: bool,
    /// The texture coordinates of the vertices: empty, or one per position.
    pub texture_coordinates: Vec<Vector2>,
    /// Three vertex indices per triangle, every one less than the number of positions.
    pub indices: Vec<u32>,
    /// The bounds of the positions; `None` without positions.
    pub bounds: Option<Scene3DBounds>,
}

impl MeshData {
    /// The number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// What a material layer adds to the colour of a model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scene3DMaterialKind {
    /// WPF's `DiffuseMaterial`: the brush lit by the lights (and by the ambient lights through
    /// `ambient_color`).
    Diffuse,
    /// WPF's `SpecularMaterial`: highlights added to what is below.
    Specular,
    /// WPF's `EmissiveMaterial`: the brush added without light.
    Emissive,
}

/// One layer of the material of a model: a `MaterialGroup` is flattened into its layers, in
/// order, each drawn over the ones before.
#[derive(Clone)]
pub struct Scene3DMaterialLayer {
    /// The kind of the layer.
    pub kind: Scene3DMaterialKind,
    /// The `Color` of the material, which filters its brush.
    pub color: Color,
    /// The `AmbientColor` of a diffuse layer (white for the other kinds).
    pub ambient_color: Color,
    /// The `SpecularPower` of a specular layer (0 for the other kinds).
    pub specular_power: f32,
    /// The brush of the layer in the form the render thread reads; `None` for a brush that has
    /// no such form yet (a visual or drawing brush, step D6 of the port). A material without a
    /// brush paints nothing, as in WPF, and is not a layer.
    pub brush: Option<SharedBrush>,
}

/// A model to draw: a `GeometryModel3D` with its world matrix and its materials.
#[derive(Clone)]
pub struct Scene3DModel {
    /// The matrix from the coordinates of the model to the world (the transforms of the model,
    /// its groups and its visuals).
    pub world: Matrix4x4,
    /// The mesh.
    pub mesh: Arc<MeshData>,
    /// The layers of the `Material`, drawn on the front faces (counterclockwise to the camera).
    pub front: Vec<Scene3DMaterialLayer>,
    /// The layers of the `BackMaterial`, drawn on the back faces.
    pub back: Vec<Scene3DMaterialLayer>,
    /// The bounds of the mesh in the world; `None` for a mesh without positions.
    pub world_bounds: Option<Scene3DBounds>,
}

/// A light in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scene3DLight {
    /// WPF's `AmbientLight`.
    Ambient {
        /// The colour of the light.
        color: Color,
    },
    /// WPF's `DirectionalLight`: the light travels along `direction` (unit length).
    Directional {
        /// The colour of the light.
        color: Color,
        /// The direction the light travels in, in the world.
        direction: Vector3,
    },
    /// WPF's `PointLight`.
    Point {
        /// The colour of the light.
        color: Color,
        /// The position of the light in the world.
        position: Vector3,
        /// The distance beyond which the light has no effect (infinite by default).
        range: f32,
        /// The constant, linear and quadratic attenuation.
        attenuation: [f32; 3],
    },
    /// WPF's `SpotLight`.
    Spot {
        /// The colour of the light.
        color: Color,
        /// The position of the light in the world.
        position: Vector3,
        /// The direction of the cone in the world (unit length).
        direction: Vector3,
        /// The distance beyond which the light has no effect.
        range: f32,
        /// The constant, linear and quadratic attenuation.
        attenuation: [f32; 3],
        /// The full angle of the cone of full light, in degrees.
        inner_cone_angle: f32,
        /// The full angle of the cone beyond which there is no light, in degrees.
        outer_cone_angle: f32,
    },
}

/// The kind of the camera a snapshot was made with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scene3DCameraKind {
    /// WPF's `PerspectiveCamera`.
    Perspective,
    /// WPF's `OrthographicCamera`.
    Orthographic,
    /// WPF's `MatrixCamera`: the matrices are the application's, no plane is known.
    Matrix,
}

/// The camera of a snapshot: its matrices for the aspect ratio of the viewport.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scene3DCamera {
    /// The kind of the camera.
    pub kind: Scene3DCameraKind,
    /// The matrix from the world to the coordinates of the camera (its transform included).
    pub view: Matrix4x4,
    /// The matrix from the coordinates of the camera to the clip space, with the near and far
    /// planes below.
    pub projection: Matrix4x4,
    /// The position of the camera in the world (for the specular highlights).
    pub position: Vector3,
    /// The near plane used by `projection`: the camera's `NearPlaneDistance`, raised to the
    /// scene when its far plane is infinite (0 for a matrix camera).
    pub near_plane_distance: f32,
    /// The far plane used by `projection`: the camera's `FarPlaneDistance`, or fitted to the
    /// bounds of the scene when that is infinite; infinite when there is nothing to fit
    /// (infinite for a matrix camera).
    pub far_plane_distance: f32,
}

/// An immutable, shared scene: see the documentation of the module.
#[derive(Clone)]
pub struct Scene3DSnapshot {
    /// The generation of the structure: meshes, materials and the objects of the tree.
    pub structure_generation: u64,
    /// The generation of the records: transforms, lights and the values of the materials.
    pub record_generation: u64,
    /// The generation of the camera and the viewport.
    pub camera_generation: u64,
    /// The viewport the camera was made for (the arranged bounds of the `Viewport3D`).
    pub viewport: Rect,
    /// The camera; `None` when the viewport has none (nothing is drawn, as in WPF).
    pub camera: Option<Scene3DCamera>,
    /// The models, in tree order (the order they are drawn in: transparent models are not
    /// sorted).
    pub models: Vec<Scene3DModel>,
    /// The lights, in tree order; every light lights every model.
    pub lights: Vec<Scene3DLight>,
    /// The bounds of the models in the world; `None` for a scene without positions.
    pub bounds: Option<Scene3DBounds>,
}

impl Scene3DSnapshot {
    /// A snapshot with nothing to draw.
    pub fn empty() -> Scene3DSnapshot {
        Scene3DSnapshot {
            structure_generation: 0,
            record_generation: 0,
            camera_generation: 0,
            viewport: Rect::default(),
            camera: None,
            models: Vec::new(),
            lights: Vec::new(),
            bounds: None,
        }
    }

    /// Whether drawing the snapshot draws anything: it has a camera and a model with a layer.
    pub fn draws_anything(&self) -> bool {
        self.camera.is_some() && self.models.iter().any(|model| !model.front.is_empty() || !model.back.is_empty())
    }

    /// The generations as one key: equal keys mean an equal scene for an executor that compares
    /// them (with the size of its target and the identities of its input textures).
    pub fn generations(&self) -> (u64, u64, u64) {
        (self.structure_generation, self.record_generation, self.camera_generation)
    }
}

/// What a drawing context is asked to draw: a snapshot, over `bounds` (the viewport in the
/// current coordinates of the context, whose transform maps them to the target).
#[derive(Clone)]
pub struct Scene3DFrame {
    /// The scene.
    pub scene: Arc<Scene3DSnapshot>,
    /// The area of the viewport, in the coordinates of the drawing context.
    pub bounds: Rect,
}

/// Compile-time proof that a snapshot can be sent to the render thread.
const _: fn() = || {
    fn shared<T: Send + Sync>() {}
    shared::<Scene3DSnapshot>();
    shared::<Scene3DFrame>();
};

/// The number of scenes drawn by a context that cannot draw them.
static SCENES_NOT_DRAWN: AtomicU64 = AtomicU64::new(0);
/// Whether the first of them was reported in the log.
static REPORTED: AtomicBool = AtomicBool::new(false);

/// How many times a scene was handed to a drawing context that cannot draw 3D scenes (it drew
/// nothing), since the process started. Steps D3 and later of the port give every backend a way
/// to draw them; until then this counts the gap.
pub fn scenes_not_drawn() -> u64 {
    SCENES_NOT_DRAWN.load(Ordering::Relaxed)
}

/// Draws `scene` over `bounds` on `context`: through its 3D capability when it has one, otherwise
/// nothing is drawn and the scene is counted ([`scenes_not_drawn`]) and, the first time, reported
/// in the log (area `Visual`, level warning). A snapshot that draws nothing is not counted.
pub fn draw_scene_3d(context: &mut dyn IDrawingContextImpl, scene: &Arc<Scene3DSnapshot>, bounds: Rect) {
    if !scene.draws_anything() || bounds.width <= 0.0 || bounds.height <= 0.0 {
        return;
    }
    let frame = Scene3DFrame { scene: scene.clone(), bounds };
    let drawn = match context.as_drawing_context_impl_with_scene_3d() {
        Some(scene_context) => scene_context.draw_scene_3d(&frame),
        None => false,
    };
    if !drawn {
        report_not_drawn();
    }
}

fn report_not_drawn() {
    SCENES_NOT_DRAWN.fetch_add(1, Ordering::Relaxed);
    if REPORTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
        logger.log(
            None,
            "A 3D scene (Viewport3D) is not drawn: the drawing context of this backend cannot draw 3D scenes yet \
             (docs/porting/media3d.md, docs/porting/gpu-rendering.md).",
        );
    }
}

#[cfg(test)]
mod tests;
