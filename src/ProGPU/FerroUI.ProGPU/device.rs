//! The WebGPU device and the ProGPU engines on it (docs/porting/progpu-backend.md, section 2.2).

use crate::notes;
use crate::outline_log::{self, Entry};
use progpu_native_sys::progpu::*;
use progpu_native_sys::webgpu::*;
use std::cell::Cell;
use std::ffi::{c_char, c_void, CStr};
use std::fmt;
use std::ptr;
use std::rc::Rc;
use std::thread::ThreadId;

/// Why a device could not be created.
#[derive(Debug)]
pub enum ProGpuDeviceError {
    /// The linked engine has another ABI version than the bindings.
    AbiMismatch,
    /// The WebGPU implementation has no adapter or no device.
    NoAdapter,
    /// The WebGPU implementation refused the device.
    NoDevice,
    /// The engine refused the device.
    Engine(progpu_native_status),
}

impl fmt::Display for ProGpuDeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProGpuDeviceError::AbiMismatch => write!(f, "the linked ProGPU engine has another ABI version than its bindings"),
            ProGpuDeviceError::NoAdapter => write!(f, "the WebGPU implementation has no adapter"),
            ProGpuDeviceError::NoDevice => write!(f, "the WebGPU implementation created no device"),
            ProGpuDeviceError::Engine(status) => write!(f, "the ProGPU engine refused the device (status {status})"),
        }
    }
}

impl std::error::Error for ProGpuDeviceError {}

/// The mark of a thread that called the WebGPU implementation: a thread-local that is destroyed
/// with the thread-locals of the thread and holds nothing.
struct ThreadMark;

impl Drop for ThreadMark {
    fn drop(&mut self) {}
}

ferroui_base::ferro_thread_local! {
    /// Whether the calling thread has its mark. It has no destructor, so it can be read at any
    /// time, also while the thread-locals of the thread are destroyed.
    static ENTERED: Cell<bool> = const { Cell::new(false) };
    static MARK: ThreadMark = const { ThreadMark };
}

/// Whether the thread-locals of the calling thread are being destroyed, or are gone, after the
/// thread called the WebGPU implementation.
///
/// The WebGPU implementation is not called then, and what the backend holds of it is not
/// released: with debug assertions it keeps the state of its locks in a thread-local of its own
/// (to report a lock taken twice by one thread), which a release of a device reads and which is
/// destroyed with the thread, so a release made by a destructor of another thread-local, where
/// the framework keeps its compositor and so the context of the backend, would panic inside a
/// function of the C ABI. A thread that ends with a device ends the use of the device: its
/// objects are left to the end of the process, which is where a window application gets here.
///
/// Thread-locals are destroyed in the reverse order of their first use. The mark of a thread is
/// first used after a call that takes the lock of a device ([`ProGpuDevice::enter`]), so it is
/// destroyed before the lock state of the implementation: while the mark is alive, that state is.
pub(crate) fn thread_is_ending() -> bool {
    ENTERED.get() && MARK.try_with(|_| ()).is_err()
}

/// A WebGPU device of the statically linked wgpu-native: the device the engines of the backend
/// render with ([`ProGpuEngine`]), which call WebGPU through the procedures of that
/// implementation (`resolve_webgpu_proc`).
pub struct ProGpuDevice {
    instance: WGPUInstance,
    adapter: WGPUAdapter,
    device: WGPUDevice,
    queue: WGPUQueue,
}

/// The format of the targets an engine renders into (an engine has one, its pipelines are built
/// for it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TargetFormat {
    /// `RGBA8Unorm`: the offscreen targets of the backend.
    Rgba8,
    /// `BGRA8Unorm`: the drawables of a window.
    Bgra8,
}

impl TargetFormat {
    fn engine_format(self) -> u32 {
        match self {
            TargetFormat::Rgba8 => PROGPU_NATIVE_TEXTURE_FORMAT_RGBA8_UNORM as u32,
            TargetFormat::Bgra8 => PROGPU_NATIVE_TEXTURE_FORMAT_BGRA8_UNORM as u32,
        }
    }

    /// The WebGPU format.
    pub fn texture_format(self) -> WGPUTextureFormat {
        match self {
            TargetFormat::Rgba8 => WGPUTextureFormat_RGBA8Unorm,
            TargetFormat::Bgra8 => WGPUTextureFormat_BGRA8Unorm,
        }
    }
}

