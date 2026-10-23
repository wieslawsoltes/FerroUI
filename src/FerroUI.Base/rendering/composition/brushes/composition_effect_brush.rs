//! The WinUI `CompositionEffectBrush` and `CompositionBackdropBrush` (phase
//! 6, not from upstream): composition brushes painted by the visuals that
//! show them, as the surface brush.

use super::composition_brush::{CompositionBrush, CompositionBrushKind};
use super::server_composition_effect_brush::EffectBrushChanges;
use super::{ServerCompositionBackdropBrush, ServerCompositionEffectBrush};
use crate::rendering::composition::effects::{EffectGraph, EffectPropertyValue};
use crate::rendering::composition::generated::CompositionBrushProps;
use crate::rendering::composition::transport::{BatchStreamWriter, IRegisterForSerialization};
use crate::rendering::composition::{AsCompositionObject, CompositionPropertySet, Compositor};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

/// What a `CompositionEffectBrush` adds to the generated
/// `CompositionBrush`: the graph of its factory and the brushes bound to its
/// source parameters.
pub(crate) struct CompositionEffectBrushData {
    pub(super) brush: CompositionBrushProps,
    pub(super) graph: Arc<EffectGraph>,
    /// The brushes bound to the source parameters, by name.
    sources: RefCell<Vec<(String, Option<Rc<CompositionBrush>>)>>,
    sources_changed: Cell<bool>,
    /// Whether the server side knows the property set of the brush.
    properties_sent: Cell<bool>,
}

impl CompositionEffectBrushData {
    fn new(graph: Arc<EffectGraph>) -> Self {
        let sources = graph.parameters.iter().map(|name| (name.clone(), None)).collect();
        Self {
            brush: CompositionBrushProps::new(),
            graph,
            sources: RefCell::new(sources),
            sources_changed: Cell::new(false),
            properties_sent: Cell::new(false),
        }
    }

    /// `SerializeChangesCore`: the generated base class, then the property
    /// set of the brush (once) and the bound brushes (when they changed).
    pub(super) fn serialize_changes_core(&self, host: &CompositionBrush, writer: &mut BatchStreamWriter<'_>) {
        self.brush.serialize_changes_core(host, writer);
        let properties = if self.properties_sent.replace(true) {
            None
        } else {
            host.object().properties().as_composition_object().server()
        };
        let sources = self.sources_changed.replace(false).then(|| {
            self.sources
                .borrow()
                .iter()
                .map(|(name, brush)| (name.clone(), brush.as_ref().map(|brush| brush.server())))
                .collect::<Vec<_>>()
        });
        let changes = (properties.is_some() || sources.is_some()).then_some(EffectBrushChanges { properties, sources });
        writer.write_value(changes);
    }
}

/// A brush that draws an effect graph over its sources (WinUI
/// `Microsoft.UI.Composition.CompositionEffectBrush`), made by
/// [`CompositionEffectFactory::create_brush`](crate::rendering::composition::effects::CompositionEffectFactory::create_brush).
///
/// The source parameters of the graph are bound to composition brushes
/// with [`set_source_parameter`](Self::set_source_parameter); the brush
/// draws the graph over the area of the visual that shows it: a Gaussian
/// blur is the blur effect of the drawing context around its source
/// (nothing is blurred on a drawing context without effects), an opacity
/// pushes an opacity, a cross-fade paints `source1` with `1 - weight` and
/// `source2` with `weight` over it, a composite paints its sources in
/// order, a colour source fills the area.
///
/// The properties named as animatable when the factory was made
/// (`Blur.BlurAmount`, `Crossfade.Weight`) are entries of
/// [`properties`](Self::properties), set to the values of the description:
/// a change of an entry, or an animation of it (`start_animation` on the
/// brush or on its properties), redraws the brush.
///
/// As the effect of a [`LayerVisual`](crate::rendering::composition::LayerVisual),
/// the first source parameter of the graph is the composed children of the
/// layer.
///
/// Deviation (winui-composition.md): the cross-fade draws its second source
/// over the first, each with its weight, rather than adding them; animations
/// of the effect properties under an animation controller are started on
/// the properties, not on the brush.
#[derive(Clone)]
pub struct CompositionEffectBrush(Rc<CompositionBrush>);

impl Deref for CompositionEffectBrush {
    type Target = Rc<CompositionBrush>;

    fn deref(&self) -> &Rc<CompositionBrush> {
        &self.0
    }
}

