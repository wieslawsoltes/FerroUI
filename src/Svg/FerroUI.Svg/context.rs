//! Audited access to the Skia recording context.
//!
//! SVG is not an alternative pixel compositing engine. Operations outside the
//! verified vector subset are still recorded by Skia, and tell the exporter to
//! replay the complete composed recording into a raster image. This preserves
//! destination-dependent blends and avoids silently dropping drawing commands.

use ferroui_base::media::effects::IEffect;
use ferroui_base::media::imaging::{BitmapBlendingMode, BitmapInterpolationMode};
use ferroui_base::media::{
    BoxShadows, Color, ColorMatrix, ColorMatrixAlphaMode, ConvolveMatrix, EdgeMode,
    GradientSpreadMethod, IBrush, IExperimentalAcrylicMaterial, IPen, ITileBrush, RenderOptions,
    TextOptions,
};
use ferroui_base::platform::{
    IBitmapImpl, IDrawingContextImpl, IDrawingContextImplWithEffects,
    IDrawingContextImplWithFragments, IDrawingContextImplWithLayerEffects,
    IDrawingContextImplWithScene3D, IDrawingContextLayerImpl,
    IDrawingContextWithAcrylicLikeSupport, IDrawingFragment, IGeometryImpl, IGlyphRunImpl,
    IPlatformRenderInterfaceRegion, LightingParameters, MultiplyLayerFactors, PixelOperation,
};
use ferroui_base::rendering::scene3d::Scene3DFrame;
use ferroui_base::{Matrix, PixelSize, Point, Rect, RoundedRect, Vector};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

/// Deterministic diagnostics collected while recording one export layer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordingAudit {
    /// Number of primitive drawing calls recorded, including clears.
    pub draw_calls: u64,
    /// Reasons this recording needs composed raster fallback. Each occurs once,
    /// in the order it was first encountered.
    pub raster_reasons: Vec<String>,
    /// Invalid input or unavailable backend capabilities that prevent a faithful
    /// export, even when raster fallback is allowed.
    pub errors: Vec<String>,
    /// Direct bitmap draws. This is an operation count, not a unique-image count;
    /// images hidden inside custom native drawing cannot be counted here.
    pub embedded_images: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Clip,
    GeometryClip,
    Layer,
    Compositing,
    Opacity,
    Mask,
    RenderOptions,
    TextOptions,
    Effect,
    Blend,
    ColorMatrix,
    Multiply,
    Convolution,
    PixelOperation,
}

/// A complete FerroUI drawing-context adapter that records through Skia and
/// accounts for SVG fidelity before serialization.
///
/// Supply a context created by
/// [`ferroui_skia::PictureRenderTarget::create_export_drawing_context`]. Resources
/// (geometries, glyph runs, bitmaps and regions) must belong to the Skia backend.
pub struct SvgDrawingContext {
    inner: Box<dyn IDrawingContextImpl>,
    audit: Rc<RefCell<RecordingAudit>>,
    states: Vec<(State, bool)>,
    fragment_open: bool,
    disposed: bool,
    recording_bounds: Option<Rect>,
    intermediate_dpi: Option<Vector>,
    on_finished: Option<Box<dyn FnOnce(RecordingAudit, bool)>>,
}

impl SvgDrawingContext {
    /// Wraps a fresh, empty recording context without changing its rendering
    /// configuration. All drawing and state changes must pass through this
    /// adapter: commands recorded before wrapping cannot be audited retroactively.
    pub fn new(mut inner: Box<dyn IDrawingContextImpl>) -> Self {
        let intermediate_dpi = inner
            .as_any_mut()
            .downcast_mut::<ferroui_skia::DrawingContextImpl>()
            .map(|context| context.intermediate_surface_dpi());
        Self {
            inner,
            audit: Rc::new(RefCell::new(RecordingAudit::default())),
            states: Vec::new(),
            fragment_open: false,
            disposed: false,
            recording_bounds: None,
            intermediate_dpi,
            on_finished: None,
        }
    }

    pub(crate) fn with_recording_bounds(mut self, bounds: Rect) -> Self {
        self.recording_bounds = Some(bounds);
        self
    }

    /// A snapshot of the diagnostics collected so far.
    pub fn audit(&self) -> RecordingAudit {
        self.audit.borrow().clone()
    }

    /// Runs `callback` once after the inner recording has been finalized, on
    /// explicit disposal or drop. The callback receives the completed audit and
    /// may retrieve the picture from the associated Skia recording target. It is
    /// not invoked if the native context fails to finalize its recording.
    pub fn on_finished(mut self, callback: impl FnOnce(RecordingAudit) + 'static) -> Self {
        self.on_finished = Some(Box::new(move |audit, finalized| {
            if finalized {
                callback(audit);
            }
        }));
        self
    }

    /// Target completion also observes a failed native finalization during
    /// unwind. A false flag means that no finished picture may be retrieved.
    pub(crate) fn on_completed(
        mut self,
        callback: impl FnOnce(RecordingAudit, bool) + 'static,
    ) -> Self {
        self.on_finished = Some(Box::new(callback));
        self
    }

    /// Finishes the recording. Calling this again is harmless. Inspect the audit
    /// afterwards: an unbalanced drawing stack is an export error.
    pub fn finish(&mut self) {
        self.dispose();
    }

    fn reason(&self, message: &str) {
        let mut audit = self.audit.borrow_mut();
        if !audit.raster_reasons.iter().any(|item| item == message) {
            audit.raster_reasons.push(message.into());
        }
    }

