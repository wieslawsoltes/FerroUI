//! Tests of the scene model: the snapshot of a tree, the generations, the fitted planes, the
//! bounds, the meshes and the control `Viewport3D`.

use crate::scene_compiler::fitted_planes;
use crate::*;
use ferroui_base::media::{
    BoxShadows, Brushes, Color, DrawingContext, Geometry, GlyphRun, IBrush, IDrawingContextCore, IEffect, IPen,
    RenderOptions, SolidColorBrush, TextOptions,
};
use ferroui_base::platform::IGeometryImpl;
use ferroui_base::rendering::scene_graph::ICustomDrawOperation;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Matrix, Point, Rect, Ref, RoundedRect, Size};
use std::rc::Rc;
use std::sync::Arc;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-5
}

/// The positions of a cube from -1 to 1 and its twelve triangles, counterclockwise from outside.
fn cube() -> Ref<MeshGeometry3D> {
    let mesh = MeshGeometry3D::new();
    mesh.set_positions(
        Point3DCollection::parse("-1,-1,1 1,-1,1 1,1,1 -1,1,1 -1,-1,-1 1,-1,-1 1,1,-1 -1,1,-1").unwrap(),
    );
    mesh.set_triangle_indices(
        Int32Collection::parse("0 1 2 0 2 3 1 5 6 1 6 2 5 4 7 5 7 6 4 0 3 4 3 7 3 2 6 3 6 7 4 5 1 4 1 0").unwrap(),
    );
    mesh
}

fn red() -> Ref<Material> {
    DiffuseMaterial::with_brush(Some(Brushes::red())).upcast()
}

fn model(mesh: &Ref<MeshGeometry3D>, transform: Option<Ref<Transform3D>>) -> Ref<GeometryModel3D> {
    let model = GeometryModel3D::with_geometry_and_material(mesh.clone().upcast(), red());
    model.set_transform(transform);
    model
}

fn camera_at(z: f64) -> Ref<PerspectiveCamera> {
    PerspectiveCamera::with_values(
        Point3D::new(0.0, 0.0, z),
        Vector3D::new(0.0, 0.0, -1.0),
        Vector3D::new(0.0, 1.0, 0.0),
        60.0,
    )
}

/// A scene of `content` seen by `camera` in a viewport of 200 by 100.
fn scene(content: Ref<Model3D>, camera: Ref<Camera>) -> Ref<Viewport3DVisual> {
    let visual = Viewport3DVisual::new();
    visual.set_camera(Some(camera));
    visual.set_viewport(Rect::new(0.0, 0.0, 200.0, 100.0));
    visual.children().add(ModelVisual3D::with_content(content).upcast());
    visual
}

// --- the snapshot ------------------------------------------------------------------------------

