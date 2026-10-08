use crate::geometry_impl::{try_get_geometry_impl, GeometryImpl};
use crate::glyph_run_impl::GlyphRunImpl;
use crate::gpu::{ISkiaGpu, ISkiaGpuRenderSession, ISkiaGrContext};
use crate::helpers::drawing_context_helper;
use crate::i_drawable_bitmap_impl::try_get_drawable_bitmap;
use crate::i_skia_api_lease_feature::{ISkiaApiLease, ISkiaApiLeaseFeature, ISkiaPlatformGraphicsApiLease};
use crate::sk_paint_cache::SkPaintCache;
use crate::sk_round_rect_cache::SkRoundRectCache;
use crate::skia_options::SkiaOptions;
use crate::skia_platform::SkiaPlatform;
use crate::skia_region_impl::SkiaRegionImpl;
use crate::skia_sharp_extensions::{
    matrix44_to_matrix, to_sk_blend_mode, to_sk_color, to_sk_matrix, to_sk_matrix44, to_sk_point, to_sk_rect,
    to_sk_sampling_options_scaled, to_sk_shader_tile_mode, to_sk_stroke_cap, to_sk_stroke_join,
};
use crate::surface_render_target::{SurfaceRenderTarget, SurfaceRenderTargetCreateInfo};
use crate::picture_render_target::PictureRenderTarget;
use ferroui_base::media::effects::IEffect;
use ferroui_base::media::{
    AcrylicBackgroundSource, BoxShadow, BoxShadows, Color, Colors, EdgeMode, IBrush, IExperimentalAcrylicMaterial,
    IGradientBrush, IPen, ISceneBrushContent, ITileBrush, MediaExtensions, RenderOptions, StretchDirection,
    TextOptions, TextRenderingMode, TileMode,
};
use ferroui_base::platform::{
    IBitmapImpl, IDrawingContextImpl, IDrawingContextImplWithEffects, IDrawingContextLayerImpl,
    IDrawingContextWithAcrylicLikeSupport, IGeometryImpl, IGlyphRunImpl, IPlatformGraphicsContext,
    IPlatformRenderInterfaceRegion, PixelFormat,
};
use ferroui_base::rendering::utilities::TileBrushCalculator;
use ferroui_base::RelativeUnit;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{FerroLocator, LocatorExtensions, Matrix, PixelSize, Point, Rect, RoundedRect, Vector};
use skia_safe as sk;
use skia_safe::canvas::SaveLayerRec;
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::rc::Rc;

/// What a drawing context draws to.
pub enum CanvasSource {
    /// The canvas of a surface.
    Surface(sk::Surface),
    /// The canvas of a picture recorder. `on_finished` receives the recorded
    /// picture when the drawing context is disposed.
    Recorder { recorder: sk::PictureRecorder, on_finished: Box<dyn FnOnce(Option<sk::Picture>)> },
    /// The drawing context has been disposed.
    Finished,
}

impl CanvasSource {
    /// The canvas.
    ///
    /// # Panics
    /// Panics when the drawing context has been disposed.
    #[inline]
    fn get(&mut self) -> &sk::Canvas {
        match self {
            CanvasSource::Surface(surface) => surface.canvas(),
            CanvasSource::Recorder { recorder, .. } => {
                recorder.recording_canvas().unwrap_or_else(|| panic!("The picture recording has ended"))
            }
            CanvasSource::Finished => panic!("The drawing context has been disposed"),
        }
    }
}

/// Context create info.
#[derive(Default)]
pub struct CreateInfo {
    /// Canvas to draw to.
    pub canvas: Option<CanvasSource>,

    /// Surface to draw to.
    pub surface: Option<sk::Surface>,

    /// Makes DPI to be applied as a hidden matrix transform.
    pub scale_drawing_to_dpi: bool,

    /// Dpi for intermediate surfaces.
    pub dpi: Vector,

    /// Render text without subpixel antialiasing.
    pub disable_subpixel_text_rendering: bool,

    /// GPU-accelerated context (optional).
    pub gr_context: Option<Rc<dyn ISkiaGrContext>>,

    /// Skia GPU provider context (optional).
    pub gpu: Option<Rc<dyn ISkiaGpu>>,

    /// The render session the context draws in (optional).
    pub current_session: Option<Rc<dyn ISkiaGpuRenderSession>>,
}

fn leased_panic() -> ! {
    panic!("The underlying graphics API is currently leased")
}

struct SkiaLeaseFeature {
    context_leased: Rc<Cell<bool>>,
    surface: sk::Surface,
    gr_context: Option<Rc<dyn ISkiaGrContext>>,
    gpu: Option<Rc<dyn ISkiaGpu>>,
    current_opacity: f64,
}

impl ISkiaApiLeaseFeature for SkiaLeaseFeature {
    fn lease(&self) -> Rc<dyn ISkiaApiLease> {
        if self.context_leased.get() {
            leased_panic();
        }

        let revert_transform = self.surface.clone().canvas().local_to_device();
        self.context_leased.set(true);

        Rc::new(ApiLease {
            context_leased: self.context_leased.clone(),
            surface: self.surface.clone(),
            gr_context: self.gr_context.clone(),
            gpu: self.gpu.clone(),
            current_opacity: self.current_opacity,
            revert_transform,
            is_disposed: Cell::new(false),
            leased: Rc::new(Cell::new(false)),
        })
    }
}

struct ApiLease {
    context_leased: Rc<Cell<bool>>,
    surface: sk::Surface,
    gr_context: Option<Rc<dyn ISkiaGrContext>>,
    gpu: Option<Rc<dyn ISkiaGpu>>,
    current_opacity: f64,
    revert_transform: sk::M44,
    is_disposed: Cell<bool>,
    /// Whether the platform graphics API is leased.
    leased: Rc<Cell<bool>>,
}

impl ApiLease {
    fn check_lease(&self) {
        if self.leased.get() {
            leased_panic();
        }
    }
}

impl ISkiaApiLease for ApiLease {
    fn with_sk_canvas(&self, action: &mut dyn FnMut(&sk::Canvas)) {
        self.check_lease();
        action(self.surface.clone().canvas());
    }

    fn gr_context(&self) -> Option<Rc<dyn ISkiaGrContext>> {
        self.gr_context.clone()
    }

    fn sk_surface(&self) -> Option<sk::Surface> {
        self.check_lease();
        Some(self.surface.clone())
    }

    fn current_opacity(&self) -> f64 {
        self.check_lease();
        self.current_opacity
    }

    fn try_lease_platform_graphics_api(&self) -> Option<Rc<dyn ISkiaPlatformGraphicsApiLease>> {
        self.check_lease();

        let context = self.gpu.as_ref()?.platform_graphics_context()?;

        if let Some(gr_context) = &self.gr_context {
            gr_context.flush();
        }
        self.leased.set(true);

        Some(Rc::new(PlatformApiLease { leased: self.leased.clone(), gr_context: self.gr_context.clone(), context }))
    }

    fn dispose(&self) {
        if !self.is_disposed.get() {
            self.surface.clone().canvas().set_matrix(&self.revert_transform);
            self.context_leased.set(false);
            self.is_disposed.set(true);
        }
    }
}

struct PlatformApiLease {
    leased: Rc<Cell<bool>>,
    gr_context: Option<Rc<dyn ISkiaGrContext>>,
    context: Rc<dyn IPlatformGraphicsContext>,
}

impl ISkiaPlatformGraphicsApiLease for PlatformApiLease {
    fn context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        self.context.clone()
    }

    fn dispose(&self) {
        self.leased.set(false);
        if let Some(gr_context) = &self.gr_context {
            gr_context.reset_context();
        }
    }
}

