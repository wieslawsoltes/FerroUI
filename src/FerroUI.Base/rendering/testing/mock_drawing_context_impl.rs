use crate::media::{BoxShadows, Color, IBrush, IEffect, IExperimentalAcrylicMaterial, IPen, RenderOptions};
use crate::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, IDrawingContextImplWithEffects, IDrawingContextLayerImpl,
    IDrawingContextWithAcrylicLikeSupport, IGeometryImpl, IGlyphRunImpl, ILockedFramebuffer,
    IPlatformRenderInterfaceRegion, IReadableBitmapImpl, IRenderTargetBitmapImpl, PixelFormat,
};
use crate::{Matrix, PixelSize, Rect, RoundedRect, Vector};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

/// The shared record of what was drawn on mock drawing contexts, one line
/// per call, in call order.
#[derive(Clone, Default)]
pub struct DrawingLog(Rc<RefCell<Vec<String>>>);

impl DrawingLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an entry.
    pub fn push(&self, entry: impl Into<String>) {
        self.0.borrow_mut().push(entry.into());
    }

    /// A copy of the entries recorded so far.
    pub fn entries(&self) -> Vec<String> {
        self.0.borrow().clone()
    }

    /// Removes and returns the entries recorded so far.
    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.0.borrow_mut())
    }

    /// The number of entries starting with `prefix`.
    pub fn count(&self, prefix: &str) -> usize {
        self.0.borrow().iter().filter(|e| e.starts_with(prefix)).count()
    }

    pub fn clear(&self) {
        self.0.borrow_mut().clear();
    }
}

fn brush_text(brush: Option<&dyn IBrush>) -> String {
    match brush {
        None => "none".to_string(),
        Some(brush) => match brush.as_solid_color_brush() {
            Some(solid) => format!("{}", solid.color()),
            None => "brush".to_string(),
        },
    }
}

fn pen_text(pen: Option<&dyn IPen>) -> String {
    match pen {
        None => "none".to_string(),
        Some(pen) => format!("{}@{}", brush_text(pen.brush().as_deref()), pen.thickness()),
    }
}

/// A platform drawing context that records every call in a [`DrawingLog`].
pub struct MockDrawingContextImpl {
    log: DrawingLog,
    transform: Matrix,
    /// Whether transform changes are recorded (`SetTransform <matrix>`).
    pub log_transforms: bool,
    /// Whether the context reports effect support (`PushEffect <clip>` /
    /// `PopEffect`). Off by default.
    pub supports_effects: bool,
    /// Whether the context reports acrylic support
    /// (`DrawRectangleWithMaterial <tint> <rect>`). Off by default.
    pub supports_acrylic: bool,
}

impl MockDrawingContextImpl {
    pub fn new(log: DrawingLog) -> Self {
        Self { log, transform: Matrix::IDENTITY, log_transforms: true, supports_effects: false, supports_acrylic: false }
    }

    /// The log this context records to.
    pub fn log(&self) -> &DrawingLog {
        &self.log
    }
}

impl IDrawingContextImpl for MockDrawingContextImpl {
    fn transform(&self) -> Matrix {
        self.transform
    }

    fn set_transform(&mut self, value: Matrix) {
        self.transform = value;
        if self.log_transforms {
            self.log.push(format!("SetTransform {value}"));
        }
    }

    fn clear(&mut self, color: Color) {
        self.log.push(format!("Clear {color}"));
    }

    fn draw_bitmap(&mut self, _source: &dyn IBitmapImpl, opacity: f64, source_rect: Rect, dest_rect: Rect) {
        self.log.push(format!("DrawBitmap {opacity} {source_rect} {dest_rect}"));
    }

    fn draw_bitmap_with_mask(
        &mut self,
        _source: &dyn IBitmapImpl,
        opacity_mask: &dyn IBrush,
        opacity_mask_rect: Rect,
        dest_rect: Rect,
    ) {
        self.log.push(format!(
            "DrawBitmapWithMask {} {opacity_mask_rect} {dest_rect}",
            brush_text(Some(opacity_mask))
        ));
    }

