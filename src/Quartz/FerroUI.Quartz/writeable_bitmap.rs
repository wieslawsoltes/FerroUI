//! Writable inputs retain their declared CPU format. A private premultiplied GPU
//! image is refreshed only when a lock commits changes; drawing never reads it back.
use crate::{bitmap::Bitmap, ffi};
use ferroui_base::{
    media::imaging::{BitmapEncoderOptions, BitmapMemory},
    platform::*,
    PixelSize, Vector,
};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    io::{self, Write},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

struct Inner {
    bitmap: Bitmap,
    memory: Mutex<BitmapMemory>,
    locked: AtomicBool,
    format: PixelFormat,
    alpha: AlphaFormat,
    stride: i32,
}
#[derive(Clone)]
pub struct WriteableBitmap(Arc<Inner>);
impl WriteableBitmap {
    pub fn new(size: PixelSize, dpi: Vector, format: PixelFormat, alpha: AlphaFormat) -> Self {
        assert!(
            matches!(
                format,
                PixelFormat::BGRA8888 | PixelFormat::RGBA8888 | PixelFormat::RGB565
            ),
            "Unsupported Quartz writable format"
        );
        let alpha = if format.has_alpha() {
            alpha
        } else {
            AlphaFormat::Opaque
        };
        let memory = BitmapMemory::new(format, alpha, size);
        let stride = memory.row_bytes();
        let bitmap = Bitmap::new(size, dpi);
        if alpha == AlphaFormat::Opaque {
            bitmap.context(|c| unsafe {
                ffi::q_clear(c, size.width as f64, size.height as f64, 0., 0., 0., 1.)
            });
        }
        Self(Arc::new(Inner {
            bitmap,
            memory: Mutex::new(memory),
            locked: AtomicBool::new(false),
            format,
            alpha,
            stride,
        }))
    }
    pub fn bitmap(&self) -> &Bitmap {
        &self.0.bitmap
    }
}
impl IBitmapImpl for WriteableBitmap {
    fn dpi(&self) -> Vector {
        self.bitmap().dpi()
    }
    fn pixel_size(&self) -> PixelSize {
        self.bitmap().pixel_size()
    }
    fn version(&self) -> i32 {
        self.bitmap().version()
    }
    fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> io::Result<()> {
        self.bitmap().save(stream, options)
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        Some(self)
    }
}
impl IReadableBitmapImpl for WriteableBitmap {
    fn format(&self) -> Option<PixelFormat> {
        Some(self.0.format)
    }
    fn alpha_format(&self) -> Option<AlphaFormat> {
        Some(self.0.alpha)
    }
    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        assert!(
            !self.0.locked.swap(true, Ordering::Acquire),
            "Quartz writable bitmap is already locked"
        );
        let original = self.0.memory.lock().unwrap().data().to_vec();
        Rc::new(Framebuffer {
            owner: self.clone(),
            bytes: RefCell::new(original.clone()),
            original,
            done: Cell::new(false),
        })
    }
}
impl IWriteableBitmapImpl for WriteableBitmap {}
struct Framebuffer {
    owner: WriteableBitmap,
    bytes: RefCell<Vec<u8>>,
    original: Vec<u8>,
    done: Cell<bool>,
}
impl ILockedFramebuffer for Framebuffer {
    fn address(&self) -> *mut u8 {
        assert!(!self.done.get());
        self.bytes.borrow_mut().as_mut_ptr()
    }
    fn with_data(&self, f: &mut dyn FnMut(&mut [u8])) {
        assert!(!self.done.get());
        f(&mut self.bytes.borrow_mut());
    }
    fn size(&self) -> PixelSize {
        self.owner.pixel_size()
    }
    fn row_bytes(&self) -> i32 {
        self.owner.0.stride
    }
    fn dpi(&self) -> Vector {
        self.owner.dpi()
    }
    fn format(&self) -> PixelFormat {
        self.owner.0.format
    }
    fn alpha_format(&self) -> AlphaFormat {
        self.owner.0.alpha
    }
    fn dispose(&self) {
        if self.done.replace(true) {
            return;
        }
        let bytes = self.bytes.borrow();
        if *bytes != self.original {
            let mut memory = self.owner.0.memory.lock().unwrap();
            memory.data_mut().copy_from_slice(&bytes);
            let size = self.owner.pixel_size();
            let stride = size.width * 4;
            let mut rgba = vec![0; stride as usize * size.height as usize];
            memory.copy_to_rgba(AlphaFormat::Premul, &mut rgba, stride);
            for p in rgba.chunks_exact_mut(4) {
                p.swap(0, 2);
            }
            self.owner.bitmap().write(&rgba);
        }
        self.owner.0.locked.store(false, Ordering::Release);
    }
}
impl Drop for Framebuffer {
    fn drop(&mut self) {
        self.dispose();
    }
}
