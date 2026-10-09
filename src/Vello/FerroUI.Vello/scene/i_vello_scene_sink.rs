use crate::vello_options::VelloRenderingMode;
use kurbo::{Affine, BezPath, Stroke};
use peniko::color::{AlphaColor, Srgb};
use peniko::{BlendMode, Extend, Fill, FontData, Gradient, ImageData, ImageQuality};

/// What a renderer of the Vello project draws of what the drawing context
/// contract asks for. The drawing context asks before it draws something a
/// renderer may lack, and fails with the stage the feature belongs to
/// rather than drawing something else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VelloSceneCapabilities {
    /// A layer that is composed with a blend mode other than source-over.
    pub blend_layers: bool,
    /// Edges without anti-aliasing. Without them the drawing context asks
    /// for anti-aliased edges in the aliased edge mode and for the clips of
    /// rectangles; what it still draws without anti-aliasing has its edges
    /// between pixels, where the two are the same.
    pub aliased_edges: bool,
    /// Rectangles whose sides stay on the axes, drawn and clipped to without
    /// anti-aliasing: the pixels whose centers they hold. A sink that has
    /// these and no other aliased edges is still asked for edges without
    /// anti-aliasing, and draws whatever is no such rectangle anti-aliased.
    pub aliased_rectangles: bool,
    /// Images as paints, with an extend mode for each axis.
    pub image_paints: bool,
    /// The pixels of the finished scene can be read back.
    pub read_back: bool,
    /// A texture of the device of the sink as a paint
    /// ([`VelloSceneBrush::Texture`]): what was drawn on the device is drawn
    /// from without leaving it.
    pub device_textures: bool,
    /// A render that composes the scene over what its target holds
    /// ([`IVelloSceneSink::retain_target`]).
    pub retained_targets: bool,
}

/// A rectangle of pixels of a target: `x0` and `y0` are its first column
/// and row, `x1` and `y1` the ones after its last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VelloScenePixelRect {
    /// The first column.
    pub x0: u16,
    /// The first row.
    pub y0: u16,
    /// The column after the last.
    pub x1: u16,
    /// The row after the last.
    pub y1: u16,
}

impl VelloScenePixelRect {
    /// The pixels both rectangles hold, when they hold any.
    pub fn intersect(self, other: Self) -> Option<Self> {
        let rect = Self {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        };
        (rect.x0 < rect.x1 && rect.y0 < rect.y1).then_some(rect)
    }
}

/// A texture of a device as a paint: premultiplied pixels and how they are
/// sampled.
#[cfg(any(feature = "hybrid", feature = "gpu"))]
#[derive(Clone, Debug)]
pub struct VelloSceneTexture {
    /// The texture.
    pub texture: std::sync::Arc<crate::gpu::VelloDeviceTexture>,
    /// How the texture continues to the left and right of its pixels.
    pub x_extend: Extend,
    /// How the texture continues above and below its pixels.
    pub y_extend: Extend,
    /// The filter the texture is sampled with.
    pub quality: ImageQuality,
    /// A factor of the alpha of every pixel.
    pub alpha: f32,
}

/// An image as a paint: premultiplied RGBA pixels and how they are sampled.
#[derive(Clone, Debug)]
pub struct VelloSceneImage {
    /// The pixels.
    pub image: ImageData,
    /// How the image continues to the left and right of its pixels.
    pub x_extend: Extend,
    /// How the image continues above and below its pixels.
    pub y_extend: Extend,
    /// The filter the image is sampled with.
    pub quality: ImageQuality,
    /// A factor of the alpha of every pixel.
    pub alpha: f32,
}

/// What a shape is painted with.
#[derive(Clone, Debug)]
pub enum VelloSceneBrush {
    /// One color.
    Solid(AlphaColor<Srgb>),
    /// A linear, radial or sweep gradient.
    Gradient(Gradient),
    /// An image.
    Image(VelloSceneImage),
    /// A texture of the device of the sink. Only for a sink whose
    /// capabilities have device textures and whose device
    /// ([`IVelloSceneSink::device`]) is the one of the texture.
    #[cfg(any(feature = "hybrid", feature = "gpu"))]
    Texture(VelloSceneTexture),
}

/// A brush with the transform from its own space to the space of the shape
/// it paints.
#[derive(Clone, Debug)]
pub struct VelloScenePaint {
    /// The brush.
    pub brush: VelloSceneBrush,
    /// From the space of the brush to the space of the shape.
    pub transform: Affine,
}

impl VelloScenePaint {
    /// A paint of one color.
    pub fn solid(color: AlphaColor<Srgb>) -> Self {
        Self { brush: VelloSceneBrush::Solid(color), transform: Affine::IDENTITY }
    }
}

