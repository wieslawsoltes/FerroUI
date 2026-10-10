//! What a window presents reaches the edges of its client area. Not a
//! port: upstream has no such test. A window of one colour is shown
//! through the compositor; its client area, as the system holds it (the
//! window prints itself, with what the desktop window manager composed of
//! it), has to be that colour to its last column and its last row, after
//! the window was shown and again after it was resized both ways.
//!
//! The rendering mode is the one of the run: the default options (ANGLE
//! first), or the one mode `FERROUI_SMOKE_RENDERING` names
//! (`infrastructure/app_manager.rs`), so the test is run once for each.

use crate::unmanaged_methods::{print_client_area, ClientAreaPicture};
use crate::window_extensions::{get_win32_client_size, wait_for, when_loaded};
use crate::{TestCase, WindowGuard};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::Color;
use ferroui_base::PixelSize;
use ferroui_controls::Window;
use std::rc::Rc;
use std::time::Duration;

/// The colour of the window: no channel is one the system leaves where
/// nothing was presented (white or black).
const FILL: [u8; 3] = [0x1e, 0x90, 0x3c];

pub fn tests(cases: &mut Vec<TestCase>) {
    cases.push(TestCase::new(
        "presented_frame_tests::presented_pixels_reach_the_last_column_and_row_after_show_and_resize".to_string(),
        presented_pixels_reach_the_last_column_and_row_after_show_and_resize,
    ));
}

/// What of a picture is not the fill: the pixels of its last column, of
/// its last row, and of the whole of it; and its last pixel.
struct Extent {
    last_column: usize,
    last_row: usize,
    all: usize,
    last_pixel: [u8; 3],
}

fn extent(picture: &ClientAreaPicture) -> Extent {
    let (width, height) = (picture.width as usize, picture.height as usize);
    let at = |x: usize, y: usize| -> [u8; 3] {
        let index = (y * width + x) * 4;
        [picture.pixels[index + 2], picture.pixels[index + 1], picture.pixels[index]]
    };
    let other = |pixel: [u8; 3]| (0..3).any(|i| (i32::from(pixel[i]) - i32::from(FILL[i])).abs() > 2);
    Extent {
        last_column: (0..height).filter(|&y| other(at(width - 1, y))).count(),
        last_row: (0..width).filter(|&x| other(at(x, height - 1))).count(),
        all: (0..height).map(|y| (0..width).filter(|&x| other(at(x, y))).count()).sum(),
        last_pixel: at(width - 1, height - 1),
    }
}

/// Waits until the window prints the fill over the whole of a client area
/// of the size the system reports, or fails with what it printed last. A
/// frame reaches the screen some time after the size of a window changed
/// (the compositor renders on its own thread and the desktop window
/// manager composes after that), so the picture is taken again for a
/// while.
fn assert_presented(window: &Window, when: &str) -> PixelSize {
    let hwnd = window.try_get_platform_handle().expect("a shown window has a handle").handle();
    let mut seen = String::from("no picture was taken");
    for _ in 0..50 {
        wait_for(Duration::from_millis(100));
        let client = get_win32_client_size(window);
        match print_client_area(hwnd) {
            Err(error) => seen = error,
            Ok(picture) => {
                let size = PixelSize::new(picture.width, picture.height);
                let extent = extent(&picture);
                seen = format!(
                    "the window prints {} by {} pixels (client area {} by {}): {} pixel(s) of the last column, {} of the last row and {} of all are not the fill {FILL:?}; the last pixel is {:?}",
                    size.width, size.height, client.width, client.height, extent.last_column, extent.last_row, extent.all, extent.last_pixel
                );
                if size == client && extent.all == 0 {
                    println!("    {when}: {seen}");
                    return size;
                }
            }
        }
    }
    panic!("{when}: {seen}");
}

fn presented_pixels_reach_the_last_column_and_row_after_show_and_resize() {
    let window = Window::new();
    window.set_width(300.0);
    window.set_height(200.0);
    window.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(FILL[0], FILL[1], FILL[2])))));
    let _guard = WindowGuard(window.clone());
    window.show();
    when_loaded(&window);

    let shown = assert_presented(&window, "after showing");

    // Larger both ways, then smaller both ways, by amounts that are not a
    // multiple of anything a surface might round its size to.
    window.set_width(437.0);
    window.set_height(291.0);
    let larger = assert_presented(&window, "after a resize to a larger size");
    assert!(larger.width > shown.width && larger.height > shown.height, "the client area did not grow: {shown:?} to {larger:?}");

    window.set_width(253.0);
    window.set_height(167.0);
    let smaller = assert_presented(&window, "after a resize to a smaller size");
    assert!(smaller.width < larger.width && smaller.height < larger.height, "the client area did not shrink: {larger:?} to {smaller:?}");
}
