//! The loaded form of an effect description: the graph an effect factory
//! checked, with the animatable properties resolved, which its brushes
//! share with their server side.

use super::{
    CanvasComposite, CompositionEffectFactoryError, CompositionEffectFactoryLoadStatus, GraphicsEffect,
    GraphicsEffectSource,
};
use crate::media::Color;

/// The most effects (and source parameters) a graph may have; a larger
/// graph is `EffectTooComplex`.
pub(crate) const MAX_EFFECT_NODES: usize = 256;

/// The deepest nesting of effects a graph may have; a deeper graph is
/// `EffectTooComplex`.
pub(crate) const MAX_EFFECT_DEPTH: usize = 32;

/// Converts the `BlurAmount` of a Win2D blur (the standard deviation of the
/// Gaussian, in device independent pixels) to the radius of the blur effect
/// of the drawing contexts, which both backends turn into a standard
/// deviation as `0.288675 * radius + 0.5`. An amount of 0.5 or less is no
/// blur.
pub fn blur_amount_to_radius(blur_amount: f64) -> f64 {
    if blur_amount.is_nan() || blur_amount <= 0.5 {
        return 0.0;
    }
    (blur_amount - 0.5) / 0.288675
}

/// A scalar member of an effect: its value in the description, and the key
/// of its entry in the `Properties` of the brush when it is animatable.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EffectScalar {
    pub(crate) key: Option<String>,
    pub(crate) value: f32,
}

impl EffectScalar {
    fn new(value: f32) -> Self {
        Self { key: None, value }
    }
}

/// A colour member of an effect; see [`EffectScalar`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EffectColor {
    pub(crate) key: Option<String>,
    pub(crate) value: Color,
}

/// What a node of a loaded graph does.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum EffectNodeKind {
    /// A source parameter: the brush bound to the name, or the composed
    /// layer of a layer visual.
    Parameter(String),
    GaussianBlur { amount: EffectScalar, source: Box<EffectNode> },
    Opacity { opacity: EffectScalar, source: Box<EffectNode> },
    CrossFade { weight: EffectScalar, source1: Box<EffectNode>, source2: Box<EffectNode> },
    /// The sources drawn over each other (source-over).
    Composite(Vec<EffectNode>),
    ColorSource(EffectColor),
}

/// A node of a loaded graph: an effect with its name, or a source
/// parameter (whose name is empty; the parameter's own name is in the
/// kind).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EffectNode {
    pub(crate) name: String,
    pub(crate) kind: EffectNodeKind,
}

impl EffectNode {
    /// The inputs of the node, in order.
    pub(crate) fn sources(&self) -> Vec<&EffectNode> {
        match &self.kind {
            EffectNodeKind::Parameter(_) | EffectNodeKind::ColorSource(_) => Vec::new(),
            EffectNodeKind::GaussianBlur { source, .. } | EffectNodeKind::Opacity { source, .. } => vec![&**source],
            EffectNodeKind::CrossFade { source1, source2, .. } => vec![&**source1, &**source2],
            EffectNodeKind::Composite(sources) => sources.iter().collect(),
        }
    }

    /// Binds the property `property` of the effects named `effect` to the
    /// entry `key`; returns whether such a property was found.
    fn bind_animatable(&mut self, effect: &str, property: &str, key: &str) -> Result<bool, ()> {
        let mut found = false;
        if self.name == effect && !self.name.is_empty() {
            let is = |name: &str| property.eq_ignore_ascii_case(name);
            match &mut self.kind {
                EffectNodeKind::GaussianBlur { amount, .. } if is("BlurAmount") => amount.key = Some(key.to_owned()),
                EffectNodeKind::Opacity { opacity, .. } if is("Opacity") => opacity.key = Some(key.to_owned()),
                EffectNodeKind::CrossFade { weight, .. } if is("Weight") || is("CrossFade") => {
                    weight.key = Some(key.to_owned())
                }
                EffectNodeKind::ColorSource(color) if is("Color") => color.key = Some(key.to_owned()),
                // A member that exists but is not animatable, or no member of
                // that name.
                _ => return Err(()),
            }
            found = true;
        }
        match &mut self.kind {
            EffectNodeKind::Parameter(_) | EffectNodeKind::ColorSource(_) => {}
            EffectNodeKind::GaussianBlur { source, .. } | EffectNodeKind::Opacity { source, .. } => {
                found |= source.bind_animatable(effect, property, key)?;
            }
            EffectNodeKind::CrossFade { source1, source2, .. } => {
                found |= source1.bind_animatable(effect, property, key)?;
                found |= source2.bind_animatable(effect, property, key)?;
            }
            EffectNodeKind::Composite(sources) => {
                for source in sources {
                    found |= source.bind_animatable(effect, property, key)?;
                }
            }
        }
        Ok(found)
    }

