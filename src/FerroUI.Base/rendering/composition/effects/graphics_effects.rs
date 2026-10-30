//! The effect descriptions an effect factory is created from: the FerroUI
//! form of the Win2D effects (`Microsoft.Graphics.Canvas.Effects`) that
//! WinUI's `Compositor.CreateEffectFactory` takes as an `IGraphicsEffect`.
//!
//! Deviation (winui-composition.md): WinUI takes any `IGraphicsEffect`
//! (an object that names a Direct2D effect and its properties); here an
//! effect is one of the descriptions of this module, plain values with the
//! members of the Win2D classes, and the factory accepts the ones the
//! drawing contexts can express.

use crate::media::{Color, ColorMatrix, ColorMatrixAlphaMode, Colors};

/// A named leaf of an effect graph that a brush is bound to (WinUI
/// `Microsoft.UI.Composition.CompositionEffectSourceParameter`): the input
/// of the effect that
/// [`CompositionEffectBrush::set_source_parameter`](crate::rendering::composition::CompositionEffectBrush::set_source_parameter)
/// fills with a composition brush, or the composed children of the layer
/// visual whose effect the brush is.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CompositionEffectSourceParameter {
    name: String,
}

impl CompositionEffectSourceParameter {
    /// `CompositionEffectSourceParameter(name)`.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    /// The name the brush is bound by.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// How a blur treats the edges of its input (Win2D `EffectBorderMode`):
/// `Soft` fades the edges (the blur reads transparent pixels beyond its
/// area), `Hard` keeps them (the blur reads its source beyond the area and
/// its output is cut to the area).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum EffectBorderMode {
    /// The edges fade to transparent (the Win2D default).
    #[default]
    Soft = 0,
    /// The edge pixels are repeated.
    Hard = 1,
}

/// The speed and quality trade-off of an effect (Win2D
/// `EffectOptimization`).
///
/// Deviation (winui-materials.md): the blurs of the drawing contexts are
/// exact at every setting; `Speed` and `Balanced` draw the output of
/// `Quality` (Direct2D's approximations are not reproduced).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum EffectOptimization {
    /// The fastest form.
    Speed = 0,
    /// A balance of speed and quality (the Win2D default).
    #[default]
    Balanced = 1,
    /// The best quality.
    Quality = 2,
}

/// How a composite effect combines its sources (Win2D `CanvasComposite`):
/// each source is composed onto what the sources before it made.
///
/// Every mode is drawn: the Porter-Duff modes with the blend layers of the
/// drawing contexts, `MaskInvert` from the pixels of the sources drawn
/// apart (winui-materials.md).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CanvasComposite {
    /// Each source drawn over the previous ones (the Win2D default).
    #[default]
    SourceOver = 0,
    DestinationOver = 1,
    SourceIn = 2,
    DestinationIn = 3,
    SourceOut = 4,
    DestinationOut = 5,
    SourceAtop = 6,
    DestinationAtop = 7,
    Xor = 8,
    Add = 9,
    Copy = 10,
    BoundedCopy = 11,
    MaskInvert = 12,
}

/// An input of an effect (WinUI `IGraphicsEffectSource`): a source
/// parameter that a brush is bound to, or another effect.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphicsEffectSource {
    /// A named leaf bound to a brush.
    Parameter(CompositionEffectSourceParameter),
    /// The output of another effect.
    Effect(Box<GraphicsEffect>),
}

impl From<CompositionEffectSourceParameter> for GraphicsEffectSource {
    fn from(parameter: CompositionEffectSourceParameter) -> Self {
        GraphicsEffectSource::Parameter(parameter)
    }
}

impl From<GraphicsEffect> for GraphicsEffectSource {
    fn from(effect: GraphicsEffect) -> Self {
        GraphicsEffectSource::Effect(Box::new(effect))
    }
}

/// An effect description (WinUI `IGraphicsEffect`): one of the Win2D
/// effects of this module.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphicsEffect {
    GaussianBlur(GaussianBlurEffect),
    Opacity(OpacityEffect),
    CrossFade(CrossFadeEffect),
    Composite(CompositeEffect),
    ColorSource(ColorSourceEffect),
    Saturation(SaturationEffect),
    Blend(BlendEffect),
    ColorMatrix(ColorMatrixEffect),
    Tint(TintEffect),
    Invert(InvertEffect),
    Grayscale(GrayscaleEffect),
    LuminanceToAlpha(LuminanceToAlphaEffect),
    ArithmeticComposite(ArithmeticCompositeEffect),
    AlphaMask(AlphaMaskEffect),
    Border(BorderEffect),
    SceneLighting(SceneLightingEffect),
    Shadow(ShadowEffect),
}

