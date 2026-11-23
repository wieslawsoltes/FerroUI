//! 3D scenes on Graphite (#608, `docs/porting/media3d.md`, section 3.19): drawn on the Metal device
//! of the context by the Metal executor, held to the CPU reference, with the steady state of a
//! moving camera and of an unchanged viewport.

use super::{pixel, GraphiteTarget, TestMetalDevice};
use crate::ImmutableBitmap;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, SharedBrush};
use ferroui_base::numerics::{Matrix4x4, Vector2, Vector3};
use ferroui_base::platform::{IDrawingContextImpl, IDrawingContextImplWithScene3D};
use ferroui_base::rendering::scene3d::{
    MeshData, Scene3DBounds, Scene3DCamera, Scene3DCameraKind, Scene3DFrame, Scene3DLight, Scene3DMaterialKind,
    Scene3DMaterialLayer, Scene3DModel, Scene3DSnapshot,
};
use ferroui_base::{PixelSize, Rect};
use ferroui_render3d::cpu::render as cpu_render;
use ferroui_render3d::CpuRenderOptions;
use std::sync::Arc;

/// The side of the render target of [`GraphiteTarget::new`], in pixels.
const SIDE: i32 = 64;

fn layer(kind: Scene3DMaterialKind, color: Color) -> Scene3DMaterialLayer {
    Scene3DMaterialLayer {
        kind,
        color: Color::from_argb(255, 255, 255, 255),
        ambient_color: Color::from_argb(255, 255, 255, 255),
        specular_power: 0.0,
        brush: SharedBrush::from_brush(&ImmutableSolidColorBrush::new(color)),
    }
}

/// A quad from (-0.5, -0.5) to (0.5, 0.5) in the plane z = 0, facing +z.
fn quad() -> Arc<MeshData> {
    let positions = vec![
        Vector3::new(-0.5, -0.5, 0.0),
        Vector3::new(0.5, -0.5, 0.0),
        Vector3::new(0.5, 0.5, 0.0),
        Vector3::new(-0.5, 0.5, 0.0),
    ];
    let bounds = Scene3DBounds::of_points(positions.iter().copied());
    Arc::new(MeshData {
        id: 9_608_005,
        version: 1,
        positions,
        normals: vec![Vector3::new(0.0, 0.0, 1.0); 4],
        normals_generated: false,
        texture_coordinates: vec![Vector2::new(0.0, 1.0); 4],
        indices: vec![0, 1, 2, 0, 2, 3],
        bounds,
    })
}

/// A red quad with a green emissive layer lit by a white ambient light, under WPF's orthographic
/// camera at `(offset, 0, 5)` looking down -z with `Width` 2 and the planes 1 and 10.
fn scene(offset: f32) -> Arc<Scene3DSnapshot> {
    let (near, far) = (1.0f32, 10.0f32);
    let position = Vector3::new(offset, 0.0, 5.0);
    let view = Matrix4x4::create_look_at(position, Vector3::new(offset, 0.0, 0.0), Vector3::new(0.0, 1.0, 0.0));
    let m33 = 1.0 / (near - far);
    let projection =
        Matrix4x4::new(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, m33, 0.0, 0.0, 0.0, near * m33, 1.0);
    let camera = Scene3DCamera {
        kind: Scene3DCameraKind::Orthographic,
        view,
        projection,
        position,
        near_plane_distance: near,
        far_plane_distance: far,
    };
    let mesh = quad();
    let model = Scene3DModel {
        world: Matrix4x4::IDENTITY,
        world_bounds: mesh.bounds,
        mesh,
        front: vec![
            layer(Scene3DMaterialKind::Diffuse, Color::from_argb(255, 255, 0, 0)),
            layer(Scene3DMaterialKind::Emissive, Color::from_argb(255, 0, 64, 0)),
        ],
        back: Vec::new(),
    };
    Arc::new(Scene3DSnapshot {
        structure_generation: 1,
        record_generation: 1,
        camera_generation: 1,
        viewport: Rect::new(0.0, 0.0, f64::from(SIDE), f64::from(SIDE)),
        camera: Some(camera),
        models: vec![model],
        lights: vec![Scene3DLight::Ambient { color: Color::from_argb(255, 255, 255, 255) }],
        bounds: None,
    })
}

/// Draws `scene` over the whole target (aliased, as [`GraphiteTarget::draw`] draws), and flushes the
/// context as the end of a frame does.
fn draw(target: &GraphiteTarget, scene: &Arc<Scene3DSnapshot>) {
    target.draw(|context: &mut dyn IDrawingContextImpl| {
        let scenes = context.as_drawing_context_impl_with_scene_3d().expect("the Skia backend draws 3D scenes");
        let bounds = Rect::new(0.0, 0.0, f64::from(SIDE), f64::from(SIDE));
        let frame = Scene3DFrame { scene: scene.clone(), bounds };
        assert!(scenes.draw_scene_3d(&frame));
    });
    crate::gpu::ISkiaGrContext::flush(&*target.gr_context);
}

