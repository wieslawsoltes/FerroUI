use crate::geometry_impl::{try_get_geometry_impl, GeometryImpl};
use crate::glyph_run_impl::GlyphRunImpl;
use crate::helpers::path_helper;
use crate::i_drawable_bitmap_impl::try_get_drawable_bitmap;
use crate::scene::{
    IVelloSceneSink, VelloSceneBrush, VelloSceneGlyph, VelloSceneGlyphRun, VelloSceneImage, VelloScenePaint,
};
use crate::surface_render_target::{SurfaceRenderTarget, SurfaceRenderTargetCreateInfo};
use crate::vello_extensions::{
    ellipse_path, rect_path, rounded_rect_path, to_affine, to_blend_mode, to_color, to_color_with_opacity,
    to_extend, to_fill, to_kurbo_point,
};
use crate::vello_options::{VelloOptions, VelloRenderingMode};
use crate::vello_platform::VelloPlatform;
use crate::vello_region_impl::VelloRegionImpl;
use crate::vello_typeface::{bold_simulation_outline_width, OBLIQUE_SKEW};
use ferroui_base::media::{
    BaselinePixelAlignment, BoxShadows, Color, Colors, EdgeMode, FontSimulations, IBrush, IGradientBrush, IPen,
    ITileBrush, RenderOptions, TextHintingMode, TextOptions, TextRenderingMode, TileMode,
};
use ferroui_base::platform::{
    IBitmapImpl, IDrawingContextImpl, IDrawingContextLayerImpl, IGeometryImpl, IGlyphRunImpl,
    IPlatformRenderInterfaceRegion,
};
use ferroui_base::rendering::utilities::TileBrushCalculator;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{
    FerroLocator, LocatorExtensions, Matrix, PixelSize, Point, Rect, RelativeUnit, RoundedRect, Vector,
};
use kurbo::{Affine, BezPath, Stroke};
use peniko::color::{AlphaColor, Srgb};
use peniko::{BlendMode, Compose, Extend, Fill, Gradient, ImageData, ImageQuality, InterpolationAlphaSpace, Mix};
use std::any::{Any, TypeId};
use std::rc::Rc;

mod acrylic;
pub(crate) mod box_shadows;
mod effects;
mod scene_brushes;

/// Context create info.
pub struct CreateInfo {
    /// The scene the context records into: of the size of the target, in
    /// the rendering mode of the target.
    pub sink: Box<dyn IVelloSceneSink>,

    /// What the target holds from the frames before, when it holds
    /// anything: the scene starts with it.
    pub backdrop: Option<ImageData>,

    /// Receives the scene when the context is disposed, to render it into
    /// the target.
    pub on_finished: Box<dyn FnOnce(&mut dyn IVelloSceneSink)>,

    /// Makes DPI to be applied as a hidden matrix transform.
    pub scale_drawing_to_dpi: bool,

    /// Dpi for intermediate surfaces.
    pub dpi: Vector,

    /// The rendering modes of intermediate surfaces, in the order they are
    /// tried.
    pub rendering_modes: Vec<VelloRenderingMode>,
}

/// The stage of the design document a feature that is not built belongs to.
///
/// # Panics
/// Always: a member of the contract that is not built yet fails rather than
/// draw something else or nothing.
#[track_caller]
pub(crate) fn not_built(feature: &str, stage: &str) -> ! {
    panic!("The Vello backend does not draw {feature} yet: {stage} of docs/porting/vello-backend.md")
}

/// What a pushed state has to end in the scene when it is popped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SavedKind {
    /// A clip.
    Clip,
    /// A layer.
    Layer,
    /// Nothing: a state that only saved the transform.
    Nothing,
    /// An effect: its layer, and its scene when it has one of its own.
    Effect,
}

/// Vello based drawing context.
///
/// The immediate calls of the contract are recorded into the scene of the
/// frame ([`IVelloSceneSink`]); the scene is rendered when the context is
/// disposed. The states the contract pushes and pops are a stack beside the
/// scene, as the canvas of Skia keeps one: a pop restores the transform that
/// was current at the push.
pub struct DrawingContextImpl {
    sink: Option<Box<dyn IVelloSceneSink>>,
    on_finished: Option<Box<dyn FnOnce(&mut dyn IVelloSceneSink)>>,
    disposables: Vec<Box<dyn FnOnce()>>,
    // TODO: Get rid of this value, it's currently used to calculate
    // intermediate sizes for tile brushes but does so ignoring the current
    // transform.
    intermediate_surface_dpi: Vector,
    rendering_modes: Vec<VelloRenderingMode>,
    state_stack: Vec<(Matrix, SavedKind)>,
    mask_stack: Vec<(Affine, PaintWrapper)>,
    opacity_stack: Vec<(f64, bool)>,
    render_options_stack: Vec<RenderOptions>,
    text_options_stack: Vec<TextOptions>,
    post_transform: Option<Matrix>,
    current_opacity: f64,
    current_transform: Matrix,
    disposed: bool,
    use_opacity_save_layer: bool,
    render_options: RenderOptions,
    text_options: TextOptions,
    /// The effects that are pushed.
    effect_stack: Vec<effects::EffectFrame>,
    /// The clips that are open in the scene of the context.
    open_clips: usize,
    /// The pixel of the target that is the top left pixel of the scene of
    /// the context: not the first one while an effect is recorded into a
    /// scene of its own.
    sink_origin: kurbo::Vec2,
}

