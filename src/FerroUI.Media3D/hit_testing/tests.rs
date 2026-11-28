//! Tests of the 3D hit testing: known triangles with their weights and distances, the sides of
//! the faces, transforms, the order of the hits, misses, degenerate triangles, the filters, the
//! rays of the cameras, the hierarchy of a mesh, and picks held to the pixels the CPU reference
//! rasterizer draws.

use super::hierarchies_built;
use crate::*;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelSize, Point, Rect, Ref};
use ferroui_render3d::cpu::{self, CpuRenderOptions};
use std::rc::Rc;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn assert_point(actual: Point3D, expected: Point3D) {
    assert_point_within(actual, expected, 1e-9);
}

/// For the rays of a viewport, made of the single-precision matrices of the snapshot.
fn assert_point_within(actual: Point3D, expected: Point3D, tolerance: f64) {
    let near = |a: f64, b: f64| (a - b).abs() < tolerance;
    assert!(
        near(actual.x, expected.x) && near(actual.y, expected.y) && near(actual.z, expected.z),
        "{actual:?} is not {expected:?}"
    );
}

fn material(brush: Rc<dyn IBrush>) -> Ref<Material> {
    DiffuseMaterial::with_brush(Some(brush)).upcast()
}

fn mesh(positions: &str, indices: &str) -> Ref<MeshGeometry3D> {
    let mesh = MeshGeometry3D::new();
    mesh.set_positions(Point3DCollection::parse(positions).unwrap());
    mesh.set_triangle_indices(Int32Collection::parse(indices).unwrap());
    mesh
}

/// The triangle (0, 0, 0), (1, 0, 0), (0, 1, 0): counterclockwise seen from +z.
fn triangle() -> Ref<MeshGeometry3D> {
    mesh("0,0,0 1,0,0 0,1,0", "0 1 2")
}

/// A square from (x0, y0) to (x1, y1) in the plane `z`, counterclockwise seen from +z.
fn square(x0: f64, y0: f64, x1: f64, y1: f64, z: f64) -> Ref<MeshGeometry3D> {
    mesh(&format!("{x0},{y0},{z} {x1},{y0},{z} {x1},{y1},{z} {x0},{y1},{z}"), "0 1 2 0 2 3")
}

/// The cube from -1 to 1, counterclockwise from outside.
fn cube() -> Ref<MeshGeometry3D> {
    mesh(
        "-1,-1,1 1,-1,1 1,1,1 -1,1,1 -1,-1,-1 1,-1,-1 1,1,-1 -1,1,-1",
        "0 1 2 0 2 3 1 5 6 1 6 2 5 4 7 5 7 6 4 0 3 4 3 7 3 2 6 3 6 7 4 5 1 4 1 0",
    )
}

fn front_model(mesh: &Ref<MeshGeometry3D>) -> Ref<GeometryModel3D> {
    GeometryModel3D::with_geometry_and_material(mesh.clone().upcast(), material(Brushes::red()))
}

fn visual_of(model: &Ref<GeometryModel3D>) -> Ref<ModelVisual3D> {
    ModelVisual3D::with_content(model.clone().upcast())
}

/// Every hit of the ray under `visual`, nearest first.
fn hits(visual: &Visual3D, origin: Point3D, direction: Vector3D) -> Vec<RayMeshGeometry3DHitTestResult> {
    let mut results = Vec::new();
    VisualTreeHelper::hit_test_ray(
        visual,
        None,
        &mut |result| {
            results.push(result.clone());
            HitTestResultBehavior::Continue
        },
        &RayHitTestParameters::new(origin, direction),
    );
    results
}

/// The distances of the hits.
fn distances(results: &[RayMeshGeometry3DHitTestResult]) -> Vec<f64> {
    results.iter().map(|result| result.distance_to_ray_origin()).collect()
}

fn down_from(x: f64, y: f64, z: f64) -> (Point3D, Vector3D) {
    (Point3D::new(x, y, z), Vector3D::new(0.0, 0.0, -1.0))
}

// --- rays and triangles ------------------------------------------------------------------------

