//! The Vulkan executor on the Vulkan implementation of the machine (lavapipe in CI), held to the
//! CPU reference of `ferroui-render3d`. Without a loader or a device a test prints why it does
//! nothing and passes; with `FERROUI_VULKAN_TEST_REQUIRE=1` it fails instead.

use super::*;
use crate::{IVulkanPlatformGraphicsContext, VulkanInstanceCreationOptions, VulkanOptions, VulkanPlatformGraphics};
use crate::{VkGetInstanceProcAddressDelegate, VulkanPlatformSpecificOptions};
use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutableSolidColorBrush};
use ferroui_base::media::{Color, GradientSpreadMethod, IBrush, SharedBrush};
use ferroui_base::numerics::{Matrix4x4, Vector2, Vector3};
use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphics, IPlatformGraphicsContext};
use ferroui_base::rendering::scene3d::{
    Scene3DBounds, Scene3DCamera, Scene3DCameraKind, Scene3DLight, Scene3DMaterialKind,
};
use ferroui_base::{PixelSize, RelativePoint, RelativeUnit};
use ferroui_render3d::cpu::render as cpu_render;
use ferroui_render3d::CpuRenderOptions;
use std::ffi::CString;
use std::rc::Rc;

/// The side of the images of the tests, in pixels.
const SIDE: u32 = 100;

fn skip(reason: &str) {
    if std::env::var("FERROUI_VULKAN_TEST_REQUIRE").is_ok_and(|value| value == "1") {
        panic!("{reason} (FERROUI_VULKAN_TEST_REQUIRE=1)");
    }
    eprintln!("passed over: {reason}");
}

fn loader() -> Result<VkGetInstanceProcAddressDelegate, String> {
    // SAFETY: the library that is opened is the Vulkan loader, whose initialisation has no
    // preconditions.
    let entry = unsafe { ash::Entry::load() }.map_err(|error| format!("no Vulkan loader: {error}"))?;
    let entry = Arc::new(entry);
    Ok(Arc::new(move |instance: isize, name: &str| {
        let Ok(name) = CString::new(name) else { return 0 };
        // SAFETY: a zero-terminated name, and an instance of this loader or null; the entry keeps
        // the library open for as long as the lookup lives.
        let address = unsafe {
            (entry.static_fn().get_instance_proc_addr)(vk::Handle::from_raw(instance as u64), name.as_ptr())
        };
        address.map_or(0, |address| address as usize as isize)
    }))
}

/// A context of the machine's implementation and an executor of its device. The context is
/// returned so that it outlives the executor.
fn executor() -> Option<(Rc<dyn IPlatformGraphicsContext>, Scene3DVkExecutor)> {
    let get_proc_address = match loader() {
        Ok(loader) => loader,
        Err(reason) => {
            skip(&reason);
            return None;
        }
    };
    let platform_options =
        VulkanPlatformSpecificOptions { get_proc_address_delegate: Some(get_proc_address), ..Default::default() };
    let mut options = VulkanOptions {
        vulkan_instance_creation_options: VulkanInstanceCreationOptions {
            application_name: Some("ferroui-vulkan 3D tests".to_string()),
            require_surface_extension: false,
            ..Default::default()
        },
        ..Default::default()
    };
    options.vulkan_device_creation_options.require_swapchain_extension = false;
    let Some(graphics) = VulkanPlatformGraphics::try_create(&options, platform_options) else {
        skip("the Vulkan implementation has no instance or no device for these options");
        return None;
    };
    let context = graphics.create_context();
    let features: &dyn IOptionalFeatureProvider = &*context;
    let vulkan = features.try_get::<dyn IVulkanPlatformGraphicsContext>().expect("a Vulkan context");
    let executor = Scene3DVkExecutor::new(vulkan.device()).expect("the executor of the device");
    Some((context, executor))
}