/// Skia based drawing context.
pub struct DrawingContextImpl {
    disposables: Vec<Box<dyn FnOnce()>>,
    // TODO: Get rid of this value, it's currently used to calculate
    // intermediate sizes for tile brushes but does so ignoring the current
    // transform.
    intermediate_surface_dpi: Vector,
    mask_stack: Vec<(sk::M44, PaintWrapper)>,
    opacity_stack: Vec<f64>,
    render_options_stack: Vec<RenderOptions>,
    text_options_stack: Vec<TextOptions>,
    post_transform: Option<Matrix>,
    current_opacity: f64,
    disable_subpixel_text_rendering: bool,
    current_transform: Option<Matrix>,
    disposed: bool,
    gr_context: Option<Rc<dyn ISkiaGrContext>>,
    gpu: Option<Rc<dyn ISkiaGpu>>,
    stroke_paint: Option<sk::Paint>,
    fill_paint: Option<sk::Paint>,
    box_shadow_paint: Option<sk::Paint>,
    session: Option<Rc<dyn ISkiaGpuRenderSession>>,
    leased: Rc<Cell<bool>>,
    use_opacity_save_layer: bool,
    canvas: CanvasSource,
    surface: Option<sk::Surface>,
    render_options: RenderOptions,
    text_options: TextOptions,
}

impl DrawingContextImpl {
    /// Creates a new drawing context.
    ///
    /// `disposables` run in order after drawing has finished, when the
    /// context is disposed.
    ///
    /// # Panics
    /// Panics when the create info holds neither a canvas nor a surface.
    pub fn new(create_info: CreateInfo, disposables: Vec<Box<dyn FnOnce()>>) -> Self {
        let canvas = match (create_info.canvas, &create_info.surface) {
            (Some(canvas), _) => canvas,
            (None, Some(surface)) => CanvasSource::Surface(surface.clone()),
            (None, None) => panic!("Invalid create info - no Canvas provided"),
        };

        let default_dpi = SkiaPlatform::default_dpi();
        let post_transform = if create_info.scale_drawing_to_dpi && !create_info.dpi.nearly_equals(default_dpi) {
            Some(Matrix::create_scale(create_info.dpi.x / default_dpi.x, create_info.dpi.y / default_dpi.y))
        } else {
            None
        };

        // The options registered on this thread, else the ones the backend
        // was initialized with (the render thread has no services).
        let use_opacity_save_layer = FerroLocator::current()
            .get_service::<SkiaOptions>()
            .map(|options| *options)
            .or_else(SkiaPlatform::options)
            .is_some_and(|options| options.use_opacity_save_layer);

        let mut context = Self {
            disposables,
            intermediate_surface_dpi: create_info.dpi,
            mask_stack: Vec::new(),
            opacity_stack: Vec::new(),
            render_options_stack: Vec::new(),
            text_options_stack: Vec::new(),
            post_transform,
            current_opacity: 1.0,
            disable_subpixel_text_rendering: create_info.disable_subpixel_text_rendering,
            current_transform: None,
            disposed: false,
            gr_context: create_info.gr_context,
            gpu: create_info.gpu,
            stroke_paint: Some(SkPaintCache::get()),
            fill_paint: Some(SkPaintCache::get()),
            box_shadow_paint: Some(SkPaintCache::get()),
            session: create_info.current_session,
            leased: Rc::new(Cell::new(false)),
            use_opacity_save_layer,
            canvas,
            surface: create_info.surface,
            render_options: RenderOptions::default(),
            text_options: TextOptions::default(),
        };

        context.set_transform(Matrix::IDENTITY);

        context
    }

    /// The GPU context the drawing context draws with, if any.
    pub fn gr_context(&self) -> Option<&Rc<dyn ISkiaGrContext>> {
        self.gr_context.as_ref()
    }

    /// The surface the drawing context draws to, if it draws to one.
    pub fn surface(&self) -> Option<&sk::Surface> {
        self.surface.as_ref()
    }

    /// The Skia canvas.
    pub fn canvas(&mut self) -> &sk::Canvas {
        self.canvas.get()
    }

    /// The render options in effect.
    pub fn render_options(&self) -> RenderOptions {
        self.render_options
    }

    /// Replaces the render options in effect.
    pub fn set_render_options(&mut self, value: RenderOptions) {
        self.render_options = value;
    }

    fn check_lease(&self) {
        if self.leased.get() {
            leased_panic();
        }
    }

    /// The text options in effect.
    pub fn text_options(&self) -> TextOptions {
        self.text_options
    }

    /// Replaces the text options in effect.
    pub fn set_text_options(&mut self, value: TextOptions) {
        self.text_options = value;
    }

    fn sk_blur_radius_to_sigma(radius: f64) -> f32 {
        if radius <= 0.0 {
            return 0.0;
        }
        0.288675 * radius as f32 + 0.5
    }

    /// Configures the paint that draws a box shadow and returns it with the
    /// clip operation that keeps the shadow outside (or inside) the box.
    fn create_box_shadow_filter(mut paint: sk::Paint, shadow: &BoxShadow, opacity: f64) -> (sk::Paint, sk::ClipOp) {
        let ac = shadow.color;

        let sigma = Self::sk_blur_radius_to_sigma(shadow.blur);
        let filter = sk::image_filters::blur((sigma, sigma), None, None, None);
        let color = sk::Color::from_argb((ac.a as f64 * opacity) as u8, ac.r, ac.g, ac.b);

        paint.reset();
        paint.set_anti_alias(true);
        paint.set_color(color);
        paint.set_image_filter(filter);

        let clip_operation = if shadow.is_inset { sk::ClipOp::Intersect } else { sk::ClipOp::Difference };

        (paint, clip_operation)
    }

    fn area_casting_shadow_in_hole(
        hole_rect: sk::Rect,
        shadow_blur: f32,
        shadow_spread: f32,
        offset_x: f32,
        offset_y: f32,
    ) -> sk::Rect {
        // Adapted from Chromium.
        let mut bounds = hole_rect;

        bounds = bounds.with_outset((shadow_blur, shadow_blur));

        if shadow_spread < 0.0 {
            bounds = bounds.with_outset((-shadow_spread, -shadow_spread));
        }

        let offset_bounds = bounds.with_offset((-offset_x, -offset_y));
        sk::Rect::new(
            bounds.left.min(offset_bounds.left),
            bounds.top.min(offset_bounds.top),
            bounds.right.max(offset_bounds.right),
            bounds.bottom.max(offset_bounds.bottom),
        )
    }

    /// Restores the canvas state. The transform of the context is whatever
    /// the canvas has afterwards.
    fn restore_canvas(&mut self) {
        let canvas = self.canvas.get();
        canvas.restore();
        self.current_transform = Some(matrix44_to_matrix(&canvas.local_to_device()));
    }

    /// The relative transform of a brush conjugated into target space: the
    /// unit square maps onto `target_rect`, the matrix acts inside that
    /// space, before the absolute brush transform.
    fn get_relative_transform(brush: &dyn IBrush, target_rect: Rect) -> Option<Matrix> {
        let relative_transform = brush.relative_transform()?;
        if target_rect.width <= 0.0 || target_rect.height <= 0.0 {
            return None;
        }

        let matrix = relative_transform.value();
        if matrix.is_identity() {
            return None;
        }

        Some(
            Matrix::create_translation(-target_rect.x, -target_rect.y)
                * Matrix::create_scale(1.0 / target_rect.width, 1.0 / target_rect.height)
                * matrix
                * Matrix::create_scale(target_rect.width, target_rect.height)
                * Matrix::create_translation(target_rect.x, target_rect.y),
        )
    }

    /// The absolute transform of a brush around its transform origin.
    fn get_absolute_transform(brush: &dyn IBrush, target_rect: Rect) -> Option<Matrix> {
        let absolute_transform = brush.transform()?;
        let transform_origin = brush.transform_origin().to_pixels_rect(target_rect);
        let offset = Matrix::create_translation(transform_origin.x, transform_origin.y);
        Some((-offset) * absolute_transform.value() * offset)
    }

    fn combine(first: Option<Matrix>, second: Option<Matrix>) -> Option<Matrix> {
        match (first, second) {
            (Some(first), Some(second)) => Some(first * second),
            (first, second) => first.or(second),
        }
    }