#[test]
fn a_ray_hits_a_known_triangle_with_its_weights_and_distance() {
    let mesh = triangle();
    let model = front_model(&mesh);
    let visual = visual_of(&model);
    let (origin, direction) = down_from(0.25, 0.25, 5.0);
    let results = hits(&visual, origin, direction);
    assert_eq!(results.len(), 1);
    let hit = &results[0];
    assert!(hit.visual_hit().ptr_eq(&visual));
    assert!(hit.model_hit().ptr_eq(&model));
    assert!(hit.mesh_hit().ptr_eq(&mesh));
    assert_point(hit.point_hit(), Point3D::new(0.25, 0.25, 0.0));
    assert!(close(hit.distance_to_ray_origin(), 5.0));
    assert_eq!((hit.vertex_index_1(), hit.vertex_index_2(), hit.vertex_index_3()), (0, 1, 2));
    assert!(
        close(hit.vertex_weight_1(), 0.5) && close(hit.vertex_weight_2(), 0.25) && close(hit.vertex_weight_3(), 0.25)
    );
    assert!(hit.is_front_face());
    // The base result types report the same.
    let ray_result: &dyn RayHitTestResult = hit;
    assert!(ray_result.model_hit().ptr_eq(&model));
    assert!(ray_result.visual_hit().ptr_eq(&visual));
    assert!(close(ray_result.distance_to_ray_origin(), 5.0));

    // The weights sum to one and give the point; a longer direction gives the same distance.
    let results = hits(&visual, Point3D::new(0.1, 0.7, 2.0), Vector3D::new(0.0, 0.0, -4.0));
    let hit = &results[0];
    assert!(close(hit.vertex_weight_1() + hit.vertex_weight_2() + hit.vertex_weight_3(), 1.0));
    assert!(close(hit.vertex_weight_2(), 0.1) && close(hit.vertex_weight_3(), 0.7));
    assert!(close(hit.distance_to_ray_origin(), 2.0));
    // A slanted ray: from (0.25, 0.25, 1) along (0.25, 0.25, -1) to (0.5, 0.5, 0), on the edge
    // from (1, 0, 0) to (0, 1, 0).
    let results = hits(&visual, Point3D::new(0.25, 0.25, 1.0), Vector3D::new(0.25, 0.25, -1.0));
    assert_eq!(results.len(), 1);
    assert_point(results[0].point_hit(), Point3D::new(0.5, 0.5, 0.0));
    assert!(close(results[0].vertex_weight_1(), 0.0));
    assert!(close(results[0].distance_to_ray_origin(), (0.25f64 * 0.25 * 2.0 + 1.0).sqrt()));
}

#[test]
fn a_face_is_hit_only_on_a_side_with_a_material() {
    let mesh = triangle();
    let model = GeometryModel3D::new();
    model.set_geometry(Some(mesh.clone().upcast()));
    let visual = visual_of(&model);
    let (front_origin, down) = down_from(0.25, 0.25, 5.0);
    let back_origin = Point3D::new(0.25, 0.25, -5.0);
    let up = Vector3D::new(0.0, 0.0, 1.0);

    // No material: nothing is hit.
    assert!(hits(&visual, front_origin, down).is_empty());
    assert!(hits(&visual, back_origin, up).is_empty());

    // A `Material`: the front only.
    model.set_material(Some(material(Brushes::red())));
    assert_eq!(hits(&visual, front_origin, down).len(), 1);
    assert!(hits(&visual, back_origin, up).is_empty());

    // A `BackMaterial` too: both, each with its side.
    model.set_back_material(Some(material(Brushes::blue())));
    assert!(hits(&visual, front_origin, down)[0].is_front_face());
    let back = hits(&visual, back_origin, up);
    assert_eq!(back.len(), 1);
    assert!(!back[0].is_front_face());
    assert_point(back[0].point_hit(), Point3D::new(0.25, 0.25, 0.0));

    // A `BackMaterial` alone: the back only.
    model.set_material(None);
    assert!(hits(&visual, front_origin, down).is_empty());
    assert_eq!(hits(&visual, back_origin, up).len(), 1);

    // A material without a brush still makes its side hit, as in WPF (it draws nothing).
    model.set_material(Some(DiffuseMaterial::new().upcast()));
    model.set_back_material(None);
    assert_eq!(hits(&visual, front_origin, down).len(), 1);
}

