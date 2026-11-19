use super::{
    STATE_MARKER, StateTransition, TRANSITION_MARKER, VisualStateGroup, decode_text, group_nodes,
};
use ferroui_designer_core::{Document, NodeId};
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
        let ast = document.ast();
        let groups = group_nodes(document);
        let mut count = BTreeMap::new();
        for (_, model, projected) in &groups {
            if *projected {
                *count.entry((model.owner, model.name.clone())).or_insert(0) += 1;
            }
        }
        groups
            .into_iter()
            .filter_map(|(style, model, projected)| {
                if !projected
                    || count.get(&(model.owner, model.name.clone())) != Some(&1)
                    || model.validate().is_err()
                {
                    return None;
                }
                let mut styles = BTreeMap::new();
                for child in ast.element_children(style) {
                    let marker = ast
                        .node(child)?
                        .children
                        .iter()
                        .filter_map(|id| ast.node(*id)?.text.as_deref())
                        .find_map(route_marker);
                    if let Some(key) = marker {
                        styles.entry(key).or_insert(child);
                    }
                }
                let routes = model
                    .transitions
                    .iter()
                    .filter_map(|transition| {
                        let id = *styles.get(&(transition.from.clone(), transition.to.clone()))?;
                        Some(ProjectedStateRoute {
                            id,
                            transition: transition.clone(),
                        })
                    })
                    .collect();
                Some(Self { model, routes })
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