    fn gradient<'g>(colors: &'g [sk::Color4f], offsets: &'g [f32], tile_mode: sk::TileMode) -> sk::gradient::Gradient<'g> {
        sk::gradient::Gradient::new(
            sk::gradient::Colors::new(colors, Some(offsets), tile_mode, None),
            sk::gradient::Interpolation::default(),
        )
    }

    /// Configures the paint for using a gradient brush.
    fn configure_gradient_brush(paint: &mut sk::Paint, target_rect: Rect, gradient_brush: &dyn IGradientBrush) {
        let tile_mode = to_sk_shader_tile_mode(gradient_brush.spread_method());
        let gradient_stops = gradient_brush.gradient_stops();
        let mut stop_colors: Vec<sk::Color4f> =
            gradient_stops.iter().map(|s| sk::Color4f::from(to_sk_color(s.color()))).collect();
        let mut stop_offsets: Vec<f32> = gradient_stops.iter().map(|s| s.offset() as f32).collect();

        if stop_colors.is_empty() {
            // Skia cannot build a gradient without stops.
            paint.set_shader(None);
            return;
        }

        if let Some(linear_gradient) = gradient_brush.as_linear_gradient_brush() {
            let start = to_sk_point(linear_gradient.start_point().to_pixels_rect(target_rect));
            let end = to_sk_point(linear_gradient.end_point().to_pixels_rect(target_rect));

            let transform = Self::combine(
                Self::get_relative_transform(gradient_brush, target_rect),
                Self::get_absolute_transform(gradient_brush, target_rect),
            )
            .map(to_sk_matrix);

            // Would be nice to cache these shaders possibly?
            let shader = sk::gradient::shaders::linear_gradient(
                (start, end),
                &Self::gradient(&stop_colors, &stop_offsets, tile_mode),
                transform.as_ref(),
            );
            paint.set_shader(shader);
        } else if let Some(radial_gradient) = gradient_brush.as_radial_gradient_brush() {
            let center_point = radial_gradient.center().to_pixels_rect(target_rect);
            let center = to_sk_point(center_point);

            let radius_x = radial_gradient.radius_x().to_value(target_rect.width);
            let radius_y = radial_gradient.radius_y().to_value(target_rect.height);

            let mut origin_point = radial_gradient.gradient_origin().to_pixels_rect(target_rect);

            let mut transform: Option<Matrix> = None;

            if radius_x != radius_y {
                transform = Some(
                    Matrix::create_translation(-center_point.x, -center_point.y)
                        * Matrix::create_scale(1.0, radius_y / radius_x)
                        * Matrix::create_translation(center_point.x, center_point.y),
                );
            }

            transform = Self::combine(transform, Self::get_relative_transform(gradient_brush, target_rect));
            transform = Self::combine(transform, Self::get_absolute_transform(gradient_brush, target_rect));

            let transform = transform.map(to_sk_matrix);

            if origin_point == center_point {
                // When the origin is the same as the center the Skia radial
                // gradient acts the same as D2D.
                let shader = sk::gradient::shaders::radial_gradient(
                    (center, radius_x as f32),
                    &Self::gradient(&stop_colors, &stop_offsets, tile_mode),
                    transform.as_ref(),
                );
                paint.set_shader(shader);
            } else {
                // When the origin is different to the center use a two point
                // conical gradient to match the behaviour of D2D.
                if radius_x != radius_y {
                    // Adjust the origin point for the radius x/y
                    // transformation by reversing it.
                    origin_point = origin_point
                        .with_y((origin_point.y - center_point.y) * radius_x / radius_y + center_point.y);
                }

                let origin = to_sk_point(origin_point);

                let end_offset = stop_offsets[stop_offsets.len() - 1];

                let mut start = origin;
                let mut radius_start = 0f32;
                let mut end = center;
                let mut radius_end = radius_x as f32;
                let reverse =
                    (center_point.x != origin_point.x || center_point.y != origin_point.y) && end_offset == 1.0;

                if reverse {
                    // Reverse the order of the stops to match D2D.
                    (start, radius_start, end, radius_end) = (end, radius_end, start, radius_start);

                    let count = stop_offsets.len();

                    let mut reversed_colors = vec![sk::Color4f::new(0.0, 0.0, 0.0, 0.0); count];
                    // And then reverse the reference point of the stops.
                    let mut reversed_stops = vec![0f32; count];

                    for i in 0..count {
                        let mut offset = 1.0 - gradient_stops[i].offset();

                        if MathUtilities::is_zero(offset) {
                            offset = 0.0;
                        }

                        let reversed_index = count - 1 - i;

                        reversed_stops[reversed_index] = offset as f32;
                        reversed_colors[reversed_index] = stop_colors[i];
                    }

                    stop_colors = reversed_colors;
                    stop_offsets = reversed_stops;
                }

                // Compose with a background colour of the final stop to match
                // D2D's behaviour of filling with the final color.
                let conical = sk::gradient::shaders::two_point_conical_gradient(
                    (start, radius_start),
                    (end, radius_end),
                    &Self::gradient(&stop_colors, &stop_offsets, tile_mode),
                    transform.as_ref(),
                );

                let background = sk::shaders::color(stop_colors[0].to_color());
                let shader =
                    conical.map(|conical| sk::shaders::blend(sk::BlendMode::SrcOver, background, conical));
                paint.set_shader(shader);
            }
        } else if let Some(conic_gradient) = gradient_brush.as_conic_gradient_brush() {
            let center = to_sk_point(conic_gradient.center().to_pixels_rect(target_rect));

            // Skia's default is that angle 0 is from the right hand side of
            // the center point but we are matching CSS where the vertical
            // point above the center is 0.
            let angle = (conic_gradient.angle() - 90.0) as f32;
            let mut rotation = sk::Matrix::rotate_deg_pivot(angle, center);

            let transform = Self::combine(
                Self::get_relative_transform(gradient_brush, target_rect),
                Self::get_absolute_transform(gradient_brush, target_rect),
            );

            if let Some(transform) = transform {
                rotation.post_concat(&to_sk_matrix(transform));
            }

            let shader = sk::gradient::shaders::sweep_gradient(
                center,
                (0.0, 360.0),
                &Self::gradient(&stop_colors, &stop_offsets, sk::TileMode::Clamp),
                &rotation,
            );
            paint.set_shader(shader);
        }
    }

    /// The Skia tile modes of a tile brush in x and y.
    fn get_tile_modes(mode: TileMode) -> (sk::TileMode, sk::TileMode) {
        (
            if mode == TileMode::None {
                sk::TileMode::Decal
            } else if mode == TileMode::FlipX || mode == TileMode::FlipXY {
                sk::TileMode::Mirror
            } else {
                sk::TileMode::Repeat
            },
            if mode == TileMode::None {
                sk::TileMode::Decal
            } else if mode == TileMode::FlipY || mode == TileMode::FlipXY {
                sk::TileMode::Mirror
            } else {
                sk::TileMode::Repeat
            },
        )
    }

