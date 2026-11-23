//! Fragments of the drawing context (docs/porting/progpu-backend.md, section 2.3.6, step R2).
//!
//! A fragment keeps what the context has already worked out for a visual: brushes, primitives,
//! widened and flattened outlines, the glyphs of a run at their raster placement. Drawing it
//! again goes through the builder calls of the first time, without the commands of the visual
//! and without preparing their geometry or text again.
//!
//! The clips in force when a fragment starts are not part of it, so a fragment drawn under other
//! clips of its ancestors takes those. A fragment is drawn again when the linear part of its
//! transform is the same and the translation differs by whole device pixels. Anything else (a
//! fractional move, a rotation, a scale) is refused, and the compositor draws the visual again.
//!
//! What cannot be kept makes `end_fragment` return `None`: images (a bitmap is borrowed for the
//! call, a layer is a texture of a frame), clears, and clips that are masks, because their
//! records hold the transform they were made under.

use crate::font::FontFace;
use crate::glyph_run::RunRaster;
use ferroui_base::platform::IDrawingFragment;
use ferroui_base::{Matrix, Point, Rect};
use progpu_native_sys::progpu::*;
use std::any::Any;
use std::sync::Arc;

/// How far a translation may be from whole device pixels and still count as whole.
const WHOLE_PIXEL_TOLERANCE: f64 = 1e-6;

/// A brush of a kept draw.
#[derive(Clone)]
pub(crate) enum FragmentBrush {
    Solid { color: progpu_native_color, opacity: f32 },
    Gradient { brush: Box<progpu_native_scene_brush>, stops: Box<[progpu_native_scene_gradient_stop]> },
}

/// A kept glyph run: the raster the run made for its placement, shared with the run.
pub(crate) struct KeptRun {
    pub(crate) face: Arc<FontFace>,
    pub(crate) raster: Arc<RunRaster>,
    pub(crate) color: progpu_native_color,
    pub(crate) aliased: bool,
    /// Glyphs at whole device pixels under the identity transform, rather than through the
    /// transform.
    pub(crate) upright: bool,
    /// The whole device pixel the glyphs of an upright run are placed from; the origin of a
    /// run drawn through the transform.
    pub(crate) base: Point,
    /// The origin of an upright run in device pixels, before it was floored to `base`.
    pub(crate) device: Point,
    pub(crate) atlas_to_logical_scale: f32,
    /// The bounds of the draw in device pixels.
    pub(crate) bounds: Rect,
    pub(crate) transform: Matrix,
}

/// A step of a fragment, in the order it was drawn.
pub(crate) enum FragmentOp {
    Shape {
        primitive: progpu_native_analytic_primitive,
        brush: FragmentBrush,
        /// In device pixels.
        bounds: Rect,
        transform: Matrix,
    },
    Path {
        path: progpu_native_scene_path_fill,
        segments: Box<[progpu_native_path_segment]>,
        brush: FragmentBrush,
        /// In the coordinates of the path.
        bounds: Rect,
        transform: Matrix,
    },
    Run(Box<KeptRun>),
    /// A clip rectangle in device pixels.
    PushClip(Rect),
    PopClip,
    /// A layer with its bounds in device pixels.
    PushLayer { bounds: Option<Rect>, opacity: f64, blend_mode: u32 },
    PopLayer,
}

fn is_affine(matrix: &Matrix) -> bool {
    matrix.m13 == 0.0 && matrix.m23 == 0.0 && matrix.m33 == 1.0
}

/// What is being recorded between `begin_fragment` and `end_fragment`.
pub(crate) struct FragmentRecorder {
    transform: Matrix,
    aliased: bool,
    ops: Vec<FragmentOp>,
    /// The brush added for the draw that comes next.
    brush: Option<FragmentBrush>,
    /// The clips and layers open inside the fragment.
    depth: u32,
    keepable: bool,
}

impl FragmentRecorder {
    pub(crate) fn new(transform: Matrix, aliased: bool) -> Self {
        Self { transform, aliased, ops: Vec::new(), brush: None, depth: 0, keepable: is_affine(&transform) }
    }

    /// What was drawn cannot be drawn again from a fragment.
    pub(crate) fn refuse(&mut self) {
        self.keepable = false;
        self.ops = Vec::new();
    }

    pub(crate) fn set_brush(&mut self, brush: FragmentBrush) {
        if self.keepable {
            self.brush = Some(brush);
        }
    }

    fn push(&mut self, op: FragmentOp) {
        if self.keepable {
            self.ops.push(op);
        }
    }

