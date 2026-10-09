use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll, Waker};

/// Why the GPU modes have no device to draw with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VelloWgpuDeviceError {
    /// What failed, in the words of `wgpu`.
    pub reason: String,
}

impl std::fmt::Display for VelloWgpuDeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "No graphics device for the GPU modes of the Vello backend: {}", self.reason)
    }
}

impl std::error::Error for VelloWgpuDeviceError {}

/// The `wgpu` device and queue the hybrid and the GPU mode draw with, and
/// what the renderers keep on that device between frames.
///
/// A device is either made by `wgpu` itself, without a surface
/// ([`VelloWgpuDevice::headless`]: tests, render target bitmaps and layers
/// of an application without a window on the GPU), or over the graphics
/// device of the platform (`gpu/metal`: the window of a desktop).
///
/// `wgpu::Device` and `wgpu::Queue` are `Send + Sync` on native targets, and
/// so is this type: the state of the renderers is behind a lock that is
/// held for the length of a render, so that two threads that render in turn
/// (the UI thread and the render thread under the compositor lock) or at
/// once (tests) use a renderer one after the other.
pub struct VelloWgpuDevice {
    adapter_info: wgpu::AdapterInfo,
    device: wgpu::Device,
    queue: wgpu::Queue,
    supports_compute: bool,
    lost: Arc<AtomicBool>,
    /// What a renderer keeps on this device (its pipelines, its atlases,
    /// the textures of images), by the type of the state.
    renderer_states: Mutex<HashMap<TypeId, Box<dyn Any + Send>>>,
}

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<VelloWgpuDevice>();
};

/// The device made without a surface, once it was asked for, or why there
/// is none.
static HEADLESS: Mutex<Option<Result<Arc<VelloWgpuDevice>, VelloWgpuDeviceError>>> = Mutex::new(None);

/// The device of the platform graphics context that was created last: what
/// is drawn into memory (layers, render target bitmaps) while a window is
/// drawn on a device is drawn on the same device.
static PREFERRED: Mutex<Weak<VelloWgpuDevice>> = Mutex::new(Weak::new());

impl VelloWgpuDevice {
    /// Wraps a device and its queue.
    pub fn new(adapter: &wgpu::Adapter, device: wgpu::Device, queue: wgpu::Queue) -> Arc<Self> {
        let lost = Arc::new(AtomicBool::new(false));
        let flag = lost.clone();
        device.set_device_lost_callback(move |_reason, _message| flag.store(true, Ordering::SeqCst));

        let supports_compute =
            adapter.get_downlevel_capabilities().flags.contains(wgpu::DownlevelFlags::COMPUTE_SHADERS);

        Arc::new(Self {
            adapter_info: adapter.get_info(),
            device,
            queue,
            supports_compute,
            lost,
            renderer_states: Mutex::new(HashMap::new()),
        })
    }

    /// The device `wgpu` makes on the default adapter of the machine,
    /// without a surface: one for the process, created when it is first
    /// asked for. A device that was lost is replaced.
    pub fn headless() -> Result<Arc<Self>, VelloWgpuDeviceError> {
        let mut headless = HEADLESS.lock().unwrap_or_else(|e| e.into_inner());

        match &*headless {
            Some(Ok(device)) if !device.is_lost() => return Ok(device.clone()),
            Some(Err(error)) => return Err(error.clone()),
            _ => {}
        }

        let created = Self::create_headless();
        *headless = Some(created.clone());
        created
    }

