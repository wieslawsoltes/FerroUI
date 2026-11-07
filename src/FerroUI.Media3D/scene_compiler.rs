//! The compilation of a scene into a [`Scene3DSnapshot`]: the tree of visuals and models
//! flattened in tree order, in single precision, with the world matrices, the material layers,
//! the lights in world space and the camera with its planes fitted to the scene
//! (`docs/porting/media3d.md`, sections 3.6 and 3.7).

use crate::material::WHITE;
use crate::transform_3d::{transform_rect, value_of};
use crate::{
    AmbientLight, Camera, DiffuseMaterial, DirectionalLight, EmissiveMaterial, GeometryModel3D, Light, Material,
    MaterialGroup, Matrix3D, MeshGeometry3D, Model3D, Model3DGroup, ModelVisual3D, PerspectiveCamera, Point3D,
    PointLight, ProjectionCamera, Rect3D, SpecularMaterial, SpotLight, Vector3D, Visual3D, Visual3DCollection,
};
use ferroui_base::media::shared_brush_of;
use ferroui_base::numerics::{Matrix4x4, Vector3};
use ferroui_base::rendering::scene3d::{
    Scene3DBounds, Scene3DCamera, Scene3DCameraKind, Scene3DLight, Scene3DMaterialKind, Scene3DMaterialLayer,
    Scene3DModel, Scene3DSnapshot,
};
use ferroui_base::{Rect, Ref, WeakRef};

/// How deep the tree is followed: deeper visuals, models and materials (or a tree that holds
/// itself) are left out.
const MAX_DEPTH: usize = 256;

/// The visual and the model a model of a snapshot was made from, by the index of the model:
/// what the 3D hit testing reports (`VisualHit`, `ModelHit`).
pub(crate) type ModelSource = (WeakRef<Visual3D>, WeakRef<GeometryModel3D>);

/// What the compilation of a viewport needs.
pub(crate) struct SceneInput<'a> {
    pub camera: Option<Ref<Camera>>,
    pub children: &'a Visual3DCollection,
    pub viewport: Rect,
    pub generations: (u64, u64, u64),
}

/// Compiles a scene: the snapshot, and the sources of its models.
pub(crate) fn compile(input: SceneInput<'_>) -> (Scene3DSnapshot, Vec<ModelSource>) {
    let mut compiler = Compiler { models: Vec::new(), lights: Vec::new(), sources: Vec::new(), bounds: Rect3D::EMPTY };
    for child in input.children.iter() {
        compiler.visit_visual(&child, &Matrix3D::IDENTITY, 0);
    }
    let camera = input.camera.as_ref().and_then(|camera| compile_camera(camera, input.viewport, compiler.bounds));
    let (structure_generation, record_generation, camera_generation) = input.generations;
    let snapshot = Scene3DSnapshot {
        structure_generation,
        record_generation,
        camera_generation,
        viewport: input.viewport,
        camera,
        models: compiler.models,
        lights: compiler.lights,
        bounds: bounds_of(compiler.bounds),
    };
    (snapshot, compiler.sources)
}

struct Compiler {
    models: Vec<Scene3DModel>,
    lights: Vec<Scene3DLight>,
    sources: Vec<ModelSource>,
    /// The bounds of the drawn models in the world.
    bounds: Rect3D,
}

impl Compiler {
    fn visit_visual(&mut self, visual: &Ref<Visual3D>, parent: &Matrix3D, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        let world = value_of(visual.transform().as_ref()) * *parent;
        let Some(model_visual) = visual.cast::<ModelVisual3D>() else { return };
        if let Some(content) = model_visual.content() {
            self.visit_model(&content, &world, visual, depth + 1);
        }
        for child in model_visual.children().iter() {
            self.visit_visual(&child, &world, depth + 1);
        }
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
            self.add_geometry_model(&geometry_model, &world, visual);
        } else if let Some(light) = model.cast::<Light>() {
            if let Some(light) = compile_light(&light, &world) {
                self.lights.push(light);
            }
        }
    }

    fn add_geometry_model(&mut self, model: &Ref<GeometryModel3D>, world: &Matrix3D, visual: &Ref<Visual3D>) {
        let Some(mesh) = model.geometry().and_then(|geometry| geometry.cast::<MeshGeometry3D>()) else {
            return;
        };
        let front = material_layers(model.material().as_ref());
        let back = material_layers(model.back_material().as_ref());
        // A model without a material with a brush draws nothing (WPF).
        if front.is_empty() && back.is_empty() {
            return;
        }
        let world_bounds = transform_rect(mesh.bounds(), world);
        self.bounds.union(world_bounds);
        self.models.push(Scene3DModel {
            world: narrow_matrix(world),
            mesh: mesh.mesh_data(),
            front,
            back,
            world_bounds: bounds_of(world_bounds),
        });
        self.sources.push((visual.downgrade(), model.downgrade()));
    }
}