    fn draw_line(&mut self, pen: Option<&dyn IPen>, p1: crate::Point, p2: crate::Point) {
        self.log.push(format!("DrawLine {} {p1} {p2}", pen_text(pen)));
    }

    fn draw_geometry(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, geometry: &dyn IGeometryImpl) {
        self.log.push(format!("DrawGeometry {} {} {}", brush_text(brush), pen_text(pen), geometry.bounds()));
    }

    fn draw_rectangle(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        rect: RoundedRect,
        box_shadows: &BoxShadows,
    ) {
        self.log.push(format!(
            "DrawRectangle {} {} {} shadows={}",
            brush_text(brush),
            pen_text(pen),
            rect.rect,
            box_shadows.count()
        ));
    }

    fn draw_region(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        _region: &dyn IPlatformRenderInterfaceRegion,
    ) {
        self.log.push(format!("DrawRegion {} {}", brush_text(brush), pen_text(pen)));
    }

    fn draw_ellipse(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, rect: Rect) {
        self.log.push(format!("DrawEllipse {} {} {rect}", brush_text(brush), pen_text(pen)));
    }

    fn draw_glyph_run(&mut self, foreground: Option<&dyn IBrush>, glyph_run: &dyn IGlyphRunImpl) {
        self.log.push(format!("DrawGlyphRun {} {}", brush_text(foreground), glyph_run.bounds()));
    }

    fn create_layer(&mut self, size: PixelSize) -> Rc<dyn IDrawingContextLayerImpl> {
        self.log.push(format!("CreateLayer {size}"));
        Rc::new(MockDrawingContextLayerImpl::new(self.log.clone(), size))
    }

    fn push_clip(&mut self, clip: Rect) {
        self.log.push(format!("PushClip {clip}"));
    }

    fn push_clip_rounded(&mut self, clip: RoundedRect) {
        self.log.push(format!("PushRoundedClip {}", clip.rect));
    }

    fn push_clip_region(&mut self, _region: &dyn IPlatformRenderInterfaceRegion) {
        self.log.push("PushClipRegion");
    }

    fn pop_clip(&mut self) {
        self.log.push("PopClip");
    }

    fn push_layer(&mut self, bounds: Rect) {
        self.log.push(format!("PushLayer {bounds}"));
    }

    fn pop_layer(&mut self) {
        self.log.push("PopLayer");
    }

    fn push_opacity(&mut self, opacity: f64, _bounds: Option<Rect>) {
        self.log.push(format!("PushOpacity {opacity}"));
    }

    fn pop_opacity(&mut self) {
        self.log.push("PopOpacity");
    }

    fn push_opacity_mask(&mut self, mask: &dyn IBrush, bounds: Rect) {
        self.log.push(format!("PushOpacityMask {} {bounds}", brush_text(Some(mask))));
    }

    fn pop_opacity_mask(&mut self) {
        self.log.push("PopOpacityMask");
    }

    fn push_geometry_clip(&mut self, clip: &dyn IGeometryImpl) {
        self.log.push(format!("PushGeometryClip {}", clip.bounds()));
    }

    fn pop_geometry_clip(&mut self) {
        self.log.push("PopGeometryClip");
    }

    fn push_render_options(&mut self, _render_options: RenderOptions) {
        self.log.push("PushRenderOptions");
    }

    fn pop_render_options(&mut self) {
        self.log.push("PopRenderOptions");
    }

    fn push_text_options(&mut self, text_options: crate::media::TextOptions) {
        self.log.push(format!(
            "PushTextOptions {:?} {:?} {:?}",
            text_options.text_rendering_mode, text_options.text_hinting_mode, text_options.baseline_pixel_alignment
        ));
    }

    fn pop_text_options(&mut self) {
        self.log.push("PopTextOptions");
    }

    fn get_feature(&mut self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }

    fn as_drawing_context_impl_with_effects(&mut self) -> Option<&mut dyn IDrawingContextImplWithEffects> {
        if self.supports_effects {
            Some(self)
        } else {
            None
        }
    }

    fn as_drawing_context_with_acrylic_like_support(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextWithAcrylicLikeSupport> {
        if self.supports_acrylic {
            Some(self)
        } else {
            None
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn dispose(&mut self) {
        self.log.push("Dispose");
    }
}

impl IDrawingContextImplWithEffects for MockDrawingContextImpl {
    fn push_effect(&mut self, clip_rect: Option<Rect>, _effect: &dyn IEffect) {
        match clip_rect {
            Some(clip_rect) => self.log.push(format!("PushEffect {clip_rect}")),
            None => self.log.push("PushEffect none"),
        }
    }

    fn pop_effect(&mut self) {
        self.log.push("PopEffect");
    }
}

impl IDrawingContextWithAcrylicLikeSupport for MockDrawingContextImpl {
    fn draw_rectangle_with_material(&mut self, material: &dyn IExperimentalAcrylicMaterial, rect: RoundedRect) {
        self.log.push(format!("DrawRectangleWithMaterial {} {}", material.tint_color(), rect.rect));
    }
}

/// An offscreen layer whose drawing contexts record to a [`DrawingLog`].
pub struct MockDrawingContextLayerImpl {
    log: DrawingLog,
    size: PixelSize,
}

impl MockDrawingContextLayerImpl {
    pub fn new(log: DrawingLog, size: PixelSize) -> Self {
        Self { log, size }
    }
}

impl IBitmapImpl for MockDrawingContextLayerImpl {
    fn dpi(&self) -> Vector {
        Vector::new(96.0, 96.0)
    }

    fn pixel_size(&self) -> PixelSize {
        self.size
    }

    fn version(&self) -> i32 {
        0
    }

    fn save(
        &self,
        _stream: &mut dyn std::io::Write,
        _options: &crate::media::imaging::BitmapEncoderOptions,
    ) -> std::io::Result<()> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "the mock layer has no pixels to save"))
    }

    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IDrawingContextLayerImpl for MockDrawingContextLayerImpl {
    fn blit(&self, _context: &mut dyn IDrawingContextImpl) {
        self.log.push("Blit");
    }

    fn can_blit(&self) -> bool {
        true
    }

    fn is_corrupted(&self) -> bool {
        false
    }

    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        Box::new(MockDrawingContextImpl::new(self.log.clone()))
    }
}

/// A render target bitmap whose drawing contexts record to a
/// [`DrawingLog`]. It has no pixels: locking it panics.
pub struct MockRenderTargetBitmapImpl {
    log: DrawingLog,
    size: PixelSize,
    dpi: Vector,
}

impl MockRenderTargetBitmapImpl {
    pub fn new(log: DrawingLog, size: PixelSize, dpi: Vector) -> Self {
        Self { log, size, dpi }
    }
}

impl IBitmapImpl for MockRenderTargetBitmapImpl {
    fn dpi(&self) -> Vector {
        self.dpi
    }

    fn pixel_size(&self) -> PixelSize {
        self.size
    }

    fn version(&self) -> i32 {
        0
    }

    fn save(
        &self,
        _stream: &mut dyn std::io::Write,
        _options: &crate::media::imaging::BitmapEncoderOptions,
    ) -> std::io::Result<()> {
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "the mock bitmap has no pixels to save"))
    }

    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        Some(self)
    }
}

impl IReadableBitmapImpl for MockRenderTargetBitmapImpl {
    fn format(&self) -> Option<PixelFormat> {
        None
    }

    fn alpha_format(&self) -> Option<AlphaFormat> {
        None
    }

    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        panic!("the mock render target bitmap has no pixels")
    }
}

impl IRenderTargetBitmapImpl for MockRenderTargetBitmapImpl {
    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        Box::new(MockDrawingContextImpl::new(self.log.clone()))
    }
}
