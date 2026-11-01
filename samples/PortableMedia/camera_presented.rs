//! Window-only PrintWindow diagnostic; no other desktop content is captured.
use super::*;
pub fn capture(
    window: &Ref<Window>,
    counters: &FrameTransferCounters,
) -> Result<RgbaImage, String> {
    let handle = window
        .try_get_platform_handle()
        .ok_or("No HWND for pixel diagnostic")?;
    let mut captured = presented::capture(handle.handle(), counters)?;
    counters
        .cpu_transfer(
            PixelTransferPurpose::Diagnostic,
            captured.pixels.len() as u64,
            PresentationRoute::SharedTexture,
        )
        .map_err(|e| e.to_string())?;
    for pixel in captured.pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
    RgbaImage::from_raw(
        captured.width as u32,
        captured.height as u32,
        captured.pixels,
    )
    .ok_or("Invalid presented pixels".into())
}
mod presented {
    use super::*;
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    /// `BITMAPINFOHEADER`, which is all of a `BITMAPINFO` for 32 bits
    /// a pixel without compression.
    #[repr(C)]
    struct BitmapInfoHeader {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bit_count: u16,
        compression: u32,
        size_image: u32,
        x_pels_per_meter: i32,
        y_pels_per_meter: i32,
        clr_used: u32,
        clr_important: u32,
    }

    #[link(name = "user32", kind = "raw-dylib")]
    extern "system" {
        fn GetClientRect(hwnd: isize, rect: *mut Rect) -> i32;
        fn GetDC(hwnd: isize) -> isize;
        fn ReleaseDC(hwnd: isize, dc: isize) -> i32;
        fn PrintWindow(hwnd: isize, dc: isize, flags: u32) -> i32;
    }

    #[link(name = "gdi32", kind = "raw-dylib")]
    extern "system" {
        fn CreateCompatibleDC(dc: isize) -> isize;
        fn CreateDIBSection(
            dc: isize,
            info: *const BitmapInfoHeader,
            usage: u32,
            bits: *mut *mut c_void,
            section: isize,
            offset: u32,
        ) -> isize;
        fn SelectObject(dc: isize, object: isize) -> isize;
        fn DeleteObject(object: isize) -> i32;
        fn DeleteDC(dc: isize) -> i32;
        fn GdiFlush() -> i32;
    }

    /// `PW_CLIENTONLY | PW_RENDERFULLCONTENT`: only this window's client area.
    const FLAGS: u32 = 1 | 2;

    /// The pixels of the client area: four bytes a pixel, blue first,
    /// rows from the top.
    pub struct Capture {
        pub pixels: Vec<u8>,
        pub width: i32,
        pub height: i32,
    }

    pub fn capture(hwnd: isize, counters: &FrameTransferCounters) -> Result<Capture, String> {
        let mut rect = Rect::default();
        // SAFETY: a rectangle of this frame the system writes to; a
        // handle that is not a window makes the call fail.
        if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
            return Err("GetClientRect failed".to_string());
        }
        let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
        if width < 1 || height < 1 {
            return Err(format!("the client area is empty ({width} by {height})"));
        }
        let byte_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or("Capture size overflow")?;
        counters
            .cpu_transfer(
                PixelTransferPurpose::Diagnostic,
                byte_len as u64,
                PresentationRoute::SharedTexture,
            )
            .map_err(|e| e.to_string())?;
        let header = BitmapInfoHeader {
            size: std::mem::size_of::<BitmapInfoHeader>() as u32,
            width,
            // A negative height: the rows run from the top.
            height: -height,
            planes: 1,
            bit_count: 32,
            compression: 0,
            size_image: 0,
            x_pels_per_meter: 0,
            y_pels_per_meter: 0,
            clr_used: 0,
            clr_important: 0,
        };
        // SAFETY: every object created here is released before the
        // function returns. The section is `width * height * 4` bytes
        // the system owns until the bitmap is deleted; they are copied
        // out, after the drawing of the system was flushed, while the
        // bitmap lives.
        unsafe {
            let window_dc = GetDC(hwnd);
            if window_dc == 0 {
                return Err("GetDC failed".to_string());
            }
            let memory_dc = CreateCompatibleDC(window_dc);
            let mut bits: *mut c_void = std::ptr::null_mut();
            let bitmap = if memory_dc == 0 {
                0
            } else {
                CreateDIBSection(memory_dc, &header, 0, &mut bits, 0, 0)
            };
            let result = if memory_dc == 0 || bitmap == 0 || bits.is_null() {
                Err("the bitmap of the capture could not be created".to_string())
            } else {
                let previous = SelectObject(memory_dc, bitmap);
                let printed = PrintWindow(hwnd, memory_dc, FLAGS) != 0;
                GdiFlush();
                let copied = counters
                    .cpu_transfer(
                        PixelTransferPurpose::Diagnostic,
                        byte_len as u64,
                        PresentationRoute::SharedTexture,
                    )
                    .map_err(|e| e.to_string());
                let pixels = if copied.is_ok() {
                    std::slice::from_raw_parts(bits as *const u8, byte_len).to_vec()
                } else {
                    Vec::new()
                };
                SelectObject(memory_dc, previous);
                if let Err(error) = copied {
                    Err(error)
                } else if printed {
                    Ok(Capture {
                        pixels,
                        width,
                        height,
                    })
                } else {
                    Err("PrintWindow failed".to_string())
                }
            };
            if bitmap != 0 {
                DeleteObject(bitmap);
            }
            if memory_dc != 0 {
                DeleteDC(memory_dc);
            }
            ReleaseDC(hwnd, window_dc);
            result
        }
    }
}
