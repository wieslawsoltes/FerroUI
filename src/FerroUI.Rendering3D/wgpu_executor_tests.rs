use super::*;
use crate::test_scenes::{diffuse, emissive, model, orthographic, quad, snapshot, white_ambient};
use ferroui_base::media::Color;
use ferroui_base::numerics::{Matrix4x4, Vector3};
use std::collections::HashSet;

#[test]
fn the_draw_stride_follows_the_alignment_of_the_device() {
    assert_eq!(176, std::mem::size_of::<DrawUniforms>());
    assert_eq!(256, draw_uniforms_stride(256));
    assert_eq!(192, draw_uniforms_stride(64));
    assert_eq!(176, draw_uniforms_stride(16));
    assert_eq!(176, draw_uniforms_stride(0));
    // The stride of the uniforms of the CPU reference is the alignment every API accepts.
    assert_eq!(crate::uniforms::DRAW_UNIFORMS_STRIDE, draw_uniforms_stride(256));
}

#[test]
fn a_pipeline_key_is_a_copy_key_of_its_fields() {
    let key = Scene3DPipelineKey {
        blend: Scene3DBlend::SourceOver,
        face: Face::Front,
        depth_write: true,
        sample_count: MULTISAMPLE_COUNT,
        color_format: COLOR_FORMAT,
        depth_format: DEPTH_FORMAT,
    };
    let copy = key;
    let mut keys = HashSet::new();
    keys.insert(key);
    keys.insert(copy);
    keys.insert(Scene3DPipelineKey { face: Face::Back, ..key });
    keys.insert(Scene3DPipelineKey { blend: Scene3DBlend::Additive, depth_write: false, ..key });
    keys.insert(Scene3DPipelineKey { sample_count: 1, ..key });
    assert_eq!(4, keys.len());
}

/// Waits for a future of wgpu that a native device completes when it is first asked.
fn block_on<F: Future>(future: F) -> F::Output {
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    let mut future = std::pin::pin!(future);
    for _ in 0..1_000_000 {
        if let std::task::Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        std::thread::yield_now();
    }
    panic!("a future of wgpu did not complete");
}

/// A device of the default adapter without a surface, or `None` on a machine without one (the
/// tests that need it then check nothing).
fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("FerroUI 3D executor tests"),
        required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
        ..wgpu::DeviceDescriptor::default()
    };
    block_on(adapter.request_device(&descriptor)).ok()
}

/// An output texture of the pool's form: 128 by 128 pixels, of which the scene draws 100 by 100.
fn output(device: &wgpu::Device) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("FerroUI 3D executor tests"),
            size: wgpu::Extent3d { width: 128, height: 128, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

/// A red quad with a green emissive layer under an orthographic camera moved by `offset` along x.
fn scene(offset: f32) -> std::sync::Arc<Scene3DSnapshot> {
    let mut camera = orthographic();
    camera.view = Matrix4x4::create_look_at(
        Vector3::new(offset, 0.0, 5.0),
        Vector3::new(offset, 0.0, 0.0),
        Vector3::new(0.0, 1.0, 0.0),
    );
    camera.position = Vector3::new(offset, 0.0, 5.0);
    let layers = vec![diffuse(Color::from_argb(255, 255, 0, 0)), emissive(Color::from_argb(255, 0, 64, 0))];
    snapshot(camera, vec![model(quad(7_001, -0.5, -0.5, 0.5, 0.5, 0.0), layers, Vec::new())], vec![white_ambient()])
}

#[test]
fn pipelines_are_created_once_per_device() {
    let Some((first_device, first_queue)) = device() else { return };
    let view = output(&first_device);
    let target = Scene3DTarget { view: &view, texture_size: (128, 128), size: (100, 100), antialiased: true };
    let mut executor = Scene3DWgpuExecutor::new(&first_device).expect("the executor of the device");
    let mut viewport = Scene3DViewport::new();

    let first = executor.render(&first_device, &first_queue, &mut viewport, &scene(0.0), &target).expect("a render");
    // The diffuse and the emissive layer, and the resolve of decision N.
    assert_eq!(3, first.pipelines_created);
    assert_eq!(3, executor.pipeline_count());
    for offset in [0.1, 0.2, 0.3] {
        let report = executor.render(&first_device, &first_queue, &mut viewport, &scene(offset), &target).expect("a render");
        assert_eq!(0, report.pipelines_created, "a steady frame makes no pipeline");
    }
    assert_eq!(3, executor.pipeline_count());

    // Another device has pipelines of its own.
    let Some((second_device, second_queue)) = device() else { return };
    let second_view = output(&second_device);
    let second_target = Scene3DTarget { view: &second_view, ..target };
    let mut second = Scene3DWgpuExecutor::new(&second_device).expect("the executor of the second device");
    let report = second
        .render(&second_device, &second_queue, &mut Scene3DViewport::new(), &scene(0.0), &second_target)
        .expect("a render");
    assert_eq!(3, report.pipelines_created);
}

#[test]
fn a_moving_camera_writes_only_the_frame_uniforms() {
    let Some((device, queue)) = device() else { return };
    let view = output(&device);
    let target = Scene3DTarget { view: &view, texture_size: (128, 128), size: (100, 100), antialiased: true };
    let mut executor = Scene3DWgpuExecutor::new(&device).expect("the executor of the device");
    let mut viewport = Scene3DViewport::new();

    let first = executor.render(&device, &queue, &mut viewport, &scene(0.0), &target).expect("a render");
    assert_eq!(1, first.meshes_uploaded);
    assert_eq!(2, first.draws);
    assert!(!first.is_uniform_only(), "the first render makes its buffers and targets");

    let moved = executor.render(&device, &queue, &mut viewport, &scene(0.25), &target).expect("a render");
    assert!(moved.is_uniform_only(), "{moved:?}");
    assert_eq!(std::mem::size_of::<FrameUniforms>() as u64, moved.uniform_bytes, "only the frame uniforms are written");

    // The same camera again writes nothing at all.
    let same = executor.render(&device, &queue, &mut viewport, &scene(0.25), &target).expect("a render");
    assert!(same.is_uniform_only());
    assert_eq!(0, same.uniform_bytes);
    assert_eq!(1, executor.mesh_count());
}