    fn error(&self, message: &str) {
        let mut audit = self.audit.borrow_mut();
        if !audit.errors.iter().any(|item| item == message) {
            audit.errors.push(message.into());
        }
    }

    fn finalize(&mut self, unwinding: bool) {
        if self.disposed {
            return;
        }
        // Finalization can invoke native code and client completion callbacks.
        // Do not retry it from Drop if either unwinds. Keep the callback local
        // so captured cleanup guards run even if native finalization fails.
        self.disposed = true;
        let on_finished = self.on_finished.take();
        if !self.states.is_empty() || self.fragment_open {
            self.error("Export recording finished with unbalanced drawing state");
        }
        if unwinding {
            self.error("Export recording was aborted while unwinding a panic");
            // A producer can panic with a native lease still active. Native
            // finalization then panics too; allowing that second panic to leave
            // Drop would abort the process and prevent failure publication.
            let finalized = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.inner.dispose();
            }))
            .is_ok();
            if !finalized {
                self.error("Native export recording could not be finalized during unwind");
            }
            if let Some(callback) = on_finished {
                let audit = self.audit();
                // Preserve the producer's original panic even if a user
                // completion callback also panics while reporting the failure.
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    callback(audit, finalized);
                }));
            }
        } else {
            self.inner.dispose();
            if let Some(callback) = on_finished {
                callback(self.audit(), true);
            }
        }
    }

    fn active(&self) -> bool {
        if self.disposed {
            self.error("Drawing after the export context was finished");
        }
        !self.disposed
    }

    fn finite(&self, values: &[f64], operation: &str) -> bool {
        // Skia consumes f32 coordinates: a finite f64 that overflows during
        // conversion is not a valid coordinate for the recording either.
        let valid = values
            .iter()
            .all(|value| value.is_finite() && (*value as f32).is_finite());
        if !valid {
            self.error(&format!("Non-finite or out-of-range {operation}"));
        }
        valid
    }

    fn rect(&self, rect: Rect) -> bool {
        self.finite(
            &[
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                rect.x + rect.width,
                rect.y + rect.height,
            ],
            "rectangle",
        )
    }

    fn rounded_rect(&self, rect: RoundedRect) -> bool {
        self.rect(rect.rect)
            && self.finite(
                &[
                    rect.radii_top_left.x,
                    rect.radii_top_left.y,
                    rect.radii_top_right.x,
                    rect.radii_top_right.y,
                    rect.radii_bottom_left.x,
                    rect.radii_bottom_left.y,
                    rect.radii_bottom_right.x,
                    rect.radii_bottom_right.y,
                ],
                "corner radii",
            )
    }

    fn matrix(&self, value: Matrix) -> bool {
        self.finite(
            &[
                value.m11, value.m12, value.m13, value.m21, value.m22, value.m23, value.m31,
                value.m32, value.m33,
            ],
            "transform",
        )
    }

    fn count_draw(&self) {
        self.audit.borrow_mut().draw_calls += 1;
    }

    fn audit_primitive_render_options(&mut self, bitmap: bool) {
        // Read the actual native state so initial configuration, inheritance
        // and independent render-option restores use Skia's exact semantics.
        // State changes alone do not rasterize an otherwise vector recording.
        let options = self
            .inner
            .as_any_mut()
            .downcast_mut::<ferroui_skia::DrawingContextImpl>()
            .map(|context| context.render_options());
        if let Some(options) = options {
            if options.edge_mode == EdgeMode::Aliased {
                self.reason("Aliased primitive edges require composed raster output");
            }
            if bitmap
                && !matches!(
                    options.bitmap_interpolation_mode,
                    BitmapInterpolationMode::Unspecified | BitmapInterpolationMode::LowQuality
                )
            {
                // Skia's SVG writer ignores its image SamplingOptions. SVG
                // image-rendering hints cannot preserve native nearest,
                // mipmap or cubic sampling consistently across consumers.
                self.reason("Bitmap interpolation mode requires composed raster output");
            }
        }
    }

    fn push_state(&mut self, state: State, forwarded: bool) {
        self.states.push((state, forwarded));
    }

    fn pop_state(&mut self, state: State) -> bool {
        if !self.active() {
            return false;
        }
        let independent = |state| matches!(state, State::RenderOptions | State::TextOptions);
        // Render/text options have independent stacks in the platform
        // contract. They can be popped across a canvas clip (or each other)
        // without affecting the saved transform and canvas nesting.
        let index = if independent(state) {
            self.states.iter().rposition(|entry| entry.0 == state)
        } else {
            self.states.iter().rposition(|entry| !independent(entry.0))
        };
        if index.is_none_or(|index| self.states[index].0 != state) {
            self.error(&format!("Unbalanced {state:?} pop in export recording"));
            return false;
        }
        self.states.remove(index.unwrap()).1
    }

    fn brush_with_bounds(&self, brush: Option<&dyn IBrush>, bounds: Option<Rect>) -> bool {
        let Some(brush) = brush else { return true };
        if !self.finite(&[brush.opacity()], "brush opacity") {
            return false;
        }
        for transform in [brush.transform(), brush.relative_transform()]
            .into_iter()
            .flatten()
        {
            let matrix = transform.value();
            if !self.matrix(matrix) {
                return false;
            }
            if matrix.contains_perspective() {
                self.reason("Perspective brush transform");
            }
        }
        let origin = brush.transform_origin().point;
        if !self.finite(&[origin.x, origin.y], "brush transform origin") {
            return false;
        }
        if brush.as_solid_color_brush().is_some() {
            return true;
        }
        if let Some(gradient) = brush.as_gradient_brush() {
            if gradient.spread_method() != GradientSpreadMethod::Pad {
                self.reason("Gradient spread method is not preserved by the Skia SVG writer");
            }
            for stop in gradient.gradient_stops() {
                if !self.finite(&[stop.offset()], "gradient stop") {
                    return false;
                }
            }
            if let Some(linear) = brush.as_linear_gradient_brush() {
                let (start, end) = (linear.start_point().point, linear.end_point().point);
                if !self.finite(&[start.x, start.y, end.x, end.y], "linear gradient") {
                    return false;
                }
                if let Some(bounds) = bounds {
                    let (start, end) = (
                        linear.start_point().to_pixels_rect(bounds),
                        linear.end_point().to_pixels_rect(bounds),
                    );
                    return self.finite(
                        &[start.x, start.y, end.x, end.y],
                        "resolved linear gradient",
                    );
                }
                return true;
            }
            if let Some(radial) = brush.as_radial_gradient_brush() {
                let (center, origin) = (radial.center(), radial.gradient_origin());
                if center != origin {
                    self.reason("Focal radial gradient uses a two-point conical shader");
                }
                let (radius_x, radius_y) = (radial.radius_x(), radial.radius_y());
                if let Some(bounds) = bounds {
                    let (center, origin) =
                        (center.to_pixels_rect(bounds), origin.to_pixels_rect(bounds));
                    if !self.finite(
                        &[
                            center.x,
                            center.y,
                            origin.x,
                            origin.y,
                            radius_x.to_value(bounds.width),
                            radius_y.to_value(bounds.height),
                        ],
                        "resolved radial gradient",
                    ) {
                        return false;
                    }
                }
                return self.finite(
                    &[
                        center.point.x,
                        center.point.y,
                        origin.point.x,
                        origin.point.y,
                        radial.radius_x().scalar,
                        radial.radius_y().scalar,
                    ],
                    "radial gradient",
                );
            }
            if let Some(conic) = brush.as_conic_gradient_brush() {
                self.reason("Conic gradient shader");
                let center = conic.center().point;
                if !self.finite(&[center.x, center.y, conic.angle()], "conic gradient") {
                    return false;
                }
                if let Some(bounds) = bounds {
                    let center = conic.center().to_pixels_rect(bounds);
                    return self.finite(&[center.x, center.y], "resolved conic gradient");
                }
                return true;
            }
            self.error("Unknown gradient brush is not implemented by Skia");
            return false;
        }
        if let Some(scene) = brush.as_scene_brush() {
            self.reason("Scene brush can contain unclassified drawing operations");
            return self.tile_brush(scene, bounds);
        }
        if let Some(image) = brush.as_image_brush() {
            self.reason("Image brush shader");
            if !self.tile_brush(image, bounds) {
                return false;
            }
            if let Some(bitmap) = image.source().and_then(|source| source.get_bitmap()) {
                if !self.bitmap(&*bitmap) {
                    return false;
                }
                if let Some(dpi) = self.intermediate_dpi {
                    // Skia resolves image source coordinates with the context's
                    // intermediate DPI, not the bitmap's own metadata DPI.
                    let source = bitmap.pixel_size().to_size_with_dpi_vector(dpi);
                    if !self.rect(image.source_rect().to_pixels(source)) {
                        return false;
                    }
                }
            }
            return true;
        }
        self.error("Unknown brush would be omitted by the Skia backend");
        false
    }

    fn tile_brush(&self, brush: &dyn ITileBrush, bounds: Option<Rect>) -> bool {
        // Empty/degenerate viewports retain native no-op behavior. Reject only
        // coordinates that cannot reach Skia's intermediate allocation/mapping
        // path as finite f32 values.
        self.rect(brush.source_rect().rect)
            && self.rect(brush.destination_rect().rect)
            && bounds
                .is_none_or(|bounds| self.rect(brush.destination_rect().to_pixels_rect(bounds)))
    }

    fn pen(&self, pen: Option<&dyn IPen>, bounds: Rect) -> bool {
        let Some(pen) = pen else { return true };
        if !self.finite(&[pen.thickness(), pen.miter_limit()], "pen") {
            return false;
        }
        if let Some(dash) = pen.dash_style() {
            if !self.finite(&[dash.offset()], "dash offset") {
                return false;
            }
            if let Some(dashes) = dash.dashes() {
                if !self.finite(&dashes, "dash pattern") {
                    return false;
                }
                // Skia applies path effects before its SVG primitive output.
                // Keep a conservative fallback until this conversion has been
                // verified for zero-length dashes and non-butt caps as well.
                if !dashes.is_empty() {
                    self.reason("Dashed stroke path effect");
                }
            }
        }
        self.rect(bounds) && self.brush_with_bounds(pen.brush().as_deref(), Some(bounds))
    }

    fn geometry(&self, geometry: &dyn IGeometryImpl) -> bool {
        if !self.rect(geometry.bounds()) {
            return false;
        }
        let Some(geometry) = ferroui_skia::try_get_geometry_impl(geometry) else {
            self.error("Geometry does not belong to the Skia backend");
            return false;
        };
        for path in [geometry.fill_path(), geometry.stroke_path()]
            .into_iter()
            .flatten()
        {
            if !path.is_finite() {
                self.error("Non-finite geometry path");
                return false;
            }
            if path.is_inverse_fill_type() {
                self.reason("Inverse path fill");
            }
        }
        true
    }

    fn region(&self, region: &dyn IPlatformRenderInterfaceRegion) -> bool {
        if region.as_any().is::<ferroui_skia::SkiaRegionImpl>() {
            true
        } else {
            self.error("Region does not belong to the Skia backend");
            false
        }
    }

    fn bitmap(&self, bitmap: &dyn IBitmapImpl) -> bool {
        match ferroui_skia::is_bitmap_cpu_readable(bitmap) {
            Some(true) => true,
            Some(false) => {
                self.error("Bitmap is not CPU-readable; provide an explicit CPU snapshot of GPU frames and textures before SVG export");
                false
            }
            None => {
                self.error("Bitmap does not belong to the Skia backend");
                false
            }
        }
    }

    fn effect(&self, effect: &dyn IEffect) -> bool {
        if let Some(blur) = effect.as_blur_effect() {
            return self.finite(&[blur.radius()], "blur radius");
        }
        if let Some(shadow) = effect.as_drop_shadow_effect() {
            return self.finite(
                &[
                    shadow.offset_x(),
                    shadow.offset_y(),
                    shadow.blur_radius(),
                    shadow.opacity(),
                ],
                "drop shadow",
            );
        }
        if let Some(custom) = effect.as_custom_effect() {
            use ferroui_base::media::effects::EffectParameterValue;
            let mut valid = true;
            custom.for_each_parameter(&mut |_, value| {
                valid &= match value {
                    EffectParameterValue::Float(value) => self.finite(&[value], "effect parameter"),
                    EffectParameterValue::Point(value) => {
                        self.finite(&[value.x, value.y], "effect parameter")
                    }
                    EffectParameterValue::Vector(value) => {
                        self.finite(&[value.x, value.y], "effect parameter")
                    }
                    EffectParameterValue::Size(value) => {
                        self.finite(&[value.width, value.height], "effect parameter")
                    }
                    EffectParameterValue::Rect(value) => self.rect(value),
                    EffectParameterValue::Image(image) => {
                        if let Some(bitmap) = image.bitmap() {
                            match ferroui_skia::is_bitmap_cpu_readable(&*bitmap) {
                                Some(true) => true,
                                Some(false) => {
                                    self.error("Shader image is not CPU-readable; provide an explicit CPU snapshot before SVG export");
                                    false
                                }
                                None if image.pixels().is_none() => {
                                    self.error("Shader image cannot provide CPU pixels and would be drawn transparent");
                                    false
                                }
                                // Skia's shader-image path explicitly supports
                                // foreign readable bitmaps through their pixels.
                                None => true,
                            }
                        } else {
                            true // An empty effect image is intentionally transparent.
                        }
                    }
                    _ => true,
                };
            });
            return valid;
        }
        if effect
            .as_any()
            .is::<ferroui_base::media::ImmutableLuminanceToAlphaEffect>()
        {
            return true;
        }
        self.error("Unknown effect would be omitted by the Skia backend");
        false
    }
}

