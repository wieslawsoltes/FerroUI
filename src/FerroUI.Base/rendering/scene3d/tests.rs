//! Tests of the operation `DrawScene3D` (#608): recorded, measured, sent through a batch and
//! replayed onto a context with and without the 3D capability. Not from upstream.

use super::*;
use crate::media::{Colors, DrawingContext, PlatformDrawingContext};
use crate::rendering::composition::drawing::{RenderDataDrawingContext, RenderDataOpcode, RenderDataStream};
use crate::rendering::composition::transport::{BatchStreamData, BatchStreamReader, BatchStreamWriter};
use crate::rendering::testing::{recorded_opcodes, DrawingLog, MockDrawingContextImpl};

/// A scene with one triangle and a camera: something to draw.
fn scene() -> Arc<Scene3DSnapshot> {
    let mesh = Arc::new(MeshData {
        id: 1,
        version: 1,
        positions: vec![Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0)],
        normals: vec![Vector3::new(0.0, 0.0, 1.0); 3],
        normals_generated: true,
        texture_coordinates: Vec::new(),
        indices: vec![0, 1, 2],
        bounds: Scene3DBounds::of_points([Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 1.0, 0.0)]),
    });
    let layer = Scene3DMaterialLayer {
        kind: Scene3DMaterialKind::Diffuse,
        color: Colors::WHITE,
        ambient_color: Colors::WHITE,
        specular_power: 0.0,
        brush: None,
    };
    let mut snapshot = Scene3DSnapshot::empty();
    snapshot.camera = Some(Scene3DCamera {
        kind: Scene3DCameraKind::Matrix,
        view: Matrix4x4::IDENTITY,
        projection: Matrix4x4::IDENTITY,
        position: Vector3::default(),
        near_plane_distance: 0.0,
        far_plane_distance: f32::INFINITY,
    });
    snapshot.bounds = mesh.bounds;
    snapshot.models.push(Scene3DModel {
        world: Matrix4x4::IDENTITY,
        world_bounds: mesh.bounds,
        mesh,
        front: vec![layer],
        back: Vec::new(),
    });
    snapshot.lights.push(Scene3DLight::Ambient { color: Colors::WHITE });
    Arc::new(snapshot)
}

#[test]
fn a_scene_is_recorded_as_one_operation_with_its_bounds() {
    let scene = scene();
    let bounds = Rect::new(10.0, 20.0, 100.0, 50.0);
    let mut stream = RenderDataStream::new();
    stream.draw_scene_3d(Some(scene.clone()), bounds);
    stream.draw_scene_3d(Some(scene.clone()), bounds);
    assert_eq!(recorded_opcodes(&stream), [RenderDataOpcode::DrawScene3D, RenderDataOpcode::DrawScene3D]);
    // The snapshot is one entry of the table, shared by both operations.
    assert_eq!(stream.resource_count(), 1);
    assert_eq!(stream.calculate_bounds(), Some(bounds));
}

#[test]
fn a_scene_that_draws_nothing_is_not_recorded_by_the_drawing_context() {
    let mut recorder = RenderDataDrawingContext::new(None);
    let log = DrawingLog::new();
    let mut platform = MockDrawingContextImpl::new(log.clone());
    platform.supports_scene_3d = true;
    {
        let mut context = DrawingContext::new(&mut recorder);
        context.draw_scene_3d(&Arc::new(Scene3DSnapshot::empty()), Rect::new(0.0, 0.0, 10.0, 10.0));
        context.draw_scene_3d(&scene(), Rect::new(0.0, 0.0, 0.0, 10.0));
    }
    {
        let mut core = PlatformDrawingContext::borrowed(&mut platform);
        let mut context = DrawingContext::new(&mut core);
        context.draw_scene_3d(&Arc::new(Scene3DSnapshot::empty()), Rect::new(0.0, 0.0, 10.0, 10.0));
        context.dispose();
    }
    assert!(log.entries().iter().all(|entry| !entry.starts_with("DrawScene3D")));
}

#[test]
fn a_scene_survives_a_batch_unchanged() {
    let scene = scene();
    let mut stream = RenderDataStream::new();
    stream.draw_scene_3d(Some(scene.clone()), Rect::new(0.0, 0.0, 64.0, 32.0));
    let mut data = BatchStreamData::new();
    stream.serialize_to(&mut BatchStreamWriter::new(&mut data));
    let mut received = RenderDataStream::new();
    let mut reader = BatchStreamReader::new(&mut data);
    received.deserialize_from(&mut reader, &|_| None);
    assert!(reader.is_object_eof() && reader.is_struct_eof());
    assert_eq!(recorded_opcodes(&received), [RenderDataOpcode::DrawScene3D]);
    match received.get_resource(0) {
        Some(crate::rendering::composition::drawing::RenderDataResource::Scene3D(sent)) => {
            assert!(Arc::ptr_eq(sent, &scene), "the snapshot crosses by reference, not by copy");
        }
        _ => panic!("the table holds the snapshot"),
    }
}

#[test]
fn a_context_with_the_capability_draws_the_scene_and_one_without_counts_it() {
    let scene = scene();
    let bounds = Rect::new(0.0, 0.0, 40.0, 30.0);
    let mut stream = RenderDataStream::new();
    stream.draw_scene_3d(Some(scene), bounds);

    let log = DrawingLog::new();
    let mut capable = MockDrawingContextImpl::new(log.clone());
    capable.log_transforms = false;
    capable.supports_scene_3d = true;
    let before = scenes_not_drawn();
    stream.replay(&mut capable);
    assert_eq!(log.take(), ["DrawScene3D 0, 0, 40, 30 1 models 1 lights"]);

    let mut incapable = MockDrawingContextImpl::new(log.clone());
    incapable.log_transforms = false;
    stream.replay(&mut incapable);
    assert!(log.entries().is_empty(), "nothing is drawn without the capability");
    // Other tests may count scenes at the same time: at least this one was counted.
    assert!(scenes_not_drawn() > before);
}

#[test]
fn the_bounds_of_a_mesh_hold_its_points() {
    let bounds = Scene3DBounds::of_points([
        Vector3::new(1.0, -2.0, 3.0),
        Vector3::new(-1.0, 4.0, 0.5),
        Vector3::new(0.0, 0.0, -6.0),
    ])
    .unwrap();
    assert_eq!(bounds.min, Vector3::new(-1.0, -2.0, -6.0));
    assert_eq!(bounds.max, Vector3::new(1.0, 4.0, 3.0));
    assert!(Scene3DBounds::of_points(std::iter::empty()).is_none());
    assert_eq!(bounds.corners().len(), 8);
}
