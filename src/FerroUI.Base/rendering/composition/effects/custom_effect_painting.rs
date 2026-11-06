//! The drawing of the custom effects that are effect graphs (#455, E2; not
//! from upstream): the graph of the type ([`CustomEffect::graph`]) around
//! the content, with the painter of the composition effects, the values of
//! its members read from the snapshot.
//!
//! The render walk pushes the graph of the effect of a visual before its
//! content and pops it after its children, as a layer visual pushes its
//! effect brush around its children: the content is the source parameter
//! `content`. Nothing is compiled or allocated per frame: the graph is
//! loaded once per type, and the drawing context draws each node with the
//! layers and effects it draws the composition effects with (a colour
//! filter of a save layer on Skia; the drop shadow and blur effects), whose
//! programs the backends cache. A frame only changes their parameters.
//!
//! [`CustomEffect::graph`]: crate::media::effects::CustomEffect::graph

use super::{EffectColor, EffectScalar};
use crate::media::effects::custom_effect_graph::CustomEffectGraph;
use crate::media::effects::ImmutableCustomEffect;
use crate::media::Color;
use crate::platform::IDrawingContextImpl;
use crate::rendering::composition::brushes::{EffectGraphHost, EffectGraphPainter};
use crate::rendering::composition::server::ServerCompositor;
use crate::{Rect, Thickness};
use std::rc::Rc;

/// The values of the members of the graph of a custom effect: those of its
/// snapshot ([`CustomEffect::for_each_graph_value`](crate::media::effects::CustomEffect::for_each_graph_value)).
struct CustomEffectGraphHost<'a> {
    effect: &'a ImmutableCustomEffect,
    graph: &'a CustomEffectGraph,
    compositor: Option<Rc<ServerCompositor>>,
}

impl CustomEffectGraphHost<'_> {
    /// The value of the snapshot for the member `key`, if the member is set
    /// by one.
    fn value(&self, key: Option<&str>) -> Option<crate::media::effects::EffectParameterValue> {
        let name = self.graph.value_name_of(key?)?;
        let mut found = None;
        self.effect.for_each_graph_value(&mut |candidate, value| {
            if found.is_none() && candidate == name {
                found = Some(value);
            }
        });
        found
    }
}

impl EffectGraphHost for CustomEffectGraphHost<'_> {
    fn scalar_value(&self, value: &EffectScalar) -> f64 {
        self.value(value.key.as_deref()).and_then(|current| current.as_float()).unwrap_or(f64::from(value.value))
    }

    fn color_value(&self, value: &EffectColor) -> Color {
        self.value(value.key.as_deref()).and_then(|current| current.as_color()).unwrap_or(value.value)
    }

    fn pixel_compositor(&self) -> Option<Rc<ServerCompositor>> {
        self.compositor.clone()
    }
}

/// Pushes the graph of the custom effect `effect` around the content drawn
/// next on `canvas`, over `bounds` (the content and the padding of the
/// effect, in the coordinates of the canvas): what is under the content and
/// the layers and effects that wrap it. Returns `false`, pushing nothing,
/// when the type of the effect has no graph that loaded
/// ([`ImmutableCustomEffect::has_graph`]); then the caller pushes it as an
/// effect of the drawing context, which draws the content unchanged.
///
/// [`pop_custom_effect_graph`] is called after the content with the same
/// effect and bounds.
pub fn push_custom_effect_graph(canvas: &mut dyn IDrawingContextImpl, effect: &ImmutableCustomEffect, bounds: Rect) -> bool {
    push_custom_effect_graph_with(canvas, effect, bounds, None)
}

/// Pops what [`push_custom_effect_graph`] pushed and draws what the graph
/// draws over the content. Returns `false`, doing nothing, when the type of
/// the effect has no graph.
pub fn pop_custom_effect_graph(canvas: &mut dyn IDrawingContextImpl, effect: &ImmutableCustomEffect, bounds: Rect) -> bool {
    pop_custom_effect_graph_with(canvas, effect, bounds, None)
}

/// [`push_custom_effect_graph`] for the render walk: `compositor` draws the
/// operations on pixels a drawing context cannot do itself.
pub(crate) fn push_custom_effect_graph_with(
    canvas: &mut dyn IDrawingContextImpl,
    effect: &ImmutableCustomEffect,
    bounds: Rect,
    compositor: Option<Rc<ServerCompositor>>,
) -> bool {
    let Some(graph) = effect.graph() else { return false };
    let host = CustomEffectGraphHost { effect, graph: &graph, compositor };
    EffectGraphPainter::new(&host).push_around_layer(&graph.graph, canvas, bounds);
    true
}

/// [`pop_custom_effect_graph`] for the render walk.
pub(crate) fn pop_custom_effect_graph_with(
    canvas: &mut dyn IDrawingContextImpl,
    effect: &ImmutableCustomEffect,
    bounds: Rect,
    compositor: Option<Rc<ServerCompositor>>,
) -> bool {
    let Some(graph) = effect.graph() else { return false };
    let host = CustomEffectGraphHost { effect, graph: &graph, compositor };
    EffectGraphPainter::new(&host).pop_around_layer(&graph.graph, canvas, bounds);
    true
}

/// How far the graph of the custom effect `effect` reaches beyond its
/// content (the blurs and shadows around the content), with the values of
/// the snapshot; `None` when the type has no graph.
pub fn custom_effect_graph_padding(effect: &ImmutableCustomEffect) -> Option<Thickness> {
    let graph = effect.graph()?;
    let host = CustomEffectGraphHost { effect, graph: &graph, compositor: None };
    Some(EffectGraphPainter::new(&host).layer_padding(&graph.graph))
}