#[test]
fn nested_transforms_place_the_hit_and_a_mirror_turns_the_faces_over() {
    let mesh = triangle();
    let model = front_model(&mesh);
    // The model: moved by (1, 0, 0); its group: scaled by 2; its visual: moved by (0, 0, -3);
    // the parent visual: moved by (0, 1, 0). The triangle is then (2, 1, -3), (4, 1, -3),
    // (2, 3, -3) in the coordinates of the parent of the outer visual.
    model.set_transform(Some(TranslateTransform3D::with_offset_xyz(1.0, 0.0, 0.0).upcast()));
    let group = Model3DGroup::new();
    group.set_transform(Some(ScaleTransform3D::with_scale_xyz(2.0, 2.0, 2.0).upcast()));
    group.children().add(model.clone().upcast());
    let inner = ModelVisual3D::with_content(group.upcast());
    inner.set_transform(Some(TranslateTransform3D::with_offset_xyz(0.0, 0.0, -3.0).upcast()));
    let outer = ModelVisual3D::new();
    outer.set_transform(Some(TranslateTransform3D::with_offset_xyz(0.0, 1.0, 0.0).upcast()));
    outer.children().add(inner.clone().upcast());

    let (origin, direction) = down_from(2.5, 1.5, 5.0);
    let results = hits(&outer, origin, direction);
    assert_eq!(results.len(), 1);
    let hit = &results[0];
    assert!(hit.visual_hit().ptr_eq(&inner));
    assert!(hit.model_hit().ptr_eq(&model));
    // The point in the coordinates of the mesh: (2.5, 1.5) is (0.25, 0.25) of the triangle.
    assert_point(hit.point_hit(), Point3D::new(0.25, 0.25, 0.0));
    assert!(close(hit.vertex_weight_2(), 0.25) && close(hit.vertex_weight_3(), 0.25));
    // The distance in the coordinates of the ray.
    assert!(close(hit.distance_to_ray_origin(), 8.0));
    // Outside the transformed triangle: a miss.
    assert!(hits(&outer, Point3D::new(0.25, 0.25, 5.0), direction).is_empty());

    // A rotation of the outer visual by 90 degrees about y turns the triangle to face +x.
    outer.set_transform(Some(
        RotateTransform3D::with_rotation(
            AxisAngleRotation3D::with_axis_angle(Vector3D::new(0.0, 1.0, 0.0), 90.0).upcast(),
        )
        .upcast(),
    ));
    // (x, y, z) turns to (z, y, -x): the triangle is in the plane x = -3, y from 0 to 2, and its
    // front faces +x.
    let results = hits(&outer, Point3D::new(10.0, 0.5, -2.5), Vector3D::new(-1.0, 0.0, 0.0));
    assert_eq!(results.len(), 1);
    assert!(close(results[0].distance_to_ray_origin(), 13.0));
    assert!(hits(&outer, Point3D::new(-10.0, 0.5, -2.5), Vector3D::new(1.0, 0.0, 0.0)).is_empty());
    assert_point(results[0].point_hit(), Point3D::new(0.25, 0.25, 0.0));

    // A mirror: the triangle scaled by -1 in x is clockwise seen from +z, so the camera there
    // sees its back, which has no material.
    let mirrored = visual_of(&front_model(&mesh));
    mirrored.set_transform(Some(ScaleTransform3D::with_scale_xyz(-1.0, 1.0, 1.0).upcast()));
    let (origin, direction) = down_from(-0.25, 0.25, 5.0);
    assert!(hits(&mirrored, origin, direction).is_empty());
    let back_only = GeometryModel3D::new();
    back_only.set_geometry(Some(mesh.clone().upcast()));
    back_only.set_back_material(Some(material(Brushes::blue())));
    mirrored.set_content(Some(back_only.upcast()));
    let results = hits(&mirrored, origin, direction);
    assert_eq!(results.len(), 1);
    assert!(!results[0].is_front_face());
    assert_point(results[0].point_hit(), Point3D::new(0.25, 0.25, 0.0));
}

#[test]
fn the_hits_come_nearest_first_and_a_stop_ends_them() {
    // Three squares at z = -2, 0 and -1, added in that order, under one parent.
    let root = ModelVisual3D::new();
    for z in [-2.0, 0.0, -1.0] {
        root.children().add(visual_of(&front_model(&square(-1.0, -1.0, 1.0, 1.0, z))).upcast());
    }
    let (origin, direction) = down_from(0.1, 0.2, 5.0);
    let results = hits(&root, origin, direction);
    let found = distances(&results);
    assert_eq!(found.len(), 3);
    assert!(close(found[0], 5.0) && close(found[1], 6.0) && close(found[2], 7.0), "{found:?}");

    // The nearest alone: the result callback stops after the first.
    let mut calls = 0;
    VisualTreeHelper::hit_test_ray(
        &root,
        None,
        &mut |_| {
            calls += 1;
            HitTestResultBehavior::Stop
        },
        &RayHitTestParameters::new(origin, direction),
    );
    assert_eq!(calls, 1);

    // A cube: the ray through its centre goes in at z = 1 and out at z = -1, where it meets the
    // inside of the back face (its back side, which has no material): one hit.
    let cube_visual = visual_of(&front_model(&cube()));
    let results = hits(&cube_visual, Point3D::new(0.2, 0.3, 5.0), direction);
    assert_eq!(distances(&results).len(), 1);
    assert!(close(results[0].distance_to_ray_origin(), 4.0));
    // With a back material both faces are hit, the front one first.
    let model = front_model(&cube());
    model.set_back_material(Some(material(Brushes::blue())));
    let results = hits(&visual_of(&model), Point3D::new(0.2, 0.3, 5.0), direction);
    let found = distances(&results);
    assert!(close(found[0], 4.0) && close(found[1], 6.0), "{found:?}");
    assert!(results[0].is_front_face() && !results[1].is_front_face());
}