impl GraphicsEffect {
    /// The name of the effect (`IGraphicsEffect.Name`): the first part of
    /// the animatable property names of the effect (`Blur.BlurAmount`).
    pub fn name(&self) -> &str {
        match self {
            GraphicsEffect::GaussianBlur(effect) => &effect.name,
            GraphicsEffect::Opacity(effect) => &effect.name,
            GraphicsEffect::CrossFade(effect) => &effect.name,
            GraphicsEffect::Composite(effect) => &effect.name,
            GraphicsEffect::ColorSource(effect) => &effect.name,
            GraphicsEffect::Saturation(effect) => &effect.name,
            GraphicsEffect::Blend(effect) => &effect.name,
            GraphicsEffect::ColorMatrix(effect) => &effect.name,
            GraphicsEffect::Tint(effect) => &effect.name,
            GraphicsEffect::Invert(effect) => &effect.name,
            GraphicsEffect::Grayscale(effect) => &effect.name,
            GraphicsEffect::LuminanceToAlpha(effect) => &effect.name,
            GraphicsEffect::ArithmeticComposite(effect) => &effect.name,
            GraphicsEffect::AlphaMask(effect) => &effect.name,
            GraphicsEffect::Border(effect) => &effect.name,
            GraphicsEffect::SceneLighting(effect) => &effect.name,
            GraphicsEffect::Shadow(effect) => &effect.name,
        }
    }

    /// The name of the Win2D class of the effect, for messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            GraphicsEffect::GaussianBlur(_) => "GaussianBlurEffect",
            GraphicsEffect::Opacity(_) => "OpacityEffect",
            GraphicsEffect::CrossFade(_) => "CrossFadeEffect",
            GraphicsEffect::Composite(_) => "CompositeEffect",
            GraphicsEffect::ColorSource(_) => "ColorSourceEffect",
            GraphicsEffect::Saturation(_) => "SaturationEffect",
            GraphicsEffect::Blend(_) => "BlendEffect",
            GraphicsEffect::ColorMatrix(_) => "ColorMatrixEffect",
            GraphicsEffect::Tint(_) => "TintEffect",
            GraphicsEffect::Invert(_) => "InvertEffect",
            GraphicsEffect::Grayscale(_) => "GrayscaleEffect",
            GraphicsEffect::LuminanceToAlpha(_) => "LuminanceToAlphaEffect",
            GraphicsEffect::ArithmeticComposite(_) => "ArithmeticCompositeEffect",
            GraphicsEffect::AlphaMask(_) => "AlphaMaskEffect",
            GraphicsEffect::Border(_) => "BorderEffect",
            GraphicsEffect::SceneLighting(_) => "SceneLightingEffect",
            GraphicsEffect::Shadow(_) => "ShadowEffect",
        }
    }
}

macro_rules! graphics_effect_conversions {
    ($($ty:ident => $variant:ident),* $(,)?) => {$(
        impl From<$ty> for GraphicsEffect {
            fn from(effect: $ty) -> Self {
                GraphicsEffect::$variant(effect)
            }
        }

        impl From<$ty> for GraphicsEffectSource {
            fn from(effect: $ty) -> Self {
                GraphicsEffectSource::Effect(Box::new(GraphicsEffect::$variant(effect)))
            }
        }
    )*};
}

graphics_effect_conversions! {
    GaussianBlurEffect => GaussianBlur,
    OpacityEffect => Opacity,
    CrossFadeEffect => CrossFade,
    CompositeEffect => Composite,
    ColorSourceEffect => ColorSource,
    SaturationEffect => Saturation,
    BlendEffect => Blend,
    ColorMatrixEffect => ColorMatrix,
    TintEffect => Tint,
    InvertEffect => Invert,
    GrayscaleEffect => Grayscale,
    LuminanceToAlphaEffect => LuminanceToAlpha,
    ArithmeticCompositeEffect => ArithmeticComposite,
    AlphaMaskEffect => AlphaMask,
    BorderEffect => Border,
    ShadowEffect => Shadow,
    SceneLightingEffect => SceneLighting,
}