/// The layers of a material, a group flattened in order.
fn material_layers(material: Option<&Ref<Material>>) -> Vec<Scene3DMaterialLayer> {
    let mut layers = Vec::new();
    if let Some(material) = material {
        collect_layers(material, &mut layers, 0);
    }
    layers
}

fn collect_layers(material: &Ref<Material>, layers: &mut Vec<Scene3DMaterialLayer>, depth: usize) {
    if depth > MAX_DEPTH {
        return;
    }
    if let Some(group) = material.cast::<MaterialGroup>() {
        for child in group.children().iter() {
            collect_layers(&child, layers, depth + 1);
        }
    } else if let Some(diffuse) = material.cast::<DiffuseMaterial>() {
        if let Some(brush) = diffuse.brush() {
            layers.push(Scene3DMaterialLayer {
                kind: Scene3DMaterialKind::Diffuse,
                color: diffuse.color(),
                ambient_color: diffuse.ambient_color(),
                specular_power: 0.0,
                brush: shared_brush_of(&*brush),
            });
        }
    } else if let Some(specular) = material.cast::<SpecularMaterial>() {
        if let Some(brush) = specular.brush() {
            layers.push(Scene3DMaterialLayer {
                kind: Scene3DMaterialKind::Specular,
                color: specular.color(),
                ambient_color: WHITE,
                specular_power: specular.specular_power() as f32,
                brush: shared_brush_of(&*brush),
            });
        }
    } else if let Some(emissive) = material.cast::<EmissiveMaterial>() {
        if let Some(brush) = emissive.brush() {
            layers.push(Scene3DMaterialLayer {
                kind: Scene3DMaterialKind::Emissive,
                color: emissive.color(),
                ambient_color: WHITE,
                specular_power: 0.0,
                brush: shared_brush_of(&*brush),
            });
        }
    }
}

/// A light in world space; `None` for a class of light the snapshot does not know.
fn compile_light(light: &Ref<Light>, world: &Matrix3D) -> Option<Scene3DLight> {
    let color = light.color();
    if light.is::<AmbientLight>() {
        return Some(Scene3DLight::Ambient { color });
    }
    if let Some(directional) = light.cast::<DirectionalLight>() {
        return Some(Scene3DLight::Directional { color, direction: narrow_direction(world, directional.direction()) });
    }
    if let Some(spot) = light.cast::<SpotLight>() {
        return Some(Scene3DLight::Spot {
            color,
            position: narrow_point(world.transform(spot.position())),
            direction: narrow_direction(world, spot.direction()),
            range: spot.range() as f32,
            attenuation: [
                spot.constant_attenuation() as f32,
                spot.linear_attenuation() as f32,
                spot.quadratic_attenuation() as f32,
            ],
            inner_cone_angle: spot.inner_cone_angle() as f32,
            outer_cone_angle: spot.outer_cone_angle() as f32,
        });
    }
    if let Some(point) = light.cast::<PointLight>() {
        return Some(Scene3DLight::Point {
            color,
            position: narrow_point(world.transform(point.position())),
            range: point.range() as f32,
            attenuation: [
                point.constant_attenuation() as f32,
                point.linear_attenuation() as f32,
                point.quadratic_attenuation() as f32,
            ],
        });
    }
    None
}

/// The camera of a snapshot for the viewport; `None` when its matrices are not finite (a
/// transform without an inverse): nothing is drawn, as in WPF.
fn compile_camera(camera: &Ref<Camera>, viewport: Rect, scene_bounds: Rect3D) -> Option<Scene3DCamera> {
    let aspect_ratio =
        if viewport.width > 0.0 && viewport.height > 0.0 { viewport.width / viewport.height } else { 1.0 };
    let view = camera.get_view_matrix();
    if !is_finite(&view) {
        return None;
    }
    let (kind, near, far) = match camera.cast::<ProjectionCamera>() {
        Some(projection_camera) => {
            let perspective = camera.is::<PerspectiveCamera>();
            let (near, far) = fitted_planes(
                &view,
                scene_bounds,
                projection_camera.near_plane_distance(),
                projection_camera.far_plane_distance(),
                perspective,
            );
            let kind = if perspective { Scene3DCameraKind::Perspective } else { Scene3DCameraKind::Orthographic };
            (kind, near, far)
        }
        None => (Scene3DCameraKind::Matrix, 0.0, f64::INFINITY),
    };
    let projection = camera.get_projection_matrix(aspect_ratio, near, far);
    if !is_finite(&projection) {
        return None;
    }
    let position = camera.world_position().unwrap_or_default();
    Some(Scene3DCamera {
        kind,
        view: narrow_matrix(&view),
        projection: narrow_matrix(&projection),
        position: narrow_point(position),
        near_plane_distance: near as f32,
        far_plane_distance: far as f32,
    })
}

