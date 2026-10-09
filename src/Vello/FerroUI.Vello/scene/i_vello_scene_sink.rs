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
    /// Edges without anti-aliasing.
    pub aliased_edges: bool,
    /// Images as paints, with an extend mode for each axis.
    pub image_paints: bool,
    /// The pixels of the finished scene can be read back.
    pub read_back: bool,
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
}