#[test]
fn a_ray_that_misses_or_points_away_hits_nothing() {
    let visual = visual_of(&front_model(&triangle()));
    // Beside the triangle, beyond its long edge.
    assert!(hits(&visual, Point3D::new(0.6, 0.6, 5.0), Vector3D::new(0.0, 0.0, -1.0)).is_empty());
    // The triangle behind the origin.
    assert!(hits(&visual, Point3D::new(0.25, 0.25, 5.0), Vector3D::new(0.0, 0.0, 1.0)).is_empty());
    // In the plane of the triangle.
    assert!(hits(&visual, Point3D::new(-1.0, 0.25, 0.0), Vector3D::new(1.0, 0.0, 0.0)).is_empty());
    // A zero direction.
    assert!(hits(&visual, Point3D::new(0.25, 0.25, 5.0), Vector3D::new(0.0, 0.0, 0.0)).is_empty());
    // A mesh without positions, and a model without geometry.
    assert!(hits(
        &visual_of(&front_model(&MeshGeometry3D::new())),
        Point3D::new(0.0, 0.0, 5.0),
        Vector3D::new(0.0, 0.0, -1.0)
    )
    .is_empty());
    let empty = ModelVisual3D::with_content(GeometryModel3D::new().upcast());
    assert!(hits(&empty, Point3D::new(0.0, 0.0, 5.0), Vector3D::new(0.0, 0.0, -1.0)).is_empty());
    // A model flattened by a scale of zero.
    let flat = visual_of(&front_model(&triangle()));
    flat.set_transform(Some(ScaleTransform3D::with_scale_xyz(1.0, 0.0, 1.0).upcast()));
    assert!(hits(&flat, Point3D::new(0.25, 0.0, 5.0), Vector3D::new(0.0, 0.0, -1.0)).is_empty());
}

#[test]
fn degenerate_triangles_are_not_hit() {
    // A triangle on a line, one with a repeated vertex, one naming a position that does not
    // exist, and an incomplete last triangle; then one real triangle far away.
    let degenerate = mesh("0,0,0 1,1,0 2,2,0 5,5,0 6,5,0 5,6,0", "0 1 2 0 0 1 0 1 9 3 4 5 0 1");
    let visual = visual_of(&front_model(&degenerate));
    let down = Vector3D::new(0.0, 0.0, -1.0);
    for (x, y) in [(0.0, 0.0), (1.0, 1.0), (0.5, 0.5), (0.9, 0.1)] {
        assert!(hits(&visual, Point3D::new(x, y, 5.0), down).is_empty(), "a hit at {x}, {y}");
    }
    let results = hits(&visual, Point3D::new(5.2, 5.2, 5.0), down);
    assert_eq!(results.len(), 1);
    assert_eq!((results[0].vertex_index_1(), results[0].vertex_index_2(), results[0].vertex_index_3()), (3, 4, 5));
    // Positions that are not numbers are not hit either.
    let invalid = mesh("NaN,0,0 1,0,0 0,1,0", "0 1 2");
    assert!(hits(&visual_of(&front_model(&invalid)), Point3D::new(0.25, 0.25, 5.0), down).is_empty());
}

// --- filters -----------------------------------------------------------------------------------

/// A visual with a square at z = 0 and two children with squares at z = -1 and z = -2.
fn filtered_tree() -> (Ref<ModelVisual3D>, Ref<ModelVisual3D>, Ref<ModelVisual3D>) {
    let a = visual_of(&front_model(&square(-1.0, -1.0, 1.0, 1.0, 0.0)));
    let b = visual_of(&front_model(&square(-1.0, -1.0, 1.0, 1.0, -1.0)));
    let c = visual_of(&front_model(&square(-1.0, -1.0, 1.0, 1.0, -2.0)));
    a.children().add(b.clone().upcast());
    a.children().add(c.clone().upcast());
    (a, b, c)
}

/// The visuals hit with `filter`, nearest first.
fn filtered_hits(
    root: &Visual3D,
    filter: &mut dyn FnMut(&Ref<Visual3D>) -> HitTestFilterBehavior,
) -> Vec<Ref<Visual3D>> {
    let mut visuals = Vec::new();
    VisualTreeHelper::hit_test_ray(
        root,
        Some(filter),
        &mut |result| {
            visuals.push(result.visual_hit().clone());
            HitTestResultBehavior::Continue
        },
        // Not on the diagonal of the squares, where two triangles meet (both would be hit).
        &RayHitTestParameters::new(Point3D::new(0.1, 0.3, 5.0), Vector3D::new(0.0, 0.0, -1.0)),
    );
    visuals
}