    /// The initial value of the animatable property bound to `key`.
    fn animatable_value(&self, key: &str) -> Option<EffectPropertyValue> {
        let own = match &self.kind {
            EffectNodeKind::GaussianBlur { amount: value, .. }
            | EffectNodeKind::Opacity { opacity: value, .. }
            | EffectNodeKind::CrossFade { weight: value, .. } => {
                (value.key.as_deref() == Some(key)).then_some(EffectPropertyValue::Scalar(value.value))
            }
            EffectNodeKind::ColorSource(color) => {
                (color.key.as_deref() == Some(key)).then_some(EffectPropertyValue::Color(color.value))
            }
            EffectNodeKind::Parameter(_) | EffectNodeKind::Composite(_) => None,
        };
        own.or_else(|| self.sources().into_iter().find_map(|source| source.animatable_value(key)))
    }

    fn collect_parameters(&self, parameters: &mut Vec<String>) {
        if let EffectNodeKind::Parameter(name) = &self.kind {
            if !parameters.iter().any(|existing| existing == name) {
                parameters.push(name.clone());
            }
        }
        for source in self.sources() {
            source.collect_parameters(parameters);
        }
    }

    /// The path (the indices of the inputs) from this node to its first
    /// source parameter in graph order.
    fn first_parameter_path(&self, path: &mut Vec<usize>) -> bool {
        if matches!(self.kind, EffectNodeKind::Parameter(_)) {
            return true;
        }
        for (index, source) in self.sources().into_iter().enumerate() {
            path.push(index);
            if source.first_parameter_path(path) {
                return true;
            }
            path.pop();
        }
        false
    }
}

/// The initial value of an animatable property, which the brush inserts
/// into its `Properties`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum EffectPropertyValue {
    Scalar(f32),
    Color(Color),
}

/// An effect graph as a factory loaded it, shared (behind an `Arc`) by the
/// brushes of the factory and their server side.
#[derive(Debug, Default)]
pub(crate) struct EffectGraph {
    /// The root effect; `None` when the factory failed to load the effect,
    /// whose brushes then draw nothing.
    pub(crate) root: Option<EffectNode>,
    /// The names of the source parameters, in graph order, once each.
    pub(crate) parameters: Vec<String>,
    /// The animatable properties, by the key given to the factory
    /// (`Blur.BlurAmount`), with their initial values.
    pub(crate) animatable: Vec<(String, EffectPropertyValue)>,
    /// The path to the first source parameter in graph order: where the
    /// composed layer goes when the brush is the effect of a layer visual.
    pub(crate) layer_path: Option<Vec<usize>>,
}

impl EffectGraph {
    /// The graph of a factory that failed to load its effect.
    pub(crate) fn failed() -> EffectGraph {
        EffectGraph::default()
    }

    /// Loads `effect` with the animatable properties `animatable` (each
    /// `EffectName.PropertyName`): the load status and error of the factory
    /// when the effect cannot be drawn.
    pub(crate) fn load(
        effect: &GraphicsEffect,
        animatable: &[&str],
    ) -> Result<EffectGraph, (CompositionEffectFactoryLoadStatus, CompositionEffectFactoryError)> {
        let mut count = 0;
        let mut root = build_node(effect, 1, &mut count)?;
        let mut properties: Vec<(String, EffectPropertyValue)> = Vec::with_capacity(animatable.len());
        for &key in animatable {
            let invalid = || {
                (CompositionEffectFactoryLoadStatus::Other, CompositionEffectFactoryError::InvalidAnimatableProperty(key.to_owned()))
            };
            let (effect_name, property) = key.split_once('.').ok_or_else(invalid)?;
            if !root.bind_animatable(effect_name, property, key).map_err(|_| invalid())? {
                return Err(invalid());
            }
            if !properties.iter().any(|(existing, _)| existing == key) {
                let value = root.animatable_value(key).ok_or_else(invalid)?;
                properties.push((key.to_owned(), value));
            }
        }
        let mut parameters = Vec::new();
        root.collect_parameters(&mut parameters);
        let mut path = Vec::new();
        let layer_path = root.first_parameter_path(&mut path).then_some(path);
        Ok(EffectGraph { root: Some(root), parameters, animatable: properties, layer_path })
    }

