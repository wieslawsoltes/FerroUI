//! The render target of the shared memory framebuffer (the port of
//! `XShm/X11ShmFramebufferRenderTarget.cs`).

use super::x11_shm_image::X11ShmImage;
use crate::x11_deferred_display_dispatcher::X11DeferredDisplayDispatcher;
use crate::xlib::{self, VisualPointer, XDisplay, XID};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::surfaces::{FramebufferLockProperties, IFramebufferRenderTarget, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat, RenderTargetSceneInfo};
use ferroui_base::{PixelSize, Vector};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

/// Maximum number of frames allowed to be in flight (submitted to the server, awaiting completion)
/// before `lock` blocks waiting for a slot to free up.
pub(crate) const MAX_IN_FLIGHT: i32 = 2;

/// Something with a size that is kept for reuse (an image, or a stand-in
/// of one in the tests).
pub(crate) trait Sized2D {
    fn pixel_size(&self) -> PixelSize;
}

impl Sized2D for Rc<X11ShmImage> {
    fn pixel_size(&self) -> PixelSize {
        self.size()
    }
}

/// What a render target keeps between frames, apart from the server: the
/// images that wait for reuse, the size they are for, and how many frames
/// the server has not completed. The methods return the images the caller
/// has to dispose.
pub(crate) struct ImagePool<I: Sized2D> {
    available_queue: VecDeque<I>,
    last_size: Option<PixelSize>,
    in_flight: i32,
    is_disposed: bool,
}

impl<I: Sized2D> ImagePool<I> {
    pub(crate) fn new() -> Self {
        Self { available_queue: VecDeque::new(), last_size: None, in_flight: 0, is_disposed: false }
    }

    /// Whether `lock` has to wait for a completion before it takes an
    /// image.
    pub(crate) fn is_full(&self) -> bool {
        self.in_flight >= MAX_IN_FLIGHT
    }

    pub(crate) fn in_flight(&self) -> i32 {
        self.in_flight
    }

    /// `GetOrCreateImage` up to the creation: an image of `size` from the
    /// queue when there is one (the caller creates one otherwise), and the
    /// images that were passed over and are to be disposed.
    pub(crate) fn take(&mut self, size: PixelSize) -> (Option<I>, Vec<I>) {
        let mut discarded = Vec::new();
        if self.last_size != Some(size) {
            discarded.extend(self.available_queue.drain(..));
        }

        self.last_size = Some(size);

        while let Some(available) = self.available_queue.pop_front() {
            if available.pixel_size() == size {
                return (Some(available), discarded);
            }

            discarded.push(available);
        }

        (None, discarded)
    }

    /// Called when a frame has been submitted to the server (XShmPutImage). Increments the in-flight count.
    pub(crate) fn on_image_submitted(&mut self) {
        self.in_flight += 1;
    }

    /// Called when X server signals completion for one of our images.
    /// Returns the image when it is to be disposed instead of reused.
    pub(crate) fn on_x_shm_completion(&mut self, image: I) -> Option<I> {
        self.in_flight -= 1;

        // Drop buffers that no longer match the current size (or belong to a disposed target) instead of
        // returning them to the reuse pool.
        if self.is_disposed || Some(image.pixel_size()) != self.last_size {
            return Some(image);
        }

        self.available_queue.push_back(image);
        None
    }

    /// `Dispose`: the images of the queue, which are to be disposed.
    // Images still in flight stay registered with the dispatcher and are disposed when their
    // completion is drained (see `on_x_shm_completion`).
    pub(crate) fn dispose(&mut self) -> Vec<I> {
        self.is_disposed = true;
        self.available_queue.drain(..).collect()
    }
}

/// Per-window XShm render target. Owns a small pool of reusable [`X11ShmImage`] buffers and
/// the in-flight counter used for backpressure. Completion events are routed back here by the shared
/// [`X11DeferredDisplayDispatcher`].
pub struct X11ShmFramebufferRenderTarget {
    deferred_display: XDisplay,
    window_x_id: XID,
    visual: VisualPointer,
    depth: i32,
    dispatcher: Rc<X11DeferredDisplayDispatcher>,
    pool: Rc<RefCell<ImagePool<Rc<X11ShmImage>>>>,
}

