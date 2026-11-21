//! The Metal executor on the system's Metal device, held to the CPU reference of
//! `ferroui-render3d`. A machine without a Metal device checks nothing.

use super::*;
use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutableSolidColorBrush};
use ferroui_base::media::{Color, GradientSpreadMethod, IBrush, SharedBrush};
use ferroui_base::numerics::{Matrix4x4, Vector2, Vector3};
use ferroui_base::rendering::scene3d::{
    Scene3DBounds, Scene3DCamera, Scene3DCameraKind, Scene3DLight, Scene3DMaterialKind,
};
use ferroui_base::{PixelSize, RelativePoint, RelativeUnit};
use ferroui_render3d::cpu::render as cpu_render;
use ferroui_render3d::CpuRenderOptions;
use objc2_metal::MTLCreateSystemDefaultDevice;
use std::sync::Arc;

/// The side of the images of the tests, in pixels.
const SIDE: u32 = 100;

/// An executor of the system's device and a queue of it, or `None` on a machine without one.
fn executor() -> Option<Scene3DMetalExecutor> {
    let device = MTLCreateSystemDefaultDevice()?;
    let queue = device.newCommandQueue()?;
    Some(Scene3DMetalExecutor::with_device(device, queue).expect("the executor of the device"))
}

/// The pixels of `size` at the top left of `texture`, read back through a buffer shared with the
/// CPU after every command of the queue ran.
fn read_back(executor: &Scene3DMetalExecutor, texture: &Scene3DMetalTexture, size: (u32, u32)) -> Vec<u8> {
    let (width, height) = (size.0 as usize, size.1 as usize);
    let buffer = executor.buffer(&[], width * height * 4).expect("a readback buffer");
    let command_buffer = executor.queue.commandBuffer().expect("a command buffer");
    let blit = command_buffer.blitCommandEncoder().expect("a blit encoder");
    // SAFETY: the region is within the texture and the buffer holds it, `width * 4` bytes a row.
    unsafe {
        blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toBuffer_destinationOffset_destinationBytesPerRow_destinationBytesPerImage(
            &texture.texture,
            0,
            0,
            MTLOrigin { x: 0, y: 0, z: 0 },
            MTLSize { width, height, depth: 1 },
            &buffer,
            0,
            width * 4,
            width * height * 4,
        );
    }
    blit.endEncoding();
    command_buffer.commit();
    command_buffer.waitUntilCompleted();
    assert_eq!(MTLCommandBufferStatus::Completed, command_buffer.status(), "the readback ran");
    // SAFETY: the copy has completed; the buffer holds `width * height * 4` bytes.
    unsafe { std::slice::from_raw_parts(buffer.contents().as_ptr().cast::<u8>(), width * height * 4) }.to_vec()
}

fn solid(color: Color) -> Option<SharedBrush> {
    SharedBrush::from_brush(&ImmutableSolidColorBrush::new(color))
}

fn layer(kind: Scene3DMaterialKind, brush: Option<SharedBrush>) -> Scene3DMaterialLayer {
    Scene3DMaterialLayer {
        kind,
        color: Color::from_argb(255, 255, 255, 255),
        ambient_color: Color::from_argb(255, 255, 255, 255),
        specular_power: if kind == Scene3DMaterialKind::Specular { 40.0 } else { 0.0 },
        brush,
    }
}

/// A quad in the plane `z` from `(x0, y0)` to `(x1, y1)`, counterclockwise seen from +z, with
/// normals along +z and texture coordinates over the unit square.
fn quad(id: u64, x0: f32, y0: f32, x1: f32, y1: f32, z: f32) -> Arc<MeshData> {
    let positions =
        vec![Vector3::new(x0, y0, z), Vector3::new(x1, y0, z), Vector3::new(x1, y1, z), Vector3::new(x0, y1, z)];
    let bounds = Scene3DBounds::of_points(positions.iter().copied());
    Arc::new(MeshData {
        id,
        version: 1,
        positions,
        normals: vec![Vector3::new(0.0, 0.0, 1.0); 4],
        normals_generated: false,
        texture_coordinates: vec![
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 0.0),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        bounds,
    })
}

fn model(mesh: Arc<MeshData>, front: Vec<Scene3DMaterialLayer>, back: Vec<Scene3DMaterialLayer>) -> Scene3DModel {
    Scene3DModel { world: Matrix4x4::IDENTITY, world_bounds: mesh.bounds, mesh, front, back }
}