#[test]
fn nested_groups_compose_their_transforms_in_tree_order() {
    let mesh = cube();
    let first = model(&mesh, Some(TranslateTransform3D::with_offset_xyz(0.0, 1.0, 0.0).upcast()));
    let second = model(&mesh, None);
    let light = DirectionalLight::new();
    light.set_transform(Some(
        RotateTransform3D::with_rotation(AxisAngleRotation3D::with_axis_angle(Vector3D::new(0.0, 1.0, 0.0), 90.0).upcast())
            .upcast(),
    ));
    let inner = Model3DGroup::new();
    inner.set_transform(Some(ScaleTransform3D::with_scale_xyz(2.0, 2.0, 2.0).upcast()));
    inner.children().add(first.clone().upcast());
    let outer = Model3DGroup::new();
    outer.set_transform(Some(TranslateTransform3D::with_offset_xyz(1.0, 0.0, 0.0).upcast()));
    outer.children().add(inner.upcast());
    outer.children().add(light.upcast());
    outer.children().add(second.upcast());
    let visual = scene(outer.upcast(), camera_at(10.0).upcast());

    let snapshot = visual.snapshot();
    assert_eq!(snapshot.models.len(), 2);
    // The first model: its translation, then the scale of the inner group, then the translation
    // of the outer group (row vectors: the model's own transform comes first).
    let world = snapshot.models[0].world;
    assert_eq!((world.m11, world.m22, world.m33), (2.0, 2.0, 2.0));
    assert_eq!((world.m41, world.m42, world.m43), (1.0, 2.0, 0.0));
    // The second model is under the outer group only.
    let world = snapshot.models[1].world;
    assert_eq!((world.m11, world.m41), (1.0, 1.0));
    // Both draw the one mesh, shared.
    assert!(Arc::ptr_eq(&snapshot.models[0].mesh, &snapshot.models[1].mesh));
    assert_eq!(snapshot.models[0].mesh.triangle_count(), 12);
    assert_eq!(snapshot.models[0].front.len(), 1);
    assert!(snapshot.models[0].back.is_empty());
    assert_eq!(snapshot.models[0].front[0].kind, Scene3DMaterialKind::Diffuse);
    // The light, rotated a quarter turn about y: (0, 0, -1) becomes (-1, 0, 0).
    assert_eq!(snapshot.lights.len(), 1);
    let Scene3DLight::Directional { direction, color } = snapshot.lights[0] else {
        panic!("a directional light");
    };
    assert_eq!(color, Color::from_rgb(255, 255, 255));
    assert!(close(direction.x as f64, -1.0) && close(direction.y as f64, 0.0) && close(direction.z as f64, 0.0));
    // The bounds of the scene: the first cube spans x -1..3, y 0..4 and z -2..2 after the
    // transforms, the second x 0..2, y -1..1 and z -1..1.
    let bounds = snapshot.bounds.unwrap();
    assert_eq!((bounds.min.x, bounds.min.y, bounds.min.z), (-1.0, -1.0, -2.0));
    assert_eq!((bounds.max.x, bounds.max.y, bounds.max.z), (3.0, 4.0, 2.0));
    // The sources of the models, for the hit testing.
    let (_, source) = visual.model_source(0).unwrap();
    assert!(source.ptr_eq(&first));
}

#[test]
fn a_material_group_is_flattened_into_layers_and_a_material_without_a_brush_is_none() {
    let group = MaterialGroup::new();
    group.children().add(red());
    group.children().add(SpecularMaterial::with_brush(Some(Brushes::white()), 20.0).upcast());
    group.children().add(DiffuseMaterial::new().upcast());
    group.children().add(EmissiveMaterial::with_brush(Some(Brushes::blue())).upcast());
    let model = GeometryModel3D::with_geometry_and_material(cube().upcast(), group.upcast());
    model.set_back_material(Some(DiffuseMaterial::new().upcast()));
    let snapshot = scene(model.upcast(), camera_at(5.0).upcast()).snapshot();
    let kinds: Vec<_> = snapshot.models[0].front.iter().map(|layer| layer.kind).collect();
    assert_eq!(kinds, [Scene3DMaterialKind::Diffuse, Scene3DMaterialKind::Specular, Scene3DMaterialKind::Emissive]);
    assert_eq!(snapshot.models[0].front[1].specular_power, 20.0);
    assert!(snapshot.models[0].front.iter().all(|layer| layer.brush.is_some()));
    assert!(snapshot.models[0].back.is_empty());

    // A model whose materials have no brush draws nothing.
    let unpainted = GeometryModel3D::with_geometry_and_material(cube().upcast(), DiffuseMaterial::new().upcast());
    let snapshot = scene(unpainted.upcast(), camera_at(5.0).upcast()).snapshot();
    assert!(snapshot.models.is_empty());
    assert!(!snapshot.draws_anything());
}

#[test]
fn the_lights_are_in_world_space() {
    let point = PointLight::with_color_and_position(Color::from_rgb(10, 20, 30), Point3D::new(1.0, 2.0, 3.0));
    point.set_transform(Some(TranslateTransform3D::with_offset_xyz(1.0, 0.0, 0.0).upcast()));
    point.set_range(50.0);
    point.set_quadratic_attenuation(0.5);
    let spot = SpotLight::with_values(
        Color::from_rgb(255, 0, 0),
        Point3D::new(0.0, 0.0, 0.0),
        Vector3D::new(0.0, 0.0, -2.0),
        45.0,
        30.0,
    );
    let group = Model3DGroup::new();
    group.children().add(AmbientLight::with_color(Color::from_rgb(64, 64, 64)).upcast());
    group.children().add(point.upcast());
    group.children().add(spot.upcast());
    group.children().add(model(&cube(), None).upcast());
    let snapshot = scene(group.upcast(), camera_at(5.0).upcast()).snapshot();
    assert_eq!(snapshot.lights.len(), 3);
    assert_eq!(snapshot.lights[0], Scene3DLight::Ambient { color: Color::from_rgb(64, 64, 64) });
    let Scene3DLight::Point { position, range, attenuation, .. } = snapshot.lights[1] else { panic!("a point light") };
    assert_eq!((position.x, position.y, position.z), (2.0, 2.0, 3.0));
    assert_eq!(range, 50.0);
    assert_eq!(attenuation, [1.0, 0.0, 0.5]);
    let Scene3DLight::Spot { direction, inner_cone_angle, outer_cone_angle, .. } = snapshot.lights[2] else {
        panic!("a spot light")
    };
    assert_eq!((direction.x, direction.y, direction.z), (0.0, 0.0, -1.0), "the direction is of unit length");
    assert_eq!((inner_cone_angle, outer_cone_angle), (30.0, 45.0));
}

