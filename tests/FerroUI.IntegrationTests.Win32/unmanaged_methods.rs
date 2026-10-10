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
