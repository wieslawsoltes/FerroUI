//! 3D scenes on the device of a GPU mode (#608, `docs/porting/media3d.md`, section 3.15, step
//! D4): the scene of a `DrawScene3D` is drawn by the wgpu executor of `ferroui-render3d` into a
//! texture of the pool (`effect_passes::pooled_texture`), which the drawing context paints into
//! its scene as it paints the other textures of the device. Nothing leaves the device.
//!
//! **The kept outputs.** Each device keeps up to [`CAPACITY`] outputs, each a texture of the pool
//! with the uniform buffers of its viewport ([`Scene3DViewport`]):
//!
//! 1. A scene whose snapshot, generations, size, scale and edge mode are those of a kept output is
//!    painted from that output: no 3D pass ([`VelloScene3DCounters::cached_output_frames`]).
//! 2. Otherwise the scene is drawn into the output of the same viewport (the one at the same
//!    device position and size), or of a scene that is gone, or a new one. The executor writes
//!    only the uniforms that changed, uploads only the meshes whose `(id, version)` is new, and
//!    makes no pipeline it has: a camera that moves writes the frame uniforms alone
//!    ([`VelloScene3DCounters::uniform_only_frames`]).
//! 3. An output still held by a scene that is not rendered yet is not drawn over: the viewport
//!    takes another texture of the pool, and the held one is the pool's again once that scene is
//!    rendered.
//!
//! **Failures.** An executor that cannot be made for a device, or a render that fails (the
//! executor already retried a failure of 4x MSAA with one sample), is reported once; the device
//! then draws its 3D scenes with the CPU reference of step D3, each counted
//! (`ferroui_render3d::scenes_drawn_on_cpu`) by the drawing context.
//!
//! The executors are kept per device and thread, as the pipelines of the other passes are, and
//! go with their device.

use super::{effect_passes, VelloDeviceTexture, VelloWgpuDevice};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::rendering::scene3d::Scene3DSnapshot;
use ferroui_base::PixelSize;
use ferroui_render3d::wgpu_executor::{
    gpu_counters, Scene3DTarget, Scene3DViewport, Scene3DWgpuError, Scene3DWgpuExecutor,
};
use std::cell::{Cell, RefCell};
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

/// The outputs a device keeps: the viewports of a window or two.
pub(crate) const CAPACITY: usize = 8;

/// A kept output: the texture a scene was drawn into, what it was drawn for, and the uniform
/// buffers of its viewport.
struct Slot {
    scene: Weak<Scene3DSnapshot>,
    generations: (u64, u64, u64),
    size: (u32, u32),
    scale_bits: u64,
    antialiased: bool,
    /// The device pixel of the top left of the viewport: what names a viewport whose snapshot
    /// changed.
    origin: (i32, i32),
    texture: Arc<VelloDeviceTexture>,
    viewport: Scene3DViewport,
    last_used: u64,
}

/// The executor of a device and its kept outputs.
struct DeviceScenes {
    device: Weak<VelloWgpuDevice>,
    /// The executor, or why the device draws its 3D scenes with the CPU reference.
    executor: Result<Scene3DWgpuExecutor, Scene3DWgpuError>,
    slots: Vec<Slot>,
    clock: u64,
}

/// What the drawing contexts of this thread did with the 3D scenes of the GPU modes, on top of
/// what the executors did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FrameCounters {
    cached_output_frames: u64,
    uniform_only_frames: u64,
    output_textures_allocated: u64,
    scenes_drawn_on_gpu: u64,
}

ferroui_base::ferro_thread_local! {
    // Kept beyond the end of the thread, as the objects of the other passes are (effect_passes.rs).
    static DEVICES: ManuallyDrop<RefCell<Vec<DeviceScenes>>> = ManuallyDrop::new(RefCell::new(Vec::new()));
    static COUNTERS: Cell<FrameCounters> = const {
        Cell::new(FrameCounters { cached_output_frames: 0, uniform_only_frames: 0, output_textures_allocated: 0, scenes_drawn_on_gpu: 0 })
    };
}

