use crate::media::{BoxShadows, Color, IBrush, IEffect, IExperimentalAcrylicMaterial, IPen, RenderOptions};
use crate::media::imaging::BitmapBlendingMode;
use crate::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, IDrawingContextImplWithEffects, IDrawingContextLayerImpl,
    IDrawingContextWithAcrylicLikeSupport, IGeometryImpl, IGlyphRunImpl, ILockedFramebuffer,
    IPlatformRenderInterfaceRegion, IReadableBitmapImpl, IRenderTargetBitmapImpl, PixelFormat, PixelFormats,
};
use crate::{Matrix, PixelSize, Rect, RoundedRect, Vector};
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// The shared record of what was drawn on mock drawing contexts, one line
/// per call, in call order.
#[derive(Clone, Default)]
pub struct DrawingLog(std::sync::Arc<std::sync::Mutex<Vec<String>>>, std::sync::Arc<std::sync::atomic::AtomicBool>);

impl DrawingLog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Makes the mock layers that record to this log hold a frame back
    /// ([`IDrawingContextLayerImpl::has_pending_frame`]): a blit shows the
    /// frame before the one drawn last, until the pending frame is
    /// completed. Such a layer logs `Blit frame <shown> of <drawn>` and
    /// `CompletePendingFrame`.
    pub fn hold_layer_frames_back(&self, hold: bool) {
        self.1.store(hold, std::sync::atomic::Ordering::SeqCst);
    }

    fn layers_hold_frames_back(&self) -> bool {
        self.1.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Appends an entry.
    pub fn push(&self, entry: impl Into<String>) {
        self.0.lock().unwrap().push(entry.into());
    }

    /// A copy of the entries recorded so far.
    pub fn entries(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }

    /// Removes and returns the entries recorded so far.
    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }

    /// The number of entries starting with `prefix`.
    pub fn count(&self, prefix: &str) -> usize {
        self.0.lock().unwrap().iter().filter(|e| e.starts_with(prefix)).count()
    }

    pub fn clear(&self) {
        self.0.lock().unwrap().clear();
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

fn effect_text(effect: &dyn IEffect) -> String {
    if let Some(blur) = effect.as_blur_effect() {
        format!("blur({})", blur.radius())
    } else if let Some(shadow) = effect.as_drop_shadow_effect() {
        format!("drop-shadow({} {} {})", shadow.offset_x(), shadow.offset_y(), shadow.blur_radius())
    } else if let Some(custom) = effect.as_custom_effect() {
        // A runtime shader effect (#455, E4) is marked as one.
        if custom.has_shader() {
            format!("shader {custom}")
        } else {
            custom.to_string()
        }
    } else {
        "effect".to_string()
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
    /// Whether explicit isolated group pushes are accepted. Off by default.
    pub supports_compositing_layers: bool,
    /// Whether pushed effects are recorded with the effect
    /// (`PushEffect <clip> blur(<radius>)`; a custom effect by its effect
    /// string, after `shader ` for a runtime shader effect). Off by default.
    pub log_effects: bool,
    /// Whether the context reports acrylic support
    /// (`DrawRectangleWithMaterial <tint> <rect>`). Off by default.
    pub supports_acrylic: bool,
    /// Whether the context reports blend and colour matrix layers
    /// (`PushBlendLayer <bounds> <mode>` / `PopBlendLayer`,
    /// `PushColorMatrixLayer <bounds>` / `PopColorMatrixLayer`). Off by
    /// default.
    pub supports_layer_effects: bool,
    /// Whether the context (with layer effects) evaluates the lights and the
    /// pixel operations itself (`DrawLighting <rect> <lights>`,
    /// `PushLightingMultiplyLayer <bounds> <lights>`,
    /// `PushPixelOperation <bounds> <operation>` / `NextPixelOperationSource`
    /// / `PopPixelOperation`), as a GPU backend does. Off by default.
    pub supports_gpu_effects: bool,
    /// Whether pushed render options are recorded with their values
    /// (`PushRenderOptions <text> <interpolation> <edge> <blending>
    /// <full opacity>`) rather than as `PushRenderOptions`. Off by default.
    pub log_render_options: bool,
    /// Whether pushed colour matrix layers are recorded with their matrix
    /// (`PushColorMatrixLayer <bounds> <alpha mode> <matrix>`, the twenty
    /// members of the row-major 4 x 5 form, rounded to three decimals). Off
    /// by default.
    pub log_matrices: bool,
    /// Whether the context draws 3D scenes (`DrawScene3D <bounds> <models>
    /// models <lights> lights`, #608), as a backend with a 3D executor does.
    /// Off by default: a scene then draws nothing and is counted.
    pub supports_scene_3d: bool,
}

impl MockDrawingContextImpl {
    pub fn new(log: DrawingLog) -> Self {
        Self {
            log,
            transform: Matrix::IDENTITY,
            log_transforms: true,
            supports_effects: false,
            supports_compositing_layers: false,
            log_effects: false,
            supports_acrylic: false,
            supports_layer_effects: false,
            supports_gpu_effects: false,
            log_render_options: false,
            log_matrices: false,
            supports_scene_3d: false,
        }
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
        let mut layer = MockDrawingContextLayerImpl::new(self.log.clone(), size);
        layer.supports_layer_effects = self.supports_layer_effects;
        layer.supports_gpu_effects = self.supports_gpu_effects;
        layer.supports_effects = self.supports_effects;
        layer.log_effects = self.log_effects;
        Rc::new(layer)
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

    fn try_push_compositing_layer(&mut self, blend: BitmapBlendingMode, opacity: f64, bounds: Rect) -> bool {
        if !self.supports_compositing_layers {
            return false;
        }
        self.log.push(format!("PushCompositingLayer {blend:?} {opacity} {bounds}"));
        true
    }

    fn pop_compositing_layer(&mut self) {
        self.log.push("PopCompositingLayer");
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

    fn push_render_options(&mut self, render_options: RenderOptions) {
        if self.log_render_options {
            self.log.push(format!(
                "PushRenderOptions {:?} {:?} {:?} {:?} {:?}",
                render_options.text_rendering_mode,
                render_options.bitmap_interpolation_mode,
                render_options.edge_mode,
                render_options.bitmap_blending_mode,
                render_options.requires_full_opacity_handling
            ));
        } else {
            self.log.push("PushRenderOptions");
        }
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

    fn as_drawing_context_impl_with_layer_effects(
        &mut self,
    ) -> Option<&mut dyn crate::platform::IDrawingContextImplWithLayerEffects> {
        if self.supports_layer_effects {
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

    fn as_drawing_context_impl_with_scene_3d(
        &mut self,
    ) -> Option<&mut dyn crate::platform::IDrawingContextImplWithScene3D> {
        if self.supports_scene_3d {
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

impl crate::platform::IDrawingContextImplWithScene3D for MockDrawingContextImpl {
    fn draw_scene_3d(&mut self, frame: &crate::rendering::scene3d::Scene3DFrame) -> bool {
        self.log.push(format!(
            "DrawScene3D {} {} models {} lights",
            frame.bounds,
            frame.scene.models.len(),
            frame.scene.lights.len()
        ));
        true
    }
}

impl IDrawingContextImplWithEffects for MockDrawingContextImpl {
    fn push_effect(&mut self, clip_rect: Option<Rect>, effect: &dyn IEffect) {
        let clip = match clip_rect {
            Some(clip_rect) => clip_rect.to_string(),
            None => "none".to_string(),
        };
        if self.log_effects {
            self.log.push(format!("PushEffect {clip} {}", effect_text(effect)));
        } else {
            self.log.push(format!("PushEffect {clip}"));
        }
    }

    fn pop_effect(&mut self) {
        self.log.push("PopEffect");
    }
}

impl crate::platform::IDrawingContextImplWithLayerEffects for MockDrawingContextImpl {
    fn push_blend_layer(&mut self, bounds: Rect, mode: crate::media::imaging::BitmapBlendingMode) {
        self.log.push(format!("PushBlendLayer {bounds} {mode:?}"));
    }

    fn pop_blend_layer(&mut self) {
        self.log.push("PopBlendLayer");
    }

    fn push_color_matrix_layer(
        &mut self,
        bounds: Rect,
        matrix: &crate::media::ColorMatrix,
        alpha_mode: crate::media::ColorMatrixAlphaMode,
    ) {
        if self.log_matrices {
            let members: Vec<String> =
                matrix.to_row_major_4x5().iter().map(|value| format!("{}", (value * 1000.0).round() / 1000.0)).collect();
            self.log.push(format!("PushColorMatrixLayer {bounds} {alpha_mode:?} [{}]", members.join(" ")));
        } else {
            self.log.push(format!("PushColorMatrixLayer {bounds} {alpha_mode:?}"));
        }
    }

    fn pop_color_matrix_layer(&mut self) {
        self.log.push("PopColorMatrixLayer");
    }

    fn push_multiply_layer(&mut self, bounds: Rect, factors: &crate::platform::MultiplyLayerFactors) {
        self.log.push(format!(
            "PushMultiplyLayer {bounds} {}x{} {}",
            factors.size.width, factors.size.height, factors.scale
        ));
    }

    fn pop_multiply_layer(&mut self) {
        self.log.push("PopMultiplyLayer");
    }

    fn push_convolve_matrix_layer(&mut self, bounds: Rect, convolution: &crate::media::ConvolveMatrix) {
        if self.log_matrices {
            let elements: Vec<String> =
                convolution.elements().iter().map(|value| format!("{}", (value * 1000.0).round() / 1000.0)).collect();
            self.log.push(format!(
                "PushConvolveMatrixLayer {bounds} {}x{} [{}]",
                convolution.kernel_width,
                convolution.kernel_height,
                elements.join(" ")
            ));
        } else {
            self.log.push(format!(
                "PushConvolveMatrixLayer {bounds} {}x{}",
                convolution.kernel_width, convolution.kernel_height
            ));
        }
    }

    fn pop_convolve_matrix_layer(&mut self) {
        self.log.push("PopConvolveMatrixLayer");
    }

    fn draw_lighting(&mut self, rect: Rect, lighting: &crate::platform::LightingParameters) -> bool {
        if self.supports_gpu_effects {
            self.log.push(format!("DrawLighting {rect} {}", lighting.lights.len()));
        }
        self.supports_gpu_effects
    }

    fn push_lighting_multiply_layer(&mut self, bounds: Rect, lighting: &crate::platform::LightingParameters) -> bool {
        if self.supports_gpu_effects {
            self.log.push(format!("PushLightingMultiplyLayer {bounds} {}", lighting.lights.len()));
        }
        self.supports_gpu_effects
    }

    fn push_pixel_operation(&mut self, bounds: Rect, operation: &crate::platform::PixelOperation) -> bool {
        if self.supports_gpu_effects {
            let operation = match operation {
                crate::platform::PixelOperation::Blend { mode, noise_grid, noise_size } => {
                    format!("Blend {mode:?} {noise_grid} {noise_size}")
                }
                crate::platform::PixelOperation::Arithmetic { k } => format!("Arithmetic {k:?}"),
                crate::platform::PixelOperation::MaskInvert => "MaskInvert".to_string(),
            };
            self.log.push(format!("PushPixelOperation {bounds} {operation}"));
        }
        self.supports_gpu_effects
    }

    fn next_pixel_operation_source(&mut self) {
        self.log.push("NextPixelOperationSource");
    }

    fn pop_pixel_operation(&mut self) {
        self.log.push("PopPixelOperation");
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
    supports_layer_effects: bool,
    supports_gpu_effects: bool,
    supports_effects: bool,
    log_effects: bool,
    /// For a layer that holds frames back
    /// ([`DrawingLog::hold_layer_frames_back`]): the number of frames drawn
    /// into the layer, and the number of the frame a blit shows.
    drawn: AtomicU32,
    shown: AtomicU32,
    /// Whether a frame was drawn since the last blit.
    drew_since_blit: AtomicBool,
}

impl MockDrawingContextLayerImpl {
    pub fn new(log: DrawingLog, size: PixelSize) -> Self {
        Self {
            log,
            size,
            supports_layer_effects: false,
            supports_gpu_effects: false,
            supports_effects: false,
            log_effects: false,
            drawn: AtomicU32::new(0),
            shown: AtomicU32::new(0),
            drew_since_blit: AtomicBool::new(false),
        }
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
        if !self.log.layers_hold_frames_back() {
            self.log.push("Blit");
            return;
        }
        // A tick that drew a frame presents the one before it.
        if self.drew_since_blit.swap(false, Ordering::SeqCst) {
            self.shown.store(self.drawn.load(Ordering::SeqCst) - 1, Ordering::SeqCst);
        }
        self.log.push(format!(
            "Blit frame {} of {}",
            self.shown.load(Ordering::SeqCst),
            self.drawn.load(Ordering::SeqCst)
        ));
    }

    fn can_blit(&self) -> bool {
        true
    }

    fn is_corrupted(&self) -> bool {
        false
    }

    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        if self.log.layers_hold_frames_back() {
            self.drawn.fetch_add(1, Ordering::SeqCst);
            self.drew_since_blit.store(true, Ordering::SeqCst);
        }
        let mut context = MockDrawingContextImpl::new(self.log.clone());
        context.supports_layer_effects = self.supports_layer_effects;
        context.supports_gpu_effects = self.supports_gpu_effects;
        context.supports_effects = self.supports_effects;
        context.log_effects = self.log_effects;
        Box::new(context)
    }

    fn has_pending_frame(&self) -> bool {
        self.log.layers_hold_frames_back() && self.shown.load(Ordering::SeqCst) < self.drawn.load(Ordering::SeqCst)
    }

    fn complete_pending_frame(&self) {
        if self.log.layers_hold_frames_back() {
            self.log.push("CompletePendingFrame");
            self.shown.store(self.drawn.load(Ordering::SeqCst), Ordering::SeqCst);
        }
    }

    fn create_shared_snapshot(&self) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
        std::sync::Arc::new(MockRenderTargetBitmapImpl::new(self.log.clone(), self.size, Vector::new(96.0, 96.0)))
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

    /// Transparent pixels: the mock draws nothing into them.
    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        let (width, height) = (self.size.width.max(0) as usize, self.size.height.max(0) as usize);
        Rc::new(MockLockedFramebuffer {
            data: std::cell::RefCell::new(vec![0; width * height * 4]),
            size: self.size,
            dpi: self.dpi,
        })
    }
}

/// The transparent RGBA pixels of a locked mock bitmap.
struct MockLockedFramebuffer {
    data: std::cell::RefCell<Vec<u8>>,
    size: PixelSize,
    dpi: Vector,
}

impl ILockedFramebuffer for MockLockedFramebuffer {
    fn address(&self) -> *mut u8 {
        self.data.borrow_mut().as_mut_ptr()
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(&mut self.data.borrow_mut());
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn row_bytes(&self) -> i32 {
        self.size.width * 4
    }

    fn dpi(&self) -> Vector {
        self.dpi
    }

    fn format(&self) -> PixelFormat {
        PixelFormats::RGBA8888
    }

    fn alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }

    fn dispose(&self) {}
}

impl IRenderTargetBitmapImpl for MockRenderTargetBitmapImpl {
    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        Box::new(MockDrawingContextImpl::new(self.log.clone()))
    }
}