    /// Configures the paint wrapper for using a tile brush.
    ///
    /// The tile is rendered into an intermediate surface that is turned
    /// into a shader; the wrapper keeps the surface alive.
    fn configure_tile_brush(
        &mut self,
        paint_wrapper: &mut PaintWrapper,
        target_box: Rect,
        tile_brush: &dyn ITileBrush,
        tile_brush_image: &dyn IBitmapImpl,
    ) {
        let dpi = self.intermediate_surface_dpi;
        let image_pixel_size = tile_brush_image.pixel_size();

        let calc =
            TileBrushCalculator::from_brush(tile_brush, image_pixel_size.to_size_with_dpi_vector(dpi), target_box.size());
        let intermediate = Rc::new(self.create_render_target(
            PixelSize::from_size_with_dpi_vector(calc.intermediate_size(), dpi),
            false,
            true,
            None,
        ));

        {
            let disposable = intermediate.clone();
            paint_wrapper.add_disposable(Box::new(move || IBitmapImpl::dispose(&*disposable)));
        }

        {
            let mut context = intermediate.create_drawing_context();

            let source_rect = Rect::from_size(image_pixel_size.to_size_with_dpi(96.0));
            let target_rect = Rect::from_size(image_pixel_size.to_size_with_dpi_vector(dpi));

            context.clear(Colors::TRANSPARENT);
            context.push_clip(calc.intermediate_clip());
            context.push_render_options(self.render_options);

            context.set_transform(calc.intermediate_transform());

            context.draw_bitmap(tile_brush_image, 1.0, source_rect, target_rect);

            context.pop_render_options();
            context.pop_clip();
            context.dispose();
        }

        let tile_mode = tile_brush.tile_mode();

        let tile_transform = if tile_mode != TileMode::None {
            sk::Matrix::translate((-(calc.destination_rect().x as f32), -(calc.destination_rect().y as f32)))
        } else {
            sk::Matrix::new_identity()
        };

        let (tile_x, tile_y) = Self::get_tile_modes(tile_mode);

        let image = crate::gpu::drawable_image(self.gr_context.as_deref(), intermediate.snapshot_image(), false);

        let mut paint_transform =
            sk::Matrix::concat(&tile_transform, &sk::Matrix::scale(((96.0 / dpi.x) as f32, (96.0 / dpi.y) as f32)));

        if tile_brush.destination_rect().unit == RelativeUnit::Relative {
            paint_transform.pre_concat(&sk::Matrix::translate((target_box.x as f32, target_box.y as f32)));
        }

        // Both brush transforms act on the tile once it sits in target
        // space, the relative one first.
        if let Some(relative_transform) = Self::get_relative_transform(tile_brush, target_box) {
            paint_transform.post_concat(&to_sk_matrix(relative_transform));
        }

        if let Some(transform) = Self::get_absolute_transform(tile_brush, target_box) {
            paint_transform.post_concat(&to_sk_matrix(transform));
        }

        let shader = image.to_shader((tile_x, tile_y), sk::SamplingOptions::default(), &paint_transform);
        paint_wrapper.paint.set_shader(shader);
    }

    fn configure_scene_brush_content(
        &mut self,
        paint_wrapper: &mut PaintWrapper,
        content: &dyn ISceneBrushContent,
        target_rect: Rect,
    ) {
        if content.use_scalable_rasterization() {
            self.configure_scene_brush_content_with_picture(paint_wrapper, content, target_rect);
        } else {
            self.configure_scene_brush_content_with_surface(paint_wrapper, content, target_rect);
        }
    }

    fn configure_scene_brush_content_with_surface(
        &mut self,
        paint_wrapper: &mut PaintWrapper,
        content: &dyn ISceneBrushContent,
        target_rect: Rect,
    ) {
        let rect = content.rect();
        let intermediate_size = rect.size();

        if intermediate_size.width >= 1.0 && intermediate_size.height >= 1.0 {
            let intermediate = self.create_render_target(
                PixelSize::from_size_with_dpi_vector(intermediate_size, self.intermediate_surface_dpi),
                false,
                true,
                None,
            );

            {
                let mut ctx = intermediate.create_drawing_context();
                ctx.push_render_options(self.render_options);
                ctx.clear(Colors::TRANSPARENT);
                let transform = if rect.top_left() == Point::default() {
                    None
                } else {
                    Some(Matrix::create_translation(-rect.x, -rect.y))
                };
                content.render(&mut *ctx, transform);
                ctx.pop_render_options();
                ctx.dispose();
            }

            self.configure_tile_brush(paint_wrapper, target_rect, &*content.brush(), &intermediate);
            IBitmapImpl::dispose(&intermediate);
        }
    }

    fn configure_scene_brush_content_with_picture(
        &mut self,
        paint_wrapper: &mut PaintWrapper,
        content: &dyn ISceneBrushContent,
        target_rect: Rect,
    ) {
        // Brushes ignore whatever layout bounds visuals have and use the
        // content bounds instead: the source rect (view box) is relative to
        // the content bounds, not to the visual or drawing.
        let brush = content.brush();
        let content_rect = content.rect();
        let source_rect = brush.source_rect().to_pixels_rect(content_rect);

        // Early escape.
        if content_rect.width <= 0.0
            || content_rect.height <= 0.0
            || source_rect.width <= 0.0
            || source_rect.height <= 0.0
        {
            paint_wrapper.paint.set_color(sk::Color::from_argb(0, 0, 0, 0));
            return;
        }

        // We are moving the render area to make the top-left corner of the
        // source rect (view box) to be at (0,0) of the tile.
        let mut content_render_transform = Matrix::create_translation(-source_rect.x, -source_rect.y);

        // The destination rect (viewport) is specified relative to the
        // target rect.
        let destination_rect = brush.destination_rect().to_pixels_rect(target_rect);

        // The tile size matches the destination rect size.
        let tile_size = destination_rect.size();

        // Apply transforms to stretch the content to match the tile.
        if source_rect.size() != tile_size {
            // Stretch the content rect to match the tile size.
            let scale = MediaExtensions::calculate_scaling(
                brush.stretch(),
                tile_size,
                source_rect.size(),
                StretchDirection::Both,
            );

            // And move the resulting rect according to the alignment rules.
            let alignment_translate = TileBrushCalculator::calculate_translate_sizes(
                brush.alignment_x(),
                brush.alignment_y(),
                source_rect.size() * scale,
                tile_size,
            );

            content_render_transform = content_render_transform
                * Matrix::create_scale_vector(scale)
                * Matrix::create_translation_vector(alignment_translate);
        }

        // Pre-rasterize the tile into a picture.
        let picture_target =
            PictureRenderTarget::new(self.gpu.clone(), self.gr_context.clone(), self.intermediate_surface_dpi);
        {
            let mut ctx = picture_target.create_drawing_context(tile_size, false);
            ctx.push_render_options(self.render_options);
            content.render(&mut ctx, Some(content_render_transform));
            ctx.pop_render_options();
            ctx.dispose();
        }
        let tile = picture_target.get_picture();

        // If there is no brush transform and the destination rect is at
        // (0,0) we don't need any transforms.
        let mut shader_transform = Matrix::IDENTITY;

        // Apply the destination rect position.
        if destination_rect.position() != Point::default() {
            shader_transform = Matrix::create_translation(destination_rect.x, destination_rect.y);
        }

        // Apply the relative and the absolute brush transform, in that
        // order.
        if let Some(relative_transform) = Self::get_relative_transform(content, target_rect) {
            shader_transform *= relative_transform;
        }

        if let Some(transform) = Self::get_absolute_transform(content, target_rect) {
            shader_transform *= transform;
        }

        // Create the shader.
        let (tile_x, tile_y) = Self::get_tile_modes(brush.tile_mode());
        let cull_rect = tile.cull_rect();
        let shader = tile.to_shader(
            (tile_x, tile_y),
            sk::FilterMode::Nearest,
            &to_sk_matrix(shader_transform),
            &sk::Rect::new(0.0, 0.0, cull_rect.width(), cull_rect.height()),
        );
        paint_wrapper.paint.set_shader(shader);
    }

    fn create_alpha_color_filter(opacity: f64) -> sk::ColorFilter {
        let opacity = opacity.min(1.0);

        let mut c = [0u8; 256];
        let mut a = [0u8; 256];
        for i in 0..256 {
            c[i] = i as u8;
            a[i] = (i as f64 * opacity) as u8;
        }

        sk::color_filters::table_argb(Some(&a), Some(&c), Some(&c), Some(&c))
            .unwrap_or_else(|| panic!("Unable to create the alpha color filter"))
    }

    /// A repeating, nearly transparent shader over the noise texture of
    /// acrylic materials.
    fn create_acrylic_noise_shader(image: &sk::Image) -> Option<sk::Shader> {
        const NOISE_OPACITY: f64 = 0.0225;

        let shader =
            image.to_shader((sk::TileMode::Repeat, sk::TileMode::Repeat), sk::SamplingOptions::default(), None)?;
        Some(shader.with_color_filter(Self::create_alpha_color_filter(NOISE_OPACITY)))
    }