impl DrawingContextImpl {
    /// Creates a new drawing context.
    ///
    /// `disposables` run in order after drawing has finished, when the
    /// context is disposed.
    pub fn new(create_info: CreateInfo, disposables: Vec<Box<dyn FnOnce()>>) -> Self {
        let default_dpi = VelloPlatform::default_dpi();
        let post_transform = if create_info.scale_drawing_to_dpi && !create_info.dpi.nearly_equals(default_dpi) {
            Some(Matrix::create_scale(create_info.dpi.x / default_dpi.x, create_info.dpi.y / default_dpi.y))
        } else {
            None
        };

        // The options registered on this thread, else the ones the backend
        // was initialized with (the render thread has no services).
        let use_opacity_save_layer = FerroLocator::current()
            .get_service::<VelloOptions>()
            .map(|options| *options)
            .or_else(VelloPlatform::options)
            .is_some_and(|options| options.use_opacity_save_layer);

        let mut context = Self {
            sink: Some(create_info.sink),
            on_finished: Some(create_info.on_finished),
            disposables,
            intermediate_surface_dpi: create_info.dpi,
            rendering_modes: create_info.rendering_modes,
            state_stack: Vec::new(),
            mask_stack: Vec::new(),
            opacity_stack: Vec::new(),
            render_options_stack: Vec::new(),
            text_options_stack: Vec::new(),
            post_transform,
            current_opacity: 1.0,
            current_transform: Matrix::IDENTITY,
            disposed: false,
            use_opacity_save_layer,
            render_options: RenderOptions::default(),
            text_options: TextOptions::default(),
            effect_stack: Vec::new(),
            open_clips: 0,
            sink_origin: kurbo::Vec2::ZERO,
        };

        if let Some(backdrop) = create_info.backdrop {
            context.draw_image_in_device_space(backdrop, BlendMode::default());
        }

        context
    }

    /// The scene.
    ///
    /// # Panics
    /// Panics when the drawing context has been disposed.
    fn sink(&mut self) -> &mut dyn IVelloSceneSink {
        match &mut self.sink {
            Some(sink) => &mut **sink,
            None => panic!("The drawing context has been disposed"),
        }
    }

    /// The rendering mode the context draws in.
    pub fn rendering_mode(&mut self) -> VelloRenderingMode {
        self.sink().rendering_mode()
    }

    /// The render options in effect.
    pub fn render_options(&self) -> RenderOptions {
        self.render_options
    }

    /// Replaces the render options in effect.
    pub fn set_render_options(&mut self, value: RenderOptions) {
        self.render_options = value;
    }

    /// The text options in effect.
    pub fn text_options(&self) -> TextOptions {
        self.text_options
    }

    /// Replaces the text options in effect.
    pub fn set_text_options(&mut self, value: TextOptions) {
        self.text_options = value;
    }

    /// The transform from the space the contract draws in to the pixels of
    /// the target.
    fn device_transform(&self) -> Affine {
        let transform = match self.post_transform {
            Some(post_transform) => to_affine(self.current_transform * post_transform),
            None => to_affine(self.current_transform),
        };

        self.pixel_transform() * transform
    }

    /// The transform from the pixels of the target to the pixels of the
    /// scene of the context.
    fn pixel_transform(&self) -> Affine {
        Affine::translate(-self.sink_origin)
    }

    fn anti_alias(&self) -> bool {
        self.edge_anti_alias(self.render_options.edge_mode != EdgeMode::Aliased)
    }

    /// Whether an edge is anti-aliased when the contract asks for it to be
    /// or not to be: as asked, and always in a rendering mode whose
    /// renderer has no aliased edges (classic `vello`).
    fn edge_anti_alias(&self, anti_alias: bool) -> bool {
        anti_alias || !self.sink.as_ref().is_some_and(|sink| sink.capabilities().aliased_edges)
    }

    /// The whole target as a path, in pixels.
    fn target_path(&mut self) -> BezPath {
        let (width, height) = (self.sink().width() as f64, self.sink().height() as f64);
        rect_path(Rect::new(0.0, 0.0, width, height))
    }

    /// Saves the transform with what the pop has to end in the scene.
    fn save(&mut self, kind: SavedKind) {
        if kind == SavedKind::Clip {
            self.open_clips += 1;
        }
        self.state_stack.push((self.current_transform, kind));
    }

    /// Ends the innermost pushed state. The transform of the context is the
    /// one that was current when the state was pushed.
    fn restore(&mut self) {
        let (transform, kind) =
            self.state_stack.pop().unwrap_or_else(|| panic!("The state stack of the drawing context is empty"));

        match kind {
            SavedKind::Clip => {
                self.open_clips -= 1;
                self.sink().pop_clip();
            }
            SavedKind::Layer => self.sink().pop_layer(),
            SavedKind::Nothing => {}
            SavedKind::Effect => self.end_effect(),
        }

        self.current_transform = transform;
    }

    fn push_clip_path(&mut self, path: &BezPath, fill_rule: Fill, anti_alias: bool) {
        let (transform, anti_alias) = (self.device_transform(), self.edge_anti_alias(anti_alias));
        self.sink().push_clip(path, fill_rule, transform, anti_alias);
        self.save(SavedKind::Clip);
    }

