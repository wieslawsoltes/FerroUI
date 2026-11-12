//! The drawings of the compositor kept as pictures (`IDrawingContextImplWithFragments`,
//! docs/porting/desktop-performance.md, 9.48).
//!
//! A drawing that is drawn again as it was is a list of commands the compositor walks in every
//! frame: brushes are turned into paints, geometries looked up, and each call recorded into the
//! picture of the frame. A fragment is the picture of those calls. While it is made the canvas of
//! the context is the canvas of a recorder of its own, which starts with the matrix the canvas of
//! the frame has, so that what the context reads back (the matrix after a restore, the matrix a
//! mask is drawn with) is what it reads without a fragment. The picture is then drawn on the
//! canvas of the frame, which holds it by reference: in the frames after, the drawing is one
//! operation of the frame's picture.
//!
//! What is recorded is everything the commands draw, whatever the clip of the frame was, so a
//! fragment is drawn again under any clip. It is drawn again when the context is in the state it
//! was made in ([`FragmentState`]) and the matrix is the one it was made with, or that matrix
//! moved by whole pixels when nothing in the fragment was laid out in the pixels of the device.
//! Otherwise the context refuses, and the compositor draws the commands.
//!
//! Only a context that records makes fragments: on a surface a picture is played back call by
//! call, which saves the walk of the commands and nothing of the drawing.

use super::{CanvasSource, ContextCanvas, DrawingContextImpl, PendingMatrix};
use crate::gpu::ISkiaGrContext;
use crate::text_rendering_settings::TextRenderingSettings;
use ferroui_base::media::{RenderOptions, TextOptions};
use ferroui_base::platform::{FragmentKeeping, IDrawingContextImplWithFragments, IDrawingFragment};
use ferroui_base::{Matrix, Vector};
use skia_safe as sk;
use std::any::Any;
use std::rc::{Rc, Weak};

/// The half size of the bounds a fragment is recorded with: nothing a drawing draws is outside
/// them, wherever it is moved to. The bounds of the picture are those of what was drawn, which
/// its recorder keeps for every operation.
const CAPTURE_EXTENT: f32 = 1.0e7;

/// The state of a drawing context that what a list of commands draws depends on, apart from the
/// matrix: a fragment is drawn again in the state it was made in.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct FragmentState {
    opacity: f64,
    render_options: RenderOptions,
    text_options: TextOptions,
    text_rendering: TextRenderingSettings,
    disable_subpixel_text_rendering: bool,
    intermediate_surface_dpi: Vector,
    post_transform: Option<Matrix>,
    use_opacity_save_layer: bool,
    /// Whether a rounded rectangle is in the clip of the canvas: the next one is then drawn in
    /// another way (`clip_rounded_rect`).
    rounded_clip: bool,
}

/// A fragment that is being made.
pub(super) struct FragmentCapture {
    recorder: sk::PictureRecorder,
    /// The matrix of the canvas of the context when the fragment was started.
    base: sk::M44,
    /// How many saves the canvas of the context has: the recorder starts without one.
    save_offset: usize,
    /// Whether the fragment can be drawn at another place (nothing in it was laid out in the
    /// pixels of the device).
    movable: bool,
    /// Whether the fragment is kept (what was drawn is drawn the same in a later frame).
    kept: bool,
    /// Whether the fragment holds a draw only the recorder of the context can draw
    /// (`ISkiaGrContext::note_direct_only_draw`).
    direct_only: bool,
    state: FragmentState,
}

impl FragmentCapture {
    /// The canvas of the recorder. What the leases of the context that ended drew is drawn into
    /// it first, as into the canvas of the frame.
    #[inline]
    pub(super) fn canvas(&mut self, source: &CanvasSource) -> &sk::Canvas {
        let canvas = self.recorder.recording_canvas().unwrap_or_else(|| panic!("The fragment has ended"));
        if let CanvasSource::Recorder { leases: Some(leases), .. } = source {
            leases.draw_pending(canvas);
        }
        canvas
    }
}

/// A drawing kept as a picture.
struct SkiaFragment {
    picture: sk::Picture,
    base: sk::M44,
    movable: bool,
    direct_only: bool,
    state: FragmentState,
    /// The GPU context of the drawing context that made the fragment: the images in the picture
    /// are its own.
    context: Option<Weak<dyn ISkiaGrContext>>,
}