#[test]
fn the_camera_follows_wpf() {
    let camera = PerspectiveCamera::new();
    assert_eq!(camera.position(), Point3D::new(0.0, 0.0, 0.0));
    assert_eq!(camera.look_direction(), Vector3D::new(0.0, 0.0, -1.0));
    assert_eq!(camera.up_direction(), Vector3D::new(0.0, 1.0, 0.0));
    assert_eq!(camera.near_plane_distance(), 0.125);
    assert_eq!(camera.far_plane_distance(), f64::INFINITY);
    assert_eq!(camera.field_of_view(), 45.0);
    assert_eq!(OrthographicCamera::new().width(), 2.0);

    // The view matrix of a camera at (0, 0, 5) looking down -z moves the world back by 5.
    let camera = camera_at(5.0);
    let view = camera.get_view_matrix();
    assert!(close(view.transform(Point3D::new(0.0, 0.0, 0.0)).z, -5.0));
    // The field of view is horizontal: the horizontal scale is 1 / tan(fov / 2), the vertical
    // one that times the aspect ratio.
    let projection = camera.get_projection_matrix(2.0, 1.0, 10.0);
    let horizontal = 1.0 / (30.0f64.to_radians()).tan();
    assert!(close(projection.m11, horizontal));
    assert!(close(projection.m22, 2.0 * horizontal));
    assert!(close(projection.m34, -1.0));
    // The near plane maps to depth 0, the far plane to depth 1.
    let near = Point4D::new(0.0, 0.0, -1.0, 1.0) * projection;
    let far = Point4D::new(0.0, 0.0, -10.0, 1.0) * projection;
    assert!(close(near.z / near.w, 0.0) && close(far.z / far.w, 1.0));

    // A transform of the camera moves the camera, so the world moves the other way.
    camera.set_transform(Some(TranslateTransform3D::with_offset_xyz(0.0, 0.0, 5.0).upcast()));
    assert!(close(camera.get_view_matrix().transform(Point3D::new(0.0, 0.0, 0.0)).z, -10.0));
    let position = camera.world_position().unwrap();
    assert!(close(position.z, 10.0));
}

// --- the generations ---------------------------------------------------------------------------

#[test]
fn a_camera_change_bumps_only_the_camera_generation() {
    let camera = camera_at(5.0);
    let visual = scene(model(&cube(), None).upcast(), camera.clone().upcast());
    let first = visual.snapshot();
    assert!(Arc::ptr_eq(&first, &visual.snapshot()), "an unchanged scene keeps its snapshot");

    camera.set_position(Point3D::new(0.0, 0.0, 6.0));
    let second = visual.snapshot();
    assert!(second.camera_generation > first.camera_generation);
    assert_eq!(second.structure_generation, first.structure_generation);
    assert_eq!(second.record_generation, first.record_generation);
    // The mesh is the one of the first snapshot: an executor uploads nothing.
    assert!(Arc::ptr_eq(&first.models[0].mesh, &second.models[0].mesh));

    // The transform of the camera and the viewport are the camera too.
    let transform = TranslateTransform3D::new();
    camera.set_transform(Some(transform.clone().upcast()));
    let third = visual.snapshot();
    transform.set_offset_x(1.0);
    visual.set_viewport(Rect::new(0.0, 0.0, 100.0, 100.0));
    let fourth = visual.snapshot();
    assert!(fourth.camera_generation >= third.camera_generation + 2);
    assert_eq!(fourth.structure_generation, first.structure_generation);
    assert_eq!(fourth.record_generation, first.record_generation);
}