/// A glyph of a run: its index in the font and its origin on the baseline,
/// in the space of the run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VelloSceneGlyph {
    /// The index of the glyph in the font.
    pub id: u32,
    /// The horizontal position of the origin of the glyph.
    pub x: f32,
    /// The vertical position of the origin of the glyph, downwards.
    pub y: f32,
}

/// Glyphs of one font that are drawn alike.
#[derive(Clone, Copy, Debug)]
pub struct VelloSceneGlyphRun<'a> {
    /// The font file and the index of the font in it.
    pub font: &'a FontData,
    /// The em size, in the units of the space of the run.
    pub font_size: f32,
    /// The units of the em square of the font.
    pub units_per_em: u16,
    /// The position in the variation space of the font, a normalized
    /// coordinate for each axis as a fixed-point number of 14 fractional
    /// bits; empty for the default instance.
    pub normalized_coords: &'a [i16],
    /// The glyphs.
    pub glyphs: &'a [VelloSceneGlyph],
    /// By how much the outline of every glyph is widened on each side, in
    /// the units of the space of the run: the bold simulation. Zero for
    /// none.
    pub embolden: f64,
    /// The horizontal shear of every glyph about its origin, in a space of
    /// which y points down: the oblique simulation. Zero for none.
    pub skew: f64,
    /// Whether the outlines are fitted to the pixels of the target, as far
    /// as the renderer does it.
    pub hint: bool,
}

/// The scene of one frame: where the drawing context records what the
/// contract's immediate calls draw, in the form of one of the renderers of
/// the Vello project.
///
/// This is the one interface the drawing context, the brushes and the pens
/// of the backend are written against; each rendering mode implements it
/// (design document, section 4). Every call carries the state it needs: the
/// sink has no current transform, paint or stroke of its own to the caller.
///
/// Transforms map the space of the shape to the pixels of the target.
pub trait IVelloSceneSink {
    /// The mode this sink renders in.
    fn rendering_mode(&self) -> VelloRenderingMode;

    /// What the renderer of this sink draws.
    fn capabilities(&self) -> VelloSceneCapabilities;

    /// The width of the scene in pixels.
    fn width(&self) -> u16;

    /// The height of the scene in pixels.
    fn height(&self) -> u16;

    /// Forgets everything that was recorded. No clip or layer is open
    /// afterwards.
    fn reset(&mut self);

    /// Fills a path. `blend_mode` composes the fill with what is below it in
    /// the innermost layer.
    fn fill(
        &mut self,
        path: &BezPath,
        fill_rule: Fill,
        transform: Affine,
        paint: &VelloScenePaint,
        blend_mode: BlendMode,
        anti_alias: bool,
    );

    /// Strokes a path.
    fn stroke(&mut self, path: &BezPath, stroke: &Stroke, transform: Affine, paint: &VelloScenePaint, anti_alias: bool);

    /// Fills the glyphs of a run: outlines with the paint, the glyphs of a
    /// colour font with their own colours as far as the renderer draws them.
    fn draw_glyph_run(
        &mut self,
        glyph_run: &VelloSceneGlyphRun<'_>,
        transform: Affine,
        paint: &VelloScenePaint,
        anti_alias: bool,
    );

    /// Restricts drawing to the fill of a path until the matching
    /// [`pop_clip`](Self::pop_clip). The clip does not isolate what is drawn
    /// in it.
    fn push_clip(&mut self, path: &BezPath, fill_rule: Fill, transform: Affine, anti_alias: bool);

    /// Ends the innermost clip.
    fn pop_clip(&mut self);

    /// Begins a layer: what is drawn until the matching
    /// [`pop_layer`](Self::pop_layer) is drawn on its own and then composed
    /// with what is below it, with `blend_mode` and multiplied by `opacity`.
    fn push_layer(&mut self, blend_mode: BlendMode, opacity: f32);

    /// Ends the innermost layer and composes it.
    fn pop_layer(&mut self);

    /// Renders the scene into premultiplied RGBA pixels, `width() * 4` bytes
    /// a row without padding, replacing what they held. Pixels nothing was
    /// drawn to are transparent.
    ///
    /// Every clip and layer has to be ended first.
    fn render_to_pixels(&mut self, pixels: &mut [u8]);

    /// Renders the scene into a texture of the device of this sink (the
    /// texture of the drawable of a window), replacing what it held. Fails
    /// for a sink that does not draw on a device, and when the device could
    /// not draw the scene.
    ///
    /// Every clip and layer has to be ended first.
    #[cfg(any(feature = "hybrid", feature = "gpu"))]
    fn render_to_texture(&mut self, _target: &crate::gpu::VelloGpuTexture<'_>) -> Result<(), String> {
        Err(format!("The {:?} rendering mode of the Vello backend renders into memory", self.rendering_mode()))
    }