/// WPF's orthographic camera at `(offset, 0, 5)` looking down -z with `Width` 2, planes 1 and 10.
fn orthographic(offset: f32) -> Scene3DCamera {
    let (near, far, width) = (1.0f32, 10.0f32, 2.0f32);
    let position = Vector3::new(offset, 0.0, 5.0);
    let view = Matrix4x4::create_look_at(position, Vector3::new(offset, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0));
    let m11 = 2.0 / width;
    let m33 = 1.0 / (near - far);
    let projection =
        Matrix4x4::new(m11, 0.0, 0.0, 0.0, 0.0, m11, 0.0, 0.0, 0.0, 0.0, m33, 0.0, 0.0, 0.0, near * m33, 1.0);
    Scene3DCamera {
        kind: Scene3DCameraKind::Orthographic,
        view,
        projection,
        position,
        near_plane_distance: near,
        far_plane_distance: far,
    }
}

/// WPF's perspective camera at `position` looking at the origin, 45 degrees, planes 1 and 20.
fn perspective(position: Vector3) -> Scene3DCamera {
    let (near, far, fov) = (1.0f32, 20.0f32, 45.0f32);
    let view = Matrix4x4::create_look_at(position, Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0));
    let m11 = 1.0 / (fov.to_radians() / 2.0).tan();
    let m33 = far / (near - far);
    let projection =
        Matrix4x4::new(m11, 0.0, 0.0, 0.0, 0.0, m11, 0.0, 0.0, 0.0, 0.0, m33, -1.0, 0.0, 0.0, near * m33, 0.0);
    Scene3DCamera {
        kind: Scene3DCameraKind::Perspective,
        view,
        projection,
        position,
        near_plane_distance: near,
        far_plane_distance: far,
    }
}

fn snapshot(camera: Scene3DCamera, models: Vec<Scene3DModel>, lights: Vec<Scene3DLight>) -> Arc<Scene3DSnapshot> {
    Arc::new(Scene3DSnapshot {
        structure_generation: 1,
        record_generation: 1,
        camera_generation: 1,
        viewport: Rect::new(0.0, 0.0, f64::from(SIDE), f64::from(SIDE)),
        camera: Some(camera),
        models,
        lights,
        bounds: None,
    })
}

fn ambient(level: u8) -> Scene3DLight {
    Scene3DLight::Ambient { color: Color::from_argb(255, level, level, level) }
}

/// A red quad with a green emissive layer under an orthographic camera moved by `offset` along x.
fn emissive_scene(offset: f32) -> Arc<Scene3DSnapshot> {
    let layers = vec![
        layer(Scene3DMaterialKind::Diffuse, solid(Color::from_argb(255, 255, 0, 0))),
        layer(Scene3DMaterialKind::Emissive, solid(Color::from_argb(255, 0, 64, 0))),
    ];
    snapshot(
        orthographic(offset),
        vec![model(quad(7_001, -0.5, -0.5, 0.5, 0.5, 0.0), layers, Vec::new())],
        vec![ambient(255)],
    )
}

/// Red to blue from the left of the box of the texture coordinates to its right, with `stops`
/// stops (more than 8 are sampled from the lookup texture).
fn horizontal_gradient(stops: usize) -> Option<SharedBrush> {
    let stops: Vec<ImmutableGradientStop> = (0..stops)
        .map(|index| {
            let f = index as f64 / (stops - 1) as f64;
            let level = (255.0 * f).round() as u8;
            ImmutableGradientStop::new(f, Color::from_argb(255, 255 - level, 0, level))
        })
        .collect();
    let brush = ImmutableLinearGradientBrush::new(
        &stops,
        1.0,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::TOP_LEFT),
        Some(RelativePoint::new(1.0, 0.0, RelativeUnit::Relative)),
        None,
    );
    SharedBrush::from_brush(&brush as &dyn IBrush)
}