    /// The noise texture of acrylic materials as a repeating, nearly
    /// transparent shader.
    fn acrylic_noise_shader(&self) -> Option<sk::Shader> {
        thread_local! {
            static ACRYLIC_NOISE: std::cell::OnceCell<Option<(sk::Image, sk::Shader)>> =
                const { std::cell::OnceCell::new() };
        }

        static NOISE_ASSET: &[u8] = include_bytes!("assets/noise_asset_256x256_png.png");

        ACRYLIC_NOISE.with(|noise| {
            let noise = noise.get_or_init(|| {
                let image = sk::Image::from_encoded(sk::Data::new_copy(NOISE_ASSET))?;
                let shader = Self::create_acrylic_noise_shader(&image)?;
                Some((image, shader))
            });
            let (image, shader) = noise.as_ref()?;

            // The cached shader samples the image itself. A GPU context that
            // draws another form of the image gets a shader over that form.
            match &self.gr_context {
                Some(gr_context) => {
                    let drawable = gr_context.drawable_image(image, false);
                    if drawable.unique_id() == image.unique_id() {
                        Some(shader.clone())
                    } else {
                        Self::create_acrylic_noise_shader(&drawable)
                    }
                }
                None => Some(shader.clone()),
            }
        })
    }

    /// Creates a paint wrapper for an acrylic material.
    fn create_acrylic_paint(&self, mut paint: sk::Paint, material: &dyn IExperimentalAcrylicMaterial) -> PaintWrapper {
        paint.set_anti_alias(true);

        let tint_color = material.tint_color();
        let tint = sk::Color::from_argb(tint_color.a, tint_color.r, tint_color.g, tint_color.b);

        let material_color = material.material_color();
        let backdrop = sk::shaders::color(sk::Color::from_argb(
            material_color.a,
            material_color.r,
            material_color.g,
            material_color.b,
        ));
        let tint_shader = sk::shaders::color(tint);
        let effective_tint = sk::shaders::blend(sk::BlendMode::SrcOver, backdrop, tint_shader);
        let compose = match self.acrylic_noise_shader() {
            Some(noise) => sk::shaders::blend(sk::BlendMode::SrcOver, effective_tint, noise),
            None => effective_tint,
        };

        paint.set_shader(compose);

        if material.background_source() == AcrylicBackgroundSource::Digger {
            paint.set_blend_mode(sk::BlendMode::Src);
        }

        PaintWrapper::new(paint)
    }

    fn create_effect(&self, effect: &dyn IEffect) -> Option<sk::ImageFilter> {
        if let Some(blur) = effect.as_blur_effect() {
            if blur.radius() <= 0.0 {
                return None;
            }
            let sigma = Self::sk_blur_radius_to_sigma(blur.radius());
            return sk::image_filters::blur((sigma, sigma), None, None, None);
        }

        if let Some(drop) = effect.as_drop_shadow_effect() {
            let sigma = if drop.blur_radius() > 0.0 { Self::sk_blur_radius_to_sigma(drop.blur_radius()) } else { 0.0 };
            let drop_color = drop.color();
            let mut alpha = drop_color.a as f64 * drop.opacity();
            if !self.use_opacity_save_layer {
                alpha *= self.current_opacity;
            }
            let color = sk::Color::from_argb(alpha.clamp(0.0, 255.0) as u8, drop_color.r, drop_color.g, drop_color.b);

            return sk::image_filters::drop_shadow(
                (drop.offset_x() as f32, drop.offset_y() as f32),
                (sigma, sigma),
                color,
                None,
                None,
                None,
            );
        }

        None
    }

    /// Creates a paint wrapper for the given brush.
    ///
    /// `paint` is a reset paint; it travels in the wrapper and comes back
    /// from [`PaintWrapper::dispose`].
    fn create_paint(&mut self, mut paint: sk::Paint, brush: &dyn IBrush, target_rect: Rect) -> PaintWrapper {
        paint.set_anti_alias(self.render_options.edge_mode != EdgeMode::Aliased);

        let opacity = brush.opacity() * if self.use_opacity_save_layer { 1.0 } else { self.current_opacity };

        if let Some(solid) = brush.as_solid_color_brush() {
            let color = solid.color();
            paint.set_color(sk::Color::from_argb((color.a as f64 * opacity) as u8, color.r, color.g, color.b));

            return PaintWrapper::new(paint);
        }

        paint.set_color(sk::Color::from_argb((255.0 * opacity) as u8, 255, 255, 255));

        if let Some(gradient) = brush.as_gradient_brush() {
            Self::configure_gradient_brush(&mut paint, target_rect, gradient);

            return PaintWrapper::new(paint);
        }

        let mut paint_wrapper = PaintWrapper::new(paint);

        let tile_brush = brush.as_tile_brush();
        let mut tile_brush_image: Option<std::sync::Arc<ferroui_base::platform::SharedBitmapImpl>> = None;

        if let Some(scene_brush) = brush.as_scene_brush() {
            match scene_brush.create_content() {
                Some(content) => {
                    self.configure_scene_brush_content(&mut paint_wrapper, &*content, target_rect);
                    content.dispose();
                    return paint_wrapper;
                }
                None => {
                    paint_wrapper.paint.set_color(sk::Color::from_argb(0, 0, 0, 0));
                }
            }
        } else if let Some(image_brush) = brush.as_image_brush() {
            tile_brush_image = image_brush
                .source()
                .and_then(|source| source.get_bitmap())
                .filter(|bitmap| try_get_drawable_bitmap(&**bitmap).is_some());
        }

        match (tile_brush, tile_brush_image) {
            (Some(tile_brush), Some(tile_brush_image)) => {
                self.configure_tile_brush(&mut paint_wrapper, target_rect, tile_brush, &*tile_brush_image);
            }
            _ => {
                paint_wrapper.paint.set_color(sk::Color::from_argb(0, 255, 255, 255));
            }
        }

        paint_wrapper
    }

    /// Creates a paint wrapper for the given pen, or hands the paint back
    /// when the pen draws nothing.
    fn try_create_paint(
        &mut self,
        paint: sk::Paint,
        pen: &dyn IPen,
        target_rect: Rect,
    ) -> Result<PaintWrapper, sk::Paint> {
        // In Skia 0 thickness means - use hairline rendering - and for us it
        // means - there is nothing rendered.
        let Some(brush) = pen.brush() else {
            return Err(paint);
        };
        if pen.thickness() == 0.0 {
            return Err(paint);
        }

        let mut rv = self.create_paint(paint, &*brush, target_rect);
        let paint = &mut rv.paint;

        paint.set_stroke(true);
        paint.set_stroke_width(pen.thickness() as f32);

        // Need to modify dashes due to Skia modifying their lengths.
        // TODO: Still something is off, dashes are now present, but don't
        // look the same as D2D ones.

        paint.set_stroke_cap(to_sk_stroke_cap(pen.line_cap()));
        paint.set_stroke_join(to_sk_stroke_join(pen.line_join()));

        paint.set_stroke_miter(pen.miter_limit() as f32);

        if let Some(dash_effect) = drawing_context_helper::try_create_dash_effect(Some(pen)) {
            paint.set_path_effect(dash_effect);
        }

        Ok(rv)
    }

    fn create_fill(&mut self, brush: &dyn IBrush, target_rect: Rect) -> PaintWrapper {
        let paint = self.fill_paint.take().unwrap_or_default();
        self.create_paint(paint, brush, target_rect)
    }

    fn release_fill(&mut self, fill: PaintWrapper) {
        self.fill_paint = Some(fill.dispose());
    }

    fn create_stroke(&mut self, pen: &dyn IPen, target_rect: Rect) -> Option<PaintWrapper> {
        let paint = self.stroke_paint.take().unwrap_or_default();
        match self.try_create_paint(paint, pen, target_rect) {
            Ok(stroke) => Some(stroke),
            Err(paint) => {
                self.stroke_paint = Some(paint);
                None
            }
        }
    }

    fn release_stroke(&mut self, stroke: PaintWrapper) {
        self.stroke_paint = Some(stroke.dispose());
    }

