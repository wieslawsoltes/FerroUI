//! Partial presentation of an EGL window surface (upstream `EglPartialPresentation` and
//! `EglBufferAgeTracker`, 070c3be990): the age of the back buffer (`EGL_EXT_buffer_age`)
//! names its contents, the region a frame repaints is declared to the driver
//! (`EGL_KHR_partial_update`), and the frame is presented with the region that changed
//! (`EGL_KHR_swap_buffers_with_damage` or `EGL_EXT_swap_buffers_with_damage`).

use super::egl_consts::{
    EGL_BAD_NATIVE_WINDOW, EGL_BAD_SURFACE, EGL_BUFFER_AGE_EXT, EGL_BUFFER_DESTROYED, EGL_SWAP_BEHAVIOR,
};
use super::{EglInterface, EglPartialPresentationSupport};
use crate::OpenGlError;
use ferroui_base::platform::{IPartialPresentationBuffer, IPartialPresentationFeature};
use ferroui_base::{PixelRect, PixelSize};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The number of presented buffers the age of a buffer is looked up in.
const HISTORY: usize = 16;

/// After this many frames in a row with a buffer of age zero, the render target prefers its
/// retained layer: the driver never gives back a buffer with contents.
const ZERO_AGE_FRAMES_BEFORE_RETAINED_LAYER: u32 = 8;

/// The contents of a back buffer.
#[derive(Debug)]
pub(crate) struct EglBuffer {
    valid: Cell<bool>,
}

impl EglBuffer {
    fn new() -> Rc<EglBuffer> {
        Rc::new(EglBuffer { valid: Cell::new(true) })
    }
}

impl IPartialPresentationBuffer for EglBuffer {
    fn is_valid(&self) -> bool {
        self.valid.get()
    }
}

/// The buffers presented last, by their age: EGL gives the age of the contents of the back
/// buffer, not the index of a buffer of a swap chain.
#[derive(Default)]
pub(crate) struct EglBufferAgeTracker {
    history: [Option<Rc<EglBuffer>>; HISTORY],
    next: usize,
    count: usize,
}

impl EglBufferAgeTracker {
    /// The buffer whose contents a back buffer of `age` holds; a new one for an age of zero
    /// (no contents), an age older than the history, or contents that are no longer valid.
    pub(crate) fn acquire(&self, age: i32) -> Rc<EglBuffer> {
        if age > 0 && (age as usize) <= self.count {
            let index = (self.next + HISTORY - age as usize) % HISTORY;
            if let Some(buffer) = self.history[index].as_ref().filter(|buffer| buffer.is_valid()) {
                return buffer.clone();
            }
        }
        EglBuffer::new()
    }

    /// `buffer` was presented.
    pub(crate) fn presented(&mut self, buffer: Rc<EglBuffer>) {
        let evicted = self.history[self.next].replace(buffer);
        self.next = (self.next + 1) % HISTORY;
        self.count = (self.count + 1).min(HISTORY);
        // A buffer can be in several slots of the history: it stays valid until its last slot
        // is evicted.
        if let Some(evicted) = evicted {
            if !self.history.iter().flatten().any(|buffer| Rc::ptr_eq(buffer, &evicted)) {
                evicted.valid.set(false);
            }
        }
    }

    /// Forgets every buffer, whose contents are no longer valid.
    pub(crate) fn invalidate(&mut self) {
        for buffer in self.history.iter_mut().filter_map(Option::take) {
            buffer.valid.set(false);
        }
        self.next = 0;
        self.count = 0;
    }
}

/// Writes `rects`, clipped to a surface of `size`, into `destination` as EGL rectangles (x,
/// y from the bottom unless the surface is flipped, width, height) and returns their count.
/// No rectangle, after clipping too, is one rectangle of zero area: both extensions take a
/// count of zero for the whole surface.
pub(crate) fn write_rects(rects: &[PixelRect], size: PixelSize, destination: &mut Vec<i32>, is_y_flipped: bool) -> i32 {
    destination.clear();
    let surface = PixelRect::new(0, 0, size.width, size.height);
    for rect in rects {
        let clipped = rect.intersect(surface);
        if clipped.width <= 0 || clipped.height <= 0 {
            continue;
        }
        let y = if is_y_flipped { clipped.y } else { size.height - clipped.bottom() };
        destination.extend_from_slice(&[clipped.x, y, clipped.width, clipped.height]);
    }
    if destination.is_empty() {
        destination.extend_from_slice(&[0, 0, 0, 0]);
    }
    (destination.len() / 4) as i32
}

