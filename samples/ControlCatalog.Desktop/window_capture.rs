//! A picture of the client area of one window as the system composed it,
//! for the screenshots of a smoke run on Windows (`FERROUI_SMOKE_SCREENSHOTS`
//! of the desktop entry point). Not a port: the managed original has no
//! such option.
//!
//! The window is asked to print itself (`PrintWindow` with the flag that
//! includes what the desktop window manager holds of it, so content
//! presented through a swap chain is there too). Nothing but the window is
//! captured: no part of the desktop around it or of another window.

use std::ffi::c_void;

#[repr(C)]
#[derive(Default)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

/// `BITMAPINFOHEADER`, which is all of a `BITMAPINFO` for 32 bits a pixel
/// without compression.
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
    fn ClientToScreen(hwnd: isize, point: *mut [i32; 2]) -> i32;
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
    fn BitBlt(dc: isize, x: i32, y: i32, width: i32, height: i32, source: isize, source_x: i32, source_y: i32, rop: u32) -> i32;
}

/// `SRCCOPY | CAPTUREBLT`: a copy that includes the windows the desktop
/// window manager composes over the screen.
const SRCCOPY_CAPTUREBLT: u32 = 0x00CC_0020 | 0x4000_0000;

/// Where the pixels of a capture come from.
#[derive(Clone, Copy, PartialEq)]
enum Source {
    /// The window prints itself (`PrintWindow`).
    Window,
    /// The screen under the client area of the window: what the desktop
    /// window manager composed there, with whatever is behind or over the
    /// window.
    Screen,
}

/// `PW_CLIENTONLY`: the client area, without the frame.
const PW_CLIENTONLY: u32 = 1;
/// `PW_RENDERFULLCONTENT`: with what the desktop window manager composed
/// of the window (Windows 8.1 and later).
const PW_RENDERFULLCONTENT: u32 = 2;

/// The pixels of the client area of a window.
pub struct Capture {
    /// Four bytes a pixel, blue first, the fourth byte 255; rows from the
    /// top, `width * 4` bytes each.
    pub pixels: Vec<u8>,
    pub width: i32,
    pub height: i32,
}

/// Captures the client area of the window, or says what failed.
pub fn capture_client_area(hwnd: isize) -> Result<Capture, String> {
    capture(hwnd, Source::Window)
}

/// Captures the rectangle of the screen the client area of the window
/// covers, as the desktop window manager composed it: the window with its
/// backdrop (what is behind a transparent window, blurred or tinted by a
/// backdrop effect). Unlike [`capture_client_area`] this reads the screen,
/// so it is only for a run on a machine whose screen holds nothing but
/// the run (a runner of the CI): the backdrop run asks for it.
pub fn capture_client_area_from_screen(hwnd: isize) -> Result<Capture, String> {
    capture(hwnd, Source::Screen)
}

fn capture(hwnd: isize, source: Source) -> Result<Capture, String> {
    let mut rect = Rect::default();
    // SAFETY: a rectangle of this frame the system writes to; a handle
    // that is not a window makes the call fail.
    if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
        return Err("GetClientRect failed".to_string());
    }
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
    // returns. The bitmap section is `width * height * 4` bytes that the
    // system owns until the bitmap is deleted; they are copied out, after
    // the drawing of the system has been flushed, while the bitmap lives.
    unsafe {
        let window_dc = GetDC(hwnd);
        if window_dc == 0 {
            return Err("GetDC failed".to_string());
        }
        let memory_dc = CreateCompatibleDC(window_dc);
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = if memory_dc == 0 { 0 } else { CreateDIBSection(memory_dc, &header, 0, &mut bits, 0, 0) };
        let result = if memory_dc == 0 || bitmap == 0 || bits.is_null() {
            Err("the bitmap of the capture could not be created".to_string())
        } else {
            let previous = SelectObject(memory_dc, bitmap);
            let printed = match source {
                Source::Window => PrintWindow(hwnd, memory_dc, PW_CLIENTONLY | PW_RENDERFULLCONTENT) != 0,
                Source::Screen => {
                    let mut origin = [0, 0];
                    let screen_dc = GetDC(0);
                    let copied = screen_dc != 0
                        && ClientToScreen(hwnd, &mut origin) != 0
                        && BitBlt(memory_dc, 0, 0, width, height, screen_dc, origin[0], origin[1], SRCCOPY_CAPTUREBLT) != 0;
                    if screen_dc != 0 {
                        ReleaseDC(0, screen_dc);
                    }
                    copied
                }
            };
            GdiFlush();
            let mut pixels = std::slice::from_raw_parts(bits as *const u8, (width * height * 4) as usize).to_vec();
            SelectObject(memory_dc, previous);
            if printed {
                // The fourth byte is not written by every kind of drawing.
                for pixel in pixels.chunks_exact_mut(4) {
                    pixel[3] = 0xff;
                }
                Ok(Capture { pixels, width, height })
            } else {
                Err(if source == Source::Window { "PrintWindow failed" } else { "the screen could not be copied" }.to_string())
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

/// How far the content of the capture reaches: one more than the last
/// column and the last row that have a pixel that is not white. A capture
/// of a window that lies partly outside the desktop is white there (the
/// system does not draw what is off the screen), and so is a surface that
/// is smaller than the client area: the reach then stops short of the
/// size of the capture.
pub fn content_extent(capture: &Capture) -> (i32, i32) {
    let (mut right, mut bottom) = (0, 0);
    for y in 0..capture.height {
        for x in 0..capture.width {
            let at = ((y * capture.width + x) * 4) as usize;
            if capture.pixels[at..at + 3] != [0xff, 0xff, 0xff] {
                right = right.max(x + 1);
                bottom = bottom.max(y + 1);
            }
        }
    }
    (right, bottom)
}

/// The number of different colours among a grid of samples of the pixels:
/// 1 for a picture of one colour, which is what a capture that shows
/// nothing looks like.
pub fn sampled_colors(capture: &Capture) -> usize {
    let mut colors = std::collections::HashSet::new();
    let (step_x, step_y) = ((capture.width / 64).max(1), (capture.height / 64).max(1));
    let mut y = 0;
    while y < capture.height {
        let mut x = 0;
        while x < capture.width {
            let at = ((y * capture.width + x) * 4) as usize;
            colors.insert([capture.pixels[at], capture.pixels[at + 1], capture.pixels[at + 2]]);
            x += step_x;
        }
        y += step_y;
    }
    colors.len()
}