/// Blurs its source (Win2D `GaussianBlurEffect`). Animatable:
/// `BlurAmount`.
#[derive(Clone, Debug, PartialEq)]
pub struct GaussianBlurEffect {
    /// The name of the effect.
    pub name: String,
    /// The standard deviation of the blur, in device independent pixels
    /// (default 3).
    pub blur_amount: f32,
    /// How the edges of the source are treated.
    pub border_mode: EffectBorderMode,
    /// The speed and quality trade-off (every setting draws the exact blur).
    pub optimization: EffectOptimization,
    /// The input.
    pub source: Option<GraphicsEffectSource>,
}

impl Default for GaussianBlurEffect {
    fn default() -> Self {
        Self {
            name: String::new(),
            blur_amount: 3.0,
            border_mode: EffectBorderMode::Soft,
            optimization: EffectOptimization::Balanced,
            source: None,
        }
    }
}

/// Multiplies the alpha of its source (Win2D `OpacityEffect`). Animatable:
/// `Opacity`.
#[derive(Clone, Debug, PartialEq)]
pub struct OpacityEffect {
    /// The name of the effect.
    pub name: String,
    /// The opacity, 0 to 1 (default 1).
    pub opacity: f32,
    /// The input.
    pub source: Option<GraphicsEffectSource>,
}

impl Default for OpacityEffect {
    fn default() -> Self {
        Self { name: String::new(), opacity: 1.0, source: None }
    }
}

/// Blends two sources by a weight (Win2D `CrossFadeEffect`): `source1` at
/// weight 0, `source2` at weight 1. Animatable: `Weight` (also as
/// `CrossFade`, the name of the Win2D member).
#[derive(Clone, Debug, PartialEq)]
pub struct CrossFadeEffect {
    /// The name of the effect.
    pub name: String,
    /// The weight of `source2`, 0 to 1 (default 0.5).
    pub weight: f32,
    /// The input shown at weight 0.
    pub source1: Option<GraphicsEffectSource>,
    /// The input shown at weight 1.
    pub source2: Option<GraphicsEffectSource>,
}

impl Default for CrossFadeEffect {
    fn default() -> Self {
        Self { name: String::new(), weight: 0.5, source1: None, source2: None }
    }
}

/// Draws its sources over each other, in order (Win2D `CompositeEffect`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompositeEffect {
    /// The name of the effect.
    pub name: String,
    /// How the sources are combined.
    pub mode: CanvasComposite,
    /// The inputs, the first at the bottom.
    pub sources: Vec<GraphicsEffectSource>,
}

/// Fills the area with a colour (Win2D `ColorSourceEffect`). Animatable:
/// `Color`.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorSourceEffect {
    /// The name of the effect.
    pub name: String,
    /// The colour (default opaque black).
    pub color: Color,
}

impl Default for ColorSourceEffect {
    fn default() -> Self {
        Self { name: String::new(), color: Colors::BLACK }
    }
}

/// Changes the saturation of its source (Win2D `SaturationEffect`): a
/// colour matrix (`ColorMatrix::saturation`). Animatable: `Saturation`.
#[derive(Clone, Debug, PartialEq)]
pub struct SaturationEffect {
    /// The name of the effect.
    pub name: String,
    /// The saturation, 0 (grey) to 1 (unchanged), up to 2 (more saturated)
    /// (default 0.5).
    pub saturation: f32,
    /// The input.
    pub source: Option<GraphicsEffectSource>,
}

impl Default for SaturationEffect {
    fn default() -> Self {
        Self { name: String::new(), saturation: 0.5, source: None }
    }
}

/// How a blend effect mixes its foreground into its background (Win2D
/// `BlendEffectMode`, the modes of Direct2D's blend effect).
///
/// The modes with a counterpart among the blend modes of the drawing
/// contexts are drawn: `Multiply`, `Screen`, `Darken`, `Lighten`,
/// `ColorBurn`, `ColorDodge`, `LinearDodge` (an addition), `Overlay`,
/// `SoftLight`, `HardLight`, `Difference`, `Exclusion`, `Hue`, `Saturation`,
/// `Color` and `Luminosity`. The other modes are drawn from the pixels of the
/// two sources (docs/porting/winui-materials.md).
///
/// `Color` keeps the hue and saturation of the foreground and the
/// luminosity of the background; `Luminosity` the other way round, as Win2D
/// documents them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum BlendEffectMode {
    /// The default of Win2D.
    #[default]
    Multiply = 0,
    Screen = 1,
    Darken = 2,
    Lighten = 3,
    Dissolve = 4,
    ColorBurn = 5,
    LinearBurn = 6,
    DarkerColor = 7,
    LighterColor = 8,
    ColorDodge = 9,
    LinearDodge = 10,
    Overlay = 11,
    SoftLight = 12,
    HardLight = 13,
    VividLight = 14,
    LinearLight = 15,
    PinLight = 16,
    HardMix = 17,
    Difference = 18,
    Exclusion = 19,
    Hue = 20,
    Saturation = 21,
    Color = 22,
    Luminosity = 23,
    Subtract = 24,
    Division = 25,
}