    /// Creates a new render target compatible with this drawing context.
    ///
    /// `is_layer` tells whether the render target is being created for a
    /// layer; `use_scaled_drawing` auto-scales the drawing to the DPI.
    fn create_render_target(
        &self,
        pixel_size: PixelSize,
        is_layer: bool,
        use_scaled_drawing: bool,
        format: Option<PixelFormat>,
    ) -> SurfaceRenderTarget {
        let create_info = SurfaceRenderTargetCreateInfo {
            width: pixel_size.width,
            height: pixel_size.height,
            dpi: self.intermediate_surface_dpi,
            format,
            disable_text_lcd_rendering: if is_layer { self.disable_subpixel_text_rendering } else { true },
            gr_context: self.gr_context.clone(),
            gpu: self.gpu.clone(),
            session: self.session.clone(),
            disable_manual_fbo: !is_layer,
            use_scaled_drawing,
        };

        SurfaceRenderTarget::new(create_info)
    }

    fn geometry_impl(geometry: &dyn IGeometryImpl) -> &dyn GeometryImpl {
        try_get_geometry_impl(geometry)
            .unwrap_or_else(|| panic!("The geometry was not created by the Skia backend"))
    }

    fn region_impl(region: &dyn IPlatformRenderInterfaceRegion) -> &SkiaRegionImpl {
        region
            .as_any()
            .downcast_ref::<SkiaRegionImpl>()
            .unwrap_or_else(|| panic!("The region was not created by the Skia backend"))
    }
}

impl IDrawingContextImpl for DrawingContextImpl {
    fn transform(&self) -> Matrix {
        // The transform is set when the context is created and re-read from
        // the canvas after every restore, so the cached value is always
        // present here.
        self.current_transform.unwrap_or(Matrix::IDENTITY)
    }

    fn set_transform(&mut self, value: Matrix) {
        self.check_lease();
        if self.current_transform == Some(value) {
            return;
        }

        self.current_transform = Some(value);

        let mut transform = value;

        if let Some(post_transform) = self.post_transform {
            transform *= post_transform;
        }

        // The canvas internally uses a 4x4 matrix; convert directly.
        self.canvas.get().set_matrix(&to_sk_matrix44(transform));
    }

    fn clear(&mut self, color: Color) {
        self.check_lease();
        self.canvas.get().clear(to_sk_color(color));
    }

    fn draw_bitmap(&mut self, source: &dyn IBitmapImpl, opacity: f64, source_rect: Rect, dest_rect: Rect) {
        self.check_lease();
        let drawable_image = try_get_drawable_bitmap(source)
            .unwrap_or_else(|| panic!("The bitmap was not created by the Skia backend"));
        let s = to_sk_rect(source_rect);
        let d = to_sk_rect(dest_rect);
        let is_upscaling = d.width() > s.width() || d.height() > s.height();

        let mut paint = SkPaintCache::get();
        let sampling_options =
            to_sk_sampling_options_scaled(self.render_options.bitmap_interpolation_mode, is_upscaling);

        paint.set_color(sk::Color::from_argb((255.0 * opacity * self.current_opacity) as u8, 255, 255, 255));
        paint.set_blend_mode(to_sk_blend_mode(self.render_options.bitmap_blending_mode));
        paint.set_anti_alias(self.render_options.edge_mode != EdgeMode::Aliased);

        drawable_image.draw(self.gr_context.as_deref(), self.canvas.get(), &s, &d, sampling_options, &paint);
        SkPaintCache::return_reset(paint);
    }

    fn draw_bitmap_with_mask(
        &mut self,
        source: &dyn IBitmapImpl,
        opacity_mask: &dyn IBrush,
        opacity_mask_rect: Rect,
        dest_rect: Rect,
    ) {
        self.check_lease();
        self.push_opacity_mask(opacity_mask, opacity_mask_rect);
        let pixel_size = source.pixel_size();
        self.draw_bitmap(
            source,
            1.0,
            Rect::new(0.0, 0.0, pixel_size.width as f64, pixel_size.height as f64),
            dest_rect,
        );
        self.pop_opacity_mask();
    }

    fn draw_line(&mut self, pen: Option<&dyn IPen>, p1: Point, p2: Point) {
        self.check_lease();

        let Some(pen) = pen else {
            return;
        };

        if let Some(stroke) = self.create_stroke(pen, Rect::from_points(p1, p2).normalize()) {
            self.canvas.get().draw_line(to_sk_point(p1), to_sk_point(p2), &stroke.paint);
            self.release_stroke(stroke);
        }
    }

    fn draw_geometry(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, geometry: &dyn IGeometryImpl) {
        self.check_lease();
        let geometry_impl = Self::geometry_impl(geometry);
        let rect = geometry.bounds();

        if let (Some(brush), Some(fill_path)) = (brush, geometry_impl.fill_path()) {
            let fill = self.create_fill(brush, rect);
            self.canvas.get().draw_path(&fill_path, &fill.paint);
            self.release_fill(fill);
        }

        if let (Some(pen), Some(stroke_path)) = (pen, geometry_impl.stroke_path()) {
            if let Some(stroke) = self.create_stroke(pen, rect.inflate(pen.thickness() / 2.0)) {
                self.canvas.get().draw_path(&stroke_path, &stroke.paint);
                self.release_stroke(stroke);
            }
        }
    }

    fn draw_rectangle(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        rect: RoundedRect,
        box_shadows: &BoxShadows,
    ) {
        if rect.rect.height <= 0.0 || rect.rect.width <= 0.0 {
            return;
        }
        self.check_lease();

        // Arbitrary chosen values.
        // On macOS Skia breaks the OpenGL context when asked to draw, e.g. a
        // (0, 0, 623, 6666600) rect.
        let no_box_shadows = BoxShadows::default();
        let box_shadows =
            if rect.rect.height > 8192.0 || rect.rect.width > 8192.0 { &no_box_shadows } else { box_shadows };

        let rc = to_sk_rect(rect.rect);
        let is_rounded = rect.is_rounded();
        let need_round_rect = rect.is_rounded() || box_shadows.has_inset_shadows();
        let sk_round_rect = if need_round_rect { Some(SkRoundRectCache::get_and_set_radii(&rc, &rect)) } else { None };

        for box_shadow in box_shadows.iter() {
            if box_shadow != BoxShadow::default() && !box_shadow.is_inset {
                let paint = self.box_shadow_paint.take().unwrap_or_default();
                let (shadow_paint, clip_operation) =
                    Self::create_box_shadow_filter(paint, &box_shadow, self.current_opacity);

                let mut spread = box_shadow.spread as f32;
                if box_shadow.is_inset {
                    spread = -spread;
                }

                self.canvas.get().save();
                if let (true, Some(sk_round_rect)) = (is_rounded, &sk_round_rect) {
                    let mut shadow_rect =
                        SkRoundRectCache::get_and_set_radii_points(sk_round_rect.rect(), sk_round_rect.radii_ref());
                    if spread != 0.0 {
                        shadow_rect.outset((spread, spread));
                    }
                    self.canvas.get().clip_rrect(sk_round_rect, clip_operation, true);

                    let old_transform = self.transform();
                    self.set_transform(
                        old_transform * Matrix::create_translation(box_shadow.offset_x, box_shadow.offset_y),
                    );
                    self.canvas.get().draw_rrect(shadow_rect, &shadow_paint);
                    self.set_transform(old_transform);
                    SkRoundRectCache::return_item(shadow_rect);
                } else {
                    let mut shadow_rect = rc;
                    if spread != 0.0 {
                        shadow_rect = shadow_rect.with_outset((spread, spread));
                    }
                    self.canvas.get().clip_rect(rc, clip_operation, None);
                    let old_transform = self.transform();
                    self.set_transform(
                        old_transform * Matrix::create_translation(box_shadow.offset_x, box_shadow.offset_y),
                    );
                    self.canvas.get().draw_rect(shadow_rect, &shadow_paint);
                    self.set_transform(old_transform);
                }

                self.restore_canvas();
                self.box_shadow_paint = Some(reset_paint(shadow_paint));
            }
        }

        if let Some(brush) = brush {
            let fill = self.create_fill(brush, rect.rect);
            match (is_rounded, &sk_round_rect) {
                (true, Some(sk_round_rect)) => self.canvas.get().draw_rrect(sk_round_rect, &fill.paint),
                _ => self.canvas.get().draw_rect(rc, &fill.paint),
            };
            self.release_fill(fill);
        }

        for box_shadow in box_shadows.iter() {
            if box_shadow != BoxShadow::default() && box_shadow.is_inset {
                let Some(sk_round_rect) = &sk_round_rect else {
                    continue;
                };

                let paint = self.box_shadow_paint.take().unwrap_or_default();
                let (shadow_paint, clip_operation) =
                    Self::create_box_shadow_filter(paint, &box_shadow, self.current_opacity);

                let spread = box_shadow.spread as f32;
                let offset_x = box_shadow.offset_x as f32;
                let offset_y = box_shadow.offset_y as f32;
                let outer_rect =
                    Self::area_casting_shadow_in_hole(rc, box_shadow.blur as f32, spread, offset_x, offset_y);

                self.canvas.get().save();
                let mut shadow_rect =
                    SkRoundRectCache::get_and_set_radii_points(sk_round_rect.rect(), sk_round_rect.radii_ref());
                if spread != 0.0 {
                    shadow_rect.inset((spread, spread));
                }
                self.canvas.get().clip_rrect(sk_round_rect, clip_operation, true);

                let old_transform = self.transform();
                self.set_transform(
                    old_transform * Matrix::create_translation(box_shadow.offset_x, box_shadow.offset_y),
                );
                let outer_rrect = sk::RRect::new_rect(outer_rect);
                self.canvas.get().draw_drrect(outer_rrect, shadow_rect, &shadow_paint);
                self.set_transform(old_transform);
                self.restore_canvas();
                SkRoundRectCache::return_item(shadow_rect);
                self.box_shadow_paint = Some(reset_paint(shadow_paint));
            }
        }

        if let Some(pen) = pen {
            if let Some(stroke) = self.create_stroke(pen, rect.rect.inflate(pen.thickness() / 2.0)) {
                match (is_rounded, &sk_round_rect) {
                    (true, Some(sk_round_rect)) => self.canvas.get().draw_rrect(sk_round_rect, &stroke.paint),
                    _ => self.canvas.get().draw_rect(rc, &stroke.paint),
                };
                self.release_stroke(stroke);
            }
        }

        if let Some(sk_round_rect) = sk_round_rect {
            SkRoundRectCache::return_item(sk_round_rect);
        }
    }

