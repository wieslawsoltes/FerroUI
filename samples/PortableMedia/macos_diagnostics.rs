//! Counted, opt-in Metal and own-window diagnostics; never part of preview.
use super::native_pixels::wait;
use ferroui_base::{
    rendering::composition::{frames::*, ICompositionGpuInterop},
    Ref,
};
use ferroui_controls::Window;
use ferroui_media::{MediaError, MediaErrorKind};
use image::RgbaImage;
use std::ffi::CString;
use std::{
    ffi::{c_char, c_void, CStr},
    ptr::NonNull,
};
extern "C" {
    fn ferro_media_transport_action(
        window: *mut c_void,
        label: *const c_char,
        role: *const c_char,
        action: i32,
    ) -> i32;
    fn ferro_media_transport_key(window: *mut c_void, key: u16, text: *const c_char) -> i32;
    fn ferro_media_window_capture(window: *mut c_void, width: u32, height: u32) -> *mut c_void;
    fn ferro_media_capture_poll(
        owner: *mut c_void,
        pixels: *mut u8,
        length: usize,
        error: *mut c_char,
        error_length: usize,
    ) -> i32;
    fn ferro_media_capture_release(owner: *mut c_void);
    fn ferro_media_read_bgra(buffer: *mut c_void, pixels: *mut u8, length: usize) -> i32;
    fn ferro_media_memory(values: *mut u64) -> i32;
}
pub fn transport_action(
    window: &Window,
    label: &str,
    role: &str,
    action: i32,
) -> Result<(), String> {
    let handle = window
        .try_get_platform_handle()
        .ok_or("No transport NSWindow")?;
    let label = CString::new(label).map_err(|e| e.to_string())?;
    let role = CString::new(role).map_err(|e| e.to_string())?;
    if unsafe {
        ferro_media_transport_action(
            handle.handle() as *mut c_void,
            label.as_ptr(),
            role.as_ptr(),
            action,
        )
    } == 0
    {
        return Err(format!(
            "Native accessibility action {action} unavailable for {label:?}/{role:?}"
        ));
    }
    Ok(())
}
pub fn transport_key(window: &Window, key: u16, text: &str) -> Result<(), String> {
    let handle = window
        .try_get_platform_handle()
        .ok_or("No keyboard NSWindow")?;
    let text = CString::new(text).map_err(|e| e.to_string())?;
    let status = unsafe { ferro_media_transport_key(handle.handle() as *mut c_void, key, text.as_ptr()) };
    if status != 1 {
        return Err(format!("Native transport keyboard dispatch failed (status {status}: -1 thread, -2 owner, -3 key window)"));
    }
    Ok(())
}
struct Capture(NonNull<c_void>);
impl Drop for Capture {
    fn drop(&mut self) {
        unsafe { ferro_media_capture_release(self.0.as_ptr()) }
    }
}
pub async fn presented(
    window: &Ref<Window>,
    counters: &FrameTransferCounters,
) -> Result<RgbaImage, String> {
    let handle = window.try_get_platform_handle().ok_or("No native window")?;
    if handle.handle_descriptor() != Some("NSWindow") {
        return Err("Expected an NSWindow".into());
    }
    let scale = window.render_scaling();
    let size = window.bounds().size();
    let (width, height) = (
        (size.width * scale).round() as u32,
        (size.height * scale).round() as u32,
    );
    let length = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .filter(|n| *n <= 64 * 1024 * 1024)
        .ok_or("Capture size exceeds diagnostic limit")?;
    // Native conversion and the independently owned Rust copy are both counted.
    counters
        .cpu_transfer(
            PixelTransferPurpose::Diagnostic,
            length as u64 * 2,
            PresentationRoute::SharedTexture,
        )
        .map_err(|e| e.to_string())?;
    let owner = Capture(
        NonNull::new(unsafe {
            ferro_media_window_capture(handle.handle() as *mut c_void, width, height)
        })
        .ok_or("Capture allocation failed")?,
    );
    let mut pixels = Some(vec![0u8; length]);
    let result = wait(move || {
        let mut error = [0 as c_char; 1024];
        let status = unsafe {
            ferro_media_capture_poll(
                owner.0.as_ptr(),
                pixels.as_mut().unwrap().as_mut_ptr(),
                length,
                error.as_mut_ptr(),
                error.len(),
            )
        };
        match status {
            0 => Ok(None),
            1 => Ok(Some(
                RgbaImage::from_raw(width, height, pixels.take().unwrap())
                    .ok_or("Invalid window pixels")?,
            )),
            _ => Err(unsafe { CStr::from_ptr(error.as_ptr()) }
                .to_string_lossy()
                .into_owned()),
        }
    })
    .await?;
    if let Some(directory) = std::env::var_os("FERROUI_MEDIA_EVIDENCE") {
        let path = std::path::PathBuf::from(directory).join("macos-own-window.png");
        result.save(&path).map_err(|e| e.to_string())?;
        counters
            .cpu_transfer(
                PixelTransferPurpose::Diagnostic,
                std::fs::metadata(path).map_err(|e| e.to_string())?.len(),
                PresentationRoute::SharedTexture,
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(result)
}
pub struct MetalFrameReader {
    device: DeviceIdentity,
    generation: u64,
}
impl MetalFrameReader {
    pub fn from_interop(
        interop: &dyn ICompositionGpuInterop,
        _budget: FrameBudget,
    ) -> Result<Self, MediaError> {
        Ok(Self {
            device: DeviceIdentity::Luid(
                interop.device_luid().ok_or_else(|| {
                    MediaError::new(MediaErrorKind::Unsupported, "Metal required")
                })?,
            ),
            generation: interop.shared_context_generation(),
        })
    }
}
pub async fn read_texture(
    reader: &MetalFrameReader,
    frame: FrameLease,
) -> Result<RgbaImage, String> {
    if frame.metadata().device.as_ref() != Some(&reader.device)
        || frame.metadata().device_generation != reader.generation
    {
        return Err("Diagnostic frame belongs to another Metal generation".into());
    }
    let m = frame.metadata();
    let buffer = frame
        .resource::<ferroui_media_apple::PixelBuffer>()
        .ok_or("Expected retained CoreVideo output")?;
    let length = (m.width as usize)
        .checked_mul(m.height as usize)
        .and_then(|n| n.checked_mul(4))
        .filter(|n| *n <= 64 * 1024 * 1024)
        .ok_or("Diagnostic image exceeds limit")?;
    frame
        .counters()
        .cpu_transfer(
            PixelTransferPurpose::Diagnostic,
            length as u64,
            PresentationRoute::SharedTexture,
        )
        .map_err(|e| e.to_string())?;
    let mut pixels = vec![0; length];
    // RendererHost publishes only after actual producer GPU completion. The
    // lease retains immutable storage through this explicit diagnostic read.
    if unsafe { ferro_media_read_bgra(buffer.as_raw(), pixels.as_mut_ptr(), pixels.len()) } == 0 {
        return Err("Cannot read completed Metal output".into());
    }
    RgbaImage::from_raw(m.width, m.height, pixels).ok_or("Invalid Metal output".into())
}
#[derive(Clone, Copy, Debug)]
pub struct Memory {
    pub footprint: u64,
    pub resident: u64,
    pub metal: u64,
    pub device: u64,
    pub ports: u64,
    pub malloc_bytes: u64,
    pub malloc_blocks: u64,
}
pub fn memory() -> Result<Memory, String> {
    let mut v = [0u64; 7];
    if unsafe { ferro_media_memory(v.as_mut_ptr()) } == 0 {
        return Err("Native memory probe failed".into());
    }
    Ok(Memory {
        footprint: v[0],
        resident: v[1],
        metal: v[2],
        device: v[3],
        ports: v[4],
        malloc_bytes: v[5],
        malloc_blocks: v[6],
    })
}