/// A same-device texture view a scene samples as an external image: the identity of the image
/// resource of the scene and the view bound to it for the frame.
#[derive(Clone, Copy, Debug)]
pub struct ExternalImage {
    /// The identifier of the image resource in the scene.
    pub resource_id: u64,
    /// The generation of the image resource in the scene.
    pub generation: u64,
    /// The view of the texture (borrowed for the frame; the engine retains it until the next
    /// table).
    pub view: WGPUTextureView,
    /// The width of the texture in pixels.
    pub width: u32,
    /// The height of the texture in pixels.
    pub height: u32,
}

/// An engine of ProGPU on a device: one retained scene (replaced by `update_scene`), rendered
/// into the views of textures of one format.
///
/// The engine is entered by one thread at a time. The compositor renders from more than one
/// thread under its lock (the thread of the render loop, and the thread of the window at the
/// synchronous points: a resize, the first frame), so every entry adopts the engine for the
/// calling thread when another thread entered it last (`progpu_native_engine_adopt_owner_thread`).
pub struct ProGpuEngine {
    device: Rc<ProGpuDevice>,
    engine: *mut progpu_native_engine,
    format: TargetFormat,
    thread: Cell<ThreadId>,
    /// The entry of the outline log the engine holds the glyph outlines up to.
    outlines_seen: Cell<u64>,
}

/// The most outlines and segments of one registration call: far below what the engine takes in
/// one call, so a long log is registered in pieces.
const REGISTRATION_OUTLINES: usize = 4096;
const REGISTRATION_SEGMENTS: usize = 1 << 18;

/// Outlines of the outline log gathered for one registration call.
#[derive(Default)]
struct Registration {
    outlines: Vec<progpu_native_registered_glyph_outline>,
    segments: Vec<progpu_native_path_segment>,
}

fn no_label() -> WGPUStringView {
    WGPUStringView { data: ptr::null(), length: 0 }
}

unsafe extern "C" fn on_adapter(status: WGPURequestAdapterStatus, adapter: WGPUAdapter, _: WGPUStringView, slot: *mut c_void, _: *mut c_void) {
    if status == WGPURequestAdapterStatus_Success {
        // SAFETY: `slot` is the `WGPUAdapter` slot of `ProGpuDevice::new`.
        unsafe { *(slot as *mut WGPUAdapter) = adapter };
    }
}

unsafe extern "C" fn on_device(status: WGPURequestDeviceStatus, device: WGPUDevice, _: WGPUStringView, slot: *mut c_void, _: *mut c_void) {
    if status == WGPURequestDeviceStatus_Success {
        // SAFETY: `slot` is the `WGPUDevice` slot of `ProGpuDevice::new`.
        unsafe { *(slot as *mut WGPUDevice) = device };
    }
}

unsafe extern "C" fn on_mapped(status: WGPUMapAsyncStatus, _: WGPUStringView, slot: *mut c_void, _: *mut c_void) {
    // SAFETY: `slot` is the flag of `ProGpuDevice::read_rgba8`.
    unsafe { *(slot as *mut i32) = if status == WGPUMapAsyncStatus_Success { 1 } else { -1 } };
}

