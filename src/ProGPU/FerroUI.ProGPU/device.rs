//! The WebGPU device and the ProGPU engine on it (docs/porting/progpu-backend.md, section 2.2).

use progpu_native_sys::progpu::*;
use progpu_native_sys::webgpu::*;
use progpu_native_sys::{LoadError, Runtime};
use std::ffi::{c_char, c_void, CStr};
use std::fmt;
use std::ptr;
use std::rc::Rc;

/// Why a device could not be created.
#[derive(Debug)]
pub enum ProGpuDeviceError {
    /// The runtime libraries were not found or could not be loaded.
    Runtime(LoadError),
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
            ProGpuDeviceError::Runtime(error) => write!(f, "{error}"),
            ProGpuDeviceError::NoAdapter => write!(f, "the WebGPU implementation has no adapter"),
            ProGpuDeviceError::NoDevice => write!(f, "the WebGPU implementation created no device"),
            ProGpuDeviceError::Engine(status) => write!(f, "the ProGPU engine refused the device (status {status})"),
        }
    }
}

impl std::error::Error for ProGpuDeviceError {}

/// A WebGPU device of the wgpu-native implementation with the ProGPU engine on it.
///
/// The engine renders into the views of textures of `RGBA8Unorm`, the target format it is created
/// for. Every call is on the thread that created the device (the engine refuses others).
pub struct ProGpuDevice {
    runtime: Rc<Runtime>,
    instance: WGPUInstance,
    adapter: WGPUAdapter,
    device: WGPUDevice,
    queue: WGPUQueue,
    engine: *mut progpu_native_engine,
}

unsafe extern "C" fn on_adapter(status: WGPURequestAdapterStatus, adapter: WGPUAdapter, _: *const c_char, userdata: *mut c_void) {
    if status == WGPURequestAdapterStatus_Success {
        // SAFETY: `userdata` is the `WGPUAdapter` slot of `ProGpuDevice::new`.
        unsafe { *(userdata as *mut WGPUAdapter) = adapter };
    }
}

unsafe extern "C" fn on_device(status: WGPURequestDeviceStatus, device: WGPUDevice, _: *const c_char, userdata: *mut c_void) {
    if status == WGPURequestDeviceStatus_Success {
        // SAFETY: `userdata` is the `WGPUDevice` slot of `ProGpuDevice::new`.
        unsafe { *(userdata as *mut WGPUDevice) = device };
    }
}

unsafe extern "C" fn on_mapped(status: WGPUBufferMapAsyncStatus, userdata: *mut c_void) {
    // SAFETY: `userdata` is the flag of `ProGpuDevice::read_rgba8`.
    unsafe { *(userdata as *mut i32) = if status == WGPUBufferMapAsyncStatus_Success { 1 } else { -1 } };
}

impl ProGpuDevice {
    /// Creates a device on the default adapter of the WebGPU implementation and the engine on it.
    pub fn new() -> Result<Self, ProGpuDeviceError> {
        let runtime = Rc::new(Runtime::load().map_err(ProGpuDeviceError::Runtime)?);
        let w = &runtime.wgpu;
        // SAFETY: the calls of the WebGPU C ABI with the handles they return; the callbacks of
        // wgpu-native run before the request returns.
        unsafe {
            let instance = w.wgpuCreateInstance(ptr::null());
            if instance.is_null() {
                return Err(ProGpuDeviceError::NoAdapter);
            }
            let mut adapter: WGPUAdapter = ptr::null_mut();
            w.wgpuInstanceRequestAdapter(instance, ptr::null(), Some(on_adapter), &mut adapter as *mut _ as *mut c_void);
            if adapter.is_null() {
                w.wgpuInstanceRelease(instance);
                return Err(ProGpuDeviceError::NoAdapter);
            }
            let mut device: WGPUDevice = ptr::null_mut();
            w.wgpuAdapterRequestDevice(adapter, ptr::null(), Some(on_device), &mut device as *mut _ as *mut c_void);
            if device.is_null() {
                w.wgpuAdapterRelease(adapter);
                w.wgpuInstanceRelease(instance);
                return Err(ProGpuDeviceError::NoDevice);
            }
            let queue = w.wgpuDeviceGetQueue(device);
            let options = progpu_native_engine_options {
                struct_size: std::mem::size_of::<progpu_native_engine_options>() as u32,
                abi_version: PROGPU_NATIVE_ABI_VERSION,
                backend_abi: PROGPU_NATIVE_BACKEND_ABI_WGPU_NATIVE_2024_05 as u32,
                target_format: PROGPU_NATIVE_TEXTURE_FORMAT_RGBA8_UNORM as u32,
                device: device as usize,
                queue: queue as usize,
                flags: 0,
            };
            let mut engine: *mut progpu_native_engine = ptr::null_mut();
            let status = runtime.progpu.progpu_native_engine_create(&options, &mut engine);
            if status != PROGPU_NATIVE_STATUS_SUCCESS {
                w.wgpuQueueRelease(queue);
                w.wgpuDeviceRelease(device);
                w.wgpuAdapterRelease(adapter);
                w.wgpuInstanceRelease(instance);
                return Err(ProGpuDeviceError::Engine(status));
            }
            Ok(Self { runtime, instance, adapter, device, queue, engine })
        }
    }