#[test]
fn the_filter_skips_visuals_their_children_or_stops_the_test() {
    let (a, b, c) = filtered_tree();
    let same = |visuals: &[Ref<Visual3D>], expected: &[&Ref<ModelVisual3D>]| {
        visuals.len() == expected.len() && visuals.iter().zip(expected).all(|(visual, e)| visual.ptr_eq(e))
    };

    let mut seen = Vec::new();
    let all = filtered_hits(&a, &mut |visual| {
        seen.push(visual.clone());
        HitTestFilterBehavior::Continue
    });
    assert!(same(&all, &[&a, &b, &c]));
    // The filter is asked of each visual once, in tree order.
    assert!(same(&seen, &[&a, &b, &c]));

    let skip_self = filtered_hits(&a, &mut |visual| {
        if visual.ptr_eq(&a) {
            HitTestFilterBehavior::ContinueSkipSelf
        } else {
            HitTestFilterBehavior::Continue
        }
    });
    assert!(same(&skip_self, &[&b, &c]));

    let skip_children = filtered_hits(&a, &mut |visual| {
        if visual.ptr_eq(&a) {
            HitTestFilterBehavior::ContinueSkipChildren
        } else {
            HitTestFilterBehavior::Continue
        }
    });
    assert!(same(&skip_children, &[&a]));

    let skip_b = filtered_hits(&a, &mut |visual| {
        if visual.ptr_eq(&b) {
            HitTestFilterBehavior::ContinueSkipSelfAndChildren
        } else {
            HitTestFilterBehavior::Continue
        }
    });
    assert!(same(&skip_b, &[&a, &c]));

    // A stop at `b`: the test ends, and the hit found before (`a`) is reported.
    let mut asked = 0;
    let stopped = filtered_hits(&a, &mut |visual| {
        asked += 1;
        if visual.ptr_eq(&b) {
            HitTestFilterBehavior::Stop
        } else {
            HitTestFilterBehavior::Continue
        }
    });
    assert!(same(&stopped, &[&a]));
    assert_eq!(asked, 2);
}

// --- the rays of a viewport ------------------------------------------------------------------

/// A viewport of `width` by `height` with `camera` and `children`.
fn viewport(camera: Ref<Camera>, width: f64, height: f64, children: &[Ref<Visual3D>]) -> Ref<Viewport3DVisual> {
    let visual = Viewport3DVisual::new();
    visual.set_camera(Some(camera));
    visual.set_viewport(Rect::new(0.0, 0.0, width, height));
    for child in children {
        visual.children().add(child.clone());
    }
    visual
}

fn perspective_at(z: f64) -> Ref<Camera> {
    PerspectiveCamera::with_values(
        Point3D::new(0.0, 0.0, z),
        Vector3D::new(0.0, 0.0, -1.0),
        Vector3D::new(0.0, 1.0, 0.0),
        90.0,
    )
    .upcast()
}

#[test]
fn a_point_of_a_perspective_viewport_hits_through_the_camera() {
    let model = front_model(&cube());
    let scene = viewport(perspective_at(5.0), 100.0, 100.0, &[visual_of(&model).upcast()]);
    // The centre: the front face at z = 1, 4 from the camera.
    let hit = VisualTreeHelper::hit_test(&*scene, Point::new(50.0, 50.0)).unwrap();
    assert!(hit.model_hit().ptr_eq(&model));
    assert_point_within(hit.point_hit(), Point3D::new(0.0, 0.0, 1.0), 1e-5);
    assert!((hit.distance_to_ray_origin() - 4.0).abs() < 1e-5);
    // A horizontal field of view of 90 degrees: the edges of the viewport are at x = -depth and
    // x = depth. A quarter from the left is x = -0.5 at the depth 1, so -2 at the depth 4 of the
    // front face: beside the cube. 38 of 100 is -0.24, so -0.96 on the front face.
    assert!(VisualTreeHelper::hit_test(&*scene, Point::new(25.0, 50.0)).is_none());
    let hit = VisualTreeHelper::hit_test(&*scene, Point::new(38.0, 50.0)).unwrap();
    assert!((hit.point_hit().x - (-0.24 * 4.0)).abs() < 1e-5, "{:?}", hit.point_hit());
    // Rows go down: a point above the centre hits y > 0.
    let hit = VisualTreeHelper::hit_test(&*scene, Point::new(50.0, 40.0)).unwrap();
    assert!((hit.point_hit().y - 0.8).abs() < 1e-5, "{:?}", hit.point_hit());
    // Outside the viewport: nothing.
    assert!(VisualTreeHelper::hit_test(&*scene, Point::new(-1.0, 50.0)).is_none());
    assert!(VisualTreeHelper::hit_test(&*scene, Point::new(50.0, 101.0)).is_none());
    // Without a camera: nothing.
    scene.set_camera(None);
    assert!(VisualTreeHelper::hit_test(&*scene, Point::new(50.0, 50.0)).is_none());
}

