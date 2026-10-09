use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat as PlatformPixelFormat, PixelFormats};
use ferroui_base::{PixelSize, Size, Vector};
use ferroui_remote_protocol::viewport::{FrameMessage, PixelFormat as ProtocolPixelFormat};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FrameStatus {
    NotRendered,
    Rendered,
    CopiedToMessage,
}

/// `new PlatformPixelFormat((PixelFormatEnum)format)`: the first three
/// members of the two enumerations are the same.
pub(super) fn platform_pixel_format(format: ProtocolPixelFormat) -> PlatformPixelFormat {
    match format {
        ProtocolPixelFormat::Rgb565 => PixelFormats::RGB565,
        ProtocolPixelFormat::Rgba8888 => PixelFormats::RGBA8888,
        ProtocolPixelFormat::Bgra8888 => PixelFormats::BGRA8888,
    }
}

/// What `_dataLock` guards.
struct FramebufferData {
    /// `_data`: for rendering only. Taken out while a frame is rendered.
    data: Option<Vec<u8>>,
    /// `_dataCopy`: for messages only.
    data_copy: Vec<u8>,
    status: FrameStatus,
    /// The monitor is held: between `lock` and the dispose of what it
    /// returned.
    locked: bool,
}

pub(super) struct Framebuffer {
    dpi: f64,
    frame_size: PixelSize,
    // Deviation (DEVIATIONS.md, Remote rendering): the original pins `_data`
    // and holds the monitor `_dataLock` from `Lock` until the framebuffer is
    // unlocked. A guard of a mutex cannot be kept in the locked framebuffer,
    // so the monitor is a flag with a condition: `lock`, `get_status` and
    // `to_message` wait while a frame is rendered, as they block in the
    // original. The monitor of the original is reentrant; this one is not.
    data_lock: Mutex<FramebufferData>,
    unlocked: Condvar,
    format: ProtocolPixelFormat,
    client_size: Size,
    render_scaling: f64,
    stride: i32,
}

impl Framebuffer {
    /// `Framebuffer.Empty`.
    pub(super) fn empty() -> Arc<Framebuffer> {
        static EMPTY: OnceLock<Arc<Framebuffer>> = OnceLock::new();
        EMPTY.get_or_init(|| Arc::new(Framebuffer::new(ProtocolPixelFormat::Rgba8888, Size::default(), 1.0))).clone()
    }

    pub(super) fn new(format: ProtocolPixelFormat, client_size: Size, render_scaling: f64) -> Framebuffer {
        let mut frame_size = PixelSize::from_size(client_size, render_scaling);
        if frame_size.width <= 0 || frame_size.height <= 0 {
            frame_size = PixelSize::EMPTY;
        }

        let bpp = if format == ProtocolPixelFormat::Rgb565 { 2 } else { 4 };
        let stride = frame_size.width * bpp;
        let data_length = (stride * frame_size.height).max(0) as usize;

        let (stride, data, data_copy) =
            if data_length > 0 { (stride, vec![0u8; data_length], vec![0u8; data_length]) } else { (0, Vec::new(), Vec::new()) };

        Framebuffer {
            dpi: render_scaling * 96.0,
            frame_size,
            data_lock: Mutex::new(FramebufferData {
                data: Some(data),
                data_copy,
                status: FrameStatus::NotRendered,
                locked: false,
            }),
            unlocked: Condvar::new(),
            format,
            client_size,
            render_scaling,
            stride,
        }
    }

    pub(super) fn format(&self) -> ProtocolPixelFormat {
        self.format
    }

    pub(super) fn client_size(&self) -> Size {
        self.client_size
    }

    pub(super) fn render_scaling(&self) -> f64 {
        self.render_scaling
    }

    pub(super) fn stride(&self) -> i32 {
        self.stride
    }

    /// `lock (_dataLock)`: waits while a frame is rendered.
    fn enter(&self) -> MutexGuard<'_, FramebufferData> {
        let mut data = self.data_lock.lock().unwrap_or_else(PoisonError::into_inner);
        while data.locked {
            data = self.unlocked.wait(data).unwrap_or_else(PoisonError::into_inner);
        }
        data
    }

    pub(super) fn get_status(&self) -> FrameStatus {
        self.enter().status
    }

    pub(super) fn lock(self: &Arc<Self>, on_unlocked: Box<dyn Fn()>) -> Rc<dyn ILockedFramebuffer> {
        let memory = {
            let mut data = self.enter();
            data.locked = true;
            data.data.take().unwrap_or_default()
        };

        Rc::new(LockedFramebuffer {
            framebuffer: self.clone(),
            memory: RefCell::new(Some(memory)),
            size: self.frame_size,
            row_bytes: self.stride,
            dpi: Vector::new(self.dpi, self.dpi),
            format: platform_pixel_format(self.format),
            alpha_format: if self.format == ProtocolPixelFormat::Rgb565 { AlphaFormat::Opaque } else { AlphaFormat::Premul },
            on_unlocked,
        })
    }

    /// The returned message is a copy of the frame.
    // Deviation (DEVIATIONS.md, Remote rendering): the message of the
    // original shares `_dataCopy` and must not be kept around; a message of
    // the port owns its bytes, so the frame is copied once more here.
    pub(super) fn to_message(&self, sequence_id: i64) -> FrameMessage {
        let data = {
            let mut data = self.enter();
            data.status = FrameStatus::CopiedToMessage;
            data.data_copy.clone()
        };

        FrameMessage {
            sequence_id,
            data: Some(data),
            format: self.format,
            width: self.frame_size.width,
            height: self.frame_size.height,
            stride: self.stride,
            dpi_x: self.dpi,
            dpi_y: self.dpi,
        }
    }
}