    /// The brush of the draw being kept; a draw without one is not kept.
    fn draw_brush(&mut self, transform: &Matrix) -> Option<FragmentBrush> {
        let brush = self.brush.take();
        if brush.is_none() || !is_affine(transform) {
            self.refuse();
            return None;
        }
        brush
    }

    pub(crate) fn shape(&mut self, primitive: progpu_native_analytic_primitive, bounds: Rect, transform: Matrix) {
        if let Some(brush) = self.draw_brush(&transform) {
            self.push(FragmentOp::Shape { primitive, brush, bounds, transform });
        }
    }

    pub(crate) fn path(
        &mut self,
        path: progpu_native_scene_path_fill,
        segments: Vec<progpu_native_path_segment>,
        bounds: Rect,
        transform: Matrix,
    ) {
        if let Some(brush) = self.draw_brush(&transform) {
            self.push(FragmentOp::Path { path, segments: segments.into_boxed_slice(), brush, bounds, transform });
        }
    }

    pub(crate) fn run(&mut self, run: KeptRun) {
        if is_affine(&run.transform) {
            self.push(FragmentOp::Run(Box::new(run)));
        } else {
            self.refuse();
        }
    }

    pub(crate) fn push_clip(&mut self, rect: Rect) {
        self.depth += 1;
        self.push(FragmentOp::PushClip(rect));
    }

    pub(crate) fn pop_clip(&mut self) {
        if self.close() {
            self.push(FragmentOp::PopClip);
        }
    }

    pub(crate) fn push_layer(&mut self, bounds: Option<Rect>, opacity: f64, blend_mode: u32) {
        self.depth += 1;
        self.push(FragmentOp::PushLayer { bounds, opacity, blend_mode });
    }

    pub(crate) fn pop_layer(&mut self) {
        if self.close() {
            self.push(FragmentOp::PopLayer);
        }
    }

    /// Closes a clip or a layer opened inside the fragment. One opened before it is not its own.
    fn close(&mut self) -> bool {
        if self.depth == 0 {
            self.refuse();
            return false;
        }
        self.depth -= 1;
        true
    }

    /// The fragment, or `None` when what was drawn cannot be kept.
    pub(crate) fn finish(self) -> Option<ProGpuFragment> {
        (self.keepable && self.depth == 0).then_some(ProGpuFragment {
            transform: self.transform,
            aliased: self.aliased,
            ops: self.ops,
        })
    }
}

/// What the context kept of a part of a frame.
pub struct ProGpuFragment {
    transform: Matrix,
    aliased: bool,
    pub(crate) ops: Vec<FragmentOp>,
}

impl IDrawingFragment for ProGpuFragment {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ProGpuFragment {
    /// The move in whole device pixels from where the fragment was recorded to `transform`, or
    /// `None` when drawing it there would not give the pixels its commands would give.
    pub(crate) fn offset_to(&self, transform: &Matrix, aliased: bool) -> Option<(f64, f64)> {
        let kept = &self.transform;
        if aliased != self.aliased
            || !is_affine(transform)
            || kept.m11 != transform.m11
            || kept.m12 != transform.m12
            || kept.m21 != transform.m21
            || kept.m22 != transform.m22
        {
            return None;
        }
        let (dx, dy) = (transform.m31 - kept.m31, transform.m32 - kept.m32);
        let (x, y) = (dx.round(), dy.round());
        if !(x.is_finite() && y.is_finite())
            || (dx - x).abs() > WHOLE_PIXEL_TOLERANCE
            || (dy - y).abs() > WHOLE_PIXEL_TOLERANCE
        {
            return None;
        }
        // An upright run is placed from the pixel its origin falls in: a rounding of the move
        // must not take the origin over the edge of that pixel.
        let stays = self.ops.iter().all(|op| match op {
            FragmentOp::Run(run) if run.upright => {
                (run.device.x + dx).floor() == run.base.x + x && (run.device.y + dy).floor() == run.base.y + y
            }
            _ => true,
        });
        stays.then_some((x, y))
    }
}

/// `transform` moved by `(x, y)` device pixels.
pub(crate) fn moved(transform: &Matrix, (x, y): (f64, f64)) -> Matrix {
    if x == 0.0 && y == 0.0 {
        *transform
    } else {
        *transform * Matrix::create_translation(x, y)
    }
}

/// `rect` moved by `(x, y)` device pixels.
pub(crate) fn moved_rect(rect: Rect, (x, y): (f64, f64)) -> Rect {
    Rect::new(rect.x + x, rect.y + y, rect.width, rect.height)
}