#[test]
fn a_viewport_hits_only_between_its_planes() {
    // A finite far plane at 4.5 from the camera: the square at depth 5 is not drawn, not hit.
    let near_square = visual_of(&front_model(&square(-1.0, -1.0, 1.0, 1.0, 1.0)));
    let far_square = visual_of(&front_model(&square(-5.0, -5.0, 5.0, 5.0, 0.0)));
    let camera = PerspectiveCamera::with_values(
        Point3D::new(0.0, 0.0, 5.0),
        Vector3D::new(0.0, 0.0, -1.0),
        Vector3D::new(0.0, 1.0, 0.0),
        90.0,
    );
    camera.set_far_plane_distance(4.5);
    let scene = viewport(camera.clone().upcast(), 100.0, 100.0, &[near_square.upcast(), far_square.upcast()]);
    let mut found = Vec::new();
    VisualTreeHelper::hit_test_point(
        &*scene,
        None,
        &mut |result| {
            found.push(result.distance_to_ray_origin());
            HitTestResultBehavior::Continue
        },
        Point::new(55.0, 45.0),
    );
    // The point is a tenth of the half-width right of and above the centre: the ray goes to
    // (0.4, 0.4) on the near square, 4 deep.
    assert_eq!(found.len(), 1);
    assert!((found[0] - 4.0 * 1.02f64.sqrt()).abs() < 1e-5, "{found:?}");
    // Planes at 4.5 and 6: the near square is in front of the near plane, the far one between
    // the planes.
    camera.set_near_plane_distance(4.5);
    camera.set_far_plane_distance(6.0);
    let hit = VisualTreeHelper::hit_test(&*scene, Point::new(50.0, 50.0)).unwrap();
    assert!((hit.distance_to_ray_origin() - 5.0).abs() < 1e-5);
}

#[test]
fn orthographic_and_matrix_cameras_cast_their_rays() {
    let model = front_model(&cube());
    let camera = OrthographicCamera::with_values(
        Point3D::new(0.0, 0.0, 5.0),
        Vector3D::new(0.0, 0.0, -1.0),
        Vector3D::new(0.0, 1.0, 0.0),
        4.0,
    );
    // A viewport twice as wide as high: the width 4 spans x from -2 to 2, y from -1 to 1.
    let scene = viewport(camera.upcast(), 200.0, 100.0, &[visual_of(&model).upcast()]);
    let hit = VisualTreeHelper::hit_test(&*scene, Point::new(80.0, 50.0)).unwrap();
    // 80 of 200 is x = -0.4; the rays are parallel, from the plane of the camera.
    assert_point_within(hit.point_hit(), Point3D::new(-0.4, 0.0, 1.0), 1e-5);
    assert!((hit.distance_to_ray_origin() - 4.0).abs() < 1e-5);
    let hit = VisualTreeHelper::hit_test(&*scene, Point::new(180.0, 10.0));
    assert!(hit.is_none(), "x = 1.6 is beside the cube");
    let hit = VisualTreeHelper::hit_test(&*scene, Point::new(149.0, 5.0)).unwrap();
    assert!((hit.point_hit().x - 0.98).abs() < 1e-5 && (hit.point_hit().y - 0.9).abs() < 1e-5);

    // A matrix camera with the matrices of a perspective camera hits as that camera does.
    let perspective = PerspectiveCamera::with_values(
        Point3D::new(1.0, 2.0, 6.0),
        Vector3D::new(-1.0, -2.0, -6.0),
        Vector3D::new(0.0, 1.0, 0.0),
        50.0,
    );
    let projection = perspective.get_projection_matrix(2.0, 1.0, 20.0);
    let matrix_camera = MatrixCamera::with_matrices(perspective.get_view_matrix(), projection);
    perspective.set_near_plane_distance(1.0);
    perspective.set_far_plane_distance(20.0);
    let by_perspective = viewport(perspective.upcast(), 200.0, 100.0, &[visual_of(&model).upcast()]);
    let by_matrix = viewport(matrix_camera.upcast(), 200.0, 100.0, &[visual_of(&model).upcast()]);
    for point in [Point::new(100.0, 50.0), Point::new(90.0, 60.0), Point::new(120.0, 35.0)] {
        let a = VisualTreeHelper::hit_test(&*by_perspective, point).unwrap();
        let b = VisualTreeHelper::hit_test(&*by_matrix, point).unwrap();
        let (pa, pb) = (a.point_hit(), b.point_hit());
        assert_point_within(pa, pb, 1e-4);
        // The same line from another origin: the matrix camera's starts on the near plane.
        assert!(b.distance_to_ray_origin() < a.distance_to_ray_origin());
    }
}

#[test]
fn a_viewport_3d_control_hit_tests_its_scene() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let control = Viewport3D::new();
    control.set_camera(Some(perspective_at(5.0)));
    let model = front_model(&cube());
    control.children().add(visual_of(&model).upcast());
    control.viewport_3d_visual().set_viewport(Rect::new(0.0, 0.0, 100.0, 100.0));
    let hit = VisualTreeHelper::hit_test(&*control, Point::new(50.0, 50.0)).unwrap();
    assert!(hit.model_hit().ptr_eq(&model));
}

// --- the hierarchy of a mesh -------------------------------------------------------------------