/// The pixels of `size` at the top left of `texture`, after everything submitted before.
fn read_back(executor: &mut Scene3DVkExecutor, texture: &Scene3DVkTexture, size: (u32, u32)) -> Vec<u8> {
    let length = size.0 as usize * size.1 as usize * 4;
    let buffer = executor.create_buffer(length, vk::BufferUsageFlags::TRANSFER_DST).expect("a readback buffer");
    let device = executor.device.clone();
    let lock = device.lock();
    let range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .level_count(1)
        .layer_count(1);
    let barrier = |from, to, src, dst| {
        vk::ImageMemoryBarrier::default()
            .src_access_mask(src)
            .dst_access_mask(dst)
            .old_layout(from)
            .new_layout(to)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(texture.image())
            .subresource_range(range)
    };
    let to_copy = barrier(
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        vk::AccessFlags::MEMORY_WRITE,
        vk::AccessFlags::TRANSFER_READ,
    );
    let back = barrier(
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags::TRANSFER_READ,
        vk::AccessFlags::SHADER_READ,
    );
    let copy = vk::BufferImageCopy::default()
        .image_subresource(
            vk::ImageSubresourceLayers::default().aspect_mask(vk::ImageAspectFlags::COLOR).layer_count(1),
        )
        .image_extent(vk::Extent3D { width: size.0, height: size.1, depth: 1 });
    let host = vk::MemoryBarrier::default()
        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
        .dst_access_mask(vk::AccessFlags::HOST_READ);
    let api = &executor.api;
    let allocate = vk::CommandBufferAllocateInfo::default()
        .command_pool(executor.command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);
    // SAFETY: the device is locked; the image is in the layout a render leaves; the buffer holds
    // the region; the copy is waited for before the mapping is read.
    let bytes = unsafe {
        let command_buffer = api.allocate_command_buffers(&allocate).expect("a command buffer")[0];
        let begin = vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        api.begin_command_buffer(command_buffer, &begin).expect("recording");
        let none = vk::DependencyFlags::empty();
        api.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::TRANSFER,
            none,
            &[],
            &[],
            &[to_copy],
        );
        let layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
        api.cmd_copy_image_to_buffer(command_buffer, texture.image(), layout, buffer.buffer, &[copy]);
        api.cmd_pipeline_barrier(
            command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER | vk::PipelineStageFlags::HOST,
            none,
            &[host],
            &[],
            &[back],
        );
        api.end_command_buffer(command_buffer).expect("recording");
        let fence = api.create_fence(&vk::FenceCreateInfo::default(), None).expect("a fence");
        let buffers = [command_buffer];
        let submit = vk::SubmitInfo::default().command_buffers(&buffers);
        let queue = vk::Queue::from_raw(device.main_queue_handle() as u64);
        api.queue_submit(queue, &[submit], fence).expect("the readback is submitted");
        api.wait_for_fences(&[fence], true, u64::MAX).expect("the readback ran");
        api.destroy_fence(fence, None);
        api.free_command_buffers(executor.command_pool, &buffers);
        std::slice::from_raw_parts(buffer.mapped, length).to_vec()
    };
    lock.dispose();
    executor.destroy_buffer(buffer);
    bytes
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

/// With one sample a pixel the executor and the CPU reference sample the same points with the same
/// fill rule: the images agree within three levels, but for the few pixels whose centre lies on an
/// edge. This holds the orientation, the winding and the blending of the Vulkan route to the
/// reference.
#[test]
fn the_executor_draws_the_scenes_as_the_cpu_reference() {
    let Some((_context, mut executor)) = executor() else { return };
    let output = executor.create_output_texture(128, 128).expect("an output texture");
    let mut failures = Vec::new();
    for (name, scene) in compared_scenes() {
        for antialiased in [false, true] {
            let mut viewport = Scene3DVkViewport::new();
            let target = Scene3DVkTarget { texture: &output, size: (SIDE, SIDE), antialiased };
            executor.render(&mut viewport, &scene, &target).expect("a render");
            let actual = read_back(&mut executor, &output, (SIDE, SIDE));
            let size = PixelSize::new(SIDE as i32, SIDE as i32);
            let expected = cpu_render(&scene, size, 1.0, CpuRenderOptions { antialiased }).rgba;
            let drawn = expected.chunks_exact(4).filter(|pixel| pixel[3] > 0).count();
            assert!(drawn > (SIDE * SIDE / 10) as usize, "{name}: the reference draws the scene ({drawn} pixels)");
            let (largest, beyond) = difference(&expected, &actual, 3);
            println!("{name} (antialiased {antialiased}): largest {largest}, {:.2} % beyond 3 levels", 100.0 * beyond);
            let share = if antialiased { 0.05 } else { 0.01 };
            if beyond > share {
                let share = 100.0 * beyond;
                let what = format!("{name} (antialiased {antialiased})");
                failures.push(format!("{what}: largest {largest}, {share:.2} % beyond 3 levels"));
            }
            viewport.dispose(&mut executor);
        }
    }
    executor.delete_output_texture(output);
    executor.dispose();
    assert!(failures.is_empty(), "{failures:#?}");
}