/// Mixes a foreground into a background (Win2D `BlendEffect`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlendEffect {
    /// The name of the effect.
    pub name: String,
    /// How the two are mixed (default `Multiply`).
    pub mode: BlendEffectMode,
    /// The input under the foreground (Direct2D's destination).
    pub background: Option<GraphicsEffectSource>,
    /// The input mixed into the background (Direct2D's source).
    pub foreground: Option<GraphicsEffectSource>,
}

/// Transforms the colours of its source by a matrix (Win2D
/// `ColorMatrixEffect`).
#[derive(Clone, Debug, PartialEq)]
pub struct ColorMatrixEffect {
    /// The name of the effect.
    pub name: String,
    /// The matrix (default the identity).
    pub color_matrix: ColorMatrix,
    /// Whether the matrix applies to premultiplied or straight colours
    /// (default premultiplied).
    pub alpha_mode: ColorMatrixAlphaMode,
    /// Whether the output is clamped to 0..1 (default `false`; the drawing
    /// contexts always clamp).
    pub clamp_output: bool,
    /// The input.
    pub source: Option<GraphicsEffectSource>,
}

impl Default for ColorMatrixEffect {
    fn default() -> Self {
        Self {
            name: String::new(),
            color_matrix: ColorMatrix::IDENTITY,
            alpha_mode: ColorMatrixAlphaMode::Premultiplied,
            clamp_output: false,
            source: None,
        }
    }
}

/// Multiplies the channels of its source by a colour (Win2D `TintEffect`).
/// Animatable: `Color`.
#[derive(Clone, Debug, PartialEq)]
pub struct TintEffect {
    /// The name of the effect.
    pub name: String,
    /// The colour (default opaque white).
    pub color: Color,
    /// Whether the output is clamped (stored; the drawing contexts clamp).
    pub clamp_output: bool,
    /// The input.
    pub source: Option<GraphicsEffectSource>,
}

impl Default for TintEffect {
    fn default() -> Self {
        Self { name: String::new(), color: Colors::WHITE, clamp_output: false, source: None }
    }
}

/// Inverts the colours of its source (Win2D `InvertEffect`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InvertEffect {
    /// The name of the effect.
    pub name: String,
    /// The input.
    pub source: Option<GraphicsEffectSource>,
}

/// Turns its source grey (Win2D `GrayscaleEffect`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GrayscaleEffect {
    /// The name of the effect.
    pub name: String,
    /// The input.
    pub source: Option<GraphicsEffectSource>,
}

/// Turns the luminance of its source into alpha (Win2D
/// `LuminanceToAlphaEffect`): black with the luminance as its alpha.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LuminanceToAlphaEffect {
    /// The name of the effect.
    pub name: String,
    /// The input.
    pub source: Option<GraphicsEffectSource>,
}

/// Combines two sources channel by channel as
/// `MultiplyAmount * s1 * s2 + Source1Amount * s1 + Source2Amount * s2 + Offset`
/// on premultiplied colours (Win2D `ArithmeticCompositeEffect`).
/// Animatable: `MultiplyAmount`, `Source1Amount`, `Source2Amount`, `Offset`.
///
/// Drawn when one of the sources is a colour source: the combination is
/// then a colour matrix of the other source. A factory made from one whose
/// sources are both images reports
/// [`CompositionEffectFactoryError::EffectTypeNotSupported`](super::CompositionEffectFactoryError::EffectTypeNotSupported).
#[derive(Clone, Debug, PartialEq)]
pub struct ArithmeticCompositeEffect {
    /// The name of the effect.
    pub name: String,
    /// The weight of the product of the sources (default 1).
    pub multiply_amount: f32,
    /// The weight of the first source (default 0).
    pub source1_amount: f32,
    /// The weight of the second source (default 0).
    pub source2_amount: f32,
    /// The constant added (default 0).
    pub offset: f32,
    /// Whether the output is clamped (stored; the drawing contexts clamp).
    pub clamp_output: bool,
    /// The first input.
    pub source1: Option<GraphicsEffectSource>,
    /// The second input.
    pub source2: Option<GraphicsEffectSource>,
}