impl IDrawingContextImpl for SvgDrawingContext {
    fn transform(&self) -> Matrix {
        self.inner.transform()
    }

    fn set_transform(&mut self, value: Matrix) {
        if !self.active() || !self.matrix(value) {
            return;
        }
        if value.contains_perspective() {
            self.reason("Perspective drawing transform");
        }
        self.inner.set_transform(value);
    }

    fn clear(&mut self, color: Color) {
        if !self.active() {
            return;
        }
        let untouched = {
            let audit = self.audit.borrow();
            audit.draw_calls == 0 && audit.raster_reasons.is_empty()
        };
        // Newly recorded targets are transparent. A first transparent clear,
        // including the compositor's clear under its dirty clip, has no effect.
        // Native canvas access and effect layers invalidate this shortcut even
        // when their drawing did not pass through a counted primitive method.
        if untouched && color.a == 0 {
            self.count_draw();
            return;
        }
        if color.a != 0
            && self
                .recording_bounds
                .is_some_and(|bounds| bounds.x != 0.0 || bounds.y != 0.0)
        {
            // The SVG device lowers clear to its zero-origin device rectangle.
            // An offset recording is replayed under an origin translation, so
            // that rectangle cannot express the original clear footprint.
            self.reason("Nontransparent clear of an offset recording");
        }
        if self.audit.borrow().draw_calls != 0 || !self.states.is_empty() {
            self.reason("Clear replaces previously composed or clipped content");
        }
        self.count_draw();
        // A canvas clear ignores its transform. The pinned SVG writer lowers
        // drawPaint to a device-sized rectangle under the current transform,
        // so make that invariant explicit in the recording. Clips are already
        // in device coordinates and remain unchanged, as does the caller's CTM.
        let transform = self.inner.transform();
        if !transform.is_identity() {
            self.inner.set_transform(Matrix::IDENTITY);
        }
        self.inner.clear(color);
        if !transform.is_identity() {
            self.inner.set_transform(transform);
        }
    }