    /// Draws an image with its pixels on the pixels of the target, whatever
    /// the transform of the context is.
    pub(crate) fn draw_image_in_device_space(&mut self, image: ImageData, blend_mode: BlendMode) {
        let pixel_transform = self.pixel_transform();
        let path = rect_path(Rect::new(0.0, 0.0, image.width as f64, image.height as f64));
        let paint = VelloScenePaint {
            brush: VelloSceneBrush::Image(VelloSceneImage {
                image,
                x_extend: Extend::Pad,
                y_extend: Extend::Pad,
                quality: ImageQuality::Low,
                alpha: 1.0,
            }),
            transform: Affine::IDENTITY,
        };

        self.sink().fill(&path, Fill::NonZero, pixel_transform, &paint, blend_mode, false);
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

    /// Configures the paint wrapper for using a gradient brush.
    ///
    /// The gradients are those of the Skia backend, kind by kind: the same
    /// points, radii and transforms, on the gradient kinds of peniko.
    /// `opacity` is the opacity the brush is drawn with.
    fn configure_gradient_brush(
        paint_wrapper: &mut PaintWrapper,
        target_rect: Rect,
        gradient_brush: &dyn IGradientBrush,
        opacity: f32,
    ) {
        let extend = to_extend(gradient_brush.spread_method());
        let gradient_stops = gradient_brush.gradient_stops();
        let mut stop_colors: Vec<AlphaColor<Srgb>> = gradient_stops.iter().map(|s| to_color(s.color())).collect();
        let mut stop_offsets: Vec<f32> = gradient_stops.iter().map(|s| s.offset() as f32).collect();

        if stop_colors.is_empty() {
            // A gradient without stops: the Skia backend, which cannot
            // build one, draws with the paint as it is, white at the
            // opacity of the brush.
            paint_wrapper.paint = Some(VelloScenePaint::solid(AlphaColor::WHITE.multiply_alpha(opacity)));
            return;
        }

        let stops = |colors: &[AlphaColor<Srgb>], offsets: &[f32]| -> Vec<(f32, AlphaColor<Srgb>)> {
            offsets.iter().copied().zip(colors.iter().copied()).collect()
        };

        // Skia interpolates the colors of the stops before it
        // premultiplies them.
        let finish = |mut gradient: Gradient, extend: Extend| {
            gradient.extend = extend;
            gradient.interpolation_alpha_space = InterpolationAlphaSpace::Unpremultiplied;
            gradient
        };

        let mut underlay = None;

        let (gradient, transform) = if let Some(linear_gradient) = gradient_brush.as_linear_gradient_brush() {
            let start = to_kurbo_point(linear_gradient.start_point().to_pixels_rect(target_rect));
            let end = to_kurbo_point(linear_gradient.end_point().to_pixels_rect(target_rect));

            let transform = Self::combine(
                Self::get_relative_transform(gradient_brush, target_rect),
                Self::get_absolute_transform(gradient_brush, target_rect),
            );

            let gradient = Gradient::new_linear(start, end).with_stops(&stops(&stop_colors, &stop_offsets)[..]);
            (finish(gradient, extend), transform)
        } else if let Some(radial_gradient) = gradient_brush.as_radial_gradient_brush() {
            let center_point = radial_gradient.center().to_pixels_rect(target_rect);
            let center = to_kurbo_point(center_point);

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

            if origin_point == center_point {
                // When the origin is the same as the center the radial
                // gradient acts the same as D2D.
                let gradient =
                    Gradient::new_radial(center, radius_x as f32).with_stops(&stops(&stop_colors, &stop_offsets)[..]);
                (finish(gradient, extend), transform)
            } else {
                // When the origin is different to the center use a two point
                // conical gradient to match the behaviour of D2D.
                if radius_x != radius_y {
                    // Adjust the origin point for the radius x/y
                    // transformation by reversing it.
                    origin_point = origin_point
                        .with_y((origin_point.y - center_point.y) * radius_x / radius_y + center_point.y);
                }

                let origin = to_kurbo_point(origin_point);

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

                    let mut reversed_colors = vec![AlphaColor::TRANSPARENT; count];
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
                // D2D's behaviour of filling with the final color: the
                // gradient is drawn over that color.
                underlay = Some(stop_colors[0]);

                let gradient = Gradient::new_two_point_radial(start, radius_start, end, radius_end)
                    .with_stops(&stops(&stop_colors, &stop_offsets)[..]);
                (finish(gradient, extend), transform)
            }
        } else if let Some(conic_gradient) = gradient_brush.as_conic_gradient_brush() {
            let center_point = conic_gradient.center().to_pixels_rect(target_rect);

            // The angle 0 of a sweep gradient is to the right of the center
            // point but we are matching CSS where the vertical point above
            // the center is 0.
            let angle = (conic_gradient.angle() - 90.0).to_radians();
            let rotation = Matrix::create_rotation_at(angle, center_point);

            let transform = Self::combine(
                Some(rotation),
                Self::combine(
                    Self::get_relative_transform(gradient_brush, target_rect),
                    Self::get_absolute_transform(gradient_brush, target_rect),
                ),
            );

            let gradient = Gradient::new_sweep(to_kurbo_point(center_point), 0.0, std::f32::consts::TAU)
                .with_stops(&stops(&stop_colors, &stop_offsets)[..]);
            (finish(gradient, Extend::Pad), transform)
        } else {
            // A gradient of a kind the contract does not have draws nothing.
            return;
        };

        let transform = transform.map(to_affine).unwrap_or(Affine::IDENTITY);

        match underlay {
            // The color and the gradient over it are composed as one, then
            // the opacity of the brush applies to both.
            Some(underlay) => {
                paint_wrapper.underlay = Some(underlay);
                paint_wrapper.layer_opacity = (opacity < 1.0).then_some(opacity);
                paint_wrapper.paint = Some(VelloScenePaint { brush: VelloSceneBrush::Gradient(gradient), transform });
            }
            None => {
                let gradient = if opacity < 1.0 { gradient.multiply_alpha(opacity) } else { gradient };
                paint_wrapper.paint = Some(VelloScenePaint { brush: VelloSceneBrush::Gradient(gradient), transform });
            }
        }
    }

    /// How a tile brush continues in x and y.
    fn get_tile_modes(mode: TileMode) -> (Extend, Extend) {
        (
            if mode == TileMode::FlipX || mode == TileMode::FlipXY { Extend::Reflect } else { Extend::Repeat },
            if mode == TileMode::FlipY || mode == TileMode::FlipXY { Extend::Reflect } else { Extend::Repeat },
        )
    }

    /// Configures the paint wrapper for using a tile brush.
    ///
    /// The tile is rendered into an intermediate surface whose pixels become
    /// the image of the paint.
    fn configure_tile_brush(
        &mut self,
        paint_wrapper: &mut PaintWrapper,
        target_box: Rect,
        tile_brush: &dyn ITileBrush,
        tile_brush_image: &dyn IBitmapImpl,
        opacity: f32,
    ) {
        let dpi = self.intermediate_surface_dpi;
        let image_pixel_size = tile_brush_image.pixel_size();

        let calc =
            TileBrushCalculator::from_brush(tile_brush, image_pixel_size.to_size_with_dpi_vector(dpi), target_box.size());
        let intermediate_size = PixelSize::from_size_with_dpi_vector(calc.intermediate_size(), dpi);
        if intermediate_size.width < 1 || intermediate_size.height < 1 {
            return;
        }
        let intermediate = self.create_render_target(intermediate_size, true);

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
            Matrix::create_translation(-calc.destination_rect().x, -calc.destination_rect().y)
        } else {
            Matrix::IDENTITY
        };

        let Some(image) = crate::i_drawable_bitmap_impl::IDrawableBitmapImpl::image(&intermediate) else {
            return;
        };
        IBitmapImpl::dispose(&intermediate);

        // From the pixels of the tile to the space of the shape: the
        // position of a relative destination, the pixels as logical units,
        // the origin of the tiling.
        let mut paint_transform = Matrix::create_scale(96.0 / dpi.x, 96.0 / dpi.y) * tile_transform;

        if tile_brush.destination_rect().unit == RelativeUnit::Relative {
            paint_transform = Matrix::create_translation(target_box.x, target_box.y) * paint_transform;
        }

        // Both brush transforms act on the tile once it sits in target
        // space, the relative one first.
        if let Some(relative_transform) = Self::get_relative_transform(tile_brush, target_box) {
            paint_transform = paint_transform * relative_transform;
        }

        if let Some(transform) = Self::get_absolute_transform(tile_brush, target_box) {
            paint_transform = paint_transform * transform;
        }

        let paint_transform = to_affine(paint_transform);
        let (x_extend, y_extend) = Self::get_tile_modes(tile_mode);

        if tile_mode == TileMode::None {
            // A tile that is not repeated paints nothing beside itself. An
            // image of peniko has no such extend mode (Skia's is "decal"):
            // the shape is clipped to the tile instead.
            let mut tile = rect_path(Rect::new(0.0, 0.0, image.width as f64, image.height as f64));
            tile.apply_affine(paint_transform);
            paint_wrapper.clip = Some(tile);
        }

        paint_wrapper.paint = Some(VelloScenePaint {
            brush: VelloSceneBrush::Image(VelloSceneImage {
                image,
                x_extend,
                y_extend,
                // The Skia backend samples the tile with its default
                // sampling: the nearest pixel.
                quality: ImageQuality::Low,
                alpha: opacity,
            }),
            transform: paint_transform,
        });
    }

