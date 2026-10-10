//! The cursors of the platform (the port of `WaylandCursorFactory.cs`).

use crate::server::persistent::i_wayland_cursor::{IWaylandCursor, WaylandCursorProxy};
use crate::server::wayland_worker_client::WaylandWorkerClient;
use ferroui_base::input::StandardCursorType;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::{AlphaFormat, ICursorFactory, ICursorImpl, ILockedFramebuffer, PixelFormat};
use ferroui_base::{PixelPoint, PixelSize, Vector};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

pub struct WaylandCursorFactory {
    client: Rc<WaylandWorkerClient>,
}

impl WaylandCursorFactory {
    pub fn new(client: Rc<WaylandWorkerClient>) -> Self {
        Self { client }
    }
}

impl ICursorFactory for WaylandCursorFactory {
    fn get_cursor(&self, cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        Rc::new(WaylandCursorImpl::new(self.client.create_standard_cursor(cursor_type)))
    }

    fn create_cursor(&self, cursor: &Bitmap, hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        // Extract the cursor pixels into a tightly-packed Bgra8888 premultiplied buffer on the UI
        // thread (the copy transcodes format + alpha). The worker thread then uploads this raw
        // buffer into a wl_buffer without touching the UI-owned bitmap.
        let size = cursor.pixel_size();
        let buffer = PixelBuffer::new(size);
        cursor.copy_pixels_to_framebuffer(&buffer);
        Rc::new(WaylandCursorImpl::new(self.client.create_bitmap_cursor(buffer.into_pixels(), size, hot_spot.x, hot_spot.y)))
    }
}

/// A cursor of the platform: the proxy of the worker's cursor.
pub struct WaylandCursorImpl {
    cursor: WaylandCursorProxy,
}

impl WaylandCursorImpl {
    pub fn new(cursor: WaylandCursorProxy) -> Self {
        Self { cursor }
    }

    pub fn cursor(&self) -> &WaylandCursorProxy {
        &self.cursor
    }
}

impl ICursorImpl for WaylandCursorImpl {
    fn dispose(&self) {
        self.cursor.destroy();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A framebuffer over pixels of the UI thread: what the reference makes with a locked
/// framebuffer over an array of its own, to have a bitmap copied into it in the format of a
/// cursor.
struct PixelBuffer {
    data: RefCell<Vec<u8>>,
    size: PixelSize,
}

impl PixelBuffer {
    fn new(size: PixelSize) -> Self {
        let length = size.width.max(0) as usize * size.height.max(0) as usize * 4;
        Self { data: RefCell::new(vec![0; length]), size }
    }

    fn into_pixels(self) -> Vec<u8> {
        self.data.into_inner()
    }
}

impl ILockedFramebuffer for PixelBuffer {
    fn address(&self) -> *mut u8 {
        self.data.borrow_mut().as_mut_ptr()
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(&mut self.data.borrow_mut());
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn row_bytes(&self) -> i32 {
        self.size.width * 4
    }

    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }

    fn format(&self) -> PixelFormat {
        PixelFormat::BGRA8888
    }

    fn alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }

    fn dispose(&self) {}
}
