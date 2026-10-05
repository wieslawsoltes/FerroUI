use super::internal::UnmanagedBlob;
use super::{AlphaFormat, ILockedFramebuffer, PixelFormat};
use crate::{PixelSize, Vector};
use std::cell::RefCell;
use std::rc::Rc;

/// Pixel memory that outlives the frames drawn into it.
///
/// A platform without a native surface to lock (a script-driven canvas, a
/// test harness) keeps one of these per render target: each frame locks it,
/// the renderer draws, and unlocking hands the pixels to `blit`.
pub struct RetainedFramebuffer {
    size: PixelSize,
    row_bytes: i32,
    format: PixelFormat,
    alpha_format: AlphaFormat,
    blob: RefCell<Option<Rc<UnmanagedBlob>>>,
}

fn validate_known_format(format: PixelFormat) -> PixelFormat {
    if format.bits_per_pixel() % 8 == 0 {
        format
    } else {
        panic!("Specified argument was out of the range of valid values. (Parameter 'format')");
    }
}

impl RetainedFramebuffer {
    /// Creates a framebuffer whose rows are exactly as long as their pixels.
    ///
    /// # Panics
    /// Panics when a pixel of `format` does not occupy whole bytes or when
    /// `size` is empty.
    pub fn new(size: PixelSize, format: PixelFormat, alpha_format: AlphaFormat) -> Rc<Self> {
        let format = validate_known_format(format);
        Self::with_row_bytes(size, format, alpha_format, format.bits_per_pixel() as i32 / 8 * size.width)
    }

    /// Creates a framebuffer with the given row length.
    ///
    /// # Panics
    /// Panics when `size` is empty or `row_bytes` is too small for a row of
    /// pixels.
    pub fn with_row_bytes(size: PixelSize, format: PixelFormat, alpha_format: AlphaFormat, row_bytes: i32) -> Rc<Self> {
        if size.width <= 0 || size.height <= 0 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'size')");
        }
        if size.width * (format.bits_per_pixel() as i32 / 8) > row_bytes {
            panic!("Specified argument was out of the range of valid values. (Parameter 'rowBytes')");
        }
        Rc::new(Self {
            size,
            row_bytes,
            format,
            alpha_format,
            blob: RefCell::new(Some(Rc::new(UnmanagedBlob::new(row_bytes * size.height)))),
        })
    }

    /// The size in device pixels.
    pub fn size(&self) -> PixelSize {
        self.size
    }

    /// The number of bytes per row.
    pub fn row_bytes(&self) -> i32 {
        self.row_bytes
    }

    /// The pixel format.
    pub fn format(&self) -> PixelFormat {
        self.format
    }

    /// The alpha format.
    pub fn alpha_format(&self) -> AlphaFormat {
        self.alpha_format
    }

    /// The address of the first pixel.
    ///
    /// # Panics
    /// Panics when the framebuffer has been disposed.
    pub fn address(&self) -> *mut u8 {
        self.blob().address()
    }

    fn blob(&self) -> Rc<UnmanagedBlob> {
        match self.blob.borrow().as_ref() {
            Some(blob) => blob.clone(),
            None => panic!("Cannot access a disposed object: RetainedFramebuffer"),
        }
    }

    /// Locks the framebuffer for one frame. `blit` runs whenever the
    /// returned framebuffer is disposed and receives this framebuffer.
    ///
    /// # Panics
    /// Panics when the framebuffer has been disposed.
    pub fn lock(self: &Rc<Self>, dpi: Vector, blit: impl Fn(&Rc<RetainedFramebuffer>) + 'static) -> Rc<dyn ILockedFramebuffer> {
        let blob = self.blob();
        let this = self.clone();
        Rc::new(RetainedLockedFramebuffer {
            address: blob.address(),
            blob,
            size: self.size,
            row_bytes: self.row_bytes,
            dpi,
            format: self.format,
            alpha_format: self.alpha_format,
            on_dispose: Box::new(move || blit(&this)),
        })
    }

    /// Releases the pixel memory. Further calls do nothing.
    pub fn dispose(&self) {
        if let Some(blob) = self.blob.borrow_mut().take() {
            blob.dispose();
        }
    }
}

/// A lock of a [`RetainedFramebuffer`].
struct RetainedLockedFramebuffer {
    address: *mut u8,
    blob: Rc<UnmanagedBlob>,
    size: PixelSize,
    row_bytes: i32,
    dpi: Vector,
    format: PixelFormat,
    alpha_format: AlphaFormat,
    on_dispose: Box<dyn Fn()>,
}

