//! The graphs of custom effects (#455, E2; not from upstream): the
//! description of [`CustomEffect::graph`], loaded once per type into the
//! effect graph the composition effects draw, with the same loader and
//! checks as `Compositor::create_effect_factory_with_properties`, and
//! registered for the prewarm of the GPU programs.
//!
//! The cache is process-wide (the render thread loads a type it meets
//! first, the registry loads the registered types on the UI thread) and is
//! keyed by the type of the effect, as a backend finds its factory by
//! [`ImmutableCustomEffect::value_type_id`](super::ImmutableCustomEffect::value_type_id).

use super::custom_effect::CUSTOM_EFFECT_CONTENT;
use super::{CustomEffect, EffectParameterValue};
use crate::logging::{LogEventLevel, Logger};
use crate::rendering::composition::effects::{
    register_effect_graph_prewarm, CompositionEffectFactoryError, EffectGraph, EffectMemberKind,
};
use std::any::TypeId;
use std::sync::{Arc, RwLock};

/// The graph of a custom effect type, loaded.
#[derive(Debug)]
pub(crate) struct CustomEffectGraph {
    /// The loaded graph, drawn around the content.
    pub(crate) graph: Arc<EffectGraph>,
    /// For each value of [`CustomEffect::for_each_graph_value`], in order,
    /// its name and the full name of the member it sets
    /// (`EffectName.PropertyName`).
    pub(crate) members: Vec<(&'static str, String)>,
}

impl CustomEffectGraph {
    /// The name of the value that sets the member `key`
    /// (`EffectName.PropertyName`).
    pub(crate) fn value_name_of(&self, key: &str) -> Option<&'static str> {
        self.members.iter().find(|(_, member)| member == key).map(|(name, _)| *name)
    }
}

/// What loading the graph of a type gave: `None` for a type without one.
type Loaded = Option<Result<Arc<CustomEffectGraph>, CompositionEffectFactoryError>>;

/// The graphs loaded, by type.
static GRAPHS: RwLock<Vec<(TypeId, Loaded)>> = RwLock::new(Vec::new());

/// The graph of `T`, loaded on the first call for the type; the error of
/// the loader when it does not load (reported once, when it is loaded).
pub(crate) fn load_custom_effect_graph<T: CustomEffect>() -> Loaded {
    let type_id = TypeId::of::<T>();
    {
        let graphs = GRAPHS.read().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((_, loaded)) = graphs.iter().find(|(id, _)| *id == type_id) {
            return loaded.clone();
        }
    }
    // Loaded outside the lock: `graph` and `default` are code of the effect.
    let loaded = load::<T>();
    {
        let mut graphs = GRAPHS.write().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((_, existing)) = graphs.iter().find(|(id, _)| *id == type_id) {
            // Another thread loaded it meanwhile.
            return existing.clone();
        }
        graphs.push((type_id, loaded.clone()));
    }
    match &loaded {
        Some(Ok(graph)) => register_effect_graph_prewarm(&format!("custom-effect:{}", T::NAME), graph.graph.clone()),
        Some(Err(error)) => {
            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, "Visual") {
                logger.log(
                    None,
                    &format!(
                        "The graph of the custom effect {} does not load ({error}); it is drawn as an effect without a graph.",
                        T::NAME
                    ),
                );
            }
        }
        None => {}
    }
    loaded
}

/// The kind of member a value sets: a number a scalar, a colour a colour.
fn member_kind(value: &EffectParameterValue) -> Option<EffectMemberKind> {
    match value {
        EffectParameterValue::Float(_) | EffectParameterValue::Int(_) => Some(EffectMemberKind::Scalar),
        EffectParameterValue::Color(_) => Some(EffectMemberKind::Color),
        _ => None,
    }
}

fn load<T: CustomEffect>() -> Loaded {
    let description = T::graph()?;
    let mut members: Vec<(&'static str, EffectMemberKind)> = Vec::new();
    let mut invalid = None;
    CustomEffect::for_each_graph_value(&T::default(), &mut |name, value| match member_kind(&value) {
        Some(kind) => members.push((name, kind)),
        None => {
            invalid.get_or_insert(name);
        }
    });
    if let Some(name) = invalid {
        return Some(Err(CompositionEffectFactoryError::InvalidAnimatableProperty(name.to_owned())));
    }
    let (mut graph, keys) = match EffectGraph::load_with_members(&description, &members) {
        Ok(loaded) => loaded,
        Err((_, error)) => return Some(Err(error)),
    };
    // The content is the only source of the graph.
    if let Some(other) = graph.parameters.iter().find(|name| name.as_str() != CUSTOM_EFFECT_CONTENT) {
        return Some(Err(CompositionEffectFactoryError::MissingSource {
            effect: "CompositionEffectSourceParameter",
            name: other.clone(),
        }));
    }
    graph.around_content = true;
    let members = members.iter().map(|(name, _)| *name).zip(keys).collect();
    Some(Ok(Arc::new(CustomEffectGraph { graph: Arc::new(graph), members })))
}