#[test]
fn the_hierarchy_is_built_once_per_version_of_a_mesh() {
    let mesh = cube();
    // Two models of one mesh.
    let root = ModelVisual3D::new();
    root.children().add(visual_of(&front_model(&mesh)).upcast());
    let second = visual_of(&front_model(&mesh));
    second.set_transform(Some(TranslateTransform3D::with_offset_xyz(0.0, 0.0, -5.0).upcast()));
    root.children().add(second.upcast());
    let (origin, direction) = down_from(0.5, 0.3, 5.0);

    let before = hierarchies_built();
    assert_eq!(hits(&root, origin, direction).len(), 2);
    assert_eq!(hierarchies_built() - before, 1);
    assert_eq!(hits(&root, origin, direction).len(), 2);
    assert_eq!(hierarchies_built() - before, 1);
    // A new version of the mesh: a new hierarchy, which sees the change.
    mesh.positions().set(0, Point3D::new(-1.0, -1.0, 2.0));
    // (-0.9, -0.95) is in the triangle (0, 1, 2) with the weights 0.95, 0.025 and 0.025: z is
    // 0.95 * 2 + 0.05 there.
    let results = hits(&root, Point3D::new(-0.9, -0.95, 5.0), direction);
    assert_eq!(hierarchies_built() - before, 2);
    assert!(close(results[0].distance_to_ray_origin(), 5.0 - 1.95), "{}", results[0].distance_to_ray_origin());
}

#[test]
fn a_mesh_of_100k_triangles_is_hit_through_its_hierarchy() {
    // A grid of 225 by 225 cells, two triangles each (101,250 triangles), in the plane z = 0.
    let cells = 225usize;
    let mut positions = Vec::with_capacity((cells + 1) * (cells + 1));
    for row in 0..=cells {
        for column in 0..=cells {
            positions.push(Point3D::new(column as f64, row as f64, 0.0));
        }
    }
    let mut indices = Vec::with_capacity(cells * cells * 6);
    let at = |row: usize, column: usize| (row * (cells + 1) + column) as i32;
    for row in 0..cells {
        for column in 0..cells {
            let (a, b, c, d) = (at(row, column), at(row, column + 1), at(row + 1, column + 1), at(row + 1, column));
            indices.extend([a, b, c, a, c, d]);
        }
    }
    let mesh = MeshGeometry3D::new();
    mesh.set_positions(Point3DCollection::from_items(positions));
    mesh.set_triangle_indices(Int32Collection::from_items(indices));
    let hierarchy = mesh.hit_test_hierarchy();
    assert_eq!(hierarchy.triangle_count(), cells * cells * 2);

    // The cell (row 100, column 37), its lower triangle (a, b, c).
    let mut found = Vec::new();
    let tested =
        hierarchy.intersect(Point3D::new(37.7, 100.2, 3.0), Vector3D::new(0.0, 0.0, -1.0), (true, false), &mut |hit| {
            found.push(hit)
        });
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].vertices, [at(100, 37) as u32, at(100, 38) as u32, at(101, 38) as u32]);
    assert!(tested <= 32, "{tested} triangles tested");
    // Through the scene as well.
    let visual = visual_of(&front_model(&mesh));
    let results = hits(&visual, Point3D::new(200.3, 10.9, 1.0), Vector3D::new(0.0, 0.0, -1.0));
    assert_eq!(results.len(), 1);
    assert_point(results[0].point_hit(), Point3D::new(200.3, 10.9, 0.0));
}

// --- picks against the CPU reference ---------------------------------------------------------

/// Renders `scene` with one sample per pixel (the centre, edge mode `Aliased`) and holds every
/// pixel whose neighbours agree to the hit test through its centre: a pixel drawn is hit, a
/// pixel not drawn is not; `model_of` names the model a drawn pixel's colour belongs to.
fn assert_picks_match_pixels(
    scene: &Viewport3DVisual,
    model_of: &dyn Fn([u8; 4]) -> Option<Ref<GeometryModel3D>>,
) -> (usize, usize) {
    let viewport = scene.viewport();
    let (width, height) = (viewport.width as u32, viewport.height as u32);
    let image = cpu::render(
        &scene.snapshot(),
        PixelSize::new(width as i32, height as i32),
        1.0,
        CpuRenderOptions { antialiased: false },
    );
    let drawn = |x: i64, y: i64| -> Option<bool> {
        if x < 0 || y < 0 || x >= width as i64 || y >= height as i64 {
            return None;
        }
        Some(image.pixel(x as u32, y as u32)[3] > 0)
    };
    let (mut hit_pixels, mut empty_pixels) = (0, 0);
    for y in 0..height as i64 {
        for x in 0..width as i64 {
            let here = drawn(x, y).unwrap();
            // An edge of a model passes near the centre of a pixel whose neighbours differ: the
            // rasterizer's fixed point and fill rule decide there, not the hit test.
            let uniform = (-1..=1).all(|dy| (-1..=1).all(|dx| drawn(x + dx, y + dy).is_none_or(|other| other == here)));
            if !uniform {
                continue;
            }
            let hit = VisualTreeHelper::hit_test(scene, Point::new(x as f64 + 0.5, y as f64 + 0.5));
            assert_eq!(hit.is_some(), here, "pixel {x}, {y}: drawn {here}, hit {}", hit.is_some());
            if let Some(hit) = hit {
                hit_pixels += 1;
                if let Some(model) = model_of(image.pixel(x as u32, y as u32)) {
                    assert!(hit.model_hit().ptr_eq(&model), "pixel {x}, {y} hits another model");
                }
            } else {
                empty_pixels += 1;
            }
        }
    }
    assert!(hit_pixels > 0 && empty_pixels > 0, "{hit_pixels} pixels hit, {empty_pixels} empty");
    (hit_pixels, empty_pixels)
}