    fn draw_bitmap(
        &mut self,
        source: &dyn IBitmapImpl,
        opacity: f64,
        source_rect: Rect,
        dest_rect: Rect,
    ) {
        if !self.active()
            || !self.bitmap(source)
            || !self.rect(source_rect)
            || !self.rect(dest_rect)
            || !self.finite(&[opacity], "bitmap opacity")
        {
            return;
        }
        self.audit_primitive_render_options(true);
        self.count_draw();
        self.audit.borrow_mut().embedded_images += 1;
        self.inner
            .draw_bitmap(source, opacity, source_rect, dest_rect);
    }

    fn draw_bitmap_with_mask(
        &mut self,
        source: &dyn IBitmapImpl,
        mask: &dyn IBrush,
        mask_rect: Rect,
        dest_rect: Rect,
    ) {
        if !self.active()
            || !self.bitmap(source)
            || !self.brush_with_bounds(Some(mask), Some(mask_rect))
            || !self.rect(mask_rect)
            || !self.rect(dest_rect)
        {
            return;
        }
        self.reason("Bitmap opacity mask");
        self.count_draw();
        self.audit.borrow_mut().embedded_images += 1;
        self.inner
            .draw_bitmap_with_mask(source, mask, mask_rect, dest_rect);
    }

    fn draw_line(&mut self, pen: Option<&dyn IPen>, p1: Point, p2: Point) {
        if !self.active()
            || !self.finite(&[p1.x, p1.y, p2.x, p2.y], "line")
            || !self.pen(pen, Rect::from_points(p1, p2).normalize())
        {
            return;
        }
        self.audit_primitive_render_options(false);
        self.count_draw();
        self.inner.draw_line(pen, p1, p2);
    }