fn log_debug(message: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Debug, LogArea::X11_PLATFORM) {
        logger.log(None, message);
    }
}

impl X11ShmFramebufferRenderTarget {
    pub fn new(
        deferred_display: XDisplay,
        window_x_id: XID,
        visual: VisualPointer,
        depth: i32,
        dispatcher: Rc<X11DeferredDisplayDispatcher>,
    ) -> Self {
        Self { deferred_display, window_x_id, visual, depth, dispatcher, pool: Rc::new(RefCell::new(ImagePool::new())) }
    }

    fn get_or_create_image(&self, size: PixelSize) -> Rc<X11ShmImage> {
        let (available, discarded) = self.pool.borrow_mut().take(size);
        for image in discarded {
            image.dispose();
        }
        if let Some(available) = available {
            log_debug("[X11ShmFramebufferRenderTarget] Reuse X11ShmImage from available queue");
            return available;
        }

        Rc::new(X11ShmImage::new(size, self.deferred_display, self.visual, self.depth))
    }
}

impl IPlatformRenderSurfaceRenderTarget for X11ShmFramebufferRenderTarget {}

impl IFramebufferRenderTarget for X11ShmFramebufferRenderTarget {
    fn lock(&self, _scene_info: &RenderTargetSceneInfo) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties) {
        log_debug("[X11ShmFramebufferRenderTarget] Lock");

        // Attempt to handle any pending shm completion events
        self.dispatcher.drain_pending_events();

        // Block until a free slot becomes available
        while self.pool.borrow().is_full() {
            self.dispatcher.drain_events_blocking_at_most_once();
        }

        // The reference reads the geometry unchecked; a window that is gone has no size.
        let geometry = xlib::x_get_geometry(self.deferred_display, self.window_x_id).unwrap_or_default();
        let size = PixelSize::new(geometry.width, geometry.height);

        let image = self.get_or_create_image(size);
        let locked = X11ShmLockedFramebuffer {
            pool: self.pool.clone(),
            dispatcher: self.dispatcher.clone(),
            window_x_id: self.window_x_id,
            image,
            sent: Cell::new(false),
        };
        (Rc::new(locked), FramebufferLockProperties::default())
    }

    fn dispose(&self) {
        log_debug("[X11ShmFramebufferRenderTarget] Dispose");
        let images = self.pool.borrow_mut().dispose();
        for image in images {
            image.dispose();
        }
    }
}

struct X11ShmLockedFramebuffer {
    pool: Rc<RefCell<ImagePool<Rc<X11ShmImage>>>>,
    dispatcher: Rc<X11DeferredDisplayDispatcher>,
    window_x_id: XID,
    image: Rc<X11ShmImage>,
    sent: Cell<bool>,
}

impl X11ShmLockedFramebuffer {
    fn send_render(&self) {
        let image = self.image.clone();
        if image.put(self.window_x_id) {
            // The request was accepted, so the server will emit a matching XShmCompletionEvent. Register a
            // completion callback and count the frame as in flight only now, so a failed put can never leave
            // the backpressure counter stuck (a frame that never completes would otherwise block Lock forever).
            let pool = self.pool.clone();
            let shm_seg = image.shm_seg();
            self.dispatcher.register_for_shm_completion(
                shm_seg,
                Box::new(move || {
                    let to_dispose = pool.borrow_mut().on_x_shm_completion(image);
                    if let Some(logger) = Logger::try_get(LogEventLevel::Debug, LogArea::X11_PLATFORM) {
                        logger.log_with_values(
                            None,
                            "[X11ShmFramebufferRenderTarget] Completion, InFlight={InFlight}",
                            &[&pool.borrow().in_flight()],
                        );
                    }
                    if let Some(image) = to_dispose {
                        image.dispose();
                    }
                }),
            );
            self.pool.borrow_mut().on_image_submitted();
            if let Some(logger) = Logger::try_get(LogEventLevel::Debug, LogArea::X11_PLATFORM) {
                logger.log_with_values(
                    None,
                    "[X11ShmFramebufferRenderTarget] Submitted, InFlight={InFlight}",
                    &[&self.pool.borrow().in_flight()],
                );
            }
            log_debug("[X11ShmLockedFramebuffer] SendRender XShmPutImage");
        } else {
            // No completion will arrive for a failed put; drop the image so it never counts toward backpressure.
            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::X11_PLATFORM) {
                logger.log(None, "[X11ShmLockedFramebuffer] XShmPutImage failed, dropping image");
            }
            image.dispose();
        }
    }
}