impl IDrawingFragment for SkiaFragment {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The matrix a picture recorded with the matrix `base` is drawn with where the canvas has
/// `device`: none, or a move by whole pixels of a picture that can be moved. `None` when the
/// picture is not what the commands draw there.
fn placement(base: &sk::M44, device: &sk::M44, movable: bool) -> Option<sk::M44> {
    if device == base {
        return Some(sk::M44::new_identity());
    }
    if !movable {
        return None;
    }
    let (mut from, mut to) = ([0.0_f32; 16], [0.0_f32; 16]);
    base.get_col_major(&mut from);
    device.get_col_major(&mut to);
    // Column major: the translation is the fourth column (12 and 13 for x and y). The matrices
    // differ in nothing else, and have no perspective, which a move after it would not commute
    // with.
    let same = (0..16).all(|index| index == 12 || index == 13 || from[index] == to[index]);
    if !same || from[3] != 0.0 || from[7] != 0.0 || from[11] != 0.0 || from[15] != 1.0 {
        return None;
    }
    let (dx, dy) = (to[12] - from[12], to[13] - from[13]);
    // Whole pixels: what is drawn falls on the pixels as it did. The move is the exact one
    // (the sum is the translation of the device again).
    let whole = dx.is_finite() && dy.is_finite() && dx.fract() == 0.0 && dy.fract() == 0.0;
    if !whole || from[12] + dx != to[12] || from[13] + dy != to[13] {
        return None;
    }
    Some(sk::M44::translate(dx, dy, 0.0))
}

impl ContextCanvas {
    /// Whether the context makes fragments: it records (see the module), and nothing draws to
    /// its canvas behind its back.
    fn makes_fragments(&self) -> bool {
        !self.eager && matches!(self.source, CanvasSource::Recorder { .. })
    }

    /// How many saves the canvas of the context has that the canvas which is drawn to has not.
    #[inline]
    pub(super) fn save_offset(&self) -> usize {
        self.capture.as_ref().map_or(0, |capture| capture.save_offset)
    }

    /// The fragment that is being made holds something laid out in the pixels of the device.
    #[inline]
    pub(super) fn note_not_movable(&mut self) {
        if let Some(capture) = &mut self.capture {
            capture.movable = false;
        }
    }

    /// The fragment that is being made holds something that is not drawn the same again.
    #[inline]
    pub(super) fn note_not_kept(&mut self) {
        if let Some(capture) = &mut self.capture {
            capture.kept = false;
        }
    }

    /// The fragment that is being made holds a draw only the recorder of the context can draw.
    #[inline]
    pub(super) fn note_direct_only(&mut self) {
        if let Some(capture) = &mut self.capture {
            capture.direct_only = true;
        }
    }

    /// The matrix the canvas of the source has for the context: the one it takes with its next
    /// use, else the one it has.
    fn source_matrix(&mut self) -> sk::M44 {
        match &self.pending {
            PendingMatrix::None => self.source.get().local_to_device(),
            PendingMatrix::Transform(transform) => self.device_matrix(*transform),
            PendingMatrix::Device(matrix) => matrix.clone(),
        }
    }

    /// The bounds of the clip in the coordinates of the context.
    pub(super) fn local_clip_bounds(&mut self) -> Option<sk::Rect> {
        let mut bounds = self.get().local_clip_bounds()?;
        if self.capture.is_some() {
            // The clip of the frame is on the canvas of the frame, which has another matrix.
            let matrix = self.target().local_to_device();
            let device = self.source.canvas_only().device_clip_bounds()?;
            let outer = matrix.invert()?.to_m33().map_rect(sk::Rect::from(device).with_outset((1.0, 1.0))).0;
            if !bounds.intersect(outer) {
                return None;
            }
        }
        Some(bounds)
    }

    /// Starts a fragment: from here on the canvas of the context is the one of a recorder of
    /// its own, with the matrix the context has for its canvas.
    fn begin_capture(&mut self, state: FragmentState) {
        if self.capture.is_some() || !self.makes_fragments() {
            return;
        }
        // (What the leases that ended drew goes into the frame here, before the fragment.)
        let base = self.source_matrix();
        let save_offset = self.source.canvas_only().save_count() - 1;
        let bounds = sk::Rect::from_ltrb(-CAPTURE_EXTENT, -CAPTURE_EXTENT, CAPTURE_EXTENT, CAPTURE_EXTENT);
        let mut recorder = sk::PictureRecorder::new();
        // With the bounds of every operation: the bounds of the picture are then those of what
        // was drawn, which the frame is drawn by (a picture outside a clip is not played).
        recorder.begin_recording(bounds, true).set_matrix(&base);
        self.pending = PendingMatrix::None;
        self.capture = Some(Box::new(FragmentCapture {
            recorder,
            base,
            save_offset,
            movable: true,
            kept: true,
            direct_only: false,
            state,
        }));
    }