    fn draw_geometry(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        geometry: &dyn IGeometryImpl,
    ) {
        if !self.active()
            || !self.brush_with_bounds(brush, Some(geometry.bounds()))
            || !self.pen(
                pen,
                geometry
                    .bounds()
                    .inflate(pen.map_or(0.0, |pen| pen.thickness() / 2.0)),
            )
            || !self.geometry(geometry)
        {
            return;
        }
        self.audit_primitive_render_options(false);
        self.count_draw();
        self.inner.draw_geometry(brush, pen, geometry);
    }

    fn draw_rectangle(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        rect: RoundedRect,
        shadows: &BoxShadows,
    ) {
        if !self.active()
            || !self.brush_with_bounds(brush, Some(rect.rect))
            || !self.pen(
                pen,
                rect.rect
                    .inflate(pen.map_or(0.0, |pen| pen.thickness() / 2.0)),
            )
            || !self.rounded_rect(rect)
        {
            return;
        }
        for shadow in shadows.iter() {
            if !self.finite(
                &[shadow.offset_x, shadow.offset_y, shadow.blur, shadow.spread],
                "box shadow",
            ) {
                return;
            }
        }
        if shadows.count() != 0 {
            self.reason("Box shadow mask filter");
        }
        self.audit_primitive_render_options(false);
        self.count_draw();
        self.inner.draw_rectangle(brush, pen, rect, shadows);
    }

    fn draw_region(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        region: &dyn IPlatformRenderInterfaceRegion,
    ) {
        if !self.active() || !self.region(region) {
            return;
        }
        let bounds = region.bounds().to_pixel_rect().to_rect(1.0);
        if !self.brush_with_bounds(brush, Some(bounds))
            || !self.pen(
                pen,
                bounds.inflate(pen.map_or(0.0, |pen| pen.thickness() / 2.0)),
            )
        {
            return;
        }
        self.audit_primitive_render_options(false);
        self.count_draw();
        self.inner.draw_region(brush, pen, region);
    }

    fn draw_ellipse(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, rect: Rect) {
        if !self.active()
            || !self.brush_with_bounds(brush, Some(rect))
            || !self.pen(
                pen,
                rect.inflate(pen.map_or(0.0, |pen| pen.thickness() / 2.0)),
            )
            || !self.rect(rect)
        {
            return;
        }
        self.audit_primitive_render_options(false);
        self.count_draw();
        self.inner.draw_ellipse(brush, pen, rect);
    }

    fn draw_glyph_run(&mut self, foreground: Option<&dyn IBrush>, run: &dyn IGlyphRunImpl) {
        if !self.active()
            || !self.brush_with_bounds(foreground, Some(run.bounds()))
            || !self.rect(run.bounds())
            || !self.finite(
                &[
                    run.font_rendering_em_size(),
                    run.baseline_origin().x,
                    run.baseline_origin().y,
                ],
                "glyph run",
            )
        {
            return;
        }
        let Some(skia_run) = run.as_any().downcast_ref::<ferroui_skia::GlyphRunImpl>() else {
            self.error("Glyph run does not belong to the Skia backend");
            return;
        };
        if !skia_run.has_vector_outlines() {
            self.reason("Glyph run requires colour, bitmap, or unavailable glyph outlines");
        }
        self.count_draw();
        self.inner.draw_glyph_run(foreground, run);
    }

    fn create_layer(&mut self, size: PixelSize) -> Rc<dyn IDrawingContextLayerImpl> {
        if !self.active() || size.width <= 0 || size.height <= 0 {
            self.error("Invalid intermediate layer size");
            return self.inner.create_layer(PixelSize::new(1, 1));
        }
        self.inner.create_layer(size)
    }

    fn create_temporary_layer(&mut self, size: PixelSize) -> Rc<dyn IDrawingContextLayerImpl> {
        if !self.active() || size.width <= 0 || size.height <= 0 {
            self.error("Invalid temporary layer size");
            return self.inner.create_temporary_layer(PixelSize::new(1, 1));
        }
        self.inner.create_temporary_layer(size)
    }

    fn try_blit_layer(&mut self, layer: &dyn IDrawingContextLayerImpl) -> bool {
        // A native Skia blit normally downcasts its destination. Intercept it
        // before that escape so a GPU-only source cannot enter a CPU picture
        // and silently disappear during SVG or raster replay.
        if self.active() && self.bitmap(layer) {
            self.reason("Intermediate layer bitmap blit");
            self.count_draw();
            self.audit.borrow_mut().embedded_images += 1;
            // Preserve Skia's device-space blit, including its transform and
            // opacity rules, rather than emulating it with draw_bitmap.
            layer.blit(self.inner.as_mut());
        }
        true
    }