impl ProGpuDevice {
    /// Creates a device on the default adapter of the WebGPU implementation.
    pub fn new() -> Result<Self, ProGpuDeviceError> {
        if !progpu_native_sys::abi_matches() {
            return Err(ProGpuDeviceError::AbiMismatch);
        }
        // SAFETY: the calls of the WebGPU C ABI with the handles they return; the callbacks of
        // the requests write the slots they are given and run before the events are processed.
        unsafe {
            let instance = wgpuCreateInstance(ptr::null());
            if instance.is_null() {
                return Err(ProGpuDeviceError::NoAdapter);
            }
            let mut adapter: WGPUAdapter = ptr::null_mut();
            wgpuInstanceRequestAdapter(
                instance,
                ptr::null(),
                WGPURequestAdapterCallbackInfo {
                    nextInChain: ptr::null_mut(),
                    mode: WGPUCallbackMode_AllowSpontaneous,
                    callback: Some(on_adapter),
                    userdata1: &mut adapter as *mut _ as *mut c_void,
                    userdata2: ptr::null_mut(),
                },
            );
            wgpuInstanceProcessEvents(instance);
            if adapter.is_null() {
                wgpuInstanceRelease(instance);
                return Err(ProGpuDeviceError::NoAdapter);
            }
            let mut device: WGPUDevice = ptr::null_mut();
            wgpuAdapterRequestDevice(
                adapter,
                ptr::null(),
                WGPURequestDeviceCallbackInfo {
                    nextInChain: ptr::null_mut(),
                    mode: WGPUCallbackMode_AllowSpontaneous,
                    callback: Some(on_device),
                    userdata1: &mut device as *mut _ as *mut c_void,
                    userdata2: ptr::null_mut(),
                },
            );
            wgpuInstanceProcessEvents(instance);
            if device.is_null() {
                wgpuAdapterRelease(adapter);
                wgpuInstanceRelease(instance);
                return Err(ProGpuDeviceError::NoDevice);
            }
            let queue = wgpuDeviceGetQueue(device);
            let device = Self { instance, adapter, device, queue };
            device.enter();
            Ok(device)
        }
    }