#[test]
fn a_pick_hits_the_model_the_cpu_reference_draws_under_it() {
    // A turned cube under a perspective camera, lit by an ambient light, in a wide viewport.
    let model = front_model(&cube());
    let turn = Transform3DGroup::new();
    turn.children().add(
        RotateTransform3D::with_rotation(
            AxisAngleRotation3D::with_axis_angle(Vector3D::new(0.0, 1.0, 0.0), 30.0).upcast(),
        )
        .upcast(),
    );
    turn.children().add(
        RotateTransform3D::with_rotation(
            AxisAngleRotation3D::with_axis_angle(Vector3D::new(1.0, 0.0, 0.0), 20.0).upcast(),
        )
        .upcast(),
    );
    model.set_transform(Some(turn.upcast()));
    let lit = Model3DGroup::new();
    lit.children().add(AmbientLight::new().upcast());
    lit.children().add(model.upcast());
    let camera = PerspectiveCamera::with_values(
        Point3D::new(0.0, 0.0, 6.0),
        Vector3D::new(0.0, 0.0, -1.0),
        Vector3D::new(0.0, 1.0, 0.0),
        45.0,
    );
    let scene = viewport(camera.upcast(), 120.0, 80.0, &[ModelVisual3D::with_content(lit.upcast()).upcast()]);
    assert_picks_match_pixels(&scene, &|_| None);
}

#[test]
fn a_pick_follows_depth_and_the_sides_as_the_cpu_reference_draws_them() {
    // An orthographic camera; a red square in front of a larger blue one, and a green square
    // turned away from the camera with only a front material (drawn nowhere, hit nowhere); all
    // lit by a white ambient light, so each draws its own colour.
    let light = ModelVisual3D::with_content(AmbientLight::new().upcast());
    let red = front_model(&square(-1.5, -1.0, 0.5, 1.0, 0.5));
    let blue = GeometryModel3D::with_geometry_and_material(
        square(-2.0, -2.0, 1.5, 1.8, -0.5).upcast(),
        material(Brushes::blue()),
    );
    let away = front_model(&mesh("1.6,-2,0 1.6,2,0 1.9,2,0 1.9,-2,0", "0 1 2 0 2 3"));
    away.set_material(Some(material(Brushes::lime())));
    // A back material on a square that faces away: drawn and hit on its back.
    let back = GeometryModel3D::new();
    back.set_geometry(Some(mesh("-2,-2.4,0 -2,-2.1,0 2,-2.1,0 2,-2.4,0", "0 1 2 0 2 3").upcast()));
    back.set_back_material(Some(material(Brushes::yellow())));
    let camera = OrthographicCamera::with_values(
        Point3D::new(0.0, 0.0, 5.0),
        Vector3D::new(0.0, 0.0, -1.0),
        Vector3D::new(0.0, 1.0, 0.0),
        5.0,
    );
    let scene = viewport(
        camera.upcast(),
        100.0,
        100.0,
        &[
            light.upcast(),
            visual_of(&blue).upcast(),
            visual_of(&red).upcast(),
            visual_of(&away).upcast(),
            visual_of(&back).upcast(),
        ],
    );
    let model_of = |pixel: [u8; 4]| match pixel {
        [255, 0, 0, 255] => Some(red.clone()),
        [0, 0, 255, 255] => Some(blue.clone()),
        [255, 255, 0, 255] => Some(back.clone()),
        other => panic!("an unexpected colour {other:?}"),
    };
    assert_picks_match_pixels(&scene, &model_of);
    // The square turned away is neither drawn nor hit.
    assert!(VisualTreeHelper::hit_test(&*scene, Point::new(85.0, 50.0)).is_none());
    // The back one is hit on its back.
    let hit = VisualTreeHelper::hit_test(&*scene, Point::new(50.0, 95.0)).unwrap();
    assert!(hit.model_hit().ptr_eq(&back) && !hit.is_front_face());
}
