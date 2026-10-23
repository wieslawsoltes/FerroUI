//! The effects of the WinUI composition API (phase 6, scoped): the effect
//! descriptions an effect factory is made from (the FerroUI form of the
//! Win2D effects WinUI takes), the factory, and the loaded graph its
//! brushes draw.
//!
//! The descriptions are the effects the drawing contexts of FerroUI can
//! express: a Gaussian blur (the blur effect of the drawing context), an
//! opacity, a cross-fade of two sources, a source-over composite and a
//! colour fill. A graph with another effect (a saturation) or mode loads
//! with the status `Other`, and its brushes draw nothing.

mod composition_effect_factory;
mod effect_graph;
mod graphics_effects;

pub use composition_effect_factory::{
    CompositionEffectFactory, CompositionEffectFactoryError, CompositionEffectFactoryLoadStatus,
};
pub use effect_graph::blur_amount_to_radius;
pub(crate) use effect_graph::{
    EffectColor, EffectGraph, EffectNode, EffectNodeKind, EffectPropertyValue, EffectScalar,
};
pub use graphics_effects::{
    CanvasComposite, ColorSourceEffect, CompositeEffect, CompositionEffectSourceParameter, CrossFadeEffect,
    EffectBorderMode, EffectOptimization, GaussianBlurEffect, GraphicsEffect, GraphicsEffectSource, OpacityEffect,
    SaturationEffect,
};
