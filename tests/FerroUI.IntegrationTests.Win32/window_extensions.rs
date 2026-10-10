use crate::unmanaged_methods::{self, RECT};
use ferroui_base::threading::{Dispatcher, DispatcherFrame, DispatcherPriority, DispatcherTimer};
use ferroui_base::{PixelPoint, PixelRect, PixelSize};
use ferroui_controls::platform::Screen;
use ferroui_controls::{Control, Window};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// How long a window may take to load before its test fails.
const LOAD_TIMEOUT: Duration = Duration::from_secs(30);

pub fn to_pixel_rect(rect: RECT) -> PixelRect {
    PixelRect::from_points(PixelPoint::new(rect.left, rect.top), PixelPoint::new(rect.right, rect.bottom))
}

/// Runs the message loop until the window is loaded. Upstream awaits a
/// task the `Loaded` event completes; here the wait is a frame of the
/// dispatcher that the event ends.
///
/// # Panics
/// Panics when the window is not loaded within thirty seconds.
pub fn when_loaded(window: &Window) {
    if window.is_loaded() {
        return;
    }

    let frame = DispatcherFrame::new();
    let timed_out = Rc::new(Cell::new(false));

    let token = {
        let frame = frame.clone();
        window.loaded(move |_, _| frame.set_continue(false))
    };
    let timer = {
        let frame = frame.clone();
        let timed_out = timed_out.clone();
        DispatcherTimer::run_once(
            move || {
                timed_out.set(true);
                frame.set_continue(false);
            },
            LOAD_TIMEOUT,
            DispatcherPriority::NORMAL,
        )
    };

    Dispatcher::ui_thread().push_frame(&frame);

    timer.dispose();
    window.remove_handler(Control::loaded_event(), token);
    assert!(!timed_out.get(), "the window was not loaded within {LOAD_TIMEOUT:?}");
}

pub fn get_screen_at_index(window: &Window, index: usize) -> Rc<Screen> {
    window.screens().all()[index].clone()
}

pub fn get_win32_client_size(window: &Window) -> PixelSize {
    let platform_handle = window.try_get_platform_handle().expect("the window has a platform handle");
    let rect = unmanaged_methods::get_client_rect(platform_handle.handle()).expect("GetClientRect");
    to_pixel_rect(rect).size()
}

pub fn get_win32_window_bounds(window: &Window) -> PixelRect {
    let platform_handle = window.try_get_platform_handle().expect("the window has a platform handle");
    let rect = unmanaged_methods::get_window_rect(platform_handle.handle()).expect("GetWindowRect");
    to_pixel_rect(rect)
}