/// A turned quad under a perspective camera lit by an ambient and a directional light, a quad
/// seen from its back with a `BackMaterial`, a material group with a specular and an emissive
/// layer, and a gradient through the lookup texture: what the CPU reference and the executor are
/// compared on.
fn compared_scenes() -> Vec<(&'static str, Arc<Scene3DSnapshot>)> {
    let turned = {
        let mut quad_model = model(
            quad(7_101, -1.0, -1.0, 1.0, 1.0, 0.0),
            vec![layer(Scene3DMaterialKind::Diffuse, solid(Color::from_argb(255, 200, 120, 40)))],
            Vec::new(),
        );
        quad_model.world = Matrix4x4::create_rotation_y(0.5);
        let light = Scene3DLight::Directional {
            color: Color::from_argb(255, 160, 160, 160),
            direction: Vector3::new(-0.3, -0.4, -1.0),
        };
        snapshot(perspective(Vector3::new(0.0, 0.0, 4.0)), vec![quad_model], vec![ambient(64), light])
    };
    let back = {
        let mut mesh = (*quad(7_102, -0.6, -0.6, 0.6, 0.6, 0.0)).clone();
        mesh.indices = vec![0, 2, 1, 0, 3, 2];
        mesh.normals = vec![Vector3::new(0.0, 0.0, -1.0); 4];
        let back_model = model(
            Arc::new(mesh),
            vec![layer(Scene3DMaterialKind::Diffuse, solid(Color::from_argb(255, 255, 0, 0)))],
            vec![layer(Scene3DMaterialKind::Diffuse, solid(Color::from_argb(255, 0, 0, 255)))],
        );
        snapshot(orthographic(0.0), vec![back_model], vec![ambient(255)])
    };
    let group = {
        let layers = vec![
            layer(Scene3DMaterialKind::Diffuse, solid(Color::from_argb(255, 90, 90, 90))),
            layer(Scene3DMaterialKind::Specular, solid(Color::from_argb(255, 255, 255, 255))),
            layer(Scene3DMaterialKind::Emissive, solid(Color::from_argb(255, 0, 0, 60))),
        ];
        let light = Scene3DLight::Directional {
            color: Color::from_argb(255, 255, 255, 255),
            direction: Vector3::new(0.2, -0.2, -1.0),
        };
        snapshot(
            perspective(Vector3::new(0.0, 0.0, 3.0)),
            vec![model(quad(7_103, -1.0, -1.0, 1.0, 1.0, 0.0), layers, Vec::new())],
            vec![ambient(40), light],
        )
    };
    let gradient = snapshot(
        orthographic(0.0),
        vec![model(
            quad(7_104, -0.8, -0.8, 0.8, 0.8, 0.0),
            vec![layer(Scene3DMaterialKind::Diffuse, horizontal_gradient(12))],
            Vec::new(),
        )],
        vec![ambient(255)],
    );
    vec![("turned", turned), ("back", back), ("group", group), ("gradient", gradient)]
}

/// How the image of the executor differs from the CPU reference: the largest difference of a
/// channel, and the share of the pixels whose largest difference is beyond `levels`.
fn difference(expected: &[u8], actual: &[u8], levels: u8) -> (u8, f64) {
    assert_eq!(expected.len(), actual.len());
    let (mut largest, mut beyond) = (0u8, 0usize);
    for (e, a) in expected.chunks_exact(4).zip(actual.chunks_exact(4)) {
        let pixel = e.iter().zip(a).map(|(e, a)| e.abs_diff(*a)).max().unwrap_or(0);
        largest = largest.max(pixel);
        beyond += usize::from(pixel > levels);
    }
    (largest, beyond as f64 / (expected.len() / 4).max(1) as f64)
}