#[test]
fn a_mesh_edit_bumps_the_structure_and_a_new_version_of_the_mesh() {
    let mesh = cube();
    let visual = scene(model(&mesh, None).upcast(), camera_at(5.0).upcast());
    let first = visual.snapshot();
    let version = mesh.version();

    mesh.positions().add(Point3D::new(0.0, 0.0, 2.0));
    assert!(mesh.version() > version);
    let second = visual.snapshot();
    assert!(second.structure_generation > first.structure_generation);
    assert_eq!(second.record_generation, first.record_generation);
    assert_eq!(second.camera_generation, first.camera_generation);
    assert_eq!(second.models[0].mesh.id, first.models[0].mesh.id);
    assert!(second.models[0].mesh.version > first.models[0].mesh.version);
    assert_eq!(second.models[0].mesh.positions.len(), 9);

    // A new collection is a new version too, as is a model added to the scene.
    mesh.set_triangle_indices(Int32Collection::parse("0 1 2").unwrap());
    let third = visual.snapshot();
    assert!(third.structure_generation > second.structure_generation);
    assert_eq!(third.models[0].mesh.triangle_count(), 1);
}

#[test]
fn a_light_or_a_transform_or_a_colour_bumps_the_records() {
    let light = DirectionalLight::new();
    let transform = TranslateTransform3D::new();
    let brush = SolidColorBrush::with_color(Color::from_rgb(255, 0, 0));
    let material = DiffuseMaterial::with_brush(Some(Rc::<dyn IBrush>::from(&brush)));
    let model = GeometryModel3D::with_geometry_and_material(cube().upcast(), material.clone().upcast());
    model.set_transform(Some(transform.clone().upcast()));
    let group = Model3DGroup::new();
    group.children().add(light.clone().upcast());
    group.children().add(model.upcast());
    let visual = scene(group.upcast(), camera_at(5.0).upcast());

    let mut previous = visual.snapshot();
    let changes: [Box<dyn Fn()>; 4] = [
        Box::new(move || light.set_color(Color::from_rgb(0, 0, 255))),
        Box::new(move || transform.set_offset_y(2.0)),
        {
            let material = material.clone();
            Box::new(move || material.set_ambient_color(Color::from_rgb(0, 0, 0)))
        },
        Box::new(move || brush.set_color(Color::from_rgb(0, 255, 0))),
    ];
    for change in changes {
        change();
        let next = visual.snapshot();
        assert!(next.record_generation > previous.record_generation);
        assert_eq!(next.structure_generation, previous.structure_generation);
        assert_eq!(next.camera_generation, previous.camera_generation);
        previous = next;
    }
    // A new brush is a new layer: the structure.
    material.set_brush(Some(Brushes::blue()));
    assert!(visual.snapshot().structure_generation > previous.structure_generation);
}

#[test]
fn removed_objects_are_no_longer_followed() {
    let light = DirectionalLight::new();
    let group = Model3DGroup::new();
    group.children().add(light.clone().upcast());
    group.children().add(model(&cube(), None).upcast());
    let visual = scene(group.clone().upcast(), camera_at(5.0).upcast());
    let structure = visual.structure_generation();
    group.children().remove_at(0);
    assert!(visual.structure_generation() > structure);
    let record = visual.record_generation();
    light.set_color(Color::from_rgb(1, 2, 3));
    assert_eq!(visual.record_generation(), record, "a removed light is not part of the scene");
    assert!(visual.snapshot().lights.is_empty());
}

// --- the planes --------------------------------------------------------------------------------

#[test]
fn an_infinite_far_plane_is_fitted_to_the_scene() {
    let snapshot = scene(model(&cube(), None).upcast(), camera_at(5.0).upcast()).snapshot();
    let camera = snapshot.camera.unwrap();
    // The cube is between 4 and 6 in front of the camera.
    assert!(camera.near_plane_distance <= 4.0 && camera.near_plane_distance > 3.9, "{}", camera.near_plane_distance);
    assert!(camera.far_plane_distance >= 6.0 && camera.far_plane_distance < 6.1, "{}", camera.far_plane_distance);
    assert_eq!(camera.kind, Scene3DCameraKind::Perspective);

    // A finite far plane is the camera's.
    let camera = camera_at(5.0);
    camera.set_far_plane_distance(100.0);
    let fitted = scene(model(&cube(), None).upcast(), camera.upcast()).snapshot().camera.unwrap();
    assert_eq!((fitted.near_plane_distance, fitted.far_plane_distance), (0.125, 100.0));

    // A scene with nothing to draw keeps the infinite plane: WPF's infinite-far projection.
    let empty = Viewport3DVisual::new();
    empty.set_camera(Some(camera_at(5.0).upcast()));
    let camera = empty.snapshot().camera.unwrap();
    assert_eq!(camera.far_plane_distance, f32::INFINITY);
    assert_eq!((camera.projection.m33, camera.projection.m34, camera.projection.m43), (-1.0, -1.0, -0.125));
}