    fn draw_region(
        &mut self,
        brush: Option<&dyn IBrush>,
        pen: Option<&dyn IPen>,
        region: &dyn IPlatformRenderInterfaceRegion,
    ) {
        let r = Self::region_impl(region);
        if r.is_empty() {
            return;
        }
        self.check_lease();

        let bounds = r.bounds().to_pixel_rect().to_rect(1.0);

        if let Some(brush) = brush {
            let fill = self.create_fill(brush, bounds);
            self.canvas.get().draw_region(&r.region(), &fill.paint);
            self.release_fill(fill);
        }

        if let Some(pen) = pen {
            if let Some(stroke) = self.create_stroke(pen, bounds.inflate(pen.thickness() / 2.0)) {
                self.canvas.get().draw_region(&r.region(), &stroke.paint);
                self.release_stroke(stroke);
            }
        }
    }

    fn draw_ellipse(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, rect: Rect) {
        if rect.height <= 0.0 || rect.width <= 0.0 {
            return;
        }
        self.check_lease();

        let rc = to_sk_rect(rect);

        if let Some(brush) = brush {
            let fill = self.create_fill(brush, rect);
            self.canvas.get().draw_oval(rc, &fill.paint);
            self.release_fill(fill);
        }

        if let Some(pen) = pen {
            if let Some(stroke) = self.create_stroke(pen, rect.inflate(pen.thickness() / 2.0)) {
                self.canvas.get().draw_oval(rc, &stroke.paint);
                self.release_stroke(stroke);
            }
        }
    }

    fn draw_glyph_run(&mut self, foreground: Option<&dyn IBrush>, glyph_run: &dyn IGlyphRunImpl) {
        self.check_lease();

        let Some(foreground) = foreground else {
            return;
        };

        let glyph_run_impl = glyph_run
            .as_any()
            .downcast_ref::<GlyphRunImpl>()
            .unwrap_or_else(|| panic!("The glyph run was not created by the Skia backend"));

        let paint_wrapper = self.create_fill(foreground, glyph_run.bounds());

        // Determine the effective text options for text rendering. Start
        // with the current pushed text options.
        let mut effective_text_options = self.text_options;

        // If subpixel rendering is disabled globally, map subpixel modes to
        // grayscale.
        if self.disable_subpixel_text_rendering {
            let mode = effective_text_options.text_rendering_mode;

            if mode == TextRenderingMode::SubpixelAntialias
                || (mode == TextRenderingMode::Unspecified
                    && (self.render_options.edge_mode == EdgeMode::Antialias
                        || self.render_options.edge_mode == EdgeMode::Unspecified))
            {
                effective_text_options.text_rendering_mode = TextRenderingMode::Antialias;
            }
        }

        // If the text rendering mode is unspecified in the text options, use
        // the one from the render options.
        if effective_text_options.text_rendering_mode == TextRenderingMode::Unspecified
            && self.render_options.text_rendering_mode != TextRenderingMode::Unspecified
        {
            effective_text_options.text_rendering_mode = self.render_options.text_rendering_mode;
        }

        let text_blob = glyph_run_impl.get_text_blob(effective_text_options, self.render_options);

        if let Some(text_blob) = text_blob {
            self.canvas.get().draw_text_blob(text_blob, to_sk_point(glyph_run.baseline_origin()), &paint_wrapper.paint);
        }

        self.release_fill(paint_wrapper);
    }

    fn create_layer(&mut self, size: PixelSize) -> Rc<dyn IDrawingContextLayerImpl> {
        self.check_lease();
        Rc::new(self.create_render_target(size, true, false, None))
    }

    fn push_clip(&mut self, clip: Rect) {
        self.check_lease();
        let canvas = self.canvas.get();
        canvas.save();
        canvas.clip_rect(to_sk_rect(clip), None, None);
    }

    fn push_clip_rounded(&mut self, clip: RoundedRect) {
        self.check_lease();

        // Get the rounded rectangle.
        let rc = to_sk_rect(clip.rect);

        // Get a round rect from the cache.
        let round_rect = SkRoundRectCache::get_and_set_radii(&rc, &clip);

        let canvas = self.canvas.get();
        canvas.save();
        canvas.clip_rrect(round_rect, None, true);

        // Should not need to reset as setting the radii overrides the values.
        SkRoundRectCache::return_item(round_rect);
    }

    fn push_clip_region(&mut self, region: &dyn IPlatformRenderInterfaceRegion) {
        let r = Self::region_impl(region);
        self.check_lease();
        let canvas = self.canvas.get();
        canvas.save();
        canvas.clip_region(&r.region(), None);
    }

    fn pop_clip(&mut self) {
        self.check_lease();
        self.restore_canvas();
    }

    fn push_layer(&mut self, bounds: Rect) {
        self.check_lease();
        let bounds = to_sk_rect(bounds);
        self.canvas.get().save_layer(&SaveLayerRec::default().bounds(&bounds));
    }

    fn pop_layer(&mut self) {
        self.check_lease();
        self.restore_canvas();
    }

    fn push_opacity(&mut self, opacity: f64, bounds: Option<Rect>) {
        self.check_lease();

        self.opacity_stack.push(self.current_opacity);

        let use_opacity_save_layer =
            self.use_opacity_save_layer || self.render_options.requires_full_opacity_handling == Some(true);

        if use_opacity_save_layer {
            // Take the current multiplied opacity.
            let opacity = self.current_opacity * opacity;

            // Opacity is applied via layering.
            self.current_opacity = 1.0;

            let mut paint = sk::Paint::default();
            paint.set_color4f(sk::Color4f::new(0.0, 0.0, 0.0, opacity as f32), None);

            match bounds {
                Some(bounds) => {
                    let rect = to_sk_rect(bounds);
                    self.canvas.get().save_layer(&SaveLayerRec::default().bounds(&rect).paint(&paint));
                }
                None => {
                    self.canvas.get().save_layer(&SaveLayerRec::default().paint(&paint));
                }
            }
        } else {
            self.current_opacity *= opacity;
        }
    }

