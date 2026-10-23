//! The effects of the WinUI composition API (phase 6, scoped): the effect
//! descriptions an effect factory is made from (the FerroUI form of the
//! Win2D effects WinUI takes), the factory, and the loaded graph its
//! brushes draw.
//!
//! The descriptions are the effects the drawing contexts of FerroUI can
//! express: a Gaussian blur (the blur effect of the drawing context), an
//! opacity, a cross-fade of two sources, composites in every mode but
//! `MaskInvert`, a colour fill, blends, colour matrices (saturation, tint,
//! invert, grayscale, luminance to alpha), an arithmetic composite with a
//! colour, an alpha mask, a border that tiles a surface and scene lighting
//! (winui-materials.md). A graph with another effect or mode loads with the
//! status `Other`, and its brushes draw nothing.

mod composition_effect_factory;
mod effect_graph;
mod graphics_effects;

pub use composition_effect_factory::{
    CompositionEffectFactory, CompositionEffectFactoryError, CompositionEffectFactoryLoadStatus,
};
pub use effect_graph::blur_amount_to_radius;
pub(crate) use effect_graph::{
    EffectColor, EffectGraph, EffectMatrix, EffectNode, EffectNodeKind, EffectPropertyValue, EffectScalar,
};
pub use graphics_effects::{
    AlphaMaskEffect, ArithmeticCompositeEffect, BlendEffect, BlendEffectMode, BorderEffect, CanvasComposite,
    CanvasEdgeBehavior, ColorMatrixEffect, ColorSourceEffect, CompositeEffect, CompositionEffectSourceParameter,
    CrossFadeEffect, EffectBorderMode, EffectOptimization, GaussianBlurEffect, GraphicsEffect, GraphicsEffectSource,
    GrayscaleEffect, InvertEffect, LuminanceToAlphaEffect, OpacityEffect, SaturationEffect, SceneLightingEffect,
    TintEffect,
};