impl ILockedFramebuffer for RetainedLockedFramebuffer {
    fn address(&self) -> *mut u8 {
        self.address
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        if !self.blob.with_data(access) {
            access(&mut []);
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
        self.format
    }

    fn alpha_format(&self) -> AlphaFormat {
        self.alpha_format
    }

    fn dispose(&self) {
        (self.on_dispose)();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PixelFormats;
    use std::cell::Cell;

    #[test]
    fn row_bytes_default_to_the_width_in_bytes() {
        let framebuffer = RetainedFramebuffer::new(PixelSize::new(10, 4), PixelFormats::RGBA8888, AlphaFormat::Premul);

        assert_eq!(PixelSize::new(10, 4), framebuffer.size());
        assert_eq!(40, framebuffer.row_bytes());
        assert_eq!(PixelFormats::RGBA8888, framebuffer.format());
        assert_eq!(AlphaFormat::Premul, framebuffer.alpha_format());
        assert!(!framebuffer.address().is_null());
    }

    #[test]
    fn a_sixteen_bit_format_takes_two_bytes_per_pixel() {
        let framebuffer = RetainedFramebuffer::new(PixelSize::new(3, 2), PixelFormats::RGB565, AlphaFormat::Opaque);

        assert_eq!(6, framebuffer.row_bytes());
    }

    #[test]
    fn lock_describes_the_retained_memory_and_blits_on_every_dispose() {
        let framebuffer = RetainedFramebuffer::with_row_bytes(
            PixelSize::new(2, 2),
            PixelFormats::BGRA8888,
            AlphaFormat::Premul,
            12,
        );
        let blitted = Rc::new(Cell::new(0));
        let seen = blitted.clone();
        let expected = framebuffer.address();

        let locked = framebuffer.lock(Vector::new(96.0, 192.0), move |fb| {
            assert_eq!(12, fb.row_bytes());
            seen.set(seen.get() + 1);
        });

        assert_eq!(expected, locked.address());
        assert_eq!(PixelSize::new(2, 2), locked.size());
        assert_eq!(12, locked.row_bytes());
        assert_eq!(Vector::new(96.0, 192.0), locked.dpi());
        assert_eq!(PixelFormats::BGRA8888, locked.format());
        assert_eq!(AlphaFormat::Premul, locked.alpha_format());
        let mut length = 0;
        locked.with_data(&mut |data| {
            length = data.len();
            data[0] = 7;
        });
        assert_eq!(24, length);
        assert_eq!(0, blitted.get());

        locked.dispose();
        assert_eq!(1, blitted.get());

        // Every unlock presents the pixels again.
        locked.dispose();
        assert_eq!(2, blitted.get());
    }

    #[test]
    fn pixels_survive_between_locks() {
        let framebuffer = RetainedFramebuffer::new(PixelSize::new(1, 1), PixelFormats::RGBA8888, AlphaFormat::Premul);

        let first = framebuffer.lock(Vector::new(96.0, 96.0), |_| {});
        first.with_data(&mut |data| data.copy_from_slice(&[1, 2, 3, 4]));
        first.dispose();

        let second = framebuffer.lock(Vector::new(96.0, 96.0), |_| {});
        let mut read = Vec::new();
        second.with_data(&mut |data| read = data.to_vec());
        assert_eq!(vec![1, 2, 3, 4], read);
    }

    #[test]
    #[should_panic(expected = "Parameter 'format'")]
    fn a_format_with_partial_bytes_is_rejected() {
        RetainedFramebuffer::new(PixelSize::new(8, 8), PixelFormats::BLACK_WHITE, AlphaFormat::Opaque);
    }

    #[test]
    #[should_panic(expected = "Parameter 'size'")]
    fn an_empty_size_is_rejected() {
        RetainedFramebuffer::new(PixelSize::new(0, 8), PixelFormats::RGBA8888, AlphaFormat::Premul);
    }

    #[test]
    #[should_panic(expected = "Parameter 'rowBytes'")]
    fn rows_shorter_than_their_pixels_are_rejected() {
        RetainedFramebuffer::with_row_bytes(PixelSize::new(4, 4), PixelFormats::RGBA8888, AlphaFormat::Premul, 15);
    }

    #[test]
    #[should_panic(expected = "Cannot access a disposed object: RetainedFramebuffer")]
    fn a_disposed_framebuffer_cannot_be_locked() {
        let framebuffer = RetainedFramebuffer::new(PixelSize::new(1, 1), PixelFormats::RGBA8888, AlphaFormat::Premul);
        framebuffer.dispose();
        framebuffer.dispose();
        framebuffer.lock(Vector::new(96.0, 96.0), |_| {});
    }
}