#[test]
fn the_planes_keep_the_near_plane_of_the_camera_and_a_scene_behind_it() {
    let view = camera_at(5.0).get_view_matrix();
    let cube = Rect3D::new(-1.0, -1.0, -1.0, 2.0, 2.0, 2.0).unwrap();
    // The camera inside the cube: the near plane stays the camera's.
    let inside = camera_at(0.0).get_view_matrix();
    let (near, far) = fitted_planes(&inside, cube, 0.125, f64::INFINITY, true);
    assert_eq!(near, 0.125);
    assert!(far >= 1.0);
    // A scene behind the camera: nothing to fit.
    let behind = Rect3D::new(-1.0, -1.0, 10.0, 2.0, 2.0, 2.0).unwrap();
    assert_eq!(fitted_planes(&view, behind, 0.125, f64::INFINITY, true), (0.125, f64::INFINITY));
    // An orthographic camera may look at a scene around it.
    let (near, far) = fitted_planes(&inside, cube, -10.0, f64::INFINITY, false);
    assert!(near < -0.9 && near >= -1.0 - 0.01, "{near}");
    assert!(far > 0.9 && far <= 1.01, "{far}");
}

// --- the bounds --------------------------------------------------------------------------------

#[test]
fn bounds_follow_the_geometry_and_the_transforms() {
    let mesh = cube();
    assert_eq!(mesh.bounds(), Rect3D::new(-1.0, -1.0, -1.0, 2.0, 2.0, 2.0).unwrap());
    assert!(MeshGeometry3D::new().bounds().is_empty());

    let moved = model(&mesh, Some(TranslateTransform3D::with_offset_xyz(1.0, 0.0, 0.0).upcast()));
    assert_eq!(moved.bounds(), Rect3D::new(0.0, -1.0, -1.0, 2.0, 2.0, 2.0).unwrap());

    let group = Model3DGroup::new();
    group.children().add(moved.upcast());
    group.children().add(model(&mesh, Some(ScaleTransform3D::with_scale_xyz(3.0, 1.0, 1.0).upcast())).upcast());
    group.children().add(AmbientLight::new().upcast());
    assert_eq!(group.bounds(), Rect3D::new(-3.0, -1.0, -1.0, 6.0, 2.0, 2.0).unwrap());
    assert!(AmbientLight::new().bounds().is_empty(), "a light has no bounds");

    // A rotation of a quarter turn about z: the box of the rotated corners.
    let rotate = RotateTransform3D::with_rotation(
        AxisAngleRotation3D::with_axis_angle(Vector3D::new(0.0, 0.0, 1.0), 90.0).upcast(),
    );
    let bounds = rotate.transform_bounds(Rect3D::new(0.0, 0.0, 0.0, 2.0, 1.0, 1.0).unwrap());
    assert!(close(bounds.x(), -1.0) && close(bounds.size_x(), 1.0) && close(bounds.size_y(), 2.0));

    let visual = ModelVisual3D::with_content(group.upcast());
    visual.set_transform(Some(TranslateTransform3D::with_offset_xyz(0.0, 0.0, 10.0).upcast()));
    assert_eq!(visual.descendant_bounds(), Rect3D::new(-3.0, -1.0, 9.0, 6.0, 2.0, 2.0).unwrap());
}