impl ILockedFramebuffer for X11ShmLockedFramebuffer {
    fn address(&self) -> *mut u8 {
        self.image.shm_addr()
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        let address = self.image.shm_addr();
        if address.is_null() || self.sent.get() {
            access(&mut []);
            return;
        }
        let size = self.image.size();
        let length = (size.width * X11ShmImage::BYTE_SIZE_OF_PIXEL * size.height) as usize;
        // SAFETY: the segment of the image has `length` bytes at its address and stays attached
        // while the image is not disposed; the frame was not sent yet, so the image is owned by
        // this lock alone, and the contract of the method forbids access from within `access`.
        access(unsafe { std::slice::from_raw_parts_mut(address, length) });
    }

    fn size(&self) -> PixelSize {
        self.image.size()
    }

    fn row_bytes(&self) -> i32 {
        self.image.size().width * X11ShmImage::BYTE_SIZE_OF_PIXEL
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

    fn dispose(&self) {
        // A frame is sent once: a second call would submit the image again.
        if !self.sent.replace(true) {
            self.send_render();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file. The images need a server; the
    // tests cover what the target decides about them.
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct Image(u32, PixelSize);

    impl Sized2D for Image {
        fn pixel_size(&self) -> PixelSize {
            self.1
        }
    }

    const SMALL: PixelSize = PixelSize { width: 10, height: 10 };
    const LARGE: PixelSize = PixelSize { width: 20, height: 10 };

    #[test]
    fn a_completed_image_of_the_current_size_is_reused() {
        let mut pool = ImagePool::<Image>::new();
        assert_eq!(pool.take(SMALL), (None, vec![]));
        pool.on_image_submitted();
        assert_eq!(pool.in_flight(), 1);
        assert_eq!(pool.on_x_shm_completion(Image(1, SMALL)), None);
        assert_eq!(pool.in_flight(), 0);
        assert_eq!(pool.take(SMALL), (Some(Image(1, SMALL)), vec![]));
        // The queue is empty again: the next frame needs a new image.
        assert_eq!(pool.take(SMALL), (None, vec![]));
    }

    #[test]
    fn two_frames_in_flight_make_the_lock_wait() {
        let mut pool = ImagePool::<Image>::new();
        assert!(!pool.is_full());
        pool.on_image_submitted();
        assert!(!pool.is_full());
        pool.on_image_submitted();
        assert!(pool.is_full());
        pool.on_x_shm_completion(Image(1, SMALL));
        assert!(!pool.is_full());
    }

    #[test]
    fn images_of_another_size_are_disposed() {
        let mut pool = ImagePool::<Image>::new();
        pool.take(SMALL);
        pool.on_image_submitted();
        pool.on_image_submitted();
        assert_eq!(pool.on_x_shm_completion(Image(1, SMALL)), None);
        // The window was resized: what waits in the queue goes, and what completes later too.
        assert_eq!(pool.take(LARGE), (None, vec![Image(1, SMALL)]));
        assert_eq!(pool.on_x_shm_completion(Image(2, SMALL)), Some(Image(2, SMALL)));
        pool.on_image_submitted();
        assert_eq!(pool.on_x_shm_completion(Image(3, LARGE)), None);
        assert_eq!(pool.take(LARGE), (Some(Image(3, LARGE)), vec![]));
    }

    #[test]
    fn a_disposed_target_disposes_its_queue_and_what_completes_afterwards() {
        let mut pool = ImagePool::<Image>::new();
        pool.take(SMALL);
        pool.on_image_submitted();
        pool.on_image_submitted();
        pool.on_x_shm_completion(Image(1, SMALL));
        assert_eq!(pool.dispose(), vec![Image(1, SMALL)]);
        assert_eq!(pool.on_x_shm_completion(Image(2, SMALL)), Some(Image(2, SMALL)));
        assert_eq!(pool.in_flight(), 0);
    }
}