    /// Creates the device on a host's Metal device, with the host's command queue of it: the
    /// device of a window platform that presents its own frames (the drawables of its layer,
    /// inside its window's Core Animation transaction), so a frame an engine renders is committed
    /// to the queue before the host commits its presentation.
    ///
    /// The queue has to allow the command buffers of a whole frame at once: the WebGPU
    /// implementation keeps them (two for each render pass with wgpu) until the frame is
    /// submitted, and a queue with the default capacity of 64 blocks the thread that asks for one
    /// more. A host makes its queue with the capacity the implementation gives its own queues
    /// (4096).
    ///
    /// # Safety
    /// `mtl_device` is a live `id<MTLDevice>` and `mtl_queue` a live `id<MTLCommandQueue>` of it;
    /// both are retained by the device.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub unsafe fn from_metal(mtl_device: *mut c_void, mtl_queue: *mut c_void) -> Result<Self, ProGpuDeviceError> {
        if !progpu_native_sys::abi_matches() {
            return Err(ProGpuDeviceError::AbiMismatch);
        }
        // SAFETY: the calls of the WebGPU C ABI with the handles they return; the host objects are
        // valid (the contract of this function).
        unsafe {
            let instance = wgpuCreateInstance(ptr::null());
            if instance.is_null() {
                return Err(ProGpuDeviceError::NoAdapter);
            }
            let device = wgpuInstanceCreateDeviceFromMetal(instance, mtl_device, mtl_queue, ptr::null());
            if device.is_null() {
                wgpuInstanceRelease(instance);
                return Err(ProGpuDeviceError::NoDevice);
            }
            let queue = wgpuDeviceGetQueue(device);
            let device = Self { instance, adapter: ptr::null_mut(), device, queue };
            device.enter();
            Ok(device)
        }
    }

    /// Marks the calling thread as one that calls the WebGPU implementation, at its first entry
    /// (see [`thread_is_ending`]). The device is polled without waiting first, which takes the
    /// lock of the device once, so the lock state the implementation keeps for the thread exists
    /// before the mark does.
    pub(crate) fn enter(&self) {
        if !ENTERED.get() {
            // SAFETY: the device is alive; a poll that does not wait only completes what the
            // device has finished.
            unsafe { wgpuDevicePoll(self.device, 0, ptr::null()) };
            MARK.with(|_| ());
            ENTERED.set(true);
        }
    }

    /// Wraps a host's two-dimensional `id<MTLTexture>` of the device's Metal device as a texture
    /// of the device (one level and one sample, with the usages the host's texture allows:
    /// `usage`), with a view an engine renders into: the texture of a drawable, wrapped for the
    /// frame. Its content is treated as uninitialized.
    ///
    /// # Safety
    /// `mtl_texture` is a live `id<MTLTexture>` of the device's Metal device of that size and
    /// format, which its owner does not destroy while the texture is in use.
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub unsafe fn wrap_metal_texture(
        &self,
        mtl_texture: *mut c_void,
        width: u32,
        height: u32,
        format: WGPUTextureFormat,
        usage: WGPUTextureUsage,
    ) -> Option<(WGPUTexture, WGPUTextureView)> {
        self.enter();
        // SAFETY: the texture is valid (the contract of this function); the descriptor is borrowed.
        unsafe {
            let texture = wgpuDeviceCreateTextureFromMetal(
                self.device,
                mtl_texture,
                &WGPUTextureDescriptor {
                    nextInChain: ptr::null_mut(),
                    label: no_label(),
                    usage,
                    dimension: WGPUTextureDimension_2D,
                    size: WGPUExtent3D { width, height, depthOrArrayLayers: 1 },
                    format,
                    mipLevelCount: 1,
                    sampleCount: 1,
                    viewFormatCount: 0,
                    viewFormats: ptr::null(),
                },
            );
            if texture.is_null() {
                return None;
            }
            Some((texture, wgpuTextureCreateView(texture, ptr::null())))
        }
    }

    /// Releases a texture of the device and its view (`create_target`, `wrap_metal_texture`).
    /// They are left alone on a thread that is ending ([`thread_is_ending`]).
    pub fn release_texture(&self, texture: WGPUTexture, view: WGPUTextureView) {
        if thread_is_ending() {
            return;
        }
        // SAFETY: the handles are owned by the caller, which gives them up here.
        unsafe {
            wgpuTextureViewRelease(view);
            wgpuTextureRelease(texture);
        }
    }

    /// The WebGPU device.
    pub fn device(&self) -> WGPUDevice {
        self.enter();
        self.device
    }

    /// The queue of the device.
    pub fn queue(&self) -> WGPUQueue {
        self.enter();
        self.queue
    }

    /// Creates a texture of `RGBA8Unorm` that can be rendered to, sampled and read back, with its
    /// view.
    pub fn create_target(&self, width: u32, height: u32) -> (WGPUTexture, WGPUTextureView) {
        self.enter();
        // SAFETY: the calls of the WebGPU C ABI with the device; the descriptor is borrowed.
        unsafe {
            let texture = wgpuDeviceCreateTexture(
                self.device,
                &WGPUTextureDescriptor {
                    nextInChain: ptr::null_mut(),
                    label: no_label(),
                    usage: WGPUTextureUsage_RenderAttachment | WGPUTextureUsage_CopySrc | WGPUTextureUsage_TextureBinding,
                    dimension: WGPUTextureDimension_2D,
                    size: WGPUExtent3D { width, height, depthOrArrayLayers: 1 },
                    format: WGPUTextureFormat_RGBA8Unorm,
                    mipLevelCount: 1,
                    sampleCount: 1,
                    viewFormatCount: 0,
                    viewFormats: ptr::null(),
                },
            );
            let view = wgpuTextureCreateView(texture, ptr::null());
            (texture, view)
        }
    }

    /// Reads a texture of `RGBA8Unorm` back: `width * height * 4` bytes, row by row.
    pub fn read_rgba8(&self, texture: WGPUTexture, width: u32, height: u32) -> Vec<u8> {
        self.enter();
        // Rows of a copy are aligned to 256 bytes.
        let bytes_per_row = (width * 4).div_ceil(256) * 256;
        let size = u64::from(bytes_per_row) * u64::from(height);
        // SAFETY: the calls of the WebGPU C ABI with the handles they return; the mapped range is
        // read while the buffer is mapped.
        unsafe {
            let buffer = wgpuDeviceCreateBuffer(
                self.device,
                &WGPUBufferDescriptor {
                    nextInChain: ptr::null_mut(),
                    label: no_label(),
                    usage: WGPUBufferUsage_MapRead | WGPUBufferUsage_CopyDst,
                    size,
                    mappedAtCreation: 0,
                },
            );
            let encoder = wgpuDeviceCreateCommandEncoder(self.device, ptr::null());
            wgpuCommandEncoderCopyTextureToBuffer(
                encoder,
                &WGPUTexelCopyTextureInfo { texture, mipLevel: 0, origin: WGPUOrigin3D { x: 0, y: 0, z: 0 }, aspect: WGPUTextureAspect_All },
                &WGPUTexelCopyBufferInfo {
                    layout: WGPUTexelCopyBufferLayout { offset: 0, bytesPerRow: bytes_per_row, rowsPerImage: height },
                    buffer,
                },
                &WGPUExtent3D { width, height, depthOrArrayLayers: 1 },
            );
            let commands = wgpuCommandEncoderFinish(encoder, ptr::null());
            wgpuQueueSubmit(self.queue, 1, &commands);
            wgpuCommandBufferRelease(commands);
            wgpuCommandEncoderRelease(encoder);
            let mut mapped = 0i32;
            wgpuBufferMapAsync(
                buffer,
                WGPUMapMode_Read,
                0,
                size as usize,
                WGPUBufferMapCallbackInfo {
                    nextInChain: ptr::null_mut(),
                    mode: WGPUCallbackMode_AllowSpontaneous,
                    callback: Some(on_mapped),
                    userdata1: &mut mapped as *mut _ as *mut c_void,
                    userdata2: ptr::null_mut(),
                },
            );
            while mapped == 0 {
                wgpuDevicePoll(self.device, 1, ptr::null());
            }
            let mut pixels = Vec::with_capacity((width * height * 4) as usize);
            if mapped == 1 {
                let data = std::slice::from_raw_parts(wgpuBufferGetConstMappedRange(buffer, 0, size as usize) as *const u8, size as usize);
                for row in 0..height as usize {
                    let start = row * bytes_per_row as usize;
                    pixels.extend_from_slice(&data[start..start + (width * 4) as usize]);
                }
                wgpuBufferUnmap(buffer);
            }
            wgpuBufferRelease(buffer);
            pixels
        }
    }
}