/// What the GPU modes did with 3D scenes on this thread since it started: what the steady-state
/// tests of `docs/porting/media3d.md` count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VelloScene3DCounters {
    /// Render pipelines made by the executors.
    pub pipelines_created: u64,
    /// Meshes uploaded (a new `(id, version)`).
    pub mesh_uploads: u64,
    /// Scenes drawn again with nothing new but uniforms: no mesh, pipeline, texture or buffer.
    pub uniform_only_frames: u64,
    /// Scenes painted from a kept output, without a 3D pass.
    pub cached_output_frames: u64,
    /// 3D passes submitted.
    pub passes: u64,
    /// Textures made for 3D scenes: the outputs taken from the pool that it had to make, and the
    /// multisampled colour, depth and resolved colour of the executors.
    pub textures_allocated: u64,
    /// Buffers made by the executors (vertices, indices, uniforms).
    pub buffers_created: u64,
    /// The bytes of uniforms written.
    pub uniform_bytes: u64,
    /// Scenes drawn on the device (a 3D pass or a kept output).
    pub scenes_drawn_on_gpu: u64,
}

/// What the GPU modes did with 3D scenes on this thread since it started.
pub fn scene3d_counters() -> VelloScene3DCounters {
    let executors = gpu_counters();
    let frames = COUNTERS.with(Cell::get);
    VelloScene3DCounters {
        pipelines_created: executors.pipelines_created,
        mesh_uploads: executors.mesh_uploads,
        uniform_only_frames: frames.uniform_only_frames,
        cached_output_frames: frames.cached_output_frames,
        passes: executors.passes,
        textures_allocated: executors.textures_created + frames.output_textures_allocated,
        buffers_created: executors.buffers_created,
        uniform_bytes: executors.uniform_bytes,
        scenes_drawn_on_gpu: frames.scenes_drawn_on_gpu,
    }
}

fn count(change: impl FnOnce(&mut FrameCounters)) {
    COUNTERS.with(|counters| {
        let mut value = counters.get();
        change(&mut value);
        counters.set(value);
    });
}

static FAILURE_REPORTED: AtomicBool = AtomicBool::new(false);

/// Reports once that a device draws its 3D scenes with the CPU reference, and why.
fn report_failure(error: &Scene3DWgpuError) {
    if FAILURE_REPORTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
        let reason = error.reason.as_str();
        logger.log_with_values(
            None,
            "The Vello backend draws 3D scenes (Viewport3D) with the CPU reference on this device: {Reason} \
             (docs/porting/media3d.md, section 3.15).",
            &[&reason],
        );
    }
}

/// A scene drawn on the device: the texture it is in, and the pixels of the image at its top left.
pub(crate) struct DeviceScene {
    pub(crate) texture: Arc<VelloDeviceTexture>,
    pub(crate) size: (u32, u32),
}