/// The near and far planes of a projection camera for a scene: with an infinite far plane
/// (WPF's default), the planes are fitted to the depths of the corners of the bounds of the scene
/// in the coordinates of the camera, widened by a small margin, the near plane never closer
/// than the camera's `NearPlaneDistance`. A finite far plane, an empty scene or a scene behind
/// the near plane keep the camera's planes (the infinite far plane then gives WPF's
/// infinite-far projection).
pub(crate) fn fitted_planes(view: &Matrix3D, bounds: Rect3D, near: f64, far: f64, perspective: bool) -> (f64, f64) {
    if far != f64::INFINITY || bounds.is_empty() {
        return (near, far);
    }
    let (x0, y0, z0) = (bounds.x(), bounds.y(), bounds.z());
    let (x1, y1, z1) = (x0 + bounds.size_x(), y0 + bounds.size_y(), z0 + bounds.size_z());
    let mut nearest = f64::INFINITY;
    let mut farthest = f64::NEG_INFINITY;
    for x in [x0, x1] {
        for y in [y0, y1] {
            for z in [z0, z1] {
                // The camera looks down its negative z axis: the depth is -z.
                let depth = -view.transform(Point3D::new(x, y, z)).z;
                nearest = nearest.min(depth);
                farthest = farthest.max(depth);
            }
        }
    }
    if !farthest.is_finite() || farthest <= near {
        return (near, far);
    }
    let margin = ((farthest - nearest) * 1e-3).max(farthest.abs() * 1e-6).max(1e-9);
    let fitted_far = farthest + margin;
    let mut fitted_near = near.max(nearest - margin);
    if perspective && fitted_near <= 0.0 {
        // A perspective projection needs a near plane in front of the camera.
        fitted_near = fitted_far * 1e-5;
    }
    (fitted_near, fitted_far)
}

fn is_finite(matrix: &Matrix3D) -> bool {
    [
        matrix.m11,
        matrix.m12,
        matrix.m13,
        matrix.m14,
        matrix.m21,
        matrix.m22,
        matrix.m23,
        matrix.m24,
        matrix.m31,
        matrix.m32,
        matrix.m33,
        matrix.m34,
        matrix.offset_x,
        matrix.offset_y,
        matrix.offset_z,
        matrix.m44,
    ]
    .iter()
    .all(|value| value.is_finite())
}

/// The matrix in single precision, element by element (both are row-vector matrices).
pub(crate) fn narrow_matrix(matrix: &Matrix3D) -> Matrix4x4 {
    Matrix4x4::new(
        matrix.m11 as f32,
        matrix.m12 as f32,
        matrix.m13 as f32,
        matrix.m14 as f32,
        matrix.m21 as f32,
        matrix.m22 as f32,
        matrix.m23 as f32,
        matrix.m24 as f32,
        matrix.m31 as f32,
        matrix.m32 as f32,
        matrix.m33 as f32,
        matrix.m34 as f32,
        matrix.offset_x as f32,
        matrix.offset_y as f32,
        matrix.offset_z as f32,
        matrix.m44 as f32,
    )
}

fn narrow_point(point: Point3D) -> Vector3 {
    Vector3::new(point.x as f32, point.y as f32, point.z as f32)
}

/// A direction transformed into the world, at unit length.
fn narrow_direction(world: &Matrix3D, direction: Vector3D) -> Vector3 {
    let mut direction = world.transform_vector(direction);
    if direction.length_squared() > 0.0 {
        direction.normalize();
    }
    Vector3::new(direction.x as f32, direction.y as f32, direction.z as f32)
}

/// The box in single precision; `None` when it is empty.
fn bounds_of(rect: Rect3D) -> Option<Scene3DBounds> {
    if rect.is_empty() {
        return None;
    }
    Some(Scene3DBounds {
        min: Vector3::new(rect.x() as f32, rect.y() as f32, rect.z() as f32),
        max: Vector3::new(
            (rect.x() + rect.size_x()) as f32,
            (rect.y() + rect.size_y()) as f32,
            (rect.z() + rect.size_z()) as f32,
        ),
    })
}
