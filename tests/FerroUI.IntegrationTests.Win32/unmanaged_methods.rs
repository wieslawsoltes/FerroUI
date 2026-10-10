//! The functions of the system the tests call themselves, declared here as
//! upstream's tests declare theirs (`UnmanagedMethods.cs` of the test
//! project), apart from the backend they test.

/// `SM_CMONITORS`: the number of display monitors of the desktop.
pub const SM_CMONITORS: i32 = 80;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[cfg(windows)]
mod native {
    use super::RECT;

    #[link(name = "user32", kind = "raw-dylib")]
    extern "system" {
        pub fn GetSystemMetrics(index: i32) -> i32;
        pub fn GetClientRect(hwnd: isize, rect: *mut RECT) -> i32;
        pub fn GetWindowRect(hwnd: isize, rect: *mut RECT) -> i32;
        pub fn GetPropW(hwnd: isize, name: *const u16) -> isize;
    }
}

/// `GetSystemMetrics`.
#[cfg(windows)]
pub fn get_system_metrics(index: i32) -> i32 {
    // SAFETY: the call takes a number and returns a number.
    unsafe { native::GetSystemMetrics(index) }
}

/// `GetClientRect`: the client rectangle of a window, or `None` when the
/// call fails.
#[cfg(windows)]
pub fn get_client_rect(hwnd: isize) -> Option<RECT> {
    let mut rect = RECT::default();
    // SAFETY: `rect` is a rectangle of this frame the system writes to; a
    // handle that is not a window makes the call fail.
    (unsafe { native::GetClientRect(hwnd, &mut rect) } != 0).then_some(rect)
}

/// `GetWindowRect`: the rectangle of a window on the desktop, or `None`
/// when the call fails.
#[cfg(windows)]
pub fn get_window_rect(hwnd: isize) -> Option<RECT> {
    let mut rect = RECT::default();
    // SAFETY: as `get_client_rect`.
    (unsafe { native::GetWindowRect(hwnd, &mut rect) } != 0).then_some(rect)
}

// On another system the tests compile and do not run (`main.rs`).
#[cfg(not(windows))]
pub fn get_system_metrics(_index: i32) -> i32 {
    0
}

#[cfg(not(windows))]
pub fn get_client_rect(_hwnd: isize) -> Option<RECT> {
    None
}

#[cfg(not(windows))]
pub fn get_window_rect(_hwnd: isize) -> Option<RECT> {
    None
}

/// `GetProp`: the value of a property of a window, or 0 when the window has
/// no property of the name.
#[cfg(windows)]
pub fn get_prop(hwnd: isize, name: &str) -> isize {
    let name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: a null-terminated string that outlives the call; a handle
    // that is not a window makes the call return 0.
    unsafe { native::GetPropW(hwnd, name.as_ptr()) }
}

/// The client area of a window as the system holds it: four bytes a pixel,
/// blue first, rows from the top.
pub struct ClientAreaPicture {
    pub pixels: Vec<u8>,
    pub width: i32,
    pub height: i32,
}

#[cfg(windows)]
mod print {
    use std::ffi::c_void;

    /// `BITMAPINFOHEADER`, which is all of a `BITMAPINFO` for 32 bits a
    /// pixel without compression.
    #[repr(C)]
    pub struct BitmapInfoHeader {
        pub size: u32,
        pub width: i32,
        pub height: i32,
        pub planes: u16,
        pub bit_count: u16,
        pub compression: u32,
        pub size_image: u32,
        pub x_pels_per_meter: i32,
        pub y_pels_per_meter: i32,
        pub clr_used: u32,
        pub clr_important: u32,
    }

    #[link(name = "user32", kind = "raw-dylib")]
    extern "system" {
        pub fn GetDC(hwnd: isize) -> isize;
        pub fn ReleaseDC(hwnd: isize, dc: isize) -> i32;
        pub fn PrintWindow(hwnd: isize, dc: isize, flags: u32) -> i32;
    }

    #[link(name = "gdi32", kind = "raw-dylib")]
    extern "system" {
        pub fn CreateCompatibleDC(dc: isize) -> isize;
        pub fn CreateDIBSection(
            dc: isize,
            info: *const BitmapInfoHeader,
            usage: u32,
            bits: *mut *mut c_void,
            section: isize,
            offset: u32,
        ) -> isize;
        pub fn SelectObject(dc: isize, object: isize) -> isize;
        pub fn DeleteObject(object: isize) -> i32;
        pub fn DeleteDC(dc: isize) -> i32;
        pub fn GdiFlush() -> i32;
    }
}

/// `PrintWindow` with `PW_CLIENTONLY | PW_RENDERFULLCONTENT`: the window
/// prints its client area into a bitmap, with what the desktop window
/// manager composed of it, so a frame presented through a swap chain is in
/// the picture as well as one copied with GDI. Nothing but the window is
/// captured.
#[cfg(windows)]
pub fn print_client_area(hwnd: isize) -> Result<ClientAreaPicture, String> {
    use print::*;

    let rect = get_client_rect(hwnd).ok_or_else(|| "GetClientRect failed".to_string())?;
    let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
    if width < 1 || height < 1 {
        return Err(format!("the client area is empty ({width} by {height})"));
    }
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
    // SAFETY: every object created here is released before the function
    // returns. The section is `width * height * 4` bytes the system owns
    // until the bitmap is deleted; they are copied out, after the drawing
    // of the system was flushed, while the bitmap lives.
    unsafe {
        let window_dc = GetDC(hwnd);
        if window_dc == 0 {
            return Err("GetDC failed".to_string());
        }
        let memory_dc = CreateCompatibleDC(window_dc);
        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let bitmap = if memory_dc == 0 { 0 } else { CreateDIBSection(memory_dc, &header, 0, &mut bits, 0, 0) };
        let result = if memory_dc == 0 || bitmap == 0 || bits.is_null() {
            Err("the bitmap of the picture could not be created".to_string())
        } else {
            let previous = SelectObject(memory_dc, bitmap);
            let printed = PrintWindow(hwnd, memory_dc, 1 | 2) != 0;
            GdiFlush();
            let pixels = std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec();
            SelectObject(memory_dc, previous);
            if printed {
                Ok(ClientAreaPicture { pixels, width, height })
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

#[cfg(not(windows))]
pub fn print_client_area(_hwnd: isize) -> Result<ClientAreaPicture, String> {
    Err("the system is not Windows".to_string())
}
