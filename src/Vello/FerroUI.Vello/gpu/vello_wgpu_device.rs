use crate::scene::VelloScenePixelRect;
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
    /// The completions of the frames of producers (video, cameras) that work submitted to the
    /// queue reads: each is completed by the queue once that work is done
    /// ([`release_frames_after_submitted_work`](Self::release_frames_after_submitted_work)).
    frame_reads: Mutex<Vec<ferroui_base::rendering::composition::frames::GpuCompletion>>,
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
            frame_reads: Mutex::new(Vec::new()),
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

    /// Completes `completion` (which retains the frames of producers that the work submitted
    /// last reads) once the queue has done the work submitted to it so far: the producers get
    /// their storage back then, and not before. Nothing waits: the callback of the queue runs
    /// when the device is polled ([`poll_frame_reads`](Self::poll_frame_reads)) or submits again.
    pub fn release_frames_after_submitted_work(
        &self,
        completion: ferroui_base::rendering::composition::frames::GpuCompletion,
    ) {
        if completion.retained_frames() == 0 {
            completion.complete();
            return;
        }
        if self.is_lost() {
            // Whether a lost device ran the work is not known: the frames stay charged.
            completion.mark_indeterminate();
            return;
        }
        self.frame_reads.lock().unwrap_or_else(|e| e.into_inner()).push(completion.clone());
        self.queue.on_submitted_work_done(move || completion.complete());
    }

    /// Advances the completions of the frames the device reads without waiting: polls the
    /// device, which runs the callbacks of the work it has done. Returns whether frames are still
    /// read by submitted work. On a lost device every frame still read is marked indeterminate
    /// (its storage stays charged, as nothing proves the device stopped reading it).
    pub fn poll_frame_reads(&self) -> bool {
        use ferroui_base::rendering::composition::frames::GpuCompletionStatus;
        let mut reads = self.frame_reads.lock().unwrap_or_else(|e| e.into_inner());
        if reads.is_empty() {
            return false;
        }
        if !self.is_lost() && self.device.poll(wgpu::PollType::Poll).is_err() {
            self.mark_lost();
        }
        if self.is_lost() {
            for completion in reads.drain(..) {
                completion.mark_indeterminate();
            }
            return false;
        }
        reads.retain(|completion| completion.status() == GpuCompletionStatus::Pending);
        !reads.is_empty()
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

        crate::gpu::with_autorelease_pool(|| f(self, state))
    }

    /// Waits until the device has drawn everything that was submitted to
    /// it. Returns `false` when the device is lost.
    ///
    /// Not for the path of a frame: for what measures or reads a frame
    /// (a benchmark that times the work of the device, a test).
    pub fn wait_for_submitted_work(&self) -> bool {
        if self.device.poll(wgpu::PollType::wait_indefinitely()).is_err() {
            self.mark_lost();
            return false;
        }
        true
    }

    /// [`wait_for_submitted_work`](Self::wait_for_submitted_work) of the
    /// device scenes are drawn with ([`shared`](Self::shared)), when there
    /// is one.
    pub fn wait_for_shared_device() {
        let preferred = PREFERRED.lock().unwrap_or_else(|e| e.into_inner()).upgrade();
        if let Some(device) = preferred {
            device.wait_for_submitted_work();
        }
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
            usage: usage | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }

    /// Copies a rectangle of `size` pixels of a texture at `from` into
    /// another of the same format at `to`, as they are.
    pub fn copy_texture_region(
        &self,
        source: &wgpu::Texture,
        from: (u32, u32),
        target: &wgpu::Texture,
        to: (u32, u32),
        size: (u32, u32),
    ) {
        let _perf = crate::perf::scope(crate::perf::Phase::TextureCopy, u64::from(size.0) * u64::from(size.1) * 4);
        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("FerroUI Vello region") });
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: source,
                mip_level: 0,
                origin: wgpu::Origin3d { x: from.0, y: from.1, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: target,
                mip_level: 0,
                origin: wgpu::Origin3d { x: to.0, y: to.1, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
        );
        crate::gpu::with_autorelease_pool(|| {
            self.queue.submit([encoder.finish()]);
        });
    }

    /// Creates a texture that holds the given RGBA pixels, to be sampled.
    pub fn create_image_texture(&self, width: u32, height: u32, rgba: &[u8]) -> wgpu::Texture {
        let _perf = crate::perf::scope(crate::perf::Phase::Upload, rgba.len() as u64);
        crate::gpu::with_autorelease_pool(|| self.create_texture_of_pixels(width, height, rgba))
    }

    fn create_texture_of_pixels(&self, width: u32, height: u32, rgba: &[u8]) -> wgpu::Texture {
        let size = wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("FerroUI Vello image"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
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
    /// Creates a texture of pixels that are written to part by part
    /// ([`write_pixels_to_texture`](Self::write_pixels_to_texture)) and
    /// copied from: the copy on the device of a frame in memory.
    pub fn create_pixel_texture(&self, width: u32, height: u32) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("FerroUI Vello frame"),
            size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    /// Writes the pixels of a rectangle of RGBA pixels, `width` a row, into
    /// the same rectangle of a texture of their size.
    pub fn write_pixels_to_texture(&self, rgba: &[u8], width: u32, rect: VelloScenePixelRect, target: &wgpu::Texture) {
        let (x0, y0) = (u32::from(rect.x0).min(target.width()), u32::from(rect.y0).min(target.height()));
        let (x1, y1) = (u32::from(rect.x1).min(target.width()), u32::from(rect.y1).min(target.height()));
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let offset = (u64::from(y0) * u64::from(width) + u64::from(x0)) * 4;
        let _perf = crate::perf::scope(crate::perf::Phase::Upload, u64::from(x1 - x0) * u64::from(y1 - y0) * 4);
        crate::gpu::with_autorelease_pool(|| {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: target,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x: x0, y: y0, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                &rgba[offset as usize..],
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * width), rows_per_image: None },
                wgpu::Extent3d { width: x1 - x0, height: y1 - y0, depth_or_array_layers: 1 },
            );
        });
    }

    pub fn copy_pixels_to_texture(&self, rgba: &[u8], width: u32, height: u32, target: &wgpu::Texture) {
        let source = self.create_image_texture(width, height, rgba);
        let source_view = source.create_view(&wgpu::TextureViewDescriptor::default());
        self.copy_texture_to_texture(&source_view, target, false);
    }

    /// Copies a texture into a texture that can be rendered to, pixel on
    /// pixel, whatever the format of four 8 bit channels of the target is:
    /// how what a scene was rendered into reaches the texture of a window.
    /// With `premultiply` the colors of the source are multiplied by its
    /// alpha on the way: the source holds colors that are not
    /// premultiplied (what the renderer of the GPU mode writes), and the
    /// target takes premultiplied ones.
    pub fn copy_texture_to_texture(&self, source: &wgpu::TextureView, target: &wgpu::Texture, premultiply: bool) {
        /// What copies a texture into a target of a format.
        struct TextureCopyState {
            blitters: HashMap<(wgpu::TextureFormat, bool), wgpu::util::TextureBlitter>,
        }

        let _perf = crate::perf::scope(
            crate::perf::Phase::TextureCopy,
            u64::from(target.width()) * u64::from(target.height()) * 4,
        );
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let format = target.format();

        self.with_renderer_state(
            |_| TextureCopyState { blitters: HashMap::new() },
            |device, state| {
                let blitter = state.blitters.entry((format, premultiply)).or_insert_with(|| {
                    let builder = wgpu::util::TextureBlitterBuilder::new(&device.device, format)
                        .sample_type(wgpu::FilterMode::Nearest);
                    match premultiply {
                        true => {
                            let replace_by = |factor| wgpu::BlendComponent {
                                src_factor: factor,
                                dst_factor: wgpu::BlendFactor::Zero,
                                operation: wgpu::BlendOperation::Add,
                            };
                            builder
                                .blend_state(wgpu::BlendState {
                                    color: replace_by(wgpu::BlendFactor::SrcAlpha),
                                    alpha: replace_by(wgpu::BlendFactor::One),
                                })
                                .build()
                        }
                        false => builder.build(),
                    }
                });

                let mut encoder = device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("FerroUI Vello copy") });
                blitter.copy(&device.device, &mut encoder, source, &target_view);
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

        let _perf = crate::perf::scope(crate::perf::Phase::ReadBack, pixels.len() as u64);
        crate::gpu::with_autorelease_pool(|| self.read_texture_pixels(texture, pixels))
    }

    fn read_texture_pixels(&self, texture: &wgpu::Texture, pixels: &mut [u8]) -> bool {
        let (width, height) = (texture.width(), texture.height());
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
    // An adapter without compute shaders (OpenGL ES 3.0 and the like) has
    // none of their limits: the hybrid mode, which needs none, is asked for
    // the limits of WebGL2, the device it also draws on in a web page.
    let compute = adapter.get_downlevel_capabilities().flags.contains(wgpu::DownlevelFlags::COMPUTE_SHADERS);
    let base = if compute { wgpu::Limits::default() } else { wgpu::Limits::downlevel_webgl2_defaults() };

    wgpu::DeviceDescriptor {
        label: Some("FerroUI Vello"),
        required_features: adapter.features() & optional,
        required_limits: wgpu::Limits { max_texture_dimension_2d: adapter_limits.max_texture_dimension_2d, ..base },
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