impl CompositionEffectBrush {
    pub(crate) fn new(compositor: &Rc<Compositor>, graph: Arc<EffectGraph>) -> CompositionEffectBrush {
        let server_graph = graph.clone();
        let brush = CompositionBrush::create(
            compositor,
            CompositionBrushKind::Effect(CompositionEffectBrushData::new(graph.clone())),
            move |c| ServerCompositionEffectBrush::new(c, server_graph),
        );
        brush.initialize_defaults();
        // The animatable properties are entries of `Properties`, with the
        // values of the description.
        let properties = brush.object().properties();
        for (key, value) in &graph.animatable {
            match *value {
                EffectPropertyValue::Scalar(value) => properties.insert_scalar(key, value),
                EffectPropertyValue::Color(value) => properties.insert_color(key, value),
            }
        }
        brush.register_for_serialization();
        CompositionEffectBrush(brush)
    }

    /// The handle of `brush` as an effect brush, if it is one.
    pub fn from_brush(brush: &Rc<CompositionBrush>) -> Option<CompositionEffectBrush> {
        matches!(brush.kind, CompositionBrushKind::Effect(_)).then(|| CompositionEffectBrush(brush.clone()))
    }

    fn data(&self) -> &CompositionEffectBrushData {
        match &self.0.kind {
            CompositionBrushKind::Effect(data) => data,
            _ => unreachable!("an effect brush handle wraps an effect brush"),
        }
    }

    /// The brush as a composition brush.
    pub fn brush(&self) -> &Rc<CompositionBrush> {
        &self.0
    }

    /// `Properties`: the property set whose entries are the animatable
    /// properties of the effect (`Blur.BlurAmount`). Inserting a value of
    /// such an entry or animating it changes what the brush draws; other
    /// entries are the application's.
    pub fn properties(&self) -> Rc<CompositionPropertySet> {
        self.0.object().properties()
    }

    /// The names of the source parameters of the graph, in graph order
    /// (empty when the factory did not load its effect).
    pub fn source_parameter_names(&self) -> Vec<String> {
        self.data().graph.parameters.clone()
    }

    /// `GetSourceParameter(name)`: the brush bound to the source parameter
    /// `name`.
    pub fn get_source_parameter(&self, name: &str) -> Option<Rc<CompositionBrush>> {
        self.data().sources.borrow().iter().find(|(existing, _)| existing == name).and_then(|(_, brush)| brush.clone())
    }

    /// `SetSourceParameter(name, source)`: binds the source parameter `name`
    /// to `source` (`None` unbinds it; an unbound parameter draws nothing).
    ///
    /// Panics when the loaded graph has no source parameter of that name
    /// (WinUI throws). A brush whose factory did not load its effect accepts
    /// any name.
    pub fn set_source_parameter(&self, name: &str, source: Option<Rc<CompositionBrush>>) {
        let data = self.data();
        if data.graph.is_loaded() && !data.graph.has_parameter(name) {
            panic!("the effect has no source parameter named {name}");
        }
        {
            let mut sources = data.sources.borrow_mut();
            match sources.iter_mut().find(|(existing, _)| existing == name) {
                Some(entry) => entry.1 = source,
                None => sources.push((name.to_owned(), source)),
            }
        }
        data.sources_changed.set(true);
        self.0.register_for_serialization();
    }
}

/// A brush that shows what is behind the visual that paints it (WinUI
/// `Microsoft.UI.Composition.CompositionBackdropBrush`), made by
/// [`Compositor::create_backdrop_brush`]; usually the source of an effect
/// brush (a blurred backdrop).
///
/// Deviation (winui-composition.md): the drawing contexts cannot read back
/// what was drawn under a rectangle, so the brush draws nothing, alone and
/// as a source of an effect.
#[derive(Clone)]
pub struct CompositionBackdropBrush(Rc<CompositionBrush>);

impl Deref for CompositionBackdropBrush {
    type Target = Rc<CompositionBrush>;

    fn deref(&self) -> &Rc<CompositionBrush> {
        &self.0
    }
}

impl CompositionBackdropBrush {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionBackdropBrush {
        let brush = CompositionBrush::create(
            compositor,
            CompositionBrushKind::Backdrop(CompositionBrushProps::new()),
            |c| ServerCompositionBackdropBrush::new(c),
        );
        brush.initialize_defaults();
        CompositionBackdropBrush(brush)
    }

    /// The handle of `brush` as a backdrop brush, if it is one.
    pub fn from_brush(brush: &Rc<CompositionBrush>) -> Option<CompositionBackdropBrush> {
        matches!(brush.kind, CompositionBrushKind::Backdrop(_)).then(|| CompositionBackdropBrush(brush.clone()))
    }
}
