//! `CompositionEffectFactory` and the factories of the effect and backdrop
//! brushes (`Compositor.CreateEffectFactory`, `CreateBackdropBrush` of
//! `Microsoft.UI.Composition`, phase 6).

use super::effect_graph::EffectGraph;
use super::{CanvasComposite, GraphicsEffect};
use crate::rendering::composition::{CompositionBackdropBrush, CompositionEffectBrush, Compositor};
use std::rc::Rc;
use std::sync::Arc;

/// Whether an effect factory loaded its effect (WinUI
/// `Microsoft.UI.Composition.CompositionEffectFactoryLoadStatus`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompositionEffectFactoryLoadStatus {
    /// The effect was loaded; its brushes draw it.
    #[default]
    Success = 0,
    /// The graph has too many effects or is nested too deeply.
    EffectTooComplex = 1,
    /// The effect is still loading (never reported: loading is immediate).
    Pending = 2,
    /// The effect cannot be loaded: an unsupported effect type or composite
    /// mode, a missing source, an animatable property that does not exist.
    /// [`CompositionEffectFactory::extended_error`] says which.
    Other = -1,
}

/// Why an effect factory did not load its effect: the `ExtendedError` of
/// WinUI's factory, as a value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositionEffectFactoryError {
    /// An effect of the graph is of a type the drawing contexts cannot
    /// draw (the name of its Win2D class).
    EffectTypeNotSupported(&'static str),
    /// A composite effect has a mode other than `SourceOver`.
    CompositeModeNotSupported(CanvasComposite),
    /// An effect of the graph has an input that is not set.
    MissingSource {
        /// The Win2D class of the effect.
        effect: &'static str,
        /// The name of the effect.
        name: String,
    },
    /// An animatable property name does not name an animatable property of
    /// a named effect of the graph (`EffectName.PropertyName`).
    InvalidAnimatableProperty(String),
    /// The graph has too many effects or is nested too deeply.
    EffectTooComplex,
}

impl std::fmt::Display for CompositionEffectFactoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompositionEffectFactoryError::EffectTypeNotSupported(effect) => {
                write!(f, "the effect type {effect} is not supported")
            }
            CompositionEffectFactoryError::CompositeModeNotSupported(mode) => {
                write!(f, "the composite mode {mode:?} is not supported")
            }
            CompositionEffectFactoryError::MissingSource { effect, name } => {
                write!(f, "the {effect} '{name}' has an input that is not set")
            }
            CompositionEffectFactoryError::InvalidAnimatableProperty(key) => {
                write!(f, "'{key}' is not an animatable property of the effect")
            }
            CompositionEffectFactoryError::EffectTooComplex => f.write_str("the effect graph is too complex"),
        }
    }
}

impl std::error::Error for CompositionEffectFactoryError {}

/// Creates the brushes of one effect graph (WinUI
/// `Microsoft.UI.Composition.CompositionEffectFactory`), made by
/// [`Compositor::create_effect_factory`].
///
/// The graph is checked when the factory is made: [`load_status`](Self::load_status)
/// is `Success` when the brushes can draw it. A factory whose effect cannot
/// be loaded still makes brushes, which draw nothing.
///
/// Deviation (winui-composition.md): WinUI throws from `CreateEffectFactory`
/// for an effect it cannot load; here the factory is made and reports the
/// failure through its load status and extended error. The factory is not a
/// composition object (no `Properties`, no animations).
pub struct CompositionEffectFactory {
    compositor: Rc<Compositor>,
    graph: Arc<EffectGraph>,
    load_status: CompositionEffectFactoryLoadStatus,
    extended_error: Option<CompositionEffectFactoryError>,
}

impl CompositionEffectFactory {
    fn new(compositor: Rc<Compositor>, effect: &GraphicsEffect, animatable_properties: &[&str]) -> Rc<Self> {
        let (graph, load_status, extended_error) = match EffectGraph::load(effect, animatable_properties) {
            Ok(graph) => (graph, CompositionEffectFactoryLoadStatus::Success, None),
            Err((status, error)) => (EffectGraph::failed(), status, Some(error)),
        };
        Rc::new(Self { compositor, graph: Arc::new(graph), load_status, extended_error })
    }

    /// The compositor of the factory.
    pub fn compositor(&self) -> &Rc<Compositor> {
        &self.compositor
    }

    /// `CreateBrush`: a brush that draws the effect. Its source parameters
    /// are bound with
    /// [`set_source_parameter`](CompositionEffectBrush::set_source_parameter);
    /// its animatable properties are entries of its `Properties`, set to the
    /// values of the description.
    pub fn create_brush(&self) -> CompositionEffectBrush {
        CompositionEffectBrush::new(&self.compositor, self.graph.clone())
    }

    /// `LoadStatus`: whether the effect was loaded.
    pub fn load_status(&self) -> CompositionEffectFactoryLoadStatus {
        self.load_status
    }

    /// `ExtendedError`: why the effect was not loaded; `None` when it was.
    pub fn extended_error(&self) -> Option<&CompositionEffectFactoryError> {
        self.extended_error.as_ref()
    }
}

impl Compositor {
    /// `CreateEffectFactory(graphicsEffect)`: a factory of brushes that draw
    /// `effect`, without animatable properties.
    pub fn create_effect_factory(&self, effect: impl Into<GraphicsEffect>) -> Rc<CompositionEffectFactory> {
        self.create_effect_factory_with_properties(effect, &[])
    }

    /// `CreateEffectFactory(graphicsEffect, animatableProperties)`: a factory
    /// of brushes that draw `effect`, whose properties named in
    /// `animatable_properties` (`EffectName.PropertyName`, as
    /// `Blur.BlurAmount`) can be changed and animated on the brushes.
    pub fn create_effect_factory_with_properties(
        &self,
        effect: impl Into<GraphicsEffect>,
        animatable_properties: &[&str],
    ) -> Rc<CompositionEffectFactory> {
        CompositionEffectFactory::new(self.this_handle(), &effect.into(), animatable_properties)
    }

    /// `CreateBackdropBrush`: a brush that shows what is behind the visual.
    ///
    /// Deviation (winui-composition.md): the drawing contexts cannot read
    /// back what was drawn under a rectangle, so the brush draws nothing
    /// (also as a source of an effect).
    pub fn create_backdrop_brush(&self) -> CompositionBackdropBrush {
        CompositionBackdropBrush::new(&self.this_handle())
    }
}