    fn prefers_bounded_effect_targets(&self) -> bool {
        self.inner.prefers_bounded_effect_targets()
    }

    fn push_clip(&mut self, clip: Rect) {
        if !self.active() {
            return;
        }
        let valid = self.rect(clip);
        self.push_state(State::Clip, valid);
        if valid {
            self.inner.push_clip(clip);
        }
    }

    fn push_clip_rounded(&mut self, clip: RoundedRect) {
        if !self.active() {
            return;
        }
        let valid = self.rounded_rect(clip);
        self.push_state(State::Clip, valid);
        if valid {
            // The pinned Skia SVG clip serializer writes getSimpleRadii() as
            // rx/ry. Unlike drawRRect (which emits a path), that loses differing
            // corner radii. Raster replay retains the complete rounded clip.
            if [
                clip.radii_top_right,
                clip.radii_bottom_right,
                clip.radii_bottom_left,
            ]
            .iter()
            .any(|radii| *radii != clip.radii_top_left)
            {
                self.reason("Rounded clip with unequal corner radii");
            }
            self.inner.push_clip_rounded(clip);
        }
    }

    fn push_clip_region(&mut self, region: &dyn IPlatformRenderInterfaceRegion) {
        if !self.active() {
            return;
        }
        let valid = self.region(region);
        self.push_state(State::Clip, valid);
        if valid {
            self.inner.push_clip_region(region);
        }
    }

    fn pop_clip(&mut self) {
        if self.pop_state(State::Clip) {
            self.inner.pop_clip();
        }
    }

    fn push_layer(&mut self, bounds: Rect) {
        if !self.active() {
            return;
        }
        let valid = self.rect(bounds);
        self.push_state(State::Layer, valid);
        if valid {
            self.inner.push_layer(bounds);
        }
    }

    fn pop_layer(&mut self) {
        if self.pop_state(State::Layer) {
            self.inner.pop_layer();
        }
    }

    fn try_push_compositing_layer(
        &mut self,
        blend: BitmapBlendingMode,
        opacity: f64,
        bounds: Rect,
    ) -> bool {
        if !self.active() || !self.rect(bounds) || !self.finite(&[opacity], "compositing opacity") {
            return false;
        }
        if !self
            .inner
            .try_push_compositing_layer(blend, opacity, bounds)
        {
            return false;
        }
        self.reason("Isolated compositing layer");
        self.push_state(State::Compositing, true);
        true
    }

    fn pop_compositing_layer(&mut self) {
        if self.pop_state(State::Compositing) {
            self.inner.pop_compositing_layer();
        }
    }

    fn push_opacity(&mut self, opacity: f64, bounds: Option<Rect>) {
        if !self.active() {
            return;
        }
        let valid =
            self.finite(&[opacity], "opacity") && bounds.is_none_or(|bounds| self.rect(bounds));
        self.push_state(State::Opacity, valid);
        if valid {
            if opacity != 1.0 {
                self.reason("Opacity may require an isolated Skia layer");
            }
            self.inner.push_opacity(opacity, bounds);
        }
    }

    fn pop_opacity(&mut self) {
        if self.pop_state(State::Opacity) {
            self.inner.pop_opacity();
        }
    }

    fn push_opacity_mask(&mut self, mask: &dyn IBrush, bounds: Rect) {
        if !self.active() {
            return;
        }
        let valid = self.brush_with_bounds(Some(mask), Some(bounds)) && self.rect(bounds);
        self.push_state(State::Mask, valid);
        if valid {
            self.reason("Opacity mask uses destination-in compositing");
            self.inner.push_opacity_mask(mask, bounds);
        }
    }

    fn pop_opacity_mask(&mut self) {
        if self.pop_state(State::Mask) {
            self.inner.pop_opacity_mask();
        }
    }

    fn push_geometry_clip(&mut self, clip: &dyn IGeometryImpl) {
        if !self.active() {
            return;
        }
        let valid = self.geometry(clip);
        self.push_state(State::GeometryClip, valid);
        if valid {
            self.inner.push_geometry_clip(clip);
        }
    }

    fn pop_geometry_clip(&mut self) {
        if self.pop_state(State::GeometryClip) {
            self.inner.pop_geometry_clip();
        }
    }

    fn push_render_options(&mut self, options: RenderOptions) {
        if !self.active() {
            return;
        }
        if !matches!(
            options.bitmap_blending_mode,
            BitmapBlendingMode::Unspecified | BitmapBlendingMode::SourceOver
        ) {
            self.reason("Bitmap blend mode requires composed raster output");
        }
        self.push_state(State::RenderOptions, true);
        self.inner.push_render_options(options);
    }

    fn pop_render_options(&mut self) {
        if self.pop_state(State::RenderOptions) {
            self.inner.pop_render_options();
        }
    }

    fn push_text_options(&mut self, options: TextOptions) {
        if !self.active() {
            return;
        }
        self.push_state(State::TextOptions, true);
        self.inner.push_text_options(options);
    }

    fn pop_text_options(&mut self) {
        if self.pop_state(State::TextOptions) {
            self.inner.pop_text_options();
        }
    }

    fn get_feature(&mut self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if !self.active() {
            return None;
        }
        let feature = self.inner.get_feature(feature_type);
        if feature.is_some() {
            self.reason("Native drawing feature bypasses SVG operation classification");
        }
        feature
    }

