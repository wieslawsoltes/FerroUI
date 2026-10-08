//! The software framebuffer of a top-level: pixels are drawn into memory
//! owned by the framebuffer and handed to the native render target when the
//! framebuffer is disposed.

use crate::helpers::ComResultExt;
use crate::interop::*;
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat};
use ferroui_base::{PixelSize, Vector};
use std::cell::RefCell;
use std::ffi::c_void;

/// Runs the given callback with the native render target if the native
/// top-level is still alive and the render target is not disposed.
///
/// Deviation (DEVIATIONS.md, Native backend): the reference callback passes
/// the native top-level, which no caller uses, and `DeferredFramebuffer`
/// keeps the render target itself. Here the callback lends the render target
/// under the lock of the surface, so that the framebuffer holds no reference
/// of its own to it: the reference counts of the native objects are not
/// atomic, and the render target is used by the thread that renders.
pub(crate) type LockTopLevel = Box<dyn Fn(&mut dyn FnMut(&IFrnSoftwareRenderTarget))>;

pub(crate) struct DeferredFramebuffer {
    lock_top_level: LockTopLevel,
    /// The pixel memory; `None` once the framebuffer is disposed.
    data: RefCell<Option<Box<[u8]>>>,
    size: PixelSize,
    row_bytes: i32,
    dpi: Vector,
    format: PixelFormat,
    alpha_format: AlphaFormat,
}

impl DeferredFramebuffer {
    pub(crate) fn new(lock_top_level: LockTopLevel, width: i32, height: i32, dpi: Vector) -> Self {
        let length = width.max(0) as usize * height.max(0) as usize * 4;
        Self {
            lock_top_level,
            data: RefCell::new(Some(vec![0u8; length].into_boxed_slice())),
            size: PixelSize::new(width, height),
            row_bytes: width * 4,
            dpi,
            format: PixelFormat::BGRA8888,
            alpha_format: AlphaFormat::default(),
        }
    }
}

impl ILockedFramebuffer for DeferredFramebuffer {
    fn address(&self) -> *mut u8 {
        match self.data.borrow_mut().as_mut() {
            Some(data) => data.as_mut_ptr(),
            None => std::ptr::null_mut(),
        }
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        let mut data = self.data.borrow_mut();
        match data.as_mut() {
            Some(data) => access(data),
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
        self.format
    }

    fn alpha_format(&self) -> AlphaFormat {
        self.alpha_format
    }

    fn dispose(&self) {
        let Some(mut data) = self.data.borrow_mut().take() else {
            return;
        };

        (self.lock_top_level)(&mut |render_target: &IFrnSoftwareRenderTarget| {
            let mut fb = FrnFramebuffer {
                data: data.as_mut_ptr() as *mut c_void,
                dpi: FrnVector { x: self.dpi.x, y: self.dpi.y },
                width: self.size.width,
                height: self.size.height,
                // The framebuffer is always created as BGRA8888.
                pixel_format: FrnPixelFormat::kFrnBgra8888,
                stride: self.row_bytes,
            };

            // SAFETY: `fb` and the pixel memory it points at outlive the
            // call; the native side copies the pixels before returning.
            unsafe { render_target.set_frame(&mut fb) }.check();
        });

        // The pixel memory is freed here.
        drop(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_microcom::{ComPtr, HResult};
    use std::cell::Cell;
    use std::rc::Rc;

    #[derive(Default)]
    struct Frames {
        count: Cell<u32>,
        last: RefCell<Option<(i32, i32, i32, FrnVector, FrnPixelFormat, Vec<u8>)>>,
    }

    struct FakeRenderTarget(Rc<Frames>);

    impl IFrnSoftwareRenderTargetImpl for FakeRenderTarget {
        fn set_frame(&self, fb: *mut FrnFramebuffer) -> Result<(), HResult> {
            // SAFETY: the framebuffer hands a valid descriptor whose data is
            // `stride * height` bytes long.
            let fb = unsafe { &*fb };
            let pixels =
                unsafe { std::slice::from_raw_parts(fb.data as *const u8, (fb.stride * fb.height) as usize) }.to_vec();
            self.0.count.set(self.0.count.get() + 1);
            *self.0.last.borrow_mut() = Some((fb.width, fb.height, fb.stride, fb.dpi, fb.pixel_format, pixels));
            Ok(())
        }
    }

    fn lock(render_target: ComPtr<IFrnSoftwareRenderTarget>, alive: Rc<Cell<bool>>) -> LockTopLevel {
        Box::new(move |cb: &mut dyn FnMut(&IFrnSoftwareRenderTarget)| {
            if alive.get() {
                cb(&*render_target);
            }
        })
    }

    fn framebuffer(frames: &Rc<Frames>, alive: &Rc<Cell<bool>>, width: i32, height: i32) -> DeferredFramebuffer {
        DeferredFramebuffer::new(
            lock(IFrnSoftwareRenderTarget::from_impl(FakeRenderTarget(frames.clone())), alive.clone()),
            width,
            height,
            Vector::new(192.0, 192.0),
        )
    }

    #[test]
    fn describes_a_bgra_buffer_of_the_requested_size() {
        let frames = Rc::new(Frames::default());
        let alive = Rc::new(Cell::new(true));
        let fb = framebuffer(&frames, &alive, 3, 2);

        assert_eq!(fb.size(), PixelSize::new(3, 2));
        assert_eq!(fb.row_bytes(), 12);
        assert_eq!(fb.dpi(), Vector::new(192.0, 192.0));
        assert_eq!(fb.format(), PixelFormat::BGRA8888);
        assert!(!fb.address().is_null());
        let mut length = 0;
        fb.with_data(&mut |data| length = data.len());
        assert_eq!(length, 24);
        assert_eq!(frames.count.get(), 0, "nothing is presented before the framebuffer is disposed");
    }

    #[test]
    fn dispose_presents_the_pixels_once() {
        let frames = Rc::new(Frames::default());
        let alive = Rc::new(Cell::new(true));
        let fb = framebuffer(&frames, &alive, 2, 2);
        fb.with_data(&mut |data| {
            for (i, byte) in data.iter_mut().enumerate() {
                *byte = i as u8;
            }
        });

        fb.dispose();

        assert_eq!(frames.count.get(), 1);
        let (width, height, stride, dpi, format, pixels) = frames.last.borrow_mut().take().unwrap();
        assert_eq!((width, height, stride), (2, 2, 8));
        assert_eq!(dpi, FrnVector { x: 192.0, y: 192.0 });
        assert_eq!(format, FrnPixelFormat::kFrnBgra8888);
        assert_eq!(pixels, (0..16).collect::<Vec<u8>>());

        // Disposed: the memory is gone and a second dispose does nothing.
        assert!(fb.address().is_null());
        fb.dispose();
        assert_eq!(frames.count.get(), 1);
    }

    #[test]
    fn dispose_presents_nothing_when_the_top_level_is_gone() {
        let frames = Rc::new(Frames::default());
        let alive = Rc::new(Cell::new(false));
        let fb = framebuffer(&frames, &alive, 4, 4);

        fb.dispose();

        assert_eq!(frames.count.get(), 0);
        assert!(fb.address().is_null());
    }
}
