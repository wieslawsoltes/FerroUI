//! The software surface of a window: frames are drawn into memory and
//! copied to the window as a device independent bitmap.

use crate::interop::unmanaged_methods::BITMAPINFOHEADER;
use ferroui_base::PixelSize;

const BYTES_PER_PIXEL: i32 = 4;

/// The pixel memory of a window and its description as a bitmap.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
pub(crate) struct FramebufferData {
    data: Vec<u8>,
    size: PixelSize,
    header: BITMAPINFOHEADER,
}

#[cfg_attr(not(any(windows, test)), allow(dead_code))]
impl FramebufferData {
    /// Allocates the memory of a framebuffer of the given size, zeroed.
    pub(crate) fn new(width: i32, height: i32) -> Self {
        let data = vec![0u8; width.max(0) as usize * height.max(0) as usize * BYTES_PER_PIXEL as usize];

        let mut header = BITMAPINFOHEADER::default();
        header.init();

        header.bi_planes = 1;
        header.bi_bit_count = (BYTES_PER_PIXEL * 8) as u16;
        header.init();

        header.bi_width = width;
        // A negative height: the rows are stored from the top down.
        header.bi_height = -height;

        Self { data, size: PixelSize::new(width, height), header }
    }

    pub(crate) fn size(&self) -> PixelSize {
        self.size
    }

    pub(crate) fn row_bytes(&self) -> i32 {
        self.size.width * BYTES_PER_PIXEL
    }

    pub(crate) fn header(&self) -> &BITMAPINFOHEADER {
        &self.header
    }

    pub(crate) fn data(&self) -> &[u8] {
        &self.data
    }

