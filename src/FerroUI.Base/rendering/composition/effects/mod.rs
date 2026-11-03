//! The effects of the WinUI composition API (phase 6, scoped): the effect
//! descriptions an effect factory is made from (the FerroUI form of the
//! Win2D effects WinUI takes), the factory, and the loaded graph its
//! brushes draw.
//!
//! The descriptions are the effects the drawing contexts of FerroUI can
//! express: a Gaussian blur (the blur effect of the drawing context), an
//! opacity, a cross-fade of two sources, composites in every mode but
//! `MaskInvert`, a colour fill, blends, colour matrices (saturation, tint,
//! invert, grayscale, sepia, linear transfer, luminance to alpha), an arithmetic composite with a
//! colour, an alpha mask, a border that tiles a surface, a matrix convolution and scene lighting
//! (winui-materials.md). A graph with another effect or mode loads with the
//! status `Other`, and its brushes draw nothing.

mod composition_effect_factory;
mod custom_effect_painting;
mod effect_graph;
mod effect_prewarm;
mod graphics_effects;

pub use composition_effect_factory::{
    CompositionEffectFactory, CompositionEffectFactoryError, CompositionEffectFactoryLoadStatus,
};
pub use custom_effect_painting::{custom_effect_graph_padding, pop_custom_effect_graph, push_custom_effect_graph};
pub(crate) use custom_effect_painting::{pop_custom_effect_graph_with, push_custom_effect_graph_with};
pub use effect_graph::blur_amount_to_radius;
pub use effect_prewarm::{effect_prewarm_names, register_effect_prewarm};
pub(crate) use effect_prewarm::{effect_prewarm_generation, effect_prewarm_since, register_effect_graph_prewarm};
pub(crate) use effect_graph::{
    EffectColor, EffectGraph, EffectMatrix, EffectMemberKind, EffectNode, EffectNodeKind, EffectPropertyValue,
    EffectScalar, PixelOp,
};
pub use graphics_effects::{
    AlphaMaskEffect, ArithmeticCompositeEffect, BlendEffect, BlendEffectMode, BorderEffect, CanvasComposite,
    CanvasEdgeBehavior, ColorMatrixEffect, ColorSourceEffect, CompositeEffect, CompositionEffectSourceParameter,
    ConvolveMatrixEffect, CrossFadeEffect, EffectBorderMode, EffectOptimization, GaussianBlurEffect, GraphicsEffect,
    GraphicsEffectSource, GrayscaleEffect, InvertEffect, LinearTransferEffect, LuminanceToAlphaEffect, OpacityEffect,
    SaturationEffect, SceneLightingEffect, SepiaEffect, ShadowEffect, TintEffect,
};
