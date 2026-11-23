//! The WebGPU device and the ProGPU engines on it (docs/porting/progpu-backend.md, section 2.2).

use crate::notes;
use crate::outline_log::{self, Entry};
use progpu_native_sys::progpu::*;
use progpu_native_sys::webgpu::*;
use std::cell::{Cell, RefCell};
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
    /// The WebGPU implementation could not map the buffer of a read-back.
    ReadBackFailed,
    /// A page cannot block on the GPU: pixels arrive through the mapping of a buffer, once the
    /// page has returned to its event loop.
    ReadBackNeedsEventLoop,
}

impl fmt::Display for ProGpuDeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProGpuDeviceError::AbiMismatch => write!(f, "the linked ProGPU engine has another ABI version than its bindings"),
            ProGpuDeviceError::NoAdapter => write!(f, "the WebGPU implementation has no adapter"),
            ProGpuDeviceError::NoDevice => write!(f, "the WebGPU implementation created no device"),
            ProGpuDeviceError::Engine(status) => write!(f, "the ProGPU engine refused the device (status {status})"),
            ProGpuDeviceError::ReadBackFailed => write!(f, "the pixels could not be read back from the GPU"),
            ProGpuDeviceError::ReadBackNeedsEventLoop => {
                write!(f, "in the browser pixels are read from the GPU asynchronously and are not available yet")
            }
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
    /// The streams the engine keeps, by scene, only while scene deltas are on: a delta is made
    /// against the last stream of the same target.
    retained_streams: RefCell<RetainedStreams>,
}

/// Whether frames are sent as the bytes that differ from the frame before
/// (`FERROUI_PROGPU_SCENE_DELTA=1`). Off by default: the engine still validates and hashes the
/// whole stream, so a delta has no measured gain yet and costs a copy of the stream here.
fn scene_deltas_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("FERROUI_PROGPU_SCENE_DELTA").is_some_and(|value| value == "1"))
}

/// The flags of the engines: `FERROUI_PROGPU_GROUP_PAGES=1` renders groups before the root pass.
/// Off by default until the frames of the gauntlet have been compared with it.
fn engine_flags() -> u64 {
    static FLAGS: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *FLAGS.get_or_init(|| {
        if std::env::var_os("FERROUI_PROGPU_GROUP_PAGES").is_some_and(|value| value == "1") {
            PROGPU_NATIVE_ENGINE_GROUPS_BEFORE_ROOT_PASS as u64
        } else {
            0
        }
    })
}

/// The bytes of the streams an engine keeps for the targets it is not rendering, while scene deltas
/// are on (the same bound here and in the engine). Without deltas it keeps one scene.
const KEPT_SCENES_BUDGET: u64 = 64 * 1024 * 1024;

/// The last stream of every scene, within [`KEPT_SCENES_BUDGET`] beside the active one.
#[derive(Default)]
struct RetainedStreams {
    streams: std::collections::HashMap<u64, Vec<u8>>,
    bytes: u64,
}

impl RetainedStreams {
    fn get(&self, scene: u64) -> Option<&[u8]> {
        self.streams.get(&scene).map(Vec::as_slice)
    }

    fn keep(&mut self, scene: u64, stream: &[u8]) {
        self.remove(scene);
        // Past the bound the other streams go: their next frames are sent whole.
        if self.bytes + stream.len() as u64 > KEPT_SCENES_BUDGET {
            self.streams.clear();
            self.bytes = 0;
        }
        self.bytes += stream.len() as u64;
        self.streams.insert(scene, stream.to_vec());
    }

    fn remove(&mut self, scene: u64) {
        if let Some(stream) = self.streams.remove(&scene) {
            self.bytes -= stream.len() as u64;
        }
    }
}

/// The unit in which two streams are compared for a delta.
const DELTA_BLOCK: usize = 256;

