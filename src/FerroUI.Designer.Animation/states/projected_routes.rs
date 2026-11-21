use super::{STATE_MARKER, StateTransition, TRANSITION_MARKER, VisualStateGroup, decode_text};
use crate::Easing;
use ferroui_designer_core::{Document, NodeId, SourceSpan};
use std::collections::BTreeMap;

/// A route in an RSX projection. The ID belongs to its first generated style,
/// so it cannot alias the ID of an ordinary animation element in the document.
#[derive(Clone, Debug)]
pub struct ProjectedStateRoute {
    pub id: NodeId,
    pub transition: StateTransition,
}

/// State declarations emitted by the RSX projector, scoped to their owning control.
pub struct ProjectedStateGroup {
    pub model: VisualStateGroup,
    pub routes: Vec<ProjectedStateRoute>,
}

impl ProjectedStateGroup {
    pub fn scan(document: &Document) -> Vec<Self> {
        StatePreviewGroup::scan(document, false)
            .into_iter()
            .map(|group| Self {
                model: group.model,
                routes: group
                    .routes
                    .into_iter()
                    .map(|route| ProjectedStateRoute {
                        id: route.id,
                        transition: route.transition,
                    })
                    .collect(),
            })
            .collect()
    }
}

/// Routes and instantaneous state poses in an accepted source scope.
pub struct StatePreviewGroup {
    pub model: VisualStateGroup,
    pub span: SourceSpan,
    pub routes: Vec<StatePreviewRoute>,
}

pub struct StatePreviewRoute {
    pub id: NodeId,
    pub transition: StateTransition,
    pub endpoint: bool,
    pub span: SourceSpan,
}

impl StatePreviewGroup {
    /// Older app previews negotiated only RSX routes. Full authoring also
    /// exposes authored XAML groups and single-state preview poses.
    pub fn scan(document: &Document, full_authoring: bool) -> Vec<Self> {
        let ast = document.ast();
        let groups = VisualStateGroup::read_group_styles(document);
        let mut last_id = ast.nodes().map(|node| node.id).max().unwrap_or_default();
        let mut count = BTreeMap::new();
        for (_, model, projected) in &groups {
            if *projected || full_authoring {
                *count.entry((model.owner, model.name.as_str())).or_insert(0) += 1;
            }
        }
        groups
            .iter()
            .filter_map(|(style, model, projected)| {
                if (!projected && !full_authoring)
                    || count.get(&(model.owner, model.name.as_str())) != Some(&1)
                    || model.validate().is_err()
                {
                    return None;
                }
                let group_node = ast.node(*style)?;
                let mut styles = BTreeMap::new();
                for child in ast.element_children(*style) {
                    let child = ast.node(child)?;
                    let marker = child
                        .children
                        .iter()
                        .filter_map(|id| ast.node(*id)?.text.as_deref())
                        .find_map(route_marker);
                    if let Some(key) = marker {
                        styles.entry(key).or_insert(child);
                    }
                }
                let mut routes = model
                    .transitions
                    .iter()
                    .filter_map(|transition| {
                        let style =
                            styles.get(&(transition.from.clone(), transition.to.clone()))?;
                        Some(StatePreviewRoute {
                            id: style.id,
                            transition: transition.clone(),
                            endpoint: false,
                            span: style.span,
                        })
                    })
                    .collect::<Vec<_>>();
                if full_authoring {
                    // Reserve IDs above every authored node. An endpoint and a
                    // default route into that same state have separate identities.
                    routes.extend(
                        model
                            .states
                            .iter()
                            .filter(|state| !state.setters.is_empty())
                            .map(|state| {
                                last_id += 1;
                                StatePreviewRoute {
                                    id: last_id,
                                    transition: StateTransition {
                                        from: None,
                                        to: state.name.clone(),
                                        duration: 0.0,
                                        easing: Easing::Linear,
                                    },
                                    endpoint: true,
                                    span: styles
                                        .get(&(None, state.name.clone()))
                                        .map_or(group_node.span, |style| style.span),
                                }
                            }),
                    );
                }
                Some(Self {
                    model: model.clone(),
                    span: group_node.span,
                    routes,
                })
            })
            .collect()
    }
}

fn route_marker(text: &str) -> Option<(Option<String>, String)> {
    let text = text.trim();
    if let Some(marker) = text.strip_prefix(STATE_MARKER) {
        let (to, _) = marker.split_once(':')?;
        return Some((None, decode_text(to)?));
    }
    let (from, rest) = text.strip_prefix(TRANSITION_MARKER)?.split_once(':')?;
    let (to, _) = rest.split_once(':')?;
    Some((Some(decode_text(from)?), decode_text(to)?))
}