struct State {
    buffers: EglBufferAgeTracker,
    surface: isize,
    size: PixelSize,
    is_y_flipped: bool,
    can_set_dirty_rects: bool,
    consecutive_zero_age_frames: u32,
    /// The rectangles of the frames, reused between sessions.
    frame_dirty_rects: Vec<i32>,
    buffer_dirty_rects: Vec<i32>,
}

/// The partial presentation of one window render target: what
/// `EglPlatformSurfaceRenderTargetBase` keeps, and what a render target that presents on its
/// own (the DRM output) keeps.
pub struct EglPartialPresentation {
    egl: Rc<EglInterface>,
    display: isize,
    support: EglPartialPresentationSupport,
    /// Set when the render target has to be made again (a declared region or a swap failed).
    corrupted: Rc<Cell<bool>>,
    prefer_retained_layer: Cell<bool>,
    state: RefCell<State>,
}

impl EglPartialPresentation {
    pub fn new(
        egl: Rc<EglInterface>,
        display: isize,
        support: EglPartialPresentationSupport,
        corrupted: Rc<Cell<bool>>,
    ) -> Rc<EglPartialPresentation> {
        Rc::new(EglPartialPresentation {
            egl,
            display,
            support,
            corrupted,
            prefer_retained_layer: Cell::new(false),
            state: RefCell::new(State {
                buffers: EglBufferAgeTracker::default(),
                surface: 0,
                size: PixelSize::default(),
                is_y_flipped: false,
                can_set_dirty_rects: false,
                consecutive_zero_age_frames: 0,
                frame_dirty_rects: Vec::new(),
                buffer_dirty_rects: Vec::new(),
            }),
        })
    }

    /// Whether the driver never gave back a buffer with contents: the render target then
    /// keeps its retained layer for the rest of its life (copying the dirty part of the layer
    /// is cheaper than repainting the scene).
    pub fn prefer_retained_layer(&self) -> bool {
        self.prefer_retained_layer.get()
    }

    /// Begins a frame on `surface`, current with the context.
    pub fn begin_draw(
        self: &Rc<Self>,
        surface: isize,
        size: PixelSize,
        is_y_flipped: bool,
    ) -> Result<Rc<EglPartialSession>, OpenGlError> {
        let mut state = self.state.borrow_mut();
        if state.surface != surface || state.size != size || state.is_y_flipped != is_y_flipped {
            Self::reset(&mut state);
            state.surface = surface;
            state.size = size;
            state.is_y_flipped = is_y_flipped;
            if self.support.partial_update {
                let behavior = self
                    .egl
                    .query_surface(self.display, surface, EGL_SWAP_BEHAVIOR)
                    .ok_or_else(|| OpenGlError::get_formatted_egl_exception("eglQuerySurface", self.egl.get_error()))?;
                state.can_set_dirty_rects = behavior == EGL_BUFFER_DESTROYED;
            }
        }
        let age = self.egl.query_surface(self.display, surface, EGL_BUFFER_AGE_EXT).ok_or_else(|| {
            OpenGlError::get_formatted_egl_exception("eglQuerySurface(EGL_BUFFER_AGE_EXT)", self.egl.get_error())
        })?;
        let buffer = state.buffers.acquire(age);
        Ok(Rc::new(EglPartialSession {
            owner: self.clone(),
            buffer,
            age,
            frame_dirty_rect_count: Cell::new(0),
            dirty_rects_set: Cell::new(false),
            native_dirty_rects_declared: Cell::new(false),
            succeeded: Cell::new(false),
            finished: Cell::new(false),
            taken: Cell::new(false),
        }))
    }

    fn reset(state: &mut State) {
        state.buffers.invalidate();
        state.surface = 0;
        state.size = PixelSize::default();
        state.consecutive_zero_age_frames = 0;
    }

    /// Forgets the buffers and the surface.
    pub fn invalidate(&self) {
        Self::reset(&mut self.state.borrow_mut());
    }
}