    fn create_headless() -> Result<Arc<Self>, VelloWgpuDeviceError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: None,
            ..Default::default()
        }))
        .map_err(|error| VelloWgpuDeviceError { reason: format!("no adapter ({error})") })?;

        let (device, queue) = block_on(adapter.request_device(&device_descriptor(&adapter)))
            .map_err(|error| VelloWgpuDeviceError { reason: format!("the adapter gave no device ({error})") })?;

        Ok(Self::new(&adapter, device, queue))
    }

    /// The device scenes are drawn into memory with: the device of the
    /// platform graphics context when there is one, the headless device
    /// otherwise.
    pub fn shared() -> Result<Arc<Self>, VelloWgpuDeviceError> {
        let preferred = PREFERRED.lock().unwrap_or_else(|e| e.into_inner()).upgrade();

        match preferred {
            Some(device) if !device.is_lost() => Ok(device),
            _ => Self::headless(),
        }
    }

    /// Makes this device the one scenes are drawn into memory with, for as
    /// long as it lives.
    pub fn prefer(self: &Arc<Self>) {
        *PREFERRED.lock().unwrap_or_else(|e| e.into_inner()) = Arc::downgrade(self);
    }

    /// The device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// The queue of the device.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// The adapter of the device: its name and its graphics API.
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// Whether the device runs compute shaders, which the GPU mode needs
    /// and the hybrid mode does not.
    pub fn supports_compute(&self) -> bool {
        self.supports_compute
    }

    /// Whether the device was lost. A lost device draws nothing: its
    /// context is recreated by the compositor.
    pub fn is_lost(&self) -> bool {
        self.lost.load(Ordering::SeqCst)
    }

    /// Marks the device as lost; what `wgpu` itself reports arrives through
    /// its callback.
    pub fn mark_lost(&self) {
        self.lost.store(true, Ordering::SeqCst);
    }

    /// Runs `f` with the state a renderer keeps on this device, which is
    /// created by `create` the first time. The lock of the states is held
    /// while `f` runs: `f` must not ask for a state itself.
    pub fn with_renderer_state<S: Any + Send, R>(
        &self,
        create: impl FnOnce(&Self) -> S,
        f: impl FnOnce(&Self, &mut S) -> R,
    ) -> R {
        let mut states = self.renderer_states.lock().unwrap_or_else(|e| e.into_inner());
        let state = states.entry(TypeId::of::<S>()).or_insert_with(|| Box::new(create(self)));
        let state = state.downcast_mut::<S>().unwrap_or_else(|| panic!("A renderer state is of the type it is kept under"));

        f(self, state)
    }

    /// Creates a texture of premultiplied RGBA pixels a scene is rendered
    /// into and read back from. `usage` is added to what reading back
    /// needs.
    pub fn create_rgba_texture(&self, width: u32, height: u32, usage: wgpu::TextureUsages) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("FerroUI Vello target"),
            size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: usage | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    /// Creates a texture that holds the given RGBA pixels, to be sampled.
    pub fn create_image_texture(&self, width: u32, height: u32, rgba: &[u8]) -> wgpu::Texture {
        let size = wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("FerroUI Vello image"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * size.width), rows_per_image: Some(size.height) },
            size,
        );

        texture
    }

    /// Copies premultiplied RGBA pixels (`width * 4` bytes a row) into a
    /// texture that can be rendered to, whatever its format of four 8 bit
    /// channels: how a scene of the CPU mode reaches the texture of a
    /// window.
    pub fn copy_pixels_to_texture(&self, rgba: &[u8], width: u32, height: u32, target: &wgpu::Texture) {
        /// What copies a texture into a target of a format.
        struct PixelCopyState {
            blitters: HashMap<wgpu::TextureFormat, wgpu::util::TextureBlitter>,
        }

        let source = self.create_image_texture(width, height, rgba);
        let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let format = target.format();

        self.with_renderer_state(
            |_| PixelCopyState { blitters: HashMap::new() },
            |device, state| {
                let blitter = state.blitters.entry(format).or_insert_with(|| {
                    wgpu::util::TextureBlitterBuilder::new(&device.device, format)
                        .sample_type(wgpu::FilterMode::Nearest)
                        .build()
                });

                let mut encoder = device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("FerroUI Vello copy") });
                blitter.copy(&device.device, &mut encoder, &source_view, &target_view);
                device.queue.submit([encoder.finish()]);
            },
        );
    }

    /// Reads the pixels of a texture of four bytes a pixel that can be
    /// copied from, into `pixels`: `width * 4` bytes a row without padding.
    /// Waits for the device to finish what was submitted.
    ///
    /// Returns `false` when the device could not hand the pixels over (it
    /// was lost): `pixels` are left as they were.
    pub fn read_texture(&self, texture: &wgpu::Texture, pixels: &mut [u8]) -> bool {
        let (width, height) = (texture.width(), texture.height());
        assert_eq!(
            pixels.len(),
            width as usize * height as usize * 4,
            "The pixels of the target do not have the size of the scene"
        );

        let bytes_per_row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FerroUI Vello read back"),
            size: u64::from(bytes_per_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("FerroUI Vello read back") });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bytes_per_row), rows_per_image: None },
            },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        self.queue.submit([encoder.finish()]);

        let mapped = Arc::new(AtomicBool::new(false));
        let flag = mapped.clone();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| flag.store(result.is_ok(), Ordering::SeqCst));

        if self.device.poll(wgpu::PollType::wait_indefinitely()).is_err() || !mapped.load(Ordering::SeqCst) {
            self.mark_lost();
            return false;
        }

        {
            let Ok(view) = buffer.slice(..).get_mapped_range() else {
                self.mark_lost();
                return false;
            };
            let row = width as usize * 4;
            for (source, target) in view.chunks_exact(bytes_per_row as usize).zip(pixels.chunks_exact_mut(row)) {
                target.copy_from_slice(&source[..row]);
            }
        }
        buffer.unmap();

        true
    }
}

/// What the renderers ask of a device: no feature that every adapter does
/// not have, and textures as large as the adapter makes them (a window on
/// a large display at a scaling of two is wider than the 8192 pixels of
/// the default limits).
pub(crate) fn device_descriptor(adapter: &wgpu::Adapter) -> wgpu::DeviceDescriptor<'static> {
    let optional = wgpu::Features::CLEAR_TEXTURE;
    let adapter_limits = adapter.limits();

    wgpu::DeviceDescriptor {
        label: Some("FerroUI Vello"),
        required_features: adapter.features() & optional,
        required_limits: wgpu::Limits {
            max_texture_dimension_2d: adapter_limits.max_texture_dimension_2d,
            ..wgpu::Limits::default()
        },
        ..Default::default()
    }
}

/// Waits for a future of `wgpu`. On native targets these are complete when
/// they are first asked: nothing is waited for.
///
/// # Panics
/// Panics for a future that is not complete after it was asked a number of
/// times: this is not an executor.
pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    let mut future = std::pin::pin!(future);

    for _ in 0..1_000_000 {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }

    panic!("A future of wgpu did not complete");
}