    fn pop_opacity(&mut self) {
        self.check_lease();

        let use_opacity_save_layer =
            self.use_opacity_save_layer || self.render_options.requires_full_opacity_handling == Some(true);

        if use_opacity_save_layer {
            self.restore_canvas();
        }

        self.current_opacity = self.opacity_stack.pop().unwrap_or_else(|| panic!("The opacity stack is empty"));
    }

    fn push_opacity_mask(&mut self, mask: &dyn IBrush, bounds: Rect) {
        self.check_lease();

        let paint = SkPaintCache::get();

        let sk_bounds = to_sk_rect(bounds);
        let canvas = self.canvas.get();
        canvas.save_layer(&SaveLayerRec::default().bounds(&sk_bounds).paint(&paint));
        let total_matrix = canvas.local_to_device();

        let mask_paint = self.create_paint(paint, mask, bounds);
        self.mask_stack.push((total_matrix, mask_paint));
    }

    fn pop_opacity_mask(&mut self) {
        self.check_lease();

        let mut paint = SkPaintCache::get();
        paint.set_blend_mode(sk::BlendMode::DstIn);

        let (transform, paint_wrapper) =
            self.mask_stack.pop().unwrap_or_else(|| panic!("The opacity mask stack is empty"));

        let canvas = self.canvas.get();
        canvas.save_layer(&SaveLayerRec::default().paint(&paint));
        SkPaintCache::return_reset(paint);

        canvas.set_matrix(&transform);
        canvas.draw_paint(&paint_wrapper.paint);

        // The wrapper resets the paint when it is disposed.
        SkPaintCache::return_item(paint_wrapper.dispose());

        self.restore_canvas();

        self.restore_canvas();
    }

    fn push_geometry_clip(&mut self, clip: &dyn IGeometryImpl) {
        self.check_lease();
        let fill_path = Self::geometry_impl(clip).fill_path();
        let canvas = self.canvas.get();
        canvas.save();
        if let Some(fill_path) = fill_path {
            canvas.clip_path(&fill_path, sk::ClipOp::Intersect, true);
        }
    }

    fn pop_geometry_clip(&mut self) {
        self.check_lease();
        self.restore_canvas();
    }

    fn push_render_options(&mut self, render_options: RenderOptions) {
        self.check_lease();

        self.render_options_stack.push(self.render_options);

        self.render_options = self.render_options.merge_with(render_options);
    }

    fn pop_render_options(&mut self) {
        self.render_options =
            self.render_options_stack.pop().unwrap_or_else(|| panic!("The render options stack is empty"));
    }

    fn push_text_options(&mut self, text_options: TextOptions) {
        self.check_lease();

        self.text_options_stack.push(self.text_options);

        self.text_options = self.text_options.merge_with(text_options);
    }

    fn pop_text_options(&mut self) {
        self.text_options =
            self.text_options_stack.pop().unwrap_or_else(|| panic!("The text options stack is empty"));
    }

    fn as_drawing_context_impl_with_effects(&mut self) -> Option<&mut dyn IDrawingContextImplWithEffects> {
        Some(self)
    }

    fn as_drawing_context_with_acrylic_like_support(
        &mut self,
    ) -> Option<&mut dyn IDrawingContextWithAcrylicLikeSupport> {
        Some(self)
    }

    fn get_feature(&mut self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn ISkiaApiLeaseFeature>() {
            let surface = self.surface.clone()?;
            let feature: Rc<dyn ISkiaApiLeaseFeature> = Rc::new(SkiaLeaseFeature {
                context_leased: self.leased.clone(),
                surface,
                gr_context: self.gr_context.clone(),
                gpu: self.gpu.clone(),
                current_opacity: self.current_opacity,
            });
            return Some(Rc::new(feature));
        }

        None
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.check_lease();
        self.disposed = true;

        // Return leased paints.
        for paint in [self.stroke_paint.take(), self.fill_paint.take(), self.box_shadow_paint.take()]
            .into_iter()
            .flatten()
        {
            SkPaintCache::return_reset(paint);
        }

        self.gr_context = None;

        for disposable in std::mem::take(&mut self.disposables) {
            disposable();
        }

        if let CanvasSource::Recorder { mut recorder, on_finished } =
            std::mem::replace(&mut self.canvas, CanvasSource::Finished)
        {
            on_finished(recorder.finish_recording_as_picture(None));
        }
    }
}

impl IDrawingContextImplWithEffects for DrawingContextImpl {
    fn push_effect(&mut self, clip_rect: Option<Rect>, effect: &dyn IEffect) {
        self.check_lease();
        let filter = self.create_effect(effect);
        let mut paint = SkPaintCache::get();
        paint.set_image_filter(filter);

        match clip_rect {
            Some(clip_rect) => {
                let bounds = to_sk_rect(clip_rect);
                self.canvas.get().save_layer(&SaveLayerRec::default().bounds(&bounds).paint(&paint));
            }
            None => {
                self.canvas.get().save_layer(&SaveLayerRec::default().paint(&paint));
            }
        }

        SkPaintCache::return_reset(paint);
    }

    fn pop_effect(&mut self) {
        self.check_lease();
        self.restore_canvas();
    }
}

impl IDrawingContextWithAcrylicLikeSupport for DrawingContextImpl {
    fn draw_rectangle_with_material(&mut self, material: &dyn IExperimentalAcrylicMaterial, rect: RoundedRect) {
        if rect.rect.height <= 0.0 || rect.rect.width <= 0.0 {
            return;
        }
        self.check_lease();

        let rc = to_sk_rect(rect.rect);

        let paint = self.fill_paint.take().unwrap_or_default();
        let fill = self.create_acrylic_paint(paint, material);

        if rect.is_rounded() {
            let sk_round_rect = SkRoundRectCache::get_and_set_radii(&rc, &rect);
            self.canvas.get().draw_rrect(sk_round_rect, &fill.paint);
            SkRoundRectCache::return_item(sk_round_rect);
        } else {
            self.canvas.get().draw_rect(rc, &fill.paint);
        }

        self.release_fill(fill);
    }
}

fn reset_paint(mut paint: sk::Paint) -> sk::Paint {
    paint.reset();
    paint
}

/// Skia paint wrapper: a configured paint and what has to stay alive (and be
/// released) with it.
pub(crate) struct PaintWrapper {
    pub paint: sk::Paint,
    // We are saving memory allocations here: up to three disposables without
    // a collection.
    disposable1: Option<Box<dyn FnOnce()>>,
    disposable2: Option<Box<dyn FnOnce()>>,
    disposable3: Option<Box<dyn FnOnce()>>,
}

impl PaintWrapper {
    pub fn new(paint: sk::Paint) -> Self {
        Self { paint, disposable1: None, disposable2: None, disposable3: None }
    }

    /// Adds a new disposable to the wrapper.
    ///
    /// # Panics
    /// Panics when the wrapper already holds three disposables.
    pub fn add_disposable(&mut self, disposable: Box<dyn FnOnce()>) {
        if self.disposable1.is_none() {
            self.disposable1 = Some(disposable);
        } else if self.disposable2.is_none() {
            self.disposable2 = Some(disposable);
        } else if self.disposable3.is_none() {
            self.disposable3 = Some(disposable);
        } else {
            panic!(
                "PaintWrapper disposable object limit reached. You need to add extra struct fields to support more disposables."
            );
        }
    }

    /// Releases the disposables and hands back the paint, reset.
    pub fn dispose(mut self) -> sk::Paint {
        self.paint.reset();
        for disposable in [self.disposable1.take(), self.disposable2.take(), self.disposable3.take()]
            .into_iter()
            .flatten()
        {
            disposable();
        }
        self.paint
    }
}