    pub(crate) fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

/// The size of the framebuffer of a client rectangle: at least one pixel in
/// each direction.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
pub(crate) fn framebuffer_size(client_width: i32, client_height: i32) -> (i32, i32) {
    (client_width.max(1), client_height.max(1))
}

#[cfg(windows)]
pub use imp::FramebufferManager;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::interop::unmanaged_methods::{
        draw_bitmap_to_window, get_client_rect, get_dpi_for_monitor, monitor_from_window, MONITOR, MONITOR_DPI_TYPE,
    };
    use ferroui_base::platform::surfaces::{
        FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
    };
    use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat};
    use ferroui_base::Vector;
    use std::any::Any;
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// What the surface and the framebuffers it locks share.
    ///
    /// `held` with its condition is the lock of the reference (a monitor
    /// that is entered by `lock` and left when the locked framebuffer is
    /// disposed): a guard of a mutex cannot be kept in the framebuffer,
    /// which is an object behind a contract. The pixel memory is only
    /// touched by the holder of that lock.
    struct Shared {
        hwnd: isize,
        held: Mutex<bool>,
        released: Condvar,
        framebuffer_data: Mutex<Option<FramebufferData>>,
        disposed: Mutex<bool>,
    }

    impl Shared {
        fn enter(&self) {
            let mut held = lock(&self.held);
            while *held {
                held = self.released.wait(held).unwrap_or_else(PoisonError::into_inner);
            }
            *held = true;
        }

        fn exit(&self) {
            *lock(&self.held) = false;
            self.released.notify_one();
        }

        fn current_dpi(&self) -> Vector {
            let monitor = monitor_from_window(self.hwnd, MONITOR::MONITOR_DEFAULTTONEAREST);

            match get_dpi_for_monitor(monitor, MONITOR_DPI_TYPE::MDT_EFFECTIVE_DPI) {
                Some((dpi_x, dpi_y)) => Vector::new(f64::from(dpi_x), f64::from(dpi_y)),
                None => Vector::new(96.0, 96.0),
            }
        }

        fn draw_and_unlock(&self) {
            if let Some(framebuffer_data) = lock(&self.framebuffer_data).as_ref() {
                draw_bitmap_to_window(self.hwnd, framebuffer_data.data(), framebuffer_data.header());
            }
            self.exit();
        }
    }

    /// The framebuffer of a window while a frame is drawn into it. Disposing
    /// it copies the frame to the window and releases the surface.
    struct LockedFramebuffer {
        shared: Arc<Shared>,
        address: *mut u8,
        size: PixelSize,
        row_bytes: i32,
        dpi: Vector,
        disposed: Cell<bool>,
    }

    impl ILockedFramebuffer for LockedFramebuffer {
        fn address(&self) -> *mut u8 {
            if self.disposed.get() {
                return std::ptr::null_mut();
            }
            self.address
        }

        fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
            if self.disposed.get() {
                access(&mut []);
                return;
            }
            match lock(&self.shared.framebuffer_data).as_mut() {
                Some(framebuffer_data) => access(framebuffer_data.data_mut()),
                None => access(&mut []),
            }
        }

        fn size(&self) -> PixelSize {
            self.size
        }

        fn row_bytes(&self) -> i32 {
            self.row_bytes
        }

        fn dpi(&self) -> Vector {
            self.dpi
        }

        fn format(&self) -> PixelFormat {
            PixelFormat::BGRA8888
        }

        fn alpha_format(&self) -> AlphaFormat {
            AlphaFormat::Premul
        }

        fn dispose(&self) {
            if self.disposed.replace(true) {
                return;
            }
            self.shared.draw_and_unlock();
        }
    }

    impl Drop for LockedFramebuffer {
        fn drop(&mut self) {
            // A framebuffer that is let go without being disposed gives the
            // surface back without presenting its frame.
            if !self.disposed.replace(true) {
                self.shared.exit();
            }
        }
    }

    /// The software surface of a window.
    ///
    /// The surface is shared with the thread that renders: every system
    /// call it makes (the client rectangle, the DPI of the monitor, the
    /// device context of the window) may be made from any thread, and the
    /// window is named by its handle, a number.
    pub struct FramebufferManager {
        shared: Arc<Shared>,
    }

    impl FramebufferManager {
        /// Creates the surface of a window.
        pub fn new(hwnd: isize) -> Self {
            Self {
                shared: Arc::new(Shared {
                    hwnd,
                    held: Mutex::new(false),
                    released: Condvar::new(),
                    framebuffer_data: Mutex::new(None),
                    disposed: Mutex::new(false),
                }),
            }
        }

        fn lock_shared(shared: &Arc<Shared>) -> Rc<dyn ILockedFramebuffer> {
            shared.enter();

            let rc = get_client_rect(shared.hwnd);
            let (width, height) = framebuffer_size(rc.right - rc.left, rc.bottom - rc.top);

            let (address, size, row_bytes) = {
                let mut slot = lock(&shared.framebuffer_data);
                if slot.as_ref().is_none_or(|data| data.size().width != width || data.size().height != height) {
                    *slot = Some(FramebufferData::new(width, height));
                }
                let framebuffer_data = slot.as_mut().expect("the framebuffer was just allocated");
                (framebuffer_data.data_mut().as_mut_ptr(), framebuffer_data.size(), framebuffer_data.row_bytes())
            };

            Rc::new(LockedFramebuffer {
                shared: shared.clone(),
                address,
                size,
                row_bytes,
                dpi: shared.current_dpi(),
                disposed: Cell::new(false),
            })
        }

        /// Locks the framebuffer of the window for a frame: the memory has
        /// the size of the client area, and disposing the framebuffer
        /// copies it to the window.
        pub fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
            Self::lock_shared(&self.shared)
        }

        /// Frees the pixel memory. A frame that is being drawn finishes
        /// first.
        pub fn dispose(&self) {
            self.shared.enter();
            *lock(&self.shared.framebuffer_data) = None;
            *lock(&self.shared.disposed) = true;
            self.shared.exit();
        }
    }

    impl IPlatformRenderSurface for FramebufferManager {
        fn is_ready(&self) -> bool {
            !*lock(&self.shared.disposed)
        }

        fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
            Some(self)
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    impl IFramebufferPlatformSurface for FramebufferManager {
        fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
            let shared = self.shared.clone();
            Rc::new(FuncFramebufferRenderTarget::new(move || Self::lock_shared(&shared)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interop::unmanaged_methods::SIZE_OF_BITMAPINFOHEADER;

    #[test]
    fn the_bitmap_of_a_framebuffer_is_32_bits_a_pixel_from_the_top_down() {
        let data = FramebufferData::new(7, 3);

        assert_eq!(data.size(), PixelSize::new(7, 3));
        assert_eq!(data.row_bytes(), 28);
        assert_eq!(data.data().len(), 84);
        assert!(data.data().iter().all(|&byte| byte == 0));

        let header = data.header();
        assert_eq!(header.bi_size, SIZE_OF_BITMAPINFOHEADER);
        assert_eq!(header.bi_size as usize, std::mem::size_of::<BITMAPINFOHEADER>());
        assert_eq!(header.bi_width, 7);
        assert_eq!(header.bi_height, -3);
        assert_eq!(header.bi_planes, 1);
        assert_eq!(header.bi_bit_count, 32);
        assert_eq!(header.bi_compression, 0);
    }

    #[test]
    fn a_framebuffer_is_never_empty() {
        assert_eq!(framebuffer_size(640, 480), (640, 480));
        // A minimized window has an empty client rectangle.
        assert_eq!(framebuffer_size(0, 0), (1, 1));
        assert_eq!(framebuffer_size(-5, 10), (1, 10));
    }
}