#[test]
fn transforms_compose_invert_and_transform_points() {
    let group = Transform3DGroup::new();
    group.children().add(ScaleTransform3D::with_scale_xyz(2.0, 2.0, 2.0).upcast());
    group.children().add(TranslateTransform3D::with_offset_xyz(1.0, 0.0, 0.0).upcast());
    assert_eq!(group.transform(Point3D::new(1.0, 1.0, 1.0)), Point3D::new(3.0, 2.0, 2.0));
    let inverse = group.inverse().unwrap();
    let back = inverse.transform(Point3D::new(3.0, 2.0, 2.0));
    assert!(close(back.x, 1.0) && close(back.y, 1.0) && close(back.z, 1.0));
    assert!(group.is_affine());
    // The cached product follows a change of a child.
    let child = group.children().get(1);
    child.cast::<TranslateTransform3D>().unwrap().set_offset_x(2.0);
    assert_eq!(group.transform(Point3D::new(0.0, 0.0, 0.0)), Point3D::new(2.0, 0.0, 0.0));

    let singular = ScaleTransform3D::with_scale_xyz(0.0, 1.0, 1.0);
    assert!(singular.inverse().is_none());
    // A rotation about a center moves the center nowhere.
    let rotate = RotateTransform3D::with_rotation_and_center(
        QuaternionRotation3D::with_quaternion(Quaternion::from_axis_angle(Vector3D::new(0.0, 0.0, 1.0), 180.0).unwrap())
            .upcast(),
        Point3D::new(1.0, 1.0, 0.0),
    );
    let center = rotate.transform(Point3D::new(1.0, 1.0, 0.0));
    assert!(close(center.x, 1.0) && close(center.y, 1.0));
    let corner = rotate.transform(Point3D::new(0.0, 0.0, 0.0));
    assert!(close(corner.x, 2.0) && close(corner.y, 2.0));
    // A zero axis is no rotation.
    let zero = AxisAngleRotation3D::with_axis_angle(Vector3D::new(0.0, 0.0, 0.0), 45.0);
    assert!(zero.internal_quaternion().is_identity());
}

// --- the mesh ----------------------------------------------------------------------------------

#[test]
fn a_mesh_without_indices_takes_its_positions_three_at_a_time_and_generates_normals() {
    let mesh = MeshGeometry3D::new();
    mesh.set_positions(Point3DCollection::parse("0,0,0 1,0,0 0,1,0 5,5,5").unwrap());
    let data = mesh.mesh_data();
    assert_eq!(data.indices, [0, 1, 2], "the incomplete last triangle is left out");
    assert!(data.normals_generated);
    assert_eq!(data.normals.len(), 4);
    assert_eq!((data.normals[0].x, data.normals[0].y, data.normals[0].z), (0.0, 0.0, 1.0));
    assert!(data.texture_coordinates.is_empty());
    assert!(Arc::ptr_eq(&data, &mesh.mesh_data()), "made once per version");

    // Indices out of range leave their triangle out; given normals and coordinates are kept.
    mesh.set_triangle_indices(Int32Collection::parse("0 1 2 0 2 9 2 1 0").unwrap());
    mesh.set_normals(Vector3DCollection::parse("0,0,2 0,0,1 0,0,1 0,0,1").unwrap());
    mesh.set_texture_coordinates(ferroui_base::media::Points::from_items([Point::new(0.0, 1.0), Point::new(1.0, 1.0)]));
    let data = mesh.mesh_data();
    assert_eq!(data.indices, [0, 1, 2, 2, 1, 0]);
    assert!(!data.normals_generated);
    assert_eq!(data.normals[0].z, 1.0, "the normals are of unit length");
    assert_eq!(data.texture_coordinates.len(), 4, "one per position");
    assert_eq!((data.texture_coordinates[1].x, data.texture_coordinates[3].y), (1.0, 0.0));
    let bounds = data.bounds.unwrap();
    assert_eq!((bounds.max.x, bounds.max.y, bounds.max.z), (5.0, 5.0, 5.0));
}

// --- the control -------------------------------------------------------------------------------

/// A drawing context core that keeps the scenes it is asked to draw.
#[derive(Default)]
struct SceneRecorder {
    scenes: Vec<(Arc<Scene3DSnapshot>, Rect)>,
}