/// The partial presentation of one frame: the feature the drawing context of the frame
/// answers.
pub struct EglPartialSession {
    owner: Rc<EglPartialPresentation>,
    buffer: Rc<EglBuffer>,
    age: i32,
    frame_dirty_rect_count: Cell<i32>,
    dirty_rects_set: Cell<bool>,
    native_dirty_rects_declared: Cell<bool>,
    succeeded: Cell<bool>,
    finished: Cell<bool>,
    /// Whether a renderer took the feature. A frame whose feature nobody took is presented
    /// whole, as without partial presentation.
    taken: Cell<bool>,
}

impl EglPartialSession {
    /// A renderer takes the feature for the frame.
    pub fn take(&self) {
        self.taken.set(true);
    }

    pub fn is_taken(&self) -> bool {
        self.taken.get()
    }

    /// Whether the frame may be presented: it succeeded and the session is not over.
    pub fn can_present(&self) -> bool {
        self.succeeded.get() && !self.finished.get()
    }

    /// Ends the session without presenting: the contents of the buffer are no longer known.
    pub fn cancel(&self) {
        if self.finished.replace(true) {
            return;
        }
        self.buffer.valid.set(false);
        self.owner.invalidate();
        // Only a region declared to the driver needs the boundary of a frame before the next.
        if self.native_dirty_rects_declared.get() {
            self.owner.corrupted.set(true);
        }
    }

    /// Presents the frame whose feature was not taken: a plain swap; no buffer is known.
    pub fn present_whole(&self, swap: impl FnOnce()) {
        self.owner.invalidate();
        self.finished.set(true);
        swap();
    }

    /// Presents the frame with the region that changed. `Ok(false)` when the window is gone
    /// (`EGL_BAD_SURFACE`, `EGL_BAD_NATIVE_WINDOW`): the render target is made again on the
    /// next frame, and the other windows render this tick.
    ///
    /// # Panics
    /// Panics when the frame did not succeed or the session is over.
    pub fn present(&self) -> Result<bool, OpenGlError> {
        assert!(self.can_present(), "Rendering must succeed before presentation.");
        let owner = &self.owner;
        let egl = &owner.egl;
        let success = {
            let state = owner.state.borrow();
            let count = self.frame_dirty_rect_count.get();
            if self.dirty_rects_set.get() && owner.support.swap_buffers_with_damage_khr {
                egl.swap_buffers_with_damage_khr(owner.display, state.surface, &state.frame_dirty_rects, count)
            } else if self.dirty_rects_set.get() && owner.support.swap_buffers_with_damage_ext {
                egl.swap_buffers_with_damage_ext(owner.display, state.surface, &state.frame_dirty_rects, count)
            } else {
                egl.swap_buffers_checked(owner.display, state.surface)
            }
        };
        if !success {
            let error = egl.get_error();
            let declared = self.native_dirty_rects_declared.get();
            self.cancel();
            if !declared {
                owner.corrupted.set(true);
            }
            if error == EGL_BAD_SURFACE || error == EGL_BAD_NATIVE_WINDOW {
                return Ok(false);
            }
            return Err(OpenGlError::get_formatted_egl_exception("eglSwapBuffers", error));
        }

        let mut state = owner.state.borrow_mut();
        state.buffers.presented(self.buffer.clone());
        state.consecutive_zero_age_frames = if self.age == 0 {
            (state.consecutive_zero_age_frames + 1).min(ZERO_AGE_FRAMES_BEFORE_RETAINED_LAYER)
        } else {
            0
        };
        if state.consecutive_zero_age_frames >= ZERO_AGE_FRAMES_BEFORE_RETAINED_LAYER {
            owner.prefer_retained_layer.set(true);
        }
        self.finished.set(true);
        Ok(true)
    }
}

impl IPartialPresentationFeature for EglPartialSession {
    fn buffer(&self) -> Option<Rc<dyn IPartialPresentationBuffer>> {
        Some(self.buffer.clone())
    }