#[test]
fn graphite_draws_a_3d_scene_on_the_gpu_as_its_cpu_reference() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let target = GraphiteTarget::new(&device);
    let before = crate::scene3d_counters();
    let scene = scene(0.0);
    draw(&target, &scene);
    let after = crate::scene3d_counters();
    assert_eq!(1, after.scenes_drawn_on_gpu - before.scenes_drawn_on_gpu, "{after:?}");
    assert_eq!(1, after.passes - before.passes, "{after:?}");

    let expected = cpu_render(&scene, PixelSize::new(SIDE, SIDE), 1.0, CpuRenderOptions { antialiased: false });
    let snapshot = target.snapshot();
    let bitmap = snapshot.as_any().downcast_ref::<ImmutableBitmap>().expect("a raster snapshot");
    let mut different = Vec::new();
    for y in 0..SIDE {
        for x in 0..SIDE {
            let [r, g, b, a] = expected.pixel(x as u32, y as u32);
            let actual = pixel(bitmap, x, y);
            let close = |e: u8, a: u8| e.abs_diff(a) <= 2;
            let same = if a == 0 {
                actual.3 == 0
            } else {
                close(r, actual.0) && close(g, actual.1) && close(b, actual.2) && close(a, actual.3)
            };
            if !same {
                different.push((x, y, [r, g, b, a], actual));
            }
        }
    }
    // With one sample a pixel both sample the centres with the same fill rule.
    let first = &different[..different.len().min(8)];
    assert!(different.len() <= (SIDE * SIDE / 100) as usize, "{} pixels differ: {first:?}", different.len());
    // The quad covers the middle half, red with the green of the emissive layer added.
    assert_eq!((255, 64, 0, 255), pixel(bitmap, SIDE / 2, SIDE / 2));
    assert_eq!(0, pixel(bitmap, 2, 2).3);
    target.dispose();
}

/// An unchanged viewport draws its kept output: no 3D pass, no upload.
#[test]
fn graphite_draws_an_unchanged_3d_scene_from_its_kept_output() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let target = GraphiteTarget::new(&device);
    let scene = scene(0.0);
    draw(&target, &scene);
    let before = crate::scene3d_counters();
    for _ in 0..4 {
        draw(&target, &scene);
    }
    let after = crate::scene3d_counters();
    assert_eq!(0, after.passes - before.passes, "{after:?}");
    assert_eq!(4, after.cached_output_frames - before.cached_output_frames, "{after:?}");
    assert_eq!(0, after.uniform_bytes - before.uniform_bytes, "{after:?}");
    assert_eq!((255, 64, 0, 255), target.pixel(SIDE / 2, SIDE / 2));
    target.dispose();
}

/// A camera that moves, once the uniform slots of the viewport hold its draws, draws a 3D pass a
/// frame that writes uniforms and nothing else: no pipeline, mesh, texture or buffer.
#[test]
fn graphite_steady_frames_of_a_moving_camera_write_only_uniforms() {
    let Some(device) = TestMetalDevice::try_new() else {
        eprintln!("skipped: no Metal device available");
        return;
    };
    let target = GraphiteTarget::new(&device);
    for frame in 0..=ferroui_metal::scene3d::UNIFORM_SLOTS {
        draw(&target, &scene(0.01 * frame as f32));
    }
    let before = crate::scene3d_counters();
    for frame in 0..6 {
        draw(&target, &scene(0.1 + 0.05 * frame as f32));
    }
    let after = crate::scene3d_counters();
    assert_eq!(6, after.passes - before.passes, "{after:?}");
    assert_eq!(6, after.uniform_only_frames - before.uniform_only_frames, "{after:?}");
    assert_eq!(before.pipelines_created, after.pipelines_created, "{after:?}");
    assert_eq!(before.mesh_uploads, after.mesh_uploads, "{after:?}");
    assert_eq!(before.textures_allocated, after.textures_allocated, "{after:?}");
    assert_eq!(before.buffers_created, after.buffers_created, "{after:?}");
    // The frame uniforms of six frames.
    let frame_bytes = std::mem::size_of::<ferroui_render3d::uniforms::FrameUniforms>() as u64;
    assert_eq!(6 * frame_bytes, after.uniform_bytes - before.uniform_bytes, "{after:?}");
    target.dispose();
}