/// The runs of `next` that differ from `previous`, with their bytes, compared in blocks; what
/// lies past the end of `previous` is one run with what touches it.
pub(crate) fn scene_delta_ranges(previous: &[u8], next: &[u8]) -> (Vec<progpu_native_scene_delta_range>, Vec<u8>) {
    let common = previous.len().min(next.len());
    let (mut ranges, mut bytes): (Vec<progpu_native_scene_delta_range>, Vec<u8>) = (Vec::new(), Vec::new());
    let mut push = |start: usize, end: usize| {
        match ranges.last_mut() {
            Some(last) if (last.offset + last.size) as usize == start => last.size += (end - start) as u64,
            _ => ranges.push(progpu_native_scene_delta_range { offset: start as u64, size: (end - start) as u64 }),
        }
        bytes.extend_from_slice(&next[start..end]);
    };
    let mut at = 0;
    while at < common {
        let end = (at + DELTA_BLOCK).min(common);
        if previous[at..end] != next[at..end] {
            push(at, end);
        }
        at = end;
    }
    if next.len() > common {
        push(common, next.len());
    }
    (ranges, bytes)
}

/// The identity of a stream: its scene and its generation.
fn stream_identity(stream: &[u8]) -> Option<(u64, u64)> {
    if stream.len() < std::mem::size_of::<progpu_native_scene_header>() {
        return None;
    }
    // SAFETY: the stream holds a header, read without assuming its alignment.
    let header = unsafe { std::ptr::read_unaligned(stream.as_ptr() as *const progpu_native_scene_header) };
    Some((header.scene_id, header.generation))
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

#[cfg(not(target_os = "emscripten"))]
unsafe extern "C" fn on_adapter(status: WGPURequestAdapterStatus, adapter: WGPUAdapter, _: WGPUStringView, slot: *mut c_void, _: *mut c_void) {
    if status == WGPURequestAdapterStatus_Success {
        // SAFETY: `slot` is the `WGPUAdapter` slot of `ProGpuDevice::new`.
        unsafe { *(slot as *mut WGPUAdapter) = adapter };
    }
}

#[cfg(not(target_os = "emscripten"))]
unsafe extern "C" fn on_device(status: WGPURequestDeviceStatus, device: WGPUDevice, _: WGPUStringView, slot: *mut c_void, _: *mut c_void) {
    if status == WGPURequestDeviceStatus_Success {
        // SAFETY: `slot` is the `WGPUDevice` slot of `ProGpuDevice::new`.
        unsafe { *(slot as *mut WGPUDevice) = device };
    }
}

#[cfg(not(target_os = "emscripten"))]
unsafe extern "C" fn on_mapped(status: WGPUMapAsyncStatus, _: WGPUStringView, slot: *mut c_void, _: *mut c_void) {
    // SAFETY: `slot` is the flag of `ProGpuDevice::read_rgba8`.
    unsafe { *(slot as *mut i32) = if status == WGPUMapAsyncStatus_Success { 1 } else { -1 } };
}

impl ProGpuDevice {
    /// Creates a device on the default adapter of the WebGPU implementation: of the graphics API
    /// of the platform on Linux (Vulkan) and Windows (Direct3D 12), where the windows are drawn
    /// through surfaces of that API, and of any API without an adapter of it.
    #[cfg(not(target_os = "emscripten"))]
    pub fn new() -> Result<Self, ProGpuDeviceError> {
        #[cfg(target_os = "linux")]
        if let Ok(device) = Self::on_backends(WGPUInstanceBackend_Vulkan) {
            return Ok(device);
        }
        #[cfg(windows)]
        if let Ok(device) = Self::on_backends(WGPUInstanceBackend_DX12) {
            return Ok(device);
        }
        Self::on_backends(WGPUInstanceBackend_All)
    }

    /// The device the page requested before the module started, which the module cannot wait
    /// for itself (the option `preinitializedWebGPUDevice` of the module).
    #[cfg(target_os = "emscripten")]
    pub fn new() -> Result<Self, ProGpuDeviceError> {
        use progpu_native_sys::browser::{progpu_native_browser_device_queue, progpu_native_browser_imported_device};
        if !progpu_native_sys::abi_matches() {
            return Err(ProGpuDeviceError::AbiMismatch);
        }
        // SAFETY: plain queries of the module; a device that is not null is a reference owned here.
        unsafe {
            let device = progpu_native_browser_imported_device();
            if device == 0 {
                return Err(ProGpuDeviceError::NoDevice);
            }
            let queue = progpu_native_browser_device_queue(device);
            if queue == 0 {
                wgpuDeviceRelease(device as WGPUDevice);
                return Err(ProGpuDeviceError::NoDevice);
            }
            Ok(Self { instance: ptr::null_mut(), adapter: ptr::null_mut(), device: device as WGPUDevice, queue: queue as WGPUQueue })
        }
    }

    /// Creates a device on the default adapter of `backends`.
    #[cfg(not(target_os = "emscripten"))]
    fn on_backends(backends: WGPUInstanceBackend) -> Result<Self, ProGpuDeviceError> {
        if !progpu_native_sys::abi_matches() {
            return Err(ProGpuDeviceError::AbiMismatch);
        }
        // SAFETY: the calls of the WebGPU C ABI with the handles they return; the callbacks of
        // the requests write the slots they are given and run before the events are processed.
        unsafe {
            let mut extras: WGPUInstanceExtras = std::mem::zeroed();
            extras.chain.sType = WGPUSType_InstanceExtras as WGPUSType;
            extras.backends = backends;
            extras.flags = WGPUInstanceFlag_Default;
            let mut descriptor: WGPUInstanceDescriptor = std::mem::zeroed();
            descriptor.nextInChain = &mut extras.chain;
            // Without a choice of API the instance is the one of the defaults.
            let instance = wgpuCreateInstance(if backends == WGPUInstanceBackend_All { ptr::null() } else { &descriptor });
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
            #[cfg(not(target_os = "emscripten"))]
            unsafe {
                wgpuDevicePoll(self.device, 0, ptr::null())
            };
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

    /// The instance of the device, which the surfaces of windows are made on.
    #[cfg(any(target_os = "linux", windows))]
    pub(crate) fn instance(&self) -> WGPUInstance {
        self.enter();
        self.instance
    }

    /// The adapter of the device, which says what a surface can do.
    #[cfg(any(target_os = "linux", windows))]
    pub(crate) fn adapter(&self) -> WGPUAdapter {
        self.enter();
        self.adapter
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
                    usage: WGPUTextureUsage_RenderAttachment
                        | WGPUTextureUsage_CopySrc
                        | WGPUTextureUsage_CopyDst
                        | WGPUTextureUsage_TextureBinding,
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

    /// A texture of `RGBA8Unorm` that is written from the CPU and sampled.
    pub(crate) fn create_image_texture(&self, width: u32, height: u32) -> (WGPUTexture, WGPUTextureView) {
        self.enter();
        // SAFETY: the calls of the WebGPU C ABI with the device; the descriptor is borrowed.
        unsafe {
            let texture = wgpuDeviceCreateTexture(
                self.device,
                &WGPUTextureDescriptor {
                    nextInChain: ptr::null_mut(),
                    label: no_label(),
                    usage: WGPUTextureUsage_TextureBinding | WGPUTextureUsage_CopyDst | WGPUTextureUsage_CopySrc,
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

    /// Writes `width * height * 4` bytes of `RGBA8` into a texture.
    pub(crate) fn write_rgba8(&self, texture: WGPUTexture, width: u32, height: u32, pixels: &[u8]) {
        debug_assert_eq!(pixels.len(), width as usize * height as usize * 4);
        self.enter();
        // SAFETY: the texture is live and of that size; the bytes are borrowed for the call.
        unsafe {
            wgpuQueueWriteTexture(
                self.queue,
                &WGPUTexelCopyTextureInfo {
                    texture,
                    mipLevel: 0,
                    origin: WGPUOrigin3D { x: 0, y: 0, z: 0 },
                    aspect: WGPUTextureAspect_All,
                },
                pixels.as_ptr() as *const c_void,
                pixels.len(),
                &WGPUTexelCopyBufferLayout { offset: 0, bytesPerRow: width * 4, rowsPerImage: height },
                &WGPUExtent3D { width, height, depthOrArrayLayers: 1 },
            );
        }
    }

    /// Copies `width` by `height` pixels between two textures of `RGBA8Unorm`, on the queue.
    pub(crate) fn copy_texture(&self, source: WGPUTexture, destination: WGPUTexture, width: u32, height: u32) {
        self.enter();
        let origin = WGPUOrigin3D { x: 0, y: 0, z: 0 };
        // SAFETY: the calls of the WebGPU C ABI with live textures of this device; the records
        // are borrowed for the calls.
        unsafe {
            let encoder = wgpuDeviceCreateCommandEncoder(self.device, ptr::null());
            wgpuCommandEncoderCopyTextureToTexture(
                encoder,
                &WGPUTexelCopyTextureInfo { texture: source, mipLevel: 0, origin, aspect: WGPUTextureAspect_All },
                &WGPUTexelCopyTextureInfo { texture: destination, mipLevel: 0, origin, aspect: WGPUTextureAspect_All },
                &WGPUExtent3D { width, height, depthOrArrayLayers: 1 },
            );
            let commands = wgpuCommandEncoderFinish(encoder, ptr::null());
            wgpuQueueSubmit(self.queue, 1, &commands);
            wgpuCommandBufferRelease(commands);
            wgpuCommandEncoderRelease(encoder);
        }
    }

    /// Copies a texture of `RGBA8Unorm` into a new buffer that can be mapped, on the queue.
    fn begin_read_back(&self, texture: WGPUTexture, width: u32, height: u32) -> ReadBack {
        self.enter();
        // Rows of a copy are aligned to 256 bytes.
        let bytes_per_row = (width * 4).div_ceil(256) * 256;
        let size = u64::from(bytes_per_row) * u64::from(height);
        // SAFETY: the calls of the WebGPU C ABI with the handles they return.
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
            ReadBack { buffer, width, height, bytes_per_row, size: size as usize }
        }
    }

    /// Reads a texture of `RGBA8Unorm` back: `width * height * 4` bytes, row by row.
    #[cfg(not(target_os = "emscripten"))]
    pub fn try_read_rgba8(&self, texture: WGPUTexture, width: u32, height: u32) -> Result<Vec<u8>, ProGpuDeviceError> {
        let back = self.begin_read_back(texture, width, height);
        let mut mapped = 0i32;
        // SAFETY: the buffer is alive; the flag outlives the wait below, which ends when the
        // callback has written it.
        unsafe {
            wgpuBufferMapAsync(
                back.buffer,
                WGPUMapMode_Read,
                0,
                back.size,
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
        }
        back.finish(mapped == 1)
    }

    /// A page cannot wait for the GPU: [`read_rgba8_async`](Self::read_rgba8_async) is the read.
    #[cfg(target_os = "emscripten")]
    pub fn try_read_rgba8(&self, _texture: WGPUTexture, _width: u32, _height: u32) -> Result<Vec<u8>, ProGpuDeviceError> {
        Err(ProGpuDeviceError::ReadBackNeedsEventLoop)
    }

    /// Reads a texture of `RGBA8Unorm` back through the mapping of a buffer: `done` is called from
    /// the event loop of the page with `width * height * 4` bytes, row by row. The copy is on the
    /// queue when this returns, so the texture may change or go away afterwards.
    #[cfg(target_os = "emscripten")]
    pub fn read_rgba8_async(&self, texture: WGPUTexture, width: u32, height: u32, done: ReadBackDone) {
        let back = self.begin_read_back(texture, width, height);
        let (buffer, size) = (back.buffer, back.size);
        let pending = Box::into_raw(Box::new(PendingReadBack { back, done }));
        // SAFETY: the buffer is alive; `on_read_back_mapped` takes the box back exactly once.
        unsafe {
            wgpuBufferMapAsync(
                buffer,
                WGPUMapMode_Read,
                0,
                size,
                WGPUBufferMapCallbackInfo {
                    nextInChain: ptr::null_mut(),
                    mode: WGPUCallbackMode_AllowSpontaneous,
                    callback: Some(on_read_back_mapped),
                    userdata1: pending as *mut c_void,
                    userdata2: ptr::null_mut(),
                },
            );
        }
    }

    /// The pixels of a texture, for the tests.
    #[cfg(test)]
    pub(crate) fn read_rgba8(&self, texture: WGPUTexture, width: u32, height: u32) -> Vec<u8> {
        self.try_read_rgba8(texture, width, height).expect("the pixels of the texture")
    }
}

/// A buffer holding the copy of a texture, until it is mapped and read.
struct ReadBack {
    buffer: WGPUBuffer,
    width: u32,
    height: u32,
    bytes_per_row: u32,
    size: usize,
}

impl ReadBack {
    /// The pixels of a buffer that was mapped (`mapped`), and the release of the buffer.
    fn finish(self, mapped: bool) -> Result<Vec<u8>, ProGpuDeviceError> {
        // SAFETY: the buffer is owned here; the range is read while the buffer is mapped.
        unsafe {
            let result = if mapped {
                let data = std::slice::from_raw_parts(wgpuBufferGetConstMappedRange(self.buffer, 0, self.size) as *const u8, self.size);
                let row_bytes = self.width as usize * 4;
                let mut pixels = Vec::with_capacity(row_bytes * self.height as usize);
                for row in 0..self.height as usize {
                    let start = row * self.bytes_per_row as usize;
                    pixels.extend_from_slice(&data[start..start + row_bytes]);
                }
                wgpuBufferUnmap(self.buffer);
                Ok(pixels)
            } else {
                Err(ProGpuDeviceError::ReadBackFailed)
            };
            wgpuBufferRelease(self.buffer);
            result
        }
    }
}

/// What receives the pixels of an asynchronous read-back.
#[cfg(target_os = "emscripten")]
pub type ReadBackDone = Box<dyn FnOnce(Result<Vec<u8>, ProGpuDeviceError>)>;

#[cfg(target_os = "emscripten")]
struct PendingReadBack {
    back: ReadBack,
    done: ReadBackDone,
}

#[cfg(target_os = "emscripten")]
unsafe extern "C" fn on_read_back_mapped(status: WGPUMapAsyncStatus, _: WGPUStringView, pending: *mut c_void, _: *mut c_void) {
    // SAFETY: the box `read_rgba8_async` gave up, and the callback runs once.
    let PendingReadBack { back, done } = *unsafe { Box::from_raw(pending as *mut PendingReadBack) };
    done(back.finish(status == WGPUMapAsyncStatus_Success));
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
            if !self.instance.is_null() {
                wgpuInstanceRelease(self.instance);
            }
        }
    }
}

impl ProGpuEngine {
    /// An engine on `device` rendering into targets of `format`, resolving its procedures from the
    /// linked wgpu-native.
    pub fn new(device: &Rc<ProGpuDevice>, format: TargetFormat) -> Result<Self, ProGpuDeviceError> {
        let mut engine: *mut progpu_native_engine = ptr::null_mut();
        // SAFETY: the options are borrowed for the call; the handles are those of the device.
        let status = unsafe { Self::create(device, format, &mut engine) };
        if status == PROGPU_NATIVE_STATUS_SUCCESS {
            if scene_deltas_enabled() {
                // SAFETY: the engine was just created on this thread.
                unsafe { progpu_native_engine_set_scene_budget(engine, KEPT_SCENES_BUDGET) };
            }
            Ok(Self {
                device: device.clone(),
                engine,
                format,
                thread: Cell::new(std::thread::current().id()),
                outlines_seen: Cell::new(0),
                retained_streams: RefCell::default(),
            })
        } else {
            Err(ProGpuDeviceError::Engine(status))
        }
    }

    /// Creates the engine of the C ABI on the WebGPU of the page.
    #[cfg(target_os = "emscripten")]
    unsafe fn create(device: &ProGpuDevice, format: TargetFormat, engine: *mut *mut progpu_native_engine) -> progpu_native_status {
        use progpu_native_sys::browser::*;
        let options = progpu_native_browser_engine_options {
            struct_size: std::mem::size_of::<progpu_native_browser_engine_options>() as u32,
            native_abi_version: PROGPU_NATIVE_ABI_VERSION,
            adapter_abi_version: PROGPU_NATIVE_BROWSER_ADAPTER_ABI_VERSION,
            target_format: format.engine_format(),
            device: device.device as usize,
            queue: device.queue as usize,
            flags: engine_flags(),
            ..Default::default()
        };
        // SAFETY: the options are borrowed for the call; the slot is the caller's.
        unsafe { progpu_native_browser_engine_create(&options, engine) }
    }

    /// Creates the engine of the C ABI, resolving its procedures from the linked wgpu-native.
    #[cfg(not(target_os = "emscripten"))]
    unsafe fn create(device: &ProGpuDevice, format: TargetFormat, engine: *mut *mut progpu_native_engine) -> progpu_native_status {
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
            flags: engine_flags(),
        };
        // SAFETY: the options are borrowed for the call; the slot is the caller's.
        unsafe { progpu_native_dawn_engine_create(&options, engine) }
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
        if scene_deltas_enabled() {
            self.update_scene_by_delta(stream)
        } else {
            self.update_whole_scene(stream)
        }
    }

    /// Sends `stream` as the bytes that differ from the stream the engine retains when it is the
    /// next generation of that scene, and whole otherwise or if the engine refuses the delta.
    pub(crate) fn update_scene_by_delta(&self, stream: &[u8]) -> Result<progpu_native_scene_metrics, String> {
        let mut retained = self.retained_streams.borrow_mut();
        let identity = stream_identity(stream);
        let base = identity.and_then(|(scene, _)| retained.get(scene));
        let follows = base.and_then(stream_identity).zip(identity).is_some_and(
            |((scene, generation), (next_scene, next_generation))| scene == next_scene && generation < next_generation,
        );
        if follows {
            let previous = base.unwrap_or_default();
            let base_generation = stream_identity(previous).map_or(0, |(_, generation)| generation);
            let (ranges, bytes) = scene_delta_ranges(previous, stream);
            let engine = self.enter();
            self.register_outlines(engine);
            let delta = progpu_native_scene_delta {
                struct_size: std::mem::size_of::<progpu_native_scene_delta>() as u32,
                range_count: ranges.len() as u32,
                scene_id: stream_identity(stream).map_or(0, |(scene, _)| scene),
                base_generation,
                stream_size: stream.len() as u64,
                ranges: ranges.as_ptr(),
                bytes: bytes.as_ptr() as *const c_void,
                byte_count: bytes.len() as u64,
            };
            // SAFETY: the engine is alive; the record and its spans are borrowed for the call.
            let (status, metrics) = unsafe {
                let mut metrics: progpu_native_scene_metrics = std::mem::zeroed();
                metrics.struct_size = std::mem::size_of::<progpu_native_scene_metrics>() as u32;
                (progpu_native_engine_update_scene_delta(engine, &delta, &mut metrics), metrics)
            };
            if status == PROGPU_NATIVE_STATUS_SUCCESS {
                if let Some((scene, _)) = identity {
                    retained.keep(scene, stream);
                }
                return Ok(metrics);
            }
        }
        let metrics = self.update_whole_scene(stream)?;
        if let Some((scene, _)) = identity {
            retained.keep(scene, stream);
        }
        Ok(metrics)
    }

    /// Forgets what the engine keeps for a scene whose target is gone.
    pub(crate) fn release_scene(&self, scene_id: u64) {
        if thread_is_ending() || !scene_deltas_enabled() {
            return;
        }
        self.retained_streams.borrow_mut().remove(scene_id);
        // SAFETY: the engine is alive.
        unsafe { progpu_native_engine_release_scene(self.enter(), scene_id) };
    }

    fn update_whole_scene(&self, stream: &[u8]) -> Result<progpu_native_scene_metrics, String> {
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
        self.render_scene_damaged(scene_id, generation, target_view, width, height, preserve, None)
    }

    /// [`render_scene`](Self::render_scene) that replays only `damage` (`x, y, width, height` in
    /// pixels) over a target that keeps its content. The engine takes it for a flat scene only:
    /// on a scene with layers or masks it would clear the target, so the caller leaves it out.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render_scene_damaged(
        &self,
        scene_id: u64,
        generation: u64,
        target_view: WGPUTextureView,
        width: u32,
        height: u32,
        preserve: bool,
        damage: Option<[f32; 4]>,
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
                if let Some([x, y, damage_width, damage_height]) = damage {
                    frame.flags |= PROGPU_NATIVE_SCENE_FRAME_DAMAGE_RECT as u32;
                    frame.damage_x = x;
                    frame.damage_y = y;
                    frame.damage_width = damage_width;
                    frame.damage_height = damage_height;
                }
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

impl ProGpuEngine {
    /// What the page uploads cost since the last call; zero if the engine refuses the call, as
    /// the counts are only for measurements.
    pub(crate) fn take_upload_counters(&self) -> progpu_native_upload_counters {
        let mut counters = progpu_native_upload_counters {
            struct_size: std::mem::size_of::<progpu_native_upload_counters>() as u32,
            ..Default::default()
        };
        // SAFETY: the engine is alive and the counters outlive the call.
        let status = unsafe { progpu_native_engine_take_upload_counters(self.enter(), &mut counters) };
        if status == PROGPU_NATIVE_STATUS_SUCCESS {
            counters
        } else {
            progpu_native_upload_counters::default()
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
