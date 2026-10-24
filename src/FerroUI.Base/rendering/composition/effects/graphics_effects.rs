//! The effect descriptions an effect factory is created from: the FerroUI
//! form of the Win2D effects (`Microsoft.Graphics.Canvas.Effects`) that
//! WinUI's `Compositor.CreateEffectFactory` takes as an `IGraphicsEffect`.
//!
//! Deviation (winui-composition.md): WinUI takes any `IGraphicsEffect`
//! (an object that names a Direct2D effect and its properties); here an
//! effect is one of the descriptions of this module, plain values with the
//! members of the Win2D classes, and the factory accepts the ones the
//! drawing contexts can express.

use crate::media::{Color, Colors};

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

/// How a blur treats the edges of its input (Win2D `EffectBorderMode`).
///
/// Deviation (winui-composition.md): stored only; the blur of the drawing
/// contexts fades the edges (`Soft`).
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
/// Deviation (winui-composition.md): stored only; the drawing contexts have
/// one quality of blur.
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

/// How a composite effect combines its sources (Win2D `CanvasComposite`).
///
/// Only `SourceOver` is supported: an effect factory made from a composite
/// effect with another mode reports
/// [`CompositionEffectFactoryError::CompositeModeNotSupported`](super::CompositionEffectFactoryError::CompositeModeNotSupported).
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
    /// How the edges of the source are treated (stored only).
    pub border_mode: EffectBorderMode,
    /// The speed and quality trade-off (stored only).
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
    /// How the sources are combined; only `SourceOver` is supported.
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

/// Changes the saturation of its source (Win2D `SaturationEffect`).
///
/// Not supported: the drawing contexts have no colour matrix, so an effect
/// factory made from a graph with a saturation effect reports
/// [`CompositionEffectFactoryError::EffectTypeNotSupported`](super::CompositionEffectFactoryError::EffectTypeNotSupported).
#[derive(Clone, Debug, PartialEq)]
pub struct SaturationEffect {
    /// The name of the effect.
    pub name: String,
    /// The saturation, 0 (grey) to 1 (unchanged) (default 0.5).
    pub saturation: f32,
    /// The input.
    pub source: Option<GraphicsEffectSource>,
}

impl Default for SaturationEffect {
    fn default() -> Self {
        Self { name: String::new(), saturation: 0.5, source: None }
    }
}
