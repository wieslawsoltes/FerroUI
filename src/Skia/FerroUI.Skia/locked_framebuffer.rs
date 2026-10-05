use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat};
use ferroui_base::{PixelSize, Vector};
use std::cell::Cell;

/// A locked framebuffer described by plain values, with an optional action
/// that runs when it is unlocked.
pub struct LockedFramebuffer {
    address: *mut u8,
    size: PixelSize,
    row_bytes: i32,
    dpi: Vector,
    format: PixelFormat,
    alpha_format: AlphaFormat,
    on_dispose: Cell<Option<Box<dyn FnOnce()>>>,
}

impl LockedFramebuffer {
    /// Describes locked pixel memory. `on_dispose` runs once, when the
    /// framebuffer is disposed.
    pub fn new(
        address: *mut u8,
        size: PixelSize,
        row_bytes: i32,
        dpi: Vector,
        format: PixelFormat,
        alpha_format: AlphaFormat,
        on_dispose: Option<Box<dyn FnOnce()>>,
    ) -> Self {
        Self { address, size, row_bytes, dpi, format, alpha_format, on_dispose: Cell::new(on_dispose) }
    }
}

impl ILockedFramebuffer for LockedFramebuffer {
    fn address(&self) -> *mut u8 {
        self.address
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        let length = self.row_bytes.max(0) as usize * self.size.height.max(0) as usize;
        if self.address.is_null() || length == 0 {
            access(&mut []);
            return;
        }

        // SAFETY: whoever created the framebuffer guarantees
        // `row_bytes * height` bytes of pixel memory at `address` for as
        // long as the framebuffer is locked.
        access(unsafe { std::slice::from_raw_parts_mut(self.address, length) });
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
        self.format
    }

    fn alpha_format(&self) -> AlphaFormat {
        self.alpha_format
    }

    fn dispose(&self) {
        if let Some(on_dispose) = self.on_dispose.take() {
            on_dispose();
        }
    }
}