    fn as_drawing_context_impl_with_effects(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextImplWithEffects> {
        Some(self)
    }
    fn as_drawing_context_impl_with_layer_effects(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextImplWithLayerEffects> {
        Some(self)
    }
    fn as_drawing_context_with_acrylic_like_support(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextWithAcrylicLikeSupport> {
        Some(self)
    }
    fn as_drawing_context_impl_with_scene_3d(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextImplWithScene3D> {
        Some(self)
    }
    fn as_drawing_context_impl_with_fragments(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextImplWithFragments> {
        if self
            .inner
            .as_drawing_context_impl_with_fragments()
            .is_some()
        {
            Some(self)
        } else {
            None
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        // Skia's SurfaceRenderTarget::blit downcasts this to DrawingContextImpl.
        // Preserve that backend contract, and conservatively classify the escape.
        self.reason("Native context access bypasses SVG operation classification");
        self.inner.as_any_mut()
    }

    fn dispose(&mut self) {
        self.finalize(false);
    }
}

impl IDrawingContextImplWithEffects for SvgDrawingContext {
    fn push_effect(&mut self, clip: Option<Rect>, effect: &dyn IEffect) {
        if !self.active() {
            return;
        }
        let valid = clip.is_none_or(|bounds| self.rect(bounds)) && self.effect(effect);
        let mut forwarded = false;
        if valid {
            self.reason("Image or custom shader effect");
            if let Some(custom) = effect.as_custom_effect() {
                // The normal Skia effect API intentionally falls back to
                // unchanged content for a missing/invalid shader. An export
                // cannot call that successful, even after rasterization.
                forwarded = self
                    .inner
                    .as_any_mut()
                    .downcast_mut::<ferroui_skia::DrawingContextImpl>()
                    .is_some_and(|context| context.try_push_custom_effect(clip, custom));
                if !forwarded {
                    self.error(&format!(
                        "Custom effect '{}' cannot execute in the Skia export context",
                        custom.name()
                    ));
                }
            } else if let Some(context) = self.inner.as_drawing_context_impl_with_effects() {
                context.push_effect(clip, effect);
                forwarded = true;
            } else {
                self.error("Backend cannot draw effects");
            }
        }
        self.push_state(State::Effect, forwarded);
    }

    fn pop_effect(&mut self) {
        if self.pop_state(State::Effect) {
            if let Some(context) = self.inner.as_drawing_context_impl_with_effects() {
                context.pop_effect();
            }
        }
    }
}

// The optional layer-effect interface uses paired pushes and pops. Never rely
// on its default no-op methods: every supported operation reaches Skia, while
// a missing implementation is recorded as an export error.
macro_rules! layer_pop {
    ($method:ident, $state:ident) => {
        fn $method(&mut self) {
            if self.pop_state(State::$state) {
                if let Some(context) = self.inner.as_drawing_context_impl_with_layer_effects() {
                    context.$method();
                }
            }
        }
    };
}

impl IDrawingContextImplWithLayerEffects for SvgDrawingContext {
    fn push_blend_layer(&mut self, bounds: Rect, mode: BitmapBlendingMode) {
        if !self.active() {
            return;
        }
        let valid = self.rect(bounds);
        let mut forwarded = false;
        if valid {
            self.reason("Blend layer");
            if let Some(context) = self.inner.as_drawing_context_impl_with_layer_effects() {
                context.push_blend_layer(bounds, mode);
                forwarded = true;
            } else {
                self.error("Backend cannot draw layer effects");
            }
        }
        self.push_state(State::Blend, forwarded);
    }
    layer_pop!(pop_blend_layer, Blend);

    fn push_color_matrix_layer(
        &mut self,
        bounds: Rect,
        matrix: &ColorMatrix,
        alpha_mode: ColorMatrixAlphaMode,
    ) {
        if !self.active() {
            return;
        }
        let valid =
            self.rect(bounds) && matrix.rows.iter().flatten().all(|value| value.is_finite());
        if !valid {
            self.error("Invalid color matrix layer");
        }
        let mut forwarded = false;
        if valid {
            self.reason("Color matrix layer");
            if let Some(context) = self.inner.as_drawing_context_impl_with_layer_effects() {
                context.push_color_matrix_layer(bounds, matrix, alpha_mode);
                forwarded = true;
            } else {
                self.error("Backend cannot draw layer effects");
            }
        }
        self.push_state(State::ColorMatrix, forwarded);
    }
    layer_pop!(pop_color_matrix_layer, ColorMatrix);

    fn push_multiply_layer(&mut self, bounds: Rect, factors: &MultiplyLayerFactors) {
        if !self.active() {
            return;
        }
        let size = factors.size;
        let expected = (size.width as u64)
            .checked_mul(size.height as u64)
            .and_then(|pixels| pixels.checked_mul(4));
        let valid = self.rect(bounds)
            && size.width > 0
            && size.height > 0
            && factors.scale.is_finite()
            && expected.is_some_and(|bytes| bytes <= factors.rgba.len() as u64);
        if !valid {
            self.error("Invalid multiply layer factors");
        }
        let mut forwarded = false;
        if valid {
            self.reason("Per-pixel multiply layer");
            if let Some(context) = self.inner.as_drawing_context_impl_with_layer_effects() {
                context.push_multiply_layer(bounds, factors);
                forwarded = true;
            } else {
                self.error("Backend cannot draw layer effects");
            }
        }
        self.push_state(State::Multiply, forwarded);
    }
    layer_pop!(pop_multiply_layer, Multiply);

    fn push_convolve_matrix_layer(&mut self, bounds: Rect, convolution: &ConvolveMatrix) {
        if !self.active() {
            return;
        }
        let valid = self.rect(bounds)
            && convolution.elements().iter().all(|value| value.is_finite())
            && convolution.divisor.is_finite()
            && convolution.bias.is_finite();
        if !valid {
            self.error("Invalid convolution layer");
        }
        let mut forwarded = false;
        if valid {
            self.reason("Convolution layer");
            if let Some(context) = self.inner.as_drawing_context_impl_with_layer_effects() {
                context.push_convolve_matrix_layer(bounds, convolution);
                forwarded = true;
            } else {
                self.error("Backend cannot draw layer effects");
            }
        }
        self.push_state(State::Convolution, forwarded);
    }
    layer_pop!(pop_convolve_matrix_layer, Convolution);

    fn draw_lighting(&mut self, rect: Rect, lighting: &LightingParameters) -> bool {
        if !self.active() || !self.rect(rect) {
            return false;
        }
        let drawn = self
            .inner
            .as_drawing_context_impl_with_layer_effects()
            .is_some_and(|context| context.draw_lighting(rect, lighting));
        if drawn {
            self.reason("Lighting shader");
            self.count_draw();
        }
        // A false return requests the compositor's CPU reference; it is not an
        // error unless that caller also fails to supply the reference image.
        drawn
    }

    fn push_lighting_multiply_layer(
        &mut self,
        bounds: Rect,
        lighting: &LightingParameters,
    ) -> bool {
        if !self.active() || !self.rect(bounds) {
            return false;
        }
        let pushed = self
            .inner
            .as_drawing_context_impl_with_layer_effects()
            .is_some_and(|context| context.push_lighting_multiply_layer(bounds, lighting));
        if pushed {
            self.reason("Lighting multiply shader");
            self.push_state(State::Multiply, true);
        }
        pushed
    }

    fn push_pixel_operation(&mut self, bounds: Rect, operation: &PixelOperation) -> bool {
        if !self.active() || !self.rect(bounds) {
            return false;
        }
        match operation {
            PixelOperation::Arithmetic { k } if !k.iter().all(|value| value.is_finite()) => {
                self.error("Non-finite pixel operation coefficients");
                return false;
            }
            PixelOperation::Blend { noise_grid, .. } if !self.rect(*noise_grid) => return false,
            _ => {}
        }
        let pushed = self
            .inner
            .as_drawing_context_impl_with_layer_effects()
            .is_some_and(|context| context.push_pixel_operation(bounds, operation));
        if pushed {
            self.reason("Pixel operation shader");
            self.push_state(State::PixelOperation, true);
        }
        pushed
    }

    fn next_pixel_operation_source(&mut self) {
        if !self.active() {
            return;
        }
        let state = self
            .states
            .iter()
            .rev()
            .find(|entry| !matches!(entry.0, State::RenderOptions | State::TextOptions));
        if state != Some(&(State::PixelOperation, true)) {
            self.error("Pixel operation source without a matching operation");
            return;
        }
        if let Some(context) = self.inner.as_drawing_context_impl_with_layer_effects() {
            context.next_pixel_operation_source();
        }
    }
    layer_pop!(pop_pixel_operation, PixelOperation);
}

impl IDrawingContextWithAcrylicLikeSupport for SvgDrawingContext {
    fn draw_rectangle_with_material(
        &mut self,
        material: &dyn IExperimentalAcrylicMaterial,
        rect: RoundedRect,
    ) {
        if !self.active() || !self.rounded_rect(rect) {
            return;
        }
        self.reason("Acrylic material shader");
        if let Some(context) = self.inner.as_drawing_context_with_acrylic_like_support() {
            context.draw_rectangle_with_material(material, rect);
            self.count_draw();
        } else {
            self.error("Backend cannot draw acrylic materials");
        }
    }
}

impl IDrawingContextImplWithScene3D for SvgDrawingContext {
    fn draw_scene_3d(&mut self, frame: &Scene3DFrame) -> bool {
        if !self.active() || !self.rect(frame.bounds) {
            return false;
        }
        let drawn = self
            .inner
            .as_drawing_context_impl_with_scene_3d()
            .is_some_and(|context| context.draw_scene_3d(frame));
        if drawn {
            self.count_draw();
            self.audit.borrow_mut().embedded_images += 1;
        } else {
            self.error("3D scene has no available Skia executor; enable the media3d feature");
        }
        drawn
    }
}

impl IDrawingContextImplWithFragments for SvgDrawingContext {
    fn begin_fragment(&mut self) {
        if !self.active() {
            return;
        }
        if self.fragment_open {
            self.error("Nested drawing fragments are not supported");
            return;
        }
        self.reason("Reusable native drawing fragment");
        if let Some(context) = self.inner.as_drawing_context_impl_with_fragments() {
            context.begin_fragment();
            self.fragment_open = true;
        } else {
            self.error("Backend cannot record drawing fragments");
        }
    }

    fn end_fragment(&mut self) -> Option<Rc<dyn IDrawingFragment>> {
        if !self.active() {
            return None;
        }
        if !self.fragment_open {
            self.error("No drawing fragment was started");
            return None;
        }
        self.fragment_open = false;
        self.inner
            .as_drawing_context_impl_with_fragments()
            .and_then(|context| context.end_fragment())
    }

    fn draw_fragment(&mut self, fragment: &Rc<dyn IDrawingFragment>) -> bool {
        if !self.active() {
            return false;
        }
        let drawn = self
            .inner
            .as_drawing_context_impl_with_fragments()
            .is_some_and(|context| context.draw_fragment(fragment));
        if drawn {
            self.reason("Reusable native drawing fragment");
            self.count_draw();
        }
        drawn
    }
}

impl Drop for SvgDrawingContext {
    fn drop(&mut self) {
        self.finalize(std::thread::panicking());
    }
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