    /// Creates a paint wrapper for the given brush.
    fn create_paint(&mut self, brush: &dyn IBrush, target_rect: Rect) -> PaintWrapper {
        let opacity = brush.opacity() * if self.use_opacity_save_layer { 1.0 } else { self.current_opacity };
        let mut paint_wrapper = PaintWrapper::new();

        if let Some(solid) = brush.as_solid_color_brush() {
            paint_wrapper.paint = Some(VelloScenePaint::solid(to_color_with_opacity(solid.color(), opacity)));

            return paint_wrapper;
        }

        // The opacity of a paint of Skia is the alpha of its color: a byte.
        let opacity = ((255.0 * opacity) as u8) as f32 / 255.0;

        if let Some(gradient) = brush.as_gradient_brush() {
            Self::configure_gradient_brush(&mut paint_wrapper, target_rect, gradient, opacity);

            return paint_wrapper;
        }

        let tile_brush = brush.as_tile_brush();
        let mut tile_brush_image: Option<std::sync::Arc<ferroui_base::platform::SharedBitmapImpl>> = None;

        if let Some(scene_brush) = brush.as_scene_brush() {
            // A scene brush without content paints nothing.
            if let Some(content) = scene_brush.create_content() {
                self.configure_scene_brush_content(&mut paint_wrapper, &*content, target_rect, opacity);
                content.dispose();
                return paint_wrapper;
            }
        } else if let Some(image_brush) = brush.as_image_brush() {
            tile_brush_image = image_brush
                .source()
                .and_then(|source| source.get_bitmap())
                .filter(|bitmap| try_get_drawable_bitmap(&**bitmap).is_some());
        }

        // A tile brush without an image paints nothing.
        if let (Some(tile_brush), Some(tile_brush_image)) = (tile_brush, tile_brush_image) {
            self.configure_tile_brush(&mut paint_wrapper, target_rect, tile_brush, &*tile_brush_image, opacity);
        }

        paint_wrapper
    }