/// With one sample a pixel (aliased edges), the executor and the CPU reference sample the same
/// points with the same fill rule: the images agree within three levels, but for the few pixels
/// whose centre lies on an edge.
#[test]
fn the_executor_draws_the_scenes_as_the_cpu_reference() {
    let Some(mut executor) = executor() else { return };
    let output = executor.create_output_texture(128, 128).expect("an output texture");
    let mut failures = Vec::new();
    for (name, scene) in compared_scenes() {
        for antialiased in [false, true] {
            let mut viewport = Scene3DMetalViewport::new();
            let target = Scene3DMetalTarget { texture: &output, size: (SIDE, SIDE), antialiased };
            executor.render(&mut viewport, &scene, &target).expect("a render");
            let actual = read_back(&executor, &output, (SIDE, SIDE));
            let size = PixelSize::new(SIDE as i32, SIDE as i32);
            let expected = cpu_render(&scene, size, 1.0, CpuRenderOptions { antialiased }).rgba;
            let drawn = expected.chunks_exact(4).filter(|pixel| pixel[3] > 0).count();
            assert!(drawn > (SIDE * SIDE / 10) as usize, "{name}: the reference draws the scene ({drawn} pixels)");
            let (largest, beyond) = difference(&expected, &actual, 3);
            println!("{name} (antialiased {antialiased}): largest {largest}, {:.2} % beyond 3 levels", 100.0 * beyond);
            // The edges of 4x MSAA (a rotated grid) and of the ordered grid of the reference
            // differ by a sample more or less; the faces do not.
            let share = if antialiased { 0.05 } else { 0.01 };
            if beyond > share {
                let share = 100.0 * beyond;
                let what = format!("{name} (antialiased {antialiased})");
                failures.push(format!("{what}: largest {largest}, {share:.2} % beyond 3 levels"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// An empty scene clears its image to transparent, and so does a scene without a camera.
#[test]
fn an_empty_scene_draws_nothing() {
    let Some(mut executor) = executor() else { return };
    let output = executor.create_output_texture(SIDE, SIDE).expect("an output texture");
    let target = Scene3DMetalTarget { texture: &output, size: (SIDE, SIDE), antialiased: true };
    let mut viewport = Scene3DMetalViewport::new();
    executor.render(&mut viewport, &emissive_scene(0.0), &target).expect("a render");
    let empty = snapshot(orthographic(0.0), Vec::new(), Vec::new());
    let report = executor.render(&mut viewport, &empty, &target).expect("a render");
    assert_eq!(0, report.draws);
    assert!(read_back(&executor, &output, (SIDE, SIDE)).iter().all(|&byte| byte == 0));
}

#[test]
fn pipelines_are_created_once_per_executor() {
    let Some(mut executor) = executor() else { return };
    let output = executor.create_output_texture(128, 128).expect("an output texture");
    let target = Scene3DMetalTarget { texture: &output, size: (SIDE, SIDE), antialiased: true };
    let mut viewport = Scene3DMetalViewport::new();
    let first = executor.render(&mut viewport, &emissive_scene(0.0), &target).expect("a render");
    // The diffuse and the emissive layer, and the resolve of decision N.
    assert_eq!(3, first.pipelines_created);
    assert_eq!(3, executor.pipeline_count());
    for offset in [0.1, 0.2, 0.3] {
        let report = executor.render(&mut viewport, &emissive_scene(offset), &target).expect("a render");
        assert_eq!(0, report.pipelines_created, "a steady frame makes no pipeline");
    }
    // The culling of the back faces and the depth test are state of the encoder, not pipelines.
    let back_faces = compared_scenes().into_iter().find(|(name, _)| *name == "back").map(|(_, scene)| scene).unwrap();
    let report = executor.render(&mut viewport, &back_faces, &target).expect("a render");
    assert_eq!(0, report.pipelines_created);
    assert_eq!(3, executor.pipeline_count());
}

/// Once every uniform slot of the viewport holds the draws, a camera that moves writes the frame
/// uniforms alone, and the same camera again writes nothing.
#[test]
fn a_moving_camera_writes_only_the_frame_uniforms() {
    let Some(mut executor) = executor() else { return };
    let output = executor.create_output_texture(128, 128).expect("an output texture");
    let target = Scene3DMetalTarget { texture: &output, size: (SIDE, SIDE), antialiased: true };
    let mut viewport = Scene3DMetalViewport::new();

    let first = executor.render(&mut viewport, &emissive_scene(0.0), &target).expect("a render");
    assert_eq!(1, first.meshes_uploaded);
    assert_eq!(2, first.draws);
    assert!(!first.is_uniform_only(), "the first render makes its buffers and targets");
    for slot in 1..UNIFORM_SLOTS {
        let report = executor.render(&mut viewport, &emissive_scene(0.0), &target).expect("a render");
        assert_eq!(0, report.meshes_uploaded, "slot {slot}");
    }

    for offset in [0.25, 0.5, 0.75, 1.0] {
        let moved = executor.render(&mut viewport, &emissive_scene(offset), &target).expect("a render");
        assert!(moved.is_uniform_only(), "{moved:?}");
        let frame = std::mem::size_of::<FrameUniforms>() as u64;
        assert_eq!(frame, moved.uniform_bytes, "only the frame uniforms are written");
    }
    // The same camera in every slot: nothing at all is written.
    for _ in 0..UNIFORM_SLOTS {
        executor.render(&mut viewport, &emissive_scene(1.0), &target).expect("a render");
    }
    let same = executor.render(&mut viewport, &emissive_scene(1.0), &target).expect("a render");
    assert_eq!(0, same.uniform_bytes);
}

/// The lookup texture of a gradient is uploaded once per device while it is drawn.
#[test]
fn the_texture_of_a_brush_is_uploaded_once() {
    let Some(mut executor) = executor() else { return };
    let output = executor.create_output_texture(SIDE, SIDE).expect("an output texture");
    let target = Scene3DMetalTarget { texture: &output, size: (SIDE, SIDE), antialiased: true };
    let mut viewport = Scene3DMetalViewport::new();
    let gradient = compared_scenes().into_iter().find(|(name, _)| *name == "gradient").map(|(_, scene)| scene).unwrap();
    let first = executor.render(&mut viewport, &gradient, &target).expect("a render");
    assert_eq!(1, first.textures_uploaded);
    assert_eq!(1, executor.texture_count());
    for _ in 0..4 {
        let report = executor.render(&mut viewport, &gradient, &target).expect("a render");
        assert_eq!(0, report.textures_uploaded);
    }
}