impl Drop for ProGpuDevice {
    fn drop(&mut self) {
        // The device of a thread that is ending is not released (`thread_is_ending`).
        if thread_is_ending() {
            return;
        }
        // SAFETY: the handles are owned here.
        unsafe {
            wgpuQueueRelease(self.queue);
            wgpuDeviceRelease(self.device);
            if !self.adapter.is_null() {
                wgpuAdapterRelease(self.adapter);
            }
            wgpuInstanceRelease(self.instance);
        }
    }
}

impl ProGpuEngine {
    /// An engine on `device` rendering into targets of `format`, resolving its procedures from the
    /// linked wgpu-native.
    pub fn new(device: &Rc<ProGpuDevice>, format: TargetFormat) -> Result<Self, ProGpuDeviceError> {
        let options = progpu_native_dawn_engine_options {
            struct_size: std::mem::size_of::<progpu_native_dawn_engine_options>() as u32,
            native_abi_version: PROGPU_NATIVE_ABI_VERSION,
            adapter_abi_version: PROGPU_NATIVE_DAWN_ADAPTER_ABI_VERSION,
            provider_abi_version: PROGPU_NATIVE_DAWN_REQUIRED_PROVIDER_ABI_VERSION,
            target_format: format.engine_format(),
            reserved: 0,
            resolver_context: progpu_native_sys::resolver_context(),
            resolve_proc: Some(progpu_native_sys::resolve_webgpu_proc),
            instance: device.instance as usize,
            device: device.device as usize,
            queue: device.queue as usize,
            flags: 0,
        };
        let mut engine: *mut progpu_native_engine = ptr::null_mut();
        // SAFETY: the options are borrowed for the call; the handles are those of the device.
        let status = unsafe { progpu_native_dawn_engine_create(&options, &mut engine) };
        if status == PROGPU_NATIVE_STATUS_SUCCESS {
            Ok(Self { device: device.clone(), engine, format, thread: Cell::new(std::thread::current().id()), outlines_seen: Cell::new(0) })
        } else {
            Err(ProGpuDeviceError::Engine(status))
        }
    }

    /// The device of the engine.
    pub fn device(&self) -> &Rc<ProGpuDevice> {
        &self.device
    }

    /// The format of the targets the engine renders into.
    pub fn format(&self) -> TargetFormat {
        self.format
    }

    /// The engine, entered by the calling thread.
    fn enter(&self) -> *mut progpu_native_engine {
        self.device.enter();
        let thread = std::thread::current().id();
        if self.thread.get() != thread {
            // SAFETY: the engine is alive; the owner of the engine serializes its entries (the
            // contract of this type), so no other thread is inside it.
            unsafe { progpu_native_engine_adopt_owner_thread(self.engine) };
            self.thread.set(thread);
        }
        self.engine
    }