    /// Creates a paint wrapper and the stroke style for the given pen, or
    /// `None` when the pen draws nothing.
    fn create_stroke(&mut self, pen: &dyn IPen, target_rect: Rect) -> Option<(PaintWrapper, Stroke)> {
        // A stroke without a thickness is a hairline to a renderer - and for
        // us it means - there is nothing rendered.
        let brush = pen.brush()?;
        if pen.thickness() == 0.0 {
            return None;
        }

        let paint_wrapper = self.create_paint(&*brush, target_rect);

        Some((paint_wrapper, path_helper::create_stroke(pen)))
    }

    /// Draws with a paint wrapper: `draw` is called once for everything the
    /// wrapper paints with, between the clip and the layer the wrapper
    /// needs, and the wrapper is released.
    fn draw_with(
        &mut self,
        paint_wrapper: PaintWrapper,
        draw: &dyn Fn(&mut dyn IVelloSceneSink, &VelloScenePaint, Affine, bool),
    ) {
        let (transform, anti_alias) = (self.device_transform(), self.anti_alias());
        let sink = self.sink();

        if paint_wrapper.paint.is_some() || paint_wrapper.underlay.is_some() {
            if let Some(clip) = &paint_wrapper.clip {
                sink.push_clip(clip, Fill::NonZero, transform, anti_alias);
            }
            if let Some(opacity) = paint_wrapper.layer_opacity {
                sink.push_layer(BlendMode::default(), opacity);
            }

            if let Some(underlay) = paint_wrapper.underlay {
                draw(sink, &VelloScenePaint::solid(underlay), transform, anti_alias);
            }
            if let Some(paint) = &paint_wrapper.paint {
                draw(sink, paint, transform, anti_alias);
            }

            if paint_wrapper.layer_opacity.is_some() {
                sink.pop_layer();
            }
            if paint_wrapper.clip.is_some() {
                sink.pop_clip();
            }
        }

        paint_wrapper.dispose();
    }

    fn fill_path(&mut self, brush: &dyn IBrush, target_rect: Rect, path: &BezPath, fill_rule: Fill) {
        let fill = self.create_paint(brush, target_rect);
        self.draw_with(fill, &|sink, paint, transform, anti_alias| {
            sink.fill(path, fill_rule, transform, paint, BlendMode::default(), anti_alias);
        });
    }

    fn stroke_path(&mut self, pen: &dyn IPen, target_rect: Rect, path: &BezPath) {
        if let Some((paint_wrapper, stroke)) = self.create_stroke(pen, target_rect) {
            self.draw_with(paint_wrapper, &|sink, paint, transform, anti_alias| {
                sink.stroke(path, &stroke, transform, paint, anti_alias);
            });
        }
    }

    /// Creates a new render target compatible with this drawing context.
    ///
    /// `use_scaled_drawing` auto-scales the drawing to the DPI.
    fn create_render_target(&self, pixel_size: PixelSize, use_scaled_drawing: bool) -> SurfaceRenderTarget {
        SurfaceRenderTarget::new(SurfaceRenderTargetCreateInfo {
            width: pixel_size.width,
            height: pixel_size.height,
            dpi: self.intermediate_surface_dpi,
            rendering_modes: self.rendering_modes.clone(),
            use_scaled_drawing,
        })
    }

    fn geometry_impl(geometry: &dyn IGeometryImpl) -> &dyn GeometryImpl {
        try_get_geometry_impl(geometry)
            .unwrap_or_else(|| panic!("The geometry was not created by the Vello backend"))
    }

    fn region_impl(region: &dyn IPlatformRenderInterfaceRegion) -> &VelloRegionImpl {
        region
            .as_any()
            .downcast_ref::<VelloRegionImpl>()
            .unwrap_or_else(|| panic!("The region was not created by the Vello backend"))
    }

    fn uses_opacity_layer(&self) -> bool {
        self.use_opacity_save_layer || self.render_options.requires_full_opacity_handling == Some(true)
    }
}

impl IDrawingContextImpl for DrawingContextImpl {
    fn transform(&self) -> Matrix {
        self.current_transform
    }

    fn set_transform(&mut self, value: Matrix) {
        self.current_transform = value;
    }

    fn clear(&mut self, color: Color) {
        // Outside every clip and layer a clear replaces all there is: the
        // scene starts over.
        if self.state_stack.is_empty() && self.mask_stack.is_empty() {
            self.sink().reset();
            if color.a == 0 {
                return;
            }
        }

        // The pixels inside the clip are replaced, not blended.
        let path = self.target_path();
        let paint = VelloScenePaint::solid(to_color(color));
        self.sink().fill(&path, Fill::NonZero, Affine::IDENTITY, &paint, BlendMode::new(Mix::Normal, Compose::Copy), false);
    }

