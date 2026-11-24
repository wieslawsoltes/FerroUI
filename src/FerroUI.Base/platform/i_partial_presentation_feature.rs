//! Partial presentation of an output buffer (upstream `IPartialPresentationFeature`,
//! 070c3be990): the render target says which of its buffers a frame draws into, and the
//! compositor repaints only what changed in that buffer since it was last drawn.

use crate::PixelRect;
use std::rc::Rc;

/// The identity of the contents of an output buffer. Two buffers are the same when they are
/// the same object (`Rc::ptr_eq`); the identity stays the same after it is invalidated.
pub trait IPartialPresentationBuffer {
    /// Whether the contents of the buffer can still be reused. Invalidation is permanent.
    fn is_valid(&self) -> bool;
}

/// Partial presentation of one drawing context of an output, which owns the feature: a
/// drawing context of a target whose properties say
/// [`supports_partial_presentation`](super::RenderTargetProperties::supports_partial_presentation)
/// answers `get_feature(TypeId::of::<dyn IPartialPresentationFeature>())` with an
/// `Rc<dyn IPartialPresentationFeature>`.
pub trait IPartialPresentationFeature {
    /// The buffer the frame draws into, or `None` when its contents cannot be tracked. A
    /// buffer the caller does not know, or one that is no longer valid, is repainted whole.
    fn buffer(&self) -> Option<Rc<dyn IPartialPresentationBuffer>>;

    /// Declares the dirty rectangles once, before anything is drawn to the output buffer, in
    /// device pixels with the origin at the top left corner. `frame_dirty_rects` are the
    /// changes since the previous presentation; `buffer_dirty_rects` is the whole region the
    /// frame repaints in this buffer, every pixel of which the caller repaints. The
    /// implementation copies what it needs after the call.
    ///
    /// `false` when the platform refused the region: the frame is not presented, and the
    /// render target reports itself corrupted, so the next frame makes a new one.
    fn set_dirty_rects(&self, frame_dirty_rects: &[PixelRect], buffer_dirty_rects: &[PixelRect]) -> bool;

    /// The drawing is complete: called after the last drawing command, before the drawing
    /// context is disposed. Without it the disposal cancels the presentation and
    /// invalidates the contents of the buffer.
    fn succeeded(&self);
}