/// An empty scene clears its image to transparent.
#[test]
fn an_empty_scene_draws_nothing() {
    let Some((_context, mut executor)) = executor() else { return };
    let output = executor.create_output_texture(SIDE, SIDE).expect("an output texture");
    let mut viewport = Scene3DVkViewport::new();
    {
        let target = Scene3DVkTarget { texture: &output, size: (SIDE, SIDE), antialiased: true };
        executor.render(&mut viewport, &emissive_scene(0.0), &target).expect("a render");
        let empty = snapshot(orthographic(0.0), Vec::new(), Vec::new());
        let report = executor.render(&mut viewport, &empty, &target).expect("a render");
        assert_eq!(0, report.draws);
    }
    assert!(read_back(&mut executor, &output, (SIDE, SIDE)).iter().all(|&byte| byte == 0));
    viewport.dispose(&mut executor);
    executor.delete_output_texture(output);
    executor.dispose();
}

/// Once every uniform slot of the viewport holds the draws, a camera that moves writes the frame
/// uniforms alone, and the same camera again writes nothing.
#[test]
fn a_moving_camera_writes_only_the_frame_uniforms() {
    let Some((_context, mut executor)) = executor() else { return };
    let output = executor.create_output_texture(128, 128).expect("an output texture");
    let mut viewport = Scene3DVkViewport::new();
    {
        let target = Scene3DVkTarget { texture: &output, size: (SIDE, SIDE), antialiased: true };
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
        for _ in 0..UNIFORM_SLOTS {
            executor.render(&mut viewport, &emissive_scene(1.0), &target).expect("a render");
        }
        let same = executor.render(&mut viewport, &emissive_scene(1.0), &target).expect("a render");
        assert_eq!(0, same.uniform_bytes);
    }
    viewport.dispose(&mut executor);
    executor.delete_output_texture(output);
    executor.dispose();
}

#[test]
fn every_module_is_spir_v() {
    for module in [
        spirv::VERTEX,
        spirv::FRAGMENT,
        spirv::GRADIENT_FRAGMENT,
        spirv::GRADIENT_TEXTURE_FRAGMENT,
        spirv::IMAGE_FRAGMENT,
        spirv::RESOLVE_VERTEX,
        spirv::RESOLVE_FRAGMENT,
    ] {
        assert_eq!(0, module.len() % 4);
        assert_eq!(0x0723_0203, spirv_words(module)[0], "the magic number of SPIR-V");
    }
}

#[test]
fn only_the_records_that_changed_are_written() {
    let mut memory = vec![0u8; 32];
    let buffer = MappedBuffer {
        buffer: vk::Buffer::null(),
        memory: vk::DeviceMemory::null(),
        mapped: memory.as_mut_ptr(),
        length: memory.len(),
    };
    let old = vec![1u8; 32];
    let mut new = old.clone();
    assert_eq!(None, write_changed(&buffer, &old, &new, 8));
    assert!(memory.iter().all(|&byte| byte == 0));
    new[9] = 7;
    new[17] = 7;
    assert_eq!(Some(16), write_changed(&buffer, &old, &new, 8));
    assert!(memory[..8].iter().all(|&byte| byte == 0), "the record before is not written");
    assert_eq!(new[8..24], memory[8..24]);
    assert!(memory[24..].iter().all(|&byte| byte == 0), "the record after is not written");
    assert_eq!(Some(32), write_changed(&buffer, &[], &new, 8), "a new length writes everything");
    assert_eq!(new, memory);
}

#[test]
fn a_report_of_uniforms_alone_is_the_steady_state() {
    let report = Scene3DVkRenderReport { sample_count: 4, draws: 3, uniform_bytes: 64, ..Default::default() };
    assert!(report.is_uniform_only());
    assert!(!Scene3DVkRenderReport { pipelines_created: 1, ..report }.is_uniform_only());
    assert!(!Scene3DVkRenderReport { meshes_uploaded: 1, ..report }.is_uniform_only());
    assert!(!Scene3DVkRenderReport { textures_uploaded: 1, ..report }.is_uniform_only());
    assert!(!Scene3DVkRenderReport { resources_created: 1, ..report }.is_uniform_only());
}