    /// The loaded runtime libraries.
    pub fn runtime(&self) -> &Rc<Runtime> {
        &self.runtime
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
            self.runtime.progpu.progpu_native_engine_get_last_error(self.engine, message.as_mut_ptr(), message.len());
            CStr::from_ptr(message.as_ptr()).to_string_lossy().into_owned()
        }
    }

    /// Reads a texture of `RGBA8Unorm` back: `width * height * 4` bytes, row by row.
    pub fn read_rgba8(&self, texture: WGPUTexture, width: u32, height: u32) -> Vec<u8> {
        let w = &self.runtime.wgpu;
        // Rows of a copy are aligned to 256 bytes.
        let bytes_per_row = (width * 4).div_ceil(256) * 256;
        let size = u64::from(bytes_per_row) * u64::from(height);
        // SAFETY: the calls of the WebGPU C ABI with the handles they return; the mapped range is
        // read while the buffer is mapped.
        unsafe {
            let buffer = w.wgpuDeviceCreateBuffer(
                self.device,
                &WGPUBufferDescriptor {
                    nextInChain: ptr::null(),
                    label: ptr::null(),
                    usage: (WGPUBufferUsage_MapRead | WGPUBufferUsage_CopyDst) as u32,
                    size,
                    mappedAtCreation: 0,
                },
            );
            let encoder = w.wgpuDeviceCreateCommandEncoder(self.device, ptr::null());
            w.wgpuCommandEncoderCopyTextureToBuffer(
                encoder,
                &WGPUImageCopyTexture {
                    nextInChain: ptr::null(),
                    texture,
                    mipLevel: 0,
                    origin: WGPUOrigin3D { x: 0, y: 0, z: 0 },
                    aspect: WGPUTextureAspect_All,
                },
                &WGPUImageCopyBuffer {
                    nextInChain: ptr::null(),
                    layout: WGPUTextureDataLayout { nextInChain: ptr::null(), offset: 0, bytesPerRow: bytes_per_row, rowsPerImage: height },
                    buffer,
                },
                &WGPUExtent3D { width, height, depthOrArrayLayers: 1 },
            );
            let commands = w.wgpuCommandEncoderFinish(encoder, ptr::null());
            w.wgpuQueueSubmit(self.queue, 1, &commands);
            w.wgpuCommandBufferRelease(commands);
            w.wgpuCommandEncoderRelease(encoder);
            let mut mapped = 0i32;
            w.wgpuBufferMapAsync(buffer, WGPUMapMode_Read as u32, 0, size as usize, Some(on_mapped), &mut mapped as *mut _ as *mut c_void);
            while mapped == 0 {
                w.wgpuDevicePoll(self.device, 1, ptr::null());
            }
            let mut pixels = Vec::with_capacity((width * height * 4) as usize);
            if mapped == 1 {
                let data = std::slice::from_raw_parts(w.wgpuBufferGetConstMappedRange(buffer, 0, size as usize) as *const u8, size as usize);
                for row in 0..height as usize {
                    let start = row * bytes_per_row as usize;
                    pixels.extend_from_slice(&data[start..start + (width * 4) as usize]);
                }
                w.wgpuBufferUnmap(buffer);
            }
            w.wgpuBufferRelease(buffer);
            pixels
        }
    }
}

impl Drop for ProGpuDevice {
    fn drop(&mut self) {
        let w = &self.runtime.wgpu;
        // SAFETY: the handles are owned here; the engine goes before the device it retains.
        unsafe {
            self.runtime.progpu.progpu_native_engine_destroy(self.engine);
            w.wgpuQueueRelease(self.queue);
            w.wgpuDeviceRelease(self.device);
            w.wgpuAdapterRelease(self.adapter);
            w.wgpuInstanceRelease(self.instance);
        }
    }
}