impl IDrawingContextCore for SceneRecorder {
    fn draw_line_core(&mut self, _pen: &Rc<dyn IPen>, _p1: Point, _p2: Point) {}
    fn draw_geometry_impl_core(
        &mut self,
        _brush: Option<&Rc<dyn IBrush>>,
        _pen: Option<&Rc<dyn IPen>>,
        _geometry: &Arc<dyn IGeometryImpl>,
    ) {
    }
    fn draw_rectangle_core(
        &mut self,
        _brush: Option<&Rc<dyn IBrush>>,
        _pen: Option<&Rc<dyn IPen>>,
        _rrect: RoundedRect,
        _box_shadows: &BoxShadows,
    ) {
    }
    fn draw_ellipse_core(&mut self, _brush: Option<&Rc<dyn IBrush>>, _pen: Option<&Rc<dyn IPen>>, _rect: Rect) {}
    fn draw_bitmap(
        &mut self,
        _source: &ferroui_base::utilities::RefCounted<ferroui_base::platform::SharedBitmapImpl>,
        _opacity: f64,
        _source_rect: Rect,
        _dest_rect: Rect,
    ) {
    }
    fn custom(&mut self, _custom: &Arc<dyn ICustomDrawOperation>) {}
    fn draw_scene_3d_core(&mut self, scene: &Arc<Scene3DSnapshot>, bounds: Rect) {
        self.scenes.push((scene.clone(), bounds));
    }
    fn draw_glyph_run(&mut self, _foreground: Option<&Rc<dyn IBrush>>, _glyph_run: &Rc<GlyphRun>) {}
    fn push_clip_core(&mut self, _rect: Rect) {}
    fn push_rounded_clip_core(&mut self, _rect: RoundedRect) {}
    fn push_geometry_clip_core(&mut self, _clip: &Ref<Geometry>) {}
    fn push_opacity_core(&mut self, _opacity: f64) {}
    fn push_opacity_mask_core(&mut self, _mask: &Rc<dyn IBrush>, _bounds: Rect) {}
    fn push_transform_core(&mut self, _matrix: Matrix) {}
    fn push_render_options_core(&mut self, _render_options: RenderOptions) {}
    fn push_text_options_core(&mut self, _text_options: TextOptions) {}
    fn push_effect_core(&mut self, _effect: &Rc<dyn IEffect>, _bounds: Rect) {}
    fn pop_clip_core(&mut self) {}
    fn pop_geometry_clip_core(&mut self) {}
    fn pop_opacity_core(&mut self) {}
    fn pop_opacity_mask_core(&mut self) {}
    fn pop_transform_core(&mut self) {}
    fn pop_render_options_core(&mut self) {}
    fn pop_text_options_core(&mut self) {}
    fn pop_effect_core(&mut self) {}
    fn dispose_core(&mut self) {}
}

fn render(viewport: &Viewport3D) -> Vec<(Arc<Scene3DSnapshot>, Rect)> {
    let mut recorder = SceneRecorder::default();
    {
        let mut context = DrawingContext::new(&mut recorder);
        viewport.render(&mut context);
    }
    recorder.scenes
}

#[test]
fn a_viewport_draws_its_scene_over_its_bounds_and_clips_to_them() {
    let _dispatcher = Dispatcher::unit_test_scope();
    register_types();
    let viewport = Viewport3D::new();
    assert!(viewport.clip_to_bounds(), "ClipToBounds is true by default, as in WPF");
    // Nothing to draw without a camera.
    viewport.children().add(ModelVisual3D::with_content(model(&cube(), None).upcast()).upcast());
    viewport.measure(Size::new(300.0, 150.0));
    viewport.arrange(Rect::new(0.0, 0.0, 300.0, 150.0));
    assert!(render(&viewport).is_empty());

    viewport.set_camera(Some(camera_at(5.0).upcast()));
    let scenes = render(&viewport);
    assert_eq!(scenes.len(), 1);
    let (scene, bounds) = &scenes[0];
    assert_eq!(*bounds, Rect::new(0.0, 0.0, 300.0, 150.0));
    assert_eq!(scene.viewport, Rect::new(0.0, 0.0, 300.0, 150.0));
    assert_eq!(scene.models.len(), 1);
    // The aspect ratio of the arranged size: the vertical scale is twice the horizontal one.
    let camera = scene.camera.unwrap();
    assert!(close(camera.projection.m22 as f64, 2.0 * camera.projection.m11 as f64));
    // The same scene while nothing changes.
    assert!(Arc::ptr_eq(scene, &render(&viewport)[0].0));
    assert_eq!(viewport.viewport_3d_visual().camera_generation(), scene.camera_generation);
    assert_eq!(viewport.desired_size(), Size::new(0.0, 0.0), "a viewport has no size of its own");
}