    /// # Panics
    /// Panics when the session is over or the rectangles were declared already.
    fn set_dirty_rects(&self, frame_dirty_rects: &[PixelRect], buffer_dirty_rects: &[PixelRect]) -> bool {
        assert!(!self.finished.get(), "The partial presentation session is over.");
        assert!(
            !self.dirty_rects_set.replace(true),
            "Dirty rectangles can only be declared once per rendering session."
        );
        let owner = &self.owner;
        let declared = {
            let mut state = owner.state.borrow_mut();
            let State {
                size,
                is_y_flipped,
                can_set_dirty_rects,
                frame_dirty_rects: frame,
                buffer_dirty_rects: buffer,
                surface,
                ..
            } = &mut *state;
            self.frame_dirty_rect_count.set(write_rects(frame_dirty_rects, *size, frame, *is_y_flipped));
            if !*can_set_dirty_rects {
                return true;
            }
            let count = write_rects(buffer_dirty_rects, *size, buffer, *is_y_flipped);
            self.native_dirty_rects_declared.set(true);
            owner.egl.set_damage_region_khr(owner.display, *surface, buffer, count)
        };
        if !declared {
            self.cancel();
        }
        declared
    }

    fn succeeded(&self) {
        assert!(!self.finished.get(), "The partial presentation session is over.");
        self.succeeded.set(true);
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream, which tests the buffer age and the rectangles in
    // `EglPartialPresentationTests`; these follow its cases.
    use super::*;

    #[test]
    fn a_buffer_is_found_by_its_age_until_it_falls_out_of_the_history() {
        let mut tracker = EglBufferAgeTracker::default();
        let first = tracker.acquire(0);
        tracker.presented(first.clone());
        let second = tracker.acquire(0);
        tracker.presented(second.clone());

        assert!(Rc::ptr_eq(&tracker.acquire(1), &second));
        assert!(Rc::ptr_eq(&tracker.acquire(2), &first));
        // Older than what was presented, or no contents: a new buffer.
        assert!(!Rc::ptr_eq(&tracker.acquire(3), &first));
        assert!(!Rc::ptr_eq(&tracker.acquire(0), &first));

        for _ in 0..HISTORY {
            tracker.presented(EglBuffer::new());
        }
        assert!(!first.is_valid());
        assert!(!second.is_valid());
    }

    #[test]
    fn a_buffer_in_several_slots_stays_valid_until_its_last_slot_goes() {
        let mut tracker = EglBufferAgeTracker::default();
        let buffer = EglBuffer::new();
        tracker.presented(buffer.clone());
        tracker.presented(buffer.clone());
        for _ in 0..HISTORY - 1 {
            tracker.presented(EglBuffer::new());
        }
        assert!(buffer.is_valid());
        tracker.presented(EglBuffer::new());
        assert!(!buffer.is_valid());
    }

    #[test]
    fn invalidating_forgets_every_buffer() {
        let mut tracker = EglBufferAgeTracker::default();
        let buffer = EglBuffer::new();
        tracker.presented(buffer.clone());
        tracker.invalidate();
        assert!(!buffer.is_valid());
        assert!(!Rc::ptr_eq(&tracker.acquire(1), &buffer));
    }

    #[test]
    fn rectangles_are_clipped_and_turned_to_the_bottom_left_origin() {
        let size = PixelSize::new(100, 50);
        let mut rects = Vec::new();
        let count =
            write_rects(&[PixelRect::new(10, 5, 20, 10), PixelRect::new(90, 40, 30, 30)], size, &mut rects, false);
        assert_eq!(count, 2);
        assert_eq!(rects, vec![10, 35, 20, 10, 90, 0, 10, 10]);

        let count = write_rects(&[PixelRect::new(10, 5, 20, 10)], size, &mut rects, true);
        assert_eq!(count, 1);
        assert_eq!(rects, vec![10, 5, 20, 10]);
    }

    #[test]
    fn no_rectangle_is_one_empty_rectangle_not_the_whole_surface() {
        let size = PixelSize::new(100, 50);
        let mut rects = vec![7; 12];
        assert_eq!(write_rects(&[], size, &mut rects, false), 1);
        assert_eq!(rects, vec![0, 0, 0, 0]);
        assert_eq!(write_rects(&[PixelRect::new(200, 200, 10, 10)], size, &mut rects, false), 1);
        assert_eq!(rects, vec![0, 0, 0, 0]);
    }
}