/// `LockedFramebuffer` over the pinned `_data` of the original: the pixels
/// of the frame that is being rendered. It belongs to the thread that
/// renders the frame.
struct LockedFramebuffer {
    framebuffer: Arc<Framebuffer>,
    /// The pixels, until the framebuffer is disposed.
    memory: RefCell<Option<Vec<u8>>>,
    size: PixelSize,
    row_bytes: i32,
    dpi: Vector,
    format: PlatformPixelFormat,
    alpha_format: AlphaFormat,
    on_unlocked: Box<dyn Fn()>,
}

impl ILockedFramebuffer for LockedFramebuffer {
    fn address(&self) -> *mut u8 {
        match self.memory.borrow_mut().as_mut() {
            Some(memory) => memory.as_mut_ptr(),
            None => std::ptr::null_mut(),
        }
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        match self.memory.borrow_mut().as_mut() {
            Some(memory) => access(memory),
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

    fn format(&self) -> PlatformPixelFormat {
        self.format
    }

    fn alpha_format(&self) -> AlphaFormat {
        self.alpha_format
    }

    fn dispose(&self) {
        let Some(memory) = self.memory.borrow_mut().take() else {
            return;
        };

        {
            let mut data = self.framebuffer.data_lock.lock().unwrap_or_else(PoisonError::into_inner);
            data.data_copy.copy_from_slice(&memory);
            data.data = Some(memory);
            data.status = FrameStatus::Rendered;
            data.locked = false;
        }
        self.framebuffer.unlocked.notify_all();
        (self.on_unlocked)();
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn the_framebuffer_is_shared_between_the_threads() {
        assert_send_sync::<Framebuffer>();
    }

    #[test]
    fn the_empty_framebuffer_has_no_pixels() {
        let empty = Framebuffer::empty();
        assert_eq!(0, empty.stride());
        assert_eq!(ProtocolPixelFormat::Rgba8888, empty.format());
        assert_eq!(FrameStatus::NotRendered, empty.get_status());
        assert!(Arc::ptr_eq(&empty, &Framebuffer::empty()));
    }

    #[test]
    fn a_size_without_pixels_gives_an_empty_frame() {
        let framebuffer = Framebuffer::new(ProtocolPixelFormat::Bgra8888, Size::new(0.0, 10.0), 1.0);
        assert_eq!(0, framebuffer.stride());
        let message = framebuffer.to_message(1);
        assert_eq!((0, 0, 0), (message.width, message.height, message.stride));
        assert_eq!(Some(Vec::new()), message.data);
    }

    #[test]
    fn the_stride_follows_the_format_and_the_scaling() {
        let framebuffer = Framebuffer::new(ProtocolPixelFormat::Rgb565, Size::new(3.0, 2.0), 2.0);
        assert_eq!(12, framebuffer.stride());
        let framebuffer = Framebuffer::new(ProtocolPixelFormat::Rgba8888, Size::new(3.0, 2.0), 2.0);
        assert_eq!(24, framebuffer.stride());
        assert_eq!(Size::new(3.0, 2.0), framebuffer.client_size());
        assert_eq!(2.0, framebuffer.render_scaling());
    }

    #[test]
    fn a_rendered_frame_is_copied_to_the_message_when_it_is_unlocked() {
        let framebuffer = Arc::new(Framebuffer::new(ProtocolPixelFormat::Rgba8888, Size::new(2.0, 2.0), 1.0));
        let unlocked = Arc::new(AtomicUsize::new(0));
        let counter = unlocked.clone();
        let locked = framebuffer.lock(Box::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        }));
        assert_eq!(PixelSize::new(2, 2), locked.size());
        assert_eq!(8, locked.row_bytes());
        assert_eq!(Vector::new(96.0, 96.0), locked.dpi());
        assert_eq!(PixelFormats::RGBA8888, locked.format());
        assert_eq!(AlphaFormat::Premul, locked.alpha_format());
        locked.with_data(&mut |data| {
            assert_eq!(16, data.len());
            data.fill(9);
        });
        assert_eq!(0, unlocked.load(Ordering::SeqCst));
        locked.dispose();
        // A second dispose does nothing.
        locked.dispose();
        assert_eq!(1, unlocked.load(Ordering::SeqCst));
        assert_eq!(FrameStatus::Rendered, framebuffer.get_status());

        let message = framebuffer.to_message(7);
        assert_eq!(FrameStatus::CopiedToMessage, framebuffer.get_status());
        assert_eq!(7, message.sequence_id);
        assert_eq!((2, 2, 8), (message.width, message.height, message.stride));
        assert_eq!((96.0, 96.0), (message.dpi_x, message.dpi_y));
        assert_eq!(Some(vec![9u8; 16]), message.data);
    }

    #[test]
    fn the_status_is_read_after_the_frame_that_is_being_rendered() {
        let framebuffer = Arc::new(Framebuffer::new(ProtocolPixelFormat::Rgb565, Size::new(1.0, 1.0), 1.0));
        let shared = framebuffer.clone();
        let (ready_sender, ready) = std::sync::mpsc::channel();
        let (release_sender, release) = std::sync::mpsc::channel::<()>();
        let renderer = std::thread::spawn(move || {
            let locked = shared.lock(Box::new(|| {}));
            assert_eq!(AlphaFormat::Opaque, locked.alpha_format());
            ready_sender.send(()).unwrap();
            release.recv().unwrap();
            locked.dispose();
        });
        ready.recv().unwrap();
        let reader = {
            let framebuffer = framebuffer.clone();
            std::thread::spawn(move || framebuffer.get_status())
        };
        release_sender.send(()).unwrap();
        renderer.join().unwrap();
        assert_eq!(FrameStatus::Rendered, reader.join().unwrap());
    }
}