    /// The device this sink draws on, for a sink that draws on one.
    #[cfg(any(feature = "hybrid", feature = "gpu"))]
    fn device(&self) -> Option<&std::sync::Arc<crate::gpu::VelloWgpuDevice>> {
        None
    }

    /// Makes the next render compose the scene over what the target holds
    /// instead of replacing it: the pixels of `cleared` are made transparent
    /// first, every other pixel of the target stays under the scene. Called
    /// again, the rectangles add up; [`reset`](Self::reset) ends it.
    ///
    /// The scene is composed over the target as a whole: nothing in it
    /// takes anything out of what the target holds, so a destructive
    /// composition (`Copy`, `Clear`, ...) acts on what the scene itself drew
    /// below it. What has to go is named in `cleared`.
    ///
    /// Whether the target of a render can be kept is up to the one that
    /// renders: the CPU sink composes over the pixels it is given, a sink
    /// on a device over the texture it is given
    /// ([`render_to_texture`](Self::render_to_texture)), and a sink on a
    /// device that is rendered into pixels draws into a texture of its own
    /// that holds nothing.
    ///
    /// # Panics
    /// Panics in a sink whose [`capabilities`](Self::capabilities) have no
    /// retained targets.
    fn retain_target(&mut self, cleared: &[VelloScenePixelRect]) {
        let _ = cleared;
        panic!("The {:?} rendering mode of the Vello backend replaces what its target holds", self.rendering_mode());
    }

    /// Renders the scene into the drawing buffer of the canvas this sink
    /// was made for, replacing what it held. Fails for a sink that does not
    /// belong to a canvas, and when the context could not draw the scene.
    ///
    /// Every clip and layer has to be ended first.
    #[cfg(feature = "hybrid-webgl")]
    fn render_to_canvas(&mut self) -> Result<(), String> {
        Err(format!("The {:?} rendering mode of the Vello backend renders into memory", self.rendering_mode()))
    }

    /// What the renderer of this sink does of blurs. A sink that does not
    /// say draws none: the drawing context then renders what is blurred on
    /// the processor and gives the sink an image.
    fn filter_capabilities(&self) -> VelloSceneFilterCapabilities {
        VelloSceneFilterCapabilities::default()
    }

    /// Begins a layer whose content is drawn on its own, passed through
    /// `filter` and then composed source-over. `transform` maps the lengths
    /// of the filter (standard deviation, offset) to the pixels of the
    /// target. Ended by [`pop_layer`](Self::pop_layer).
    ///
    /// # Panics
    /// Panics in a sink whose
    /// [`filter_capabilities`](Self::filter_capabilities) have no filter
    /// layers.
    fn push_filter_layer(&mut self, filter: &VelloSceneFilter, transform: Affine) {
        let _ = (filter, transform);
        panic!("The {:?} rendering mode of the Vello backend has no filter layers", self.rendering_mode());
    }

    /// Fills the blur of a rounded rectangle of one circular corner radius
    /// with a color: what a Gaussian of `std_deviation` makes of the
    /// rectangle, computed in closed form. With `invert` the blur of
    /// everything but the rectangle is filled.
    ///
    /// # Panics
    /// Panics in a sink whose
    /// [`filter_capabilities`](Self::filter_capabilities) have no blurred
    /// rounded rectangles.
    fn fill_blurred_rounded_rect(
        &mut self,
        rect: kurbo::Rect,
        radius: f64,
        std_deviation: f64,
        invert: bool,
        transform: Affine,
        color: AlphaColor<Srgb>,
    ) {
        let _ = (rect, radius, std_deviation, invert, transform, color);
        panic!("The {:?} rendering mode of the Vello backend has no blurred rounded rectangles", self.rendering_mode());
    }
}

/// What a renderer does of blurs (design document, section 4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VelloSceneFilterCapabilities {
    /// Layers that are blurred or given a drop shadow when they are
    /// composed ([`IVelloSceneSink::push_filter_layer`]).
    pub filter_layers: bool,
    /// The blur of a rounded rectangle in closed form
    /// ([`IVelloSceneSink::fill_blurred_rounded_rect`]).
    pub blurred_rounded_rects: bool,
}

/// What a filter layer does to its content. Lengths are in the space of the
/// transform the layer is pushed with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VelloSceneFilter {
    /// A Gaussian blur. Beyond its content the layer is transparent.
    Blur {
        /// The standard deviation of the Gaussian.
        std_deviation: f32,
    },
    /// The content over its shadow: the alpha of the content, blurred,
    /// moved and given a color.
    DropShadow {
        /// How far the shadow is moved to the right.
        dx: f32,
        /// How far the shadow is moved down.
        dy: f32,
        /// The standard deviation of the blur of the shadow; none when 0.
        std_deviation: f32,
        /// The color of the shadow.
        color: AlphaColor<Srgb>,
    },
}