impl Default for ArithmeticCompositeEffect {
    fn default() -> Self {
        Self {
            name: String::new(),
            multiply_amount: 1.0,
            source1_amount: 0.0,
            source2_amount: 0.0,
            offset: 0.0,
            clamp_output: false,
            source1: None,
            source2: None,
        }
    }
}

/// Keeps its source where its mask is opaque (Win2D `AlphaMaskEffect`):
/// the source multiplied by the alpha of the mask.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AlphaMaskEffect {
    /// The name of the effect.
    pub name: String,
    /// The input.
    pub source: Option<GraphicsEffectSource>,
    /// The mask.
    pub alpha_mask: Option<GraphicsEffectSource>,
}

/// How a border effect extends its source beyond its edges (Win2D
/// `CanvasEdgeBehavior`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CanvasEdgeBehavior {
    /// The edge pixels repeat (the Win2D default).
    #[default]
    Clamp = 0,
    /// The source tiles.
    Wrap = 1,
    /// The source tiles, every other tile mirrored.
    Mirror = 2,
}

/// Extends its source beyond its edges (Win2D `BorderEffect`): a surface
/// brush source tiles over the area of the brush when an edge behaviour is
/// `Wrap` (or `Mirror`, drawn as `Wrap`); any other source, or `Clamp` on
/// both sides, is drawn as it is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BorderEffect {
    /// The name of the effect.
    pub name: String,
    /// The horizontal behaviour.
    pub extend_x: CanvasEdgeBehavior,
    /// The vertical behaviour.
    pub extend_y: CanvasEdgeBehavior,
    /// The input.
    pub source: Option<GraphicsEffectSource>,
}

/// The lighting of the lights that reach the visual that paints the brush
/// (WinUI `Microsoft.UI.Composition.Effects.SceneLightingEffect`): the sum,
/// for every point of the brush, of the ambient lights times `AmbientAmount`
/// and of the diffuse light of the other lights times `DiffuseAmount`,
/// opaque. Animatable: `AmbientAmount`, `DiffuseAmount`, `SpecularAmount`,
/// `SpecularShine`.
///
/// The lights are the composition lights that target the visual (or one of
/// its ancestors), and, for a XAML brush, the XAML lights that target the
/// brush in the subtree of the element that holds them.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneLightingEffect {
    /// The name of the effect.
    pub name: String,
    /// The weight of the ambient lights (default 1).
    pub ambient_amount: f32,
    /// The weight of the diffuse light (default 1).
    pub diffuse_amount: f32,
    /// The weight of the specular light (default 0; stored, flat visuals
    /// have no specular highlight here).
    pub specular_amount: f32,
    /// The shininess of the specular light (default 16; stored).
    pub specular_shine: f32,
    /// The normal map (stored; the lit surface is flat).
    pub normal_map_source: Option<GraphicsEffectSource>,
}

impl Default for SceneLightingEffect {
    fn default() -> Self {
        Self {
            name: String::new(),
            ambient_amount: 1.0,
            diffuse_amount: 1.0,
            specular_amount: 0.0,
            specular_shine: 16.0,
            normal_map_source: None,
        }
    }
}

/// The shadow of its source (Win2D `ShadowEffect`): the alpha of the source
/// in the shadow colour, blurred. Animatable: `BlurAmount`, `ShadowColor`.
#[derive(Clone, Debug, PartialEq)]
pub struct ShadowEffect {
    /// The name of the effect.
    pub name: String,
    /// The standard deviation of the blur, in device independent pixels
    /// (default 1).
    pub blur_amount: f32,
    /// The colour of the shadow (default opaque black); its alpha scales the
    /// alpha of the source.
    pub shadow_color: Color,
    /// The speed and quality trade-off (every setting draws the exact blur).
    pub optimization: EffectOptimization,
    /// The input.
    pub source: Option<GraphicsEffectSource>,
}

impl Default for ShadowEffect {
    fn default() -> Self {
        Self {
            name: String::new(),
            blur_amount: 1.0,
            shadow_color: Color::from_argb(255, 0, 0, 0),
            optimization: EffectOptimization::Balanced,
            source: None,
        }
    }
}