    /// The engine of the C ABI, entered by the calling thread (for the calls this type does not
    /// wrap).
    pub fn raw(&self) -> *mut progpu_native_engine {
        self.enter()
    }

    /// The last error the engine reported.
    pub fn last_error(&self) -> String {
        let mut message = [0 as c_char; 512];
        // SAFETY: the engine is alive; the buffer is owned here.
        unsafe {
            progpu_native_engine_get_last_error(self.enter(), message.as_mut_ptr(), message.len());
            CStr::from_ptr(message.as_ptr()).to_string_lossy().into_owned()
        }
    }

    /// Brings the glyph outlines of the engine up to the outline log: the outlines runs named
    /// since the engine read the log last are registered, the faces dropped since are released.
    /// A scene names its glyph outlines by key, so this comes before every scene.
    fn register_outlines(&self, engine: *mut progpu_native_engine) {
        let seen = self.outlines_seen.get();
        if outline_log::last() == seen {
            return;
        }
        let (entries, last) = outline_log::after(seen);
        let mut registration = Registration::default();
        for entry in &entries {
            match entry {
                Entry::Outline(outline) => {
                    if registration.outlines.len() >= REGISTRATION_OUTLINES
                        || (!registration.outlines.is_empty() && registration.segments.len() + outline.segments.len() > REGISTRATION_SEGMENTS)
                    {
                        self.register(engine, &mut registration);
                    }
                    registration.outlines.push(progpu_native_registered_glyph_outline {
                        key: outline.key(),
                        segment_offset: registration.segments.len() as u64,
                        segment_count: outline.segments.len() as u64,
                        min_x: outline.min_x,
                        min_y: outline.min_y,
                        max_x: outline.max_x,
                        max_y: outline.max_y,
                    });
                    registration.segments.extend_from_slice(&outline.segments);
                }
                Entry::Release(instance) => {
                    // In the order of the log: the outlines before the release of their face.
                    self.register(engine, &mut registration);
                    // SAFETY: the engine is alive and entered by this thread.
                    let status = unsafe { progpu_native_engine_release_glyph_face(engine, *instance) };
                    if status != PROGPU_NATIVE_STATUS_SUCCESS {
                        notes::note_once("glyph-face-release", &format!("the engine did not release the glyph outlines of a face (status {status}): {}", self.last_error()));
                    }
                }
            }
        }
        self.register(engine, &mut registration);
        self.outlines_seen.set(last);
    }

    /// Registers the gathered outlines with the engine and empties the gathering. The engine
    /// takes all outlines of a call or none: when it refuses the call, the outlines are
    /// registered one at a time, so one outline the engine does not take (it fails the frames
    /// that draw it, and says so) does not take the others with it.
    fn register(&self, engine: *mut progpu_native_engine, registration: &mut Registration) {
        if registration.outlines.is_empty() {
            return;
        }
        let (outlines, segments) = (&registration.outlines, &registration.segments);
        // SAFETY: the engine is alive and entered by this thread; the spans are borrowed for
        // the call and the engine copies what it keeps.
        let status = unsafe { progpu_native_engine_register_glyph_outlines(engine, outlines.as_ptr(), outlines.len(), segments.as_ptr(), segments.len()) };
        if status != PROGPU_NATIVE_STATUS_SUCCESS {
            for outline in outlines {
                // SAFETY: as above, one outline of the gathering.
                let status = unsafe { progpu_native_engine_register_glyph_outlines(engine, outline, 1, segments.as_ptr(), segments.len()) };
                if status != PROGPU_NATIVE_STATUS_SUCCESS {
                    notes::note_once(
                        "glyph-outline-registration",
                        &format!("the engine did not take the outline of glyph {} (status {status}): {}", outline.key.glyph, self.last_error()),
                    );
                }
            }
        }
        registration.outlines.clear();
        registration.segments.clear();
    }