/// The texture of the device that holds `scene` drawn at `size` pixels (and `scale` device
/// pixels per unit, with or without antialiased edges), its top left at the device pixel
/// `origin`: a kept output when nothing changed, otherwise a new render. `None` when the device
/// cannot draw 3D scenes (reported once): the caller draws the CPU reference.
pub(crate) fn scene_texture(
    device: &Arc<VelloWgpuDevice>,
    scene: &Arc<Scene3DSnapshot>,
    size: PixelSize,
    scale: f64,
    antialiased: bool,
    origin: (i32, i32),
) -> Option<DeviceScene> {
    let size = (size.width.max(1) as u32, size.height.max(1) as u32);
    let scale_bits = scale.to_bits();
    let generations = scene.generations();
    DEVICES.with(|devices| {
        let mut devices = devices.borrow_mut();
        // The executors of devices that are gone or lost go with them.
        devices.retain(|entry| entry.device.upgrade().is_some_and(|device| !device.is_lost()));
        let index = match devices.iter().position(|entry| std::ptr::eq(entry.device.as_ptr(), Arc::as_ptr(device))) {
            Some(index) => index,
            None => {
                let executor = Scene3DWgpuExecutor::new(device.device());
                if let Err(error) = &executor {
                    report_failure(error);
                }
                devices.push(DeviceScenes { device: Arc::downgrade(device), executor, slots: Vec::new(), clock: 0 });
                devices.len() - 1
            }
        };
        let entry = &mut devices[index];
        let Ok(executor) = &mut entry.executor else { return None };
        entry.clock += 1;
        let clock = entry.clock;

        // 1. A kept output of this scene.
        let same_scene = |slot: &Slot| std::ptr::eq(slot.scene.as_ptr(), Arc::as_ptr(scene));
        if let Some(slot) = entry.slots.iter_mut().find(|slot| {
            let slot: &Slot = slot;
            same_scene(slot)
                && slot.generations == generations
                && slot.size == size
                && slot.scale_bits == scale_bits
                && slot.antialiased == antialiased
        }) {
            slot.last_used = clock;
            count(|counters| {
                counters.cached_output_frames += 1;
                counters.scenes_drawn_on_gpu += 1;
            });
            return Some(DeviceScene { texture: slot.texture.clone(), size });
        }

        // 2. The output to draw into: the same viewport (its position and size), the output of a
        //    scene that is gone, a new one, or the least recently used.
        let gone = |slot: &Slot| slot.scene.strong_count() == 0;
        let same_viewport = |slot: &Slot| slot.origin == origin && slot.size == size;
        let chosen = entry
            .slots
            .iter()
            .position(|slot| same_viewport(slot) && (gone(slot) || same_scene(slot)))
            .or_else(|| entry.slots.iter().position(same_viewport))
            .or_else(|| {
                entry
                    .slots
                    .iter()
                    .enumerate()
                    .filter(|&(_, slot)| gone(slot))
                    .min_by_key(|(_, slot)| slot.last_used)
                    .map(|(index, _)| index)
            });
        let (texture_width, texture_height) = effect_passes::pooled_size(device, size.0, size.1);
        let allocations = effect_passes::texture_allocations();
        let slot_index = match chosen {
            Some(index) => index,
            None if entry.slots.len() < CAPACITY => {
                let texture = effect_passes::pooled_texture(device, texture_width, texture_height);
                entry.slots.push(Slot {
                    scene: Weak::new(),
                    generations: (0, 0, 0),
                    size,
                    scale_bits,
                    antialiased,
                    origin,
                    texture,
                    viewport: Scene3DViewport::new(),
                    last_used: clock,
                });
                entry.slots.len() - 1
            }
            None => entry
                .slots
                .iter()
                .enumerate()
                .min_by_key(|(_, slot)| slot.last_used)
                .map(|(index, _)| index)
                .unwrap_or(0),
        };
        let slot = &mut entry.slots[slot_index];
        // A texture of another size, or one a scene that is not rendered yet still paints with
        // (held by more than the pool and this output): another of the pool.
        if slot.texture.width() != texture_width
            || slot.texture.height() != texture_height
            || Arc::strong_count(&slot.texture) > 2
        {
            slot.texture = effect_passes::pooled_texture(device, texture_width, texture_height);
        }
        let new_textures = effect_passes::texture_allocations() - allocations;

        let target = Scene3DTarget {
            view: slot.texture.view(),
            texture_size: (texture_width, texture_height),
            size,
            antialiased,
        };
        let rendered = super::with_autorelease_pool(|| {
            executor.render(device.device(), device.queue(), &mut slot.viewport, scene, &target)
        });
        match rendered {
            Ok(report) => {
                slot.scene = Arc::downgrade(scene);
                slot.generations = generations;
                slot.size = size;
                slot.scale_bits = scale_bits;
                slot.antialiased = antialiased;
                slot.origin = origin;
                slot.last_used = clock;
                let uniform_only = report.is_uniform_only() && new_textures == 0;
                count(|counters| {
                    counters.output_textures_allocated += new_textures;
                    counters.scenes_drawn_on_gpu += 1;
                    if uniform_only {
                        counters.uniform_only_frames += 1;
                    }
                });
                Some(DeviceScene { texture: slot.texture.clone(), size })
            }
            Err(error) => {
                // The device draws its 3D scenes with the CPU reference from now on.
                report_failure(&error);
                entry.slots.clear();
                entry.executor = Err(error);
                None
            }
        }
    })
}