    fn draw_bitmap(&mut self, source: &dyn IBitmapImpl, opacity: f64, source_rect: Rect, dest_rect: Rect) {
        let drawable_image = try_get_drawable_bitmap(source)
            .unwrap_or_else(|| panic!("The bitmap was not created by the Vello backend"));
        let Some(image) = drawable_image.image() else {
            return;
        };
        if source_rect.width <= 0.0 || source_rect.height <= 0.0 || dest_rect.width <= 0.0 || dest_rect.height <= 0.0 {
            return;
        }

        // The source rectangle of the image onto the destination rectangle.
        let mut image_transform = Affine::translate((dest_rect.x, dest_rect.y))
            * Affine::scale_non_uniform(dest_rect.width / source_rect.width, dest_rect.height / source_rect.height)
            * Affine::translate((-source_rect.x, -source_rect.y));

        let is_upscaling = dest_rect.width > source_rect.width || dest_rect.height > source_rect.height;
        let (quality, mipmaps) =
            crate::vello_extensions::to_sampling(self.render_options.bitmap_interpolation_mode, is_upscaling);

        let alpha = ((255.0 * opacity * self.current_opacity) as u8) as f32 / 255.0;
        let blend_mode = to_blend_mode(self.render_options.bitmap_blending_mode);
        let (transform, anti_alias) = (self.device_transform(), self.anti_alias());

        // An image that is drawn reduced in a mode with mipmaps is sampled
        // from the two levels of its mipmap that are nearest to the size
        // it is drawn at.
        let mut image = image;
        if mipmaps {
            use crate::helpers::mipmap_helper;

            if let Some(levels) = mipmap_helper::levels(&image, transform * image_transform) {
                if blend_mode == BlendMode::default() {
                    let path = rect_path(dest_rect);
                    mipmap_helper::fill_with_levels(
                        self.sink(),
                        &path,
                        transform,
                        image_transform,
                        &levels,
                        alpha,
                        anti_alias,
                    );
                    return;
                }

                // Another blending mode composes the image with what is
                // under it inside the rectangle only, which a layer of the
                // two levels would not: one image stands for both.
                if let Some(prefiltered) = levels.blended() {
                    image_transform *=
                        Affine::scale_non_uniform(1.0 / prefiltered.scale_x, 1.0 / prefiltered.scale_y);
                    image = prefiltered.image;
                }
            }
        }

        let paint = VelloScenePaint {
            brush: VelloSceneBrush::Image(VelloSceneImage {
                image,
                x_extend: Extend::Pad,
                y_extend: Extend::Pad,
                quality,
                alpha,
            }),
            transform: image_transform,
        };

        self.sink().fill(&rect_path(dest_rect), Fill::NonZero, transform, &paint, blend_mode, anti_alias);
    }

    fn draw_bitmap_with_mask(
        &mut self,
        source: &dyn IBitmapImpl,
        opacity_mask: &dyn IBrush,
        opacity_mask_rect: Rect,
        dest_rect: Rect,
    ) {
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
        let Some(pen) = pen else {
            return;
        };

        let mut path = BezPath::new();
        path.move_to(to_kurbo_point(p1));
        path.line_to(to_kurbo_point(p2));

        self.stroke_path(pen, Rect::from_points(p1, p2).normalize(), &path);
    }