    /// Sends a scene stream to the engine (`progpu_native_engine_update_scene`): only when its
    /// generation changed; the engine keeps the scene and renders it until the next stream.
    pub fn update_scene(&self, stream: &[u8]) -> Result<progpu_native_scene_metrics, String> {
        let engine = self.enter();
        self.register_outlines(engine);
        // SAFETY: the engine is alive; the stream is borrowed for the call.
        unsafe {
            let mut metrics: progpu_native_scene_metrics = std::mem::zeroed();
            metrics.struct_size = std::mem::size_of::<progpu_native_scene_metrics>() as u32;
            let status = progpu_native_engine_update_scene(
                engine,
                stream.as_ptr() as *const c_void,
                stream.len(),
                &mut metrics,
            );
            if status == PROGPU_NATIVE_STATUS_SUCCESS {
                Ok(metrics)
            } else {
                Err(format!("the engine refused the scene (status {status}, validation error {} at {}): {}", metrics.validation_error, metrics.error_offset, self.last_error()))
            }
        }
    }

    /// Binds the same-device texture views the retained scene samples as external images: the
    /// complete table of the scene (it replaces the one before).
    pub fn bind_external_images(&self, images: &[ExternalImage]) -> Result<(), String> {
        let bindings: Vec<progpu_native_scene_external_image_binding> = images
            .iter()
            .map(|image| progpu_native_scene_external_image_binding {
                struct_size: std::mem::size_of::<progpu_native_scene_external_image_binding>() as u32,
                flags: PROGPU_NATIVE_SCENE_EXTERNAL_IMAGE_PRIMARY as u32,
                resource_id: image.resource_id,
                generation: image.generation,
                texture_view: image.view as usize,
                width: image.width,
                height: image.height,
                reserved0: 0,
                reserved1: 0,
            })
            .collect();
        // SAFETY: the engine is alive; the table is borrowed for the call and the engine retains
        // the views of it.
        let status = unsafe { progpu_native_engine_bind_scene_external_images(self.enter(), bindings.as_ptr(), bindings.len()) };
        if status == PROGPU_NATIVE_STATUS_SUCCESS {
            Ok(())
        } else {
            Err(format!("the engine refused the external images of the scene (status {status}): {}", self.last_error()))
        }
    }

    /// Renders the retained scene `scene_id` at `generation` into `target_view` (a view of the
    /// engine's format of `width` by `height` pixels) and submits the work. With `preserve` the
    /// scene is drawn over the content of the target; without it the target is cleared to
    /// transparent first.
    pub fn render_scene(
        &self,
        scene_id: u64,
        generation: u64,
        target_view: WGPUTextureView,
        width: u32,
        height: u32,
        preserve: bool,
    ) -> Result<progpu_native_scene_frame_metrics, String> {
        // SAFETY: the engine is alive; the view is borrowed for the call.
        unsafe {
            let mut frame: progpu_native_scene_frame = std::mem::zeroed();
            frame.struct_size = std::mem::size_of::<progpu_native_scene_frame>() as u32;
            frame.width = width;
            frame.height = height;
            // The scenes of the backend are recorded in pixels (the compositor scales).
            frame.dpi_scale = 1.0;
            frame.target_view = target_view as usize;
            frame.scene_id = scene_id;
            frame.generation = generation;
            if preserve {
                frame.flags = PROGPU_NATIVE_SCENE_FRAME_PRESERVE_TARGET as u32;
            }
            if crate::perf::enabled() {
                frame.flags |= PROGPU_NATIVE_SCENE_FRAME_CAPTURE_CPU_STAGES as u32;
            }
            let mut metrics: progpu_native_scene_frame_metrics = std::mem::zeroed();
            metrics.struct_size = std::mem::size_of::<progpu_native_scene_frame_metrics>() as u32;
            let status = progpu_native_engine_render_scene(self.enter(), &frame, &mut metrics);
            if status == PROGPU_NATIVE_STATUS_SUCCESS {
                Ok(metrics)
            } else {
                Err(format!("the engine could not render the scene (status {status}): {}", self.last_error()))
            }
        }
    }
}

impl Drop for ProGpuEngine {
    fn drop(&mut self) {
        // The engine of a thread that is ending is not destroyed: destroying it releases its
        // objects of the device (`thread_is_ending`).
        if thread_is_ending() {
            return;
        }
        // SAFETY: the engine is owned here and goes before the device it retains.
        unsafe { progpu_native_engine_destroy(self.enter()) };
    }
}
