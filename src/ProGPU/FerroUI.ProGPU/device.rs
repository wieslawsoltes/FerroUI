//! The WebGPU device and the ProGPU engine on it (docs/porting/progpu-backend.md, section 2.2).

use progpu_native_sys::progpu::*;
use progpu_native_sys::webgpu::*;
use std::ffi::{c_char, c_void, CStr};
use std::fmt;
use std::ptr;

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

/// A WebGPU device of the statically linked wgpu-native with the ProGPU engine on it, which
/// calls WebGPU through the procedures of that implementation (`resolve_webgpu_proc`).
///
/// The engine renders into the views of textures of `RGBA8Unorm`, the target format it is created
/// for. Every call is on the thread that created the device (the engine refuses others).
pub struct ProGpuDevice {
    instance: WGPUInstance,
    adapter: WGPUAdapter,
    device: WGPUDevice,
    queue: WGPUQueue,
    engine: *mut progpu_native_engine,
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
    /// Creates a device on the default adapter of the WebGPU implementation and the engine on it.
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
            match Self::create_engine(instance, device, queue) {
                Ok(engine) => Ok(Self { instance, adapter, device, queue, engine }),
                Err(status) => {
                    wgpuQueueRelease(queue);
                    wgpuDeviceRelease(device);
                    wgpuAdapterRelease(adapter);
                    wgpuInstanceRelease(instance);
                    Err(ProGpuDeviceError::Engine(status))
                }
            }
        }
    }

    /// Creates the device on a host's Metal device, with the host's command queue of it, and the
    /// engine on it: the device of a window platform that presents its own frames (the drawables
    /// of its layer, inside its window's Core Animation transaction), so a frame the engine
    /// renders is committed to the queue before the host commits its presentation. The queue must
    /// have room for the command buffers a frame leaves uncommitted until it is submitted
    /// (`newCommandQueueWithMaxCommandBufferCount`, 4096 as wgpu gives its own queues).
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
            match Self::create_engine(instance, device, queue) {
                Ok(engine) => Ok(Self { instance, adapter: ptr::null_mut(), device, queue, engine }),
                Err(status) => {
                    wgpuQueueRelease(queue);
                    wgpuDeviceRelease(device);
                    wgpuInstanceRelease(instance);
                    Err(ProGpuDeviceError::Engine(status))
                }
            }
        }
    }

    /// Wraps a host's two-dimensional `id<MTLTexture>` of the device's Metal device as a texture
    /// of the device (`RGBA8Unorm` or `BGRA8Unorm`, one level and one sample, usable as a render
    /// attachment, a copy source and a sampled texture), with a view the engine renders into: the
    /// texture of a drawable, wrapped for the frame. Its content is treated as uninitialized.
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
    ) -> Option<(WGPUTexture, WGPUTextureView)> {
        // SAFETY: the texture is valid (the contract of this function); the descriptor is borrowed.
        unsafe {
            let texture = wgpuDeviceCreateTextureFromMetal(
                self.device,
                mtl_texture,
                &WGPUTextureDescriptor {
                    nextInChain: ptr::null_mut(),
                    label: no_label(),
                    usage: WGPUTextureUsage_RenderAttachment | WGPUTextureUsage_CopySrc | WGPUTextureUsage_TextureBinding,
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

    /// The engine on `device`, resolving its procedures from the linked wgpu-native.
    ///
    /// # Safety
    /// The handles are live handles of the linked implementation, `queue` the queue of `device`.
    unsafe fn create_engine(
        instance: WGPUInstance,
        device: WGPUDevice,
        queue: WGPUQueue,
    ) -> Result<*mut progpu_native_engine, progpu_native_status> {
        let options = progpu_native_dawn_engine_options {
            struct_size: std::mem::size_of::<progpu_native_dawn_engine_options>() as u32,
            native_abi_version: PROGPU_NATIVE_ABI_VERSION,
            adapter_abi_version: PROGPU_NATIVE_DAWN_ADAPTER_ABI_VERSION,
            provider_abi_version: PROGPU_NATIVE_DAWN_REQUIRED_PROVIDER_ABI_VERSION,
            target_format: PROGPU_NATIVE_TEXTURE_FORMAT_RGBA8_UNORM as u32,
            reserved: 0,
            resolver_context: progpu_native_sys::resolver_context(),
            resolve_proc: Some(progpu_native_sys::resolve_webgpu_proc),
            instance: instance as usize,
            device: device as usize,
            queue: queue as usize,
            flags: 0,
        };
        let mut engine: *mut progpu_native_engine = ptr::null_mut();
        // SAFETY: the options are borrowed for the call; the handles are valid (the contract).
        let status = unsafe { progpu_native_dawn_engine_create(&options, &mut engine) };
        if status == PROGPU_NATIVE_STATUS_SUCCESS { Ok(engine) } else { Err(status) }
    }

    /// The engine.
    pub fn engine(&self) -> *mut progpu_native_engine {
        self.engine
    }

    /// The WebGPU device.
    pub fn device(&self) -> WGPUDevice {
        self.device
    }

    /// The queue of the device.
    pub fn queue(&self) -> WGPUQueue {
        self.queue
    }

    /// The last error the engine reported.
    pub fn last_error(&self) -> String {
        let mut message = [0 as c_char; 512];
        // SAFETY: the engine is alive; the buffer is owned here.
        unsafe {
            progpu_native_engine_get_last_error(self.engine, message.as_mut_ptr(), message.len());
            CStr::from_ptr(message.as_ptr()).to_string_lossy().into_owned()
        }
    }

    /// Sends a scene stream to the engine (`progpu_native_engine_update_scene`): only when its
    /// generation changed; the engine keeps the scene and renders it until the next stream.
    pub fn update_scene(&self, stream: &[u8]) -> Result<progpu_native_scene_metrics, String> {
        // SAFETY: the engine is alive; the stream is borrowed for the call.
        unsafe {
            let mut metrics: progpu_native_scene_metrics = std::mem::zeroed();
            metrics.struct_size = std::mem::size_of::<progpu_native_scene_metrics>() as u32;
            let status = progpu_native_engine_update_scene(
                self.engine,
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

    /// Renders the retained scene `scene_id` at `generation` into `target_view` (an
    /// `RGBA8Unorm` view of `width` by `height` pixels) and submits the work.
    pub fn render_scene(
        &self,
        scene_id: u64,
        generation: u64,
        target_view: WGPUTextureView,
        width: u32,
        height: u32,
        dpi_scale: f32,
    ) -> Result<progpu_native_scene_frame_metrics, String> {
        // SAFETY: the engine is alive; the view is borrowed for the call.
        unsafe {
            let mut frame: progpu_native_scene_frame = std::mem::zeroed();
            frame.struct_size = std::mem::size_of::<progpu_native_scene_frame>() as u32;
            frame.width = width;
            frame.height = height;
            frame.dpi_scale = dpi_scale;
            frame.target_view = target_view as usize;
            frame.scene_id = scene_id;
            frame.generation = generation;
            let mut metrics: progpu_native_scene_frame_metrics = std::mem::zeroed();
            metrics.struct_size = std::mem::size_of::<progpu_native_scene_frame_metrics>() as u32;
            let status = progpu_native_engine_render_scene(self.engine, &frame, &mut metrics);
            if status == PROGPU_NATIVE_STATUS_SUCCESS {
                Ok(metrics)
            } else {
                Err(format!("the engine could not render the scene (status {status}): {}", self.last_error()))
            }
        }
    }

    /// Creates a texture of `RGBA8Unorm` that can be rendered to and read back, with its view.
    pub fn create_target(&self, width: u32, height: u32) -> (WGPUTexture, WGPUTextureView) {
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
        // SAFETY: the handles are owned here; the engine goes before the device it retains.
        unsafe {
            progpu_native_engine_destroy(self.engine);
            wgpuQueueRelease(self.queue);
            wgpuDeviceRelease(self.device);
            if !self.adapter.is_null() {
                wgpuAdapterRelease(self.adapter);
            }
            wgpuInstanceRelease(self.instance);
        }
    }
}