    /// Ends the fragment that is being made: its picture is drawn on the canvas of the source,
    /// which is the canvas of the context again. Returns the picture with what is known of it.
    fn end_capture(&mut self) -> Option<(sk::Picture, Box<FragmentCapture>)> {
        self.capture.as_ref()?;
        let (matrix, open_saves) = {
            let canvas = self.target();
            (canvas.local_to_device(), canvas.save_count() - 1)
        };
        let mut capture = self.capture.take()?;
        let picture = capture.recorder.finish_recording_as_picture(None);
        // The canvas of the source takes the matrix the context had for the recorder, with its
        // next use: the picture is drawn in the coordinates of the device.
        if matches!(self.pending, PendingMatrix::None) {
            self.pending = PendingMatrix::Device(matrix.clone());
        }
        let canvas = self.source.canvas_only();
        if let Some(picture) = &picture {
            if canvas.local_to_device() != sk::M44::new_identity() {
                canvas.reset_matrix();
            }
            canvas.draw_picture(picture, None, None);
        }
        // Saves the drawing left open (the contract has none) are saves of the context still: its
        // pops find them, with the matrix the drawing ended with.
        for save in 0..open_saves {
            if save == 0 {
                canvas.set_matrix(&matrix);
            }
            canvas.save();
            capture.kept = false;
        }
        if self.eager {
            self.apply_pending();
        }
        picture.map(|picture| (picture, capture))
    }

    /// Ends the fragment that is being made, if any, keeping nothing of it.
    pub(super) fn abandon_capture(&mut self) {
        let _ = self.end_capture();
    }

    /// Draws a picture that was recorded with the matrix `base`, when it is what its commands
    /// draw with the matrix the context has now. Returns whether it was drawn.
    fn draw_kept(&mut self, picture: &sk::Picture, base: &sk::M44, movable: bool) -> bool {
        if self.capture.is_some() {
            return false;
        }
        let device = self.source_matrix();
        let Some(placement) = placement(base, &device, movable) else { return false };
        let canvas = self.source.get();
        if canvas.local_to_device() != placement {
            canvas.set_matrix(&placement);
        }
        canvas.draw_picture(picture, None, None);
        // The context has the matrix it had, which its canvas takes with its next use.
        if matches!(self.pending, PendingMatrix::None) {
            self.pending = PendingMatrix::Device(device);
        }
        if self.eager {
            self.apply_pending();
        }
        true
    }
}

impl DrawingContextImpl {
    fn fragment_state(&self) -> FragmentState {
        FragmentState {
            opacity: self.current_opacity,
            render_options: self.render_options,
            text_options: self.text_options,
            text_rendering: self.text_rendering,
            disable_subpixel_text_rendering: self.disable_subpixel_text_rendering,
            intermediate_surface_dpi: self.intermediate_surface_dpi,
            post_transform: self.post_transform,
            use_opacity_save_layer: self.use_opacity_save_layer,
            rounded_clip: !self.rounded_clip_elements.is_empty(),
        }
    }

    /// Whether `context` is the GPU context this drawing context draws with.
    fn is_context(&self, context: &Option<Weak<dyn ISkiaGrContext>>) -> bool {
        match (&self.gr_context, context) {
            (Some(own), Some(context)) => std::ptr::addr_eq(Rc::as_ptr(own), context.as_ptr()),
            (None, None) => true,
            _ => false,
        }
    }
}

impl IDrawingContextImplWithFragments for DrawingContextImpl {
    fn fragment_keeping(&mut self) -> FragmentKeeping {
        // A drawing that changes with every frame is drawn once more and never played: only
        // one that stays is worth a picture.
        if self.canvas.makes_fragments() {
            FragmentKeeping::Settled
        } else {
            FragmentKeeping::None
        }
    }

    fn begin_fragment(&mut self) {
        self.check_lease();
        let state = self.fragment_state();
        self.canvas.begin_capture(state);
    }

    fn end_fragment(&mut self) -> Option<Rc<dyn IDrawingFragment>> {
        let (picture, capture) = self.canvas.end_capture()?;
        // A drawing that leaves the context in another state is not one unit to draw again.
        if !capture.kept || capture.state != self.fragment_state() {
            return None;
        }
        Some(Rc::new(SkiaFragment {
            picture,
            base: capture.base,
            movable: capture.movable,
            direct_only: capture.direct_only,
            state: capture.state,
            context: self.gr_context.as_ref().map(Rc::downgrade),
        }))
    }

    fn draw_fragment(&mut self, fragment: &Rc<dyn IDrawingFragment>) -> bool {
        self.check_lease();
        let Some(fragment) = fragment.as_any().downcast_ref::<SkiaFragment>() else { return false };
        if fragment.state != self.fragment_state() || !self.is_context(&fragment.context) {
            return false;
        }
        if !self.canvas.draw_kept(&fragment.picture, &fragment.base, fragment.movable) {
            return false;
        }
        if fragment.direct_only {
            if let Some(gr_context) = &self.gr_context {
                gr_context.note_direct_only_draw();
            }
        }
        true
    }
}

#[cfg(test)]
#[path = "fragment_tests.rs"]
mod tests;