    fn draw_geometry(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, geometry: &dyn IGeometryImpl) {
        let geometry_impl = Self::geometry_impl(geometry);
        let rect = geometry.bounds();

        if let (Some(brush), Some(fill_path)) = (brush, geometry_impl.fill_path()) {
            self.fill_path(brush, rect, fill_path.path(), to_fill(fill_path.fill_rule()));
        }

        if let (Some(pen), Some(stroke_path)) = (pen, geometry_impl.stroke_path()) {
            self.stroke_path(pen, rect.inflate(pen.thickness() / 2.0), stroke_path.path());
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

        // Arbitrary chosen values, as in the Skia backend: a box that is
        // this large has no shadows.
        let no_box_shadows = BoxShadows::default();
        let box_shadows =
            if rect.rect.height > 8192.0 || rect.rect.width > 8192.0 { &no_box_shadows } else { box_shadows };

        self.draw_box_shadows(&rect, box_shadows, false);

        let path = rounded_rect_path(rect);

        if let Some(brush) = brush {
            self.fill_path(brush, rect.rect, &path, Fill::NonZero);
        }

        self.draw_box_shadows(&rect, box_shadows, true);

        if let Some(pen) = pen {
            self.stroke_path(pen, rect.rect.inflate(pen.thickness() / 2.0), &path);
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

        let bounds = r.bounds().to_pixel_rect().to_rect(1.0);
        let path = r.path();

        if let Some(brush) = brush {
            self.fill_path(brush, bounds, &path, Fill::NonZero);
        }

        if let Some(pen) = pen {
            self.stroke_path(pen, bounds.inflate(pen.thickness() / 2.0), &path);
        }
    }

    fn draw_ellipse(&mut self, brush: Option<&dyn IBrush>, pen: Option<&dyn IPen>, rect: Rect) {
        if rect.height <= 0.0 || rect.width <= 0.0 {
            return;
        }

        let path = ellipse_path(rect);

        if let Some(brush) = brush {
            self.fill_path(brush, rect, &path, Fill::NonZero);
        }

        if let Some(pen) = pen {
            self.stroke_path(pen, rect.inflate(pen.thickness() / 2.0), &path);
        }
    }

    fn draw_glyph_run(&mut self, foreground: Option<&dyn IBrush>, glyph_run: &dyn IGlyphRunImpl) {
        let Some(foreground) = foreground else {
            return;
        };

        let glyph_run_impl = glyph_run
            .as_any()
            .downcast_ref::<GlyphRunImpl>()
            .unwrap_or_else(|| panic!("The glyph run was not created by the Vello backend"));

        // A font no renderer draws glyphs of (see `VelloFontFace`).
        if !glyph_run_impl.face().is_drawable() {
            return;
        }

        let paint_wrapper = self.create_paint(foreground, glyph_run.bounds());

        // Determine the effective text options for text rendering. Start
        // with the current pushed text options.
        let mut effective_text_options = self.text_options;

        // If the text rendering mode is unspecified in the text options, use
        // the one from the render options, and without one there the edge
        // mode, as the glyph run of the Skia backend does.
        if effective_text_options.text_rendering_mode == TextRenderingMode::Unspecified {
            effective_text_options.text_rendering_mode =
                if self.render_options.text_rendering_mode != TextRenderingMode::Unspecified {
                    self.render_options.text_rendering_mode
                } else if self.render_options.edge_mode == EdgeMode::Aliased {
                    TextRenderingMode::Alias
                } else {
                    TextRenderingMode::SubpixelAntialias
                };
        }

        // No renderer of the Vello project draws text for the sub-pixels of
        // a display: the sub-pixel mode is the grey-scale one here, as it
        // is in the Skia backend where sub-pixel text is disabled.
        let anti_alias = effective_text_options.text_rendering_mode != TextRenderingMode::Alias;

        // Hinting as far as the renderer has it: the light and the strong
        // mode are its one, vertical, mode.
        let hint = effective_text_options.text_hinting_mode != TextHintingMode::None;

        // Sub-pixel positioning is enabled when the edging is not alias,
        // and the baseline is on a row of pixels unless that is disabled.
        // Both are meant in pixels: they apply while the text is upright.
        let transform = self.device_transform();
        let [a, b, c, d, e, f] = transform.as_coeffs();
        let upright = b == 0.0 && c == 0.0 && a > 0.0 && d > 0.0;

        let snap_baseline =
            upright && effective_text_options.baseline_pixel_alignment != BaselinePixelAlignment::Unaligned;
        let whole_pixels = upright && !anti_alias;

        let origin = glyph_run.baseline_origin();
        let snap_x = |x: f64| if whole_pixels { ((a * x + e).round() - e) / a } else { x };
        let snap_y = |y: f64| if snap_baseline { ((d * y + f).round() - f) / d } else { y };

        let glyphs: Vec<VelloSceneGlyph> = glyph_run_impl
            .glyph_indices()
            .iter()
            .zip(glyph_run_impl.glyph_positions())
            .map(|(glyph_index, (x, y))| VelloSceneGlyph {
                id: *glyph_index as u32,
                x: snap_x(origin.x + *x as f64) as f32,
                y: (snap_y(origin.y) + *y as f64) as f32,
            })
            .collect();

        let face = glyph_run_impl.face();
        let em_size = glyph_run.font_rendering_em_size();
        let font_simulations = face.font_simulations();

        let scene_glyph_run = VelloSceneGlyphRun {
            font: face.data(),
            font_size: em_size as f32,
            units_per_em: face.units_per_em(),
            normalized_coords: face.normalized_coord_bits(),
            glyphs: &glyphs,
            embolden: if font_simulations.contains(FontSimulations::Bold) {
                bold_simulation_outline_width(em_size) / 2.0
            } else {
                0.0
            },
            skew: if font_simulations.contains(FontSimulations::Oblique) { OBLIQUE_SKEW } else { 0.0 },
            hint,
        };

        self.draw_with(paint_wrapper, &|sink, paint, transform, _| {
            sink.draw_glyph_run(&scene_glyph_run, transform, paint, anti_alias);
        });
    }

    fn create_layer(&mut self, size: PixelSize) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(self.create_render_target(size, false))
    }

    fn push_clip(&mut self, clip: Rect) {
        // The edges of a rectangle clip are not anti-aliased, those of a
        // rounded rectangle and of a geometry are, whatever the edge mode
        // is: what the Skia backend asks of its canvas.
        self.push_clip_path(&rect_path(clip), Fill::NonZero, false);
    }

    fn push_clip_rounded(&mut self, clip: RoundedRect) {
        self.push_clip_path(&rounded_rect_path(clip), Fill::NonZero, true);
    }

    fn push_clip_region(&mut self, region: &dyn IPlatformRenderInterfaceRegion) {
        let path = Self::region_impl(region).path();

        // A region is a set of pixels of the target: it is not transformed
        // and has no edge inside a pixel.
        let (pixel_transform, anti_alias) = (self.pixel_transform(), self.edge_anti_alias(false));
        self.sink().push_clip(&path, Fill::NonZero, pixel_transform, anti_alias);
        self.save(SavedKind::Clip);
    }

    fn pop_clip(&mut self) {
        self.restore();
    }

    fn push_layer(&mut self, _bounds: Rect) {
        // The bounds are what the caller draws in: a layer of the scene is
        // as large as what is drawn into it.
        self.sink().push_layer(BlendMode::default(), 1.0);
        self.save(SavedKind::Layer);
    }

    fn pop_layer(&mut self) {
        self.restore();
    }

    fn push_opacity(&mut self, opacity: f64, _bounds: Option<Rect>) {
        let use_opacity_layer = self.uses_opacity_layer();
        self.opacity_stack.push((self.current_opacity, use_opacity_layer));

        if use_opacity_layer {
            // Take the current multiplied opacity.
            let opacity = self.current_opacity * opacity;

            // Opacity is applied via layering.
            self.current_opacity = 1.0;

            self.sink().push_layer(BlendMode::default(), opacity.clamp(0.0, 1.0) as f32);
            self.save(SavedKind::Layer);
        } else {
            self.current_opacity *= opacity;
        }
    }

    fn pop_opacity(&mut self) {
        let (opacity, used_opacity_layer) =
            self.opacity_stack.pop().unwrap_or_else(|| panic!("The opacity stack is empty"));

        if used_opacity_layer {
            self.restore();
        }

        self.current_opacity = opacity;
    }

    fn push_opacity_mask(&mut self, mask: &dyn IBrush, bounds: Rect) {
        self.sink().push_layer(BlendMode::default(), 1.0);
        self.save(SavedKind::Layer);

        let transform = self.device_transform();
        let mask_paint = self.create_paint(mask, bounds);
        self.mask_stack.push((transform, mask_paint));
    }

    fn pop_opacity_mask(&mut self) {
        let (transform, paint_wrapper) =
            self.mask_stack.pop().unwrap_or_else(|| panic!("The opacity mask stack is empty"));

        // What was drawn since the push is kept where the mask is opaque:
        // the mask is drawn over all of it and composed "destination in".
        let target = self.target_path();
        let sink = self.sink();
        sink.push_layer(BlendMode::new(Mix::Normal, Compose::DestIn), 1.0);

        if let Some(underlay) = paint_wrapper.underlay {
            sink.fill(&target, Fill::NonZero, Affine::IDENTITY, &VelloScenePaint::solid(underlay), BlendMode::default(), false);
        }
        if let Some(paint) = &paint_wrapper.paint {
            // The paint was made for the transform of the push; the path is
            // in pixels.
            let paint = VelloScenePaint { brush: paint.brush.clone(), transform: transform * paint.transform };
            match &paint_wrapper.clip {
                Some(clip) => {
                    sink.push_clip(clip, Fill::NonZero, transform, false);
                    sink.fill(&target, Fill::NonZero, Affine::IDENTITY, &paint, BlendMode::default(), false);
                    sink.pop_clip();
                }
                None => sink.fill(&target, Fill::NonZero, Affine::IDENTITY, &paint, BlendMode::default(), false),
            }
        }

        sink.pop_layer();
        paint_wrapper.dispose();

        self.restore();
    }

    fn push_geometry_clip(&mut self, clip: &dyn IGeometryImpl) {
        match Self::geometry_impl(clip).fill_path() {
            Some(fill_path) => self.push_clip_path(fill_path.path(), to_fill(fill_path.fill_rule()), true),
            None => self.save(SavedKind::Nothing),
        }
    }

    fn pop_geometry_clip(&mut self) {
        self.restore();
    }

    fn push_render_options(&mut self, render_options: RenderOptions) {
        self.render_options_stack.push(self.render_options);

        self.render_options = self.render_options.merge_with(render_options);
    }

    fn pop_render_options(&mut self) {
        self.render_options =
            self.render_options_stack.pop().unwrap_or_else(|| panic!("The render options stack is empty"));
    }

    fn push_text_options(&mut self, text_options: TextOptions) {
        self.text_options_stack.push(self.text_options);

        self.text_options = self.text_options.merge_with(text_options);
    }

    fn pop_text_options(&mut self) {
        self.text_options =
            self.text_options_stack.pop().unwrap_or_else(|| panic!("The text options stack is empty"));
    }

    fn as_drawing_context_impl_with_effects(
        &mut self,
    ) -> Option<&mut dyn ferroui_base::platform::IDrawingContextImplWithEffects> {
        Some(self)
    }

    fn as_drawing_context_with_acrylic_like_support(
        &mut self,
    ) -> Option<&mut dyn ferroui_base::platform::IDrawingContextWithAcrylicLikeSupport> {
        Some(self)
    }

    fn get_feature(&mut self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn dispose(&mut self) {
        if self.disposed {
            return;
        }
        self.disposed = true;

        // States the caller left pushed end with the frame, as the canvas of
        // Skia restores them.
        for (_, paint_wrapper) in std::mem::take(&mut self.mask_stack) {
            paint_wrapper.dispose();
        }
        while !self.state_stack.is_empty() {
            self.restore();
        }

        if let (Some(mut sink), Some(on_finished)) = (self.sink.take(), self.on_finished.take()) {
            on_finished(&mut *sink);
        }

        for disposable in std::mem::take(&mut self.disposables) {
            disposable();
        }
    }
}

/// A paint and what has to surround the drawing with it.
///
/// The paint wrapper of the Skia backend holds a paint of Skia, which can
/// compose shaders and sample an image as nothing beside itself; a paint of
/// a scene of this backend is one brush, so what such a paint of Skia does
/// in one is listed here and drawn in steps
/// ([`DrawingContextImpl::draw_with`]).
pub(crate) struct PaintWrapper {
    /// The paint, or `None` when the brush paints nothing.
    paint: Option<VelloScenePaint>,
    /// A color that is drawn before the paint, under it.
    underlay: Option<AlphaColor<Srgb>>,
    /// The opacity the underlay and the paint are composed with as one.
    layer_opacity: Option<f32>,
    /// What the paint is limited to, in the space of the shape.
    clip: Option<BezPath>,
}

impl PaintWrapper {
    fn new() -> Self {
        Self { paint: None, underlay: None, layer_opacity: None, clip: None }
    }

    /// Releases what the paint holds.
    fn dispose(self) {}
}