    /// Whether the graph was loaded.
    pub(crate) fn is_loaded(&self) -> bool {
        self.root.is_some()
    }

    /// Whether `key` is one of the animatable properties of the graph.
    pub(crate) fn is_animatable(&self, key: &str) -> bool {
        self.animatable.iter().any(|(existing, _)| existing == key)
    }

    /// Whether the graph has a source parameter named `name`.
    pub(crate) fn has_parameter(&self, name: &str) -> bool {
        self.parameters.iter().any(|existing| existing == name)
    }
}

type LoadError = (CompositionEffectFactoryLoadStatus, CompositionEffectFactoryError);

fn too_complex() -> LoadError {
    (CompositionEffectFactoryLoadStatus::EffectTooComplex, CompositionEffectFactoryError::EffectTooComplex)
}

fn build_source(source: &Option<GraphicsEffectSource>, owner: &GraphicsEffect, depth: usize, count: &mut usize) -> Result<EffectNode, LoadError> {
    match source {
        None => Err((
            CompositionEffectFactoryLoadStatus::Other,
            CompositionEffectFactoryError::MissingSource { effect: owner.type_name(), name: owner.name().to_owned() },
        )),
        Some(GraphicsEffectSource::Parameter(parameter)) => {
            *count += 1;
            if *count > MAX_EFFECT_NODES {
                return Err(too_complex());
            }
            Ok(EffectNode { name: String::new(), kind: EffectNodeKind::Parameter(parameter.name().to_owned()) })
        }
        Some(GraphicsEffectSource::Effect(effect)) => build_node(effect, depth + 1, count),
    }
}

fn build_node(effect: &GraphicsEffect, depth: usize, count: &mut usize) -> Result<EffectNode, LoadError> {
    *count += 1;
    if depth > MAX_EFFECT_DEPTH || *count > MAX_EFFECT_NODES {
        return Err(too_complex());
    }
    let kind = match effect {
        GraphicsEffect::GaussianBlur(blur) => EffectNodeKind::GaussianBlur {
            amount: EffectScalar::new(blur.blur_amount),
            source: Box::new(build_source(&blur.source, effect, depth, count)?),
        },
        GraphicsEffect::Opacity(opacity) => EffectNodeKind::Opacity {
            opacity: EffectScalar::new(opacity.opacity),
            source: Box::new(build_source(&opacity.source, effect, depth, count)?),
        },
        GraphicsEffect::CrossFade(cross_fade) => EffectNodeKind::CrossFade {
            weight: EffectScalar::new(cross_fade.weight),
            source1: Box::new(build_source(&cross_fade.source1, effect, depth, count)?),
            source2: Box::new(build_source(&cross_fade.source2, effect, depth, count)?),
        },
        GraphicsEffect::Composite(composite) => {
            if composite.mode != CanvasComposite::SourceOver {
                return Err((
                    CompositionEffectFactoryLoadStatus::Other,
                    CompositionEffectFactoryError::CompositeModeNotSupported(composite.mode),
                ));
            }
            let mut sources = Vec::with_capacity(composite.sources.len());
            for source in &composite.sources {
                sources.push(build_source(&Some(source.clone()), effect, depth, count)?);
            }
            EffectNodeKind::Composite(sources)
        }
        GraphicsEffect::ColorSource(color) => {
            EffectNodeKind::ColorSource(EffectColor { key: None, value: color.color })
        }
        // Deviation (winui-composition.md): the drawing contexts have no
        // colour matrix.
        GraphicsEffect::Saturation(_) => {
            return Err((
                CompositionEffectFactoryLoadStatus::Other,
                CompositionEffectFactoryError::EffectTypeNotSupported(effect.type_name()),
            ))
        }
    };
    Ok(EffectNode { name: effect.name().to_owned(), kind })
}
