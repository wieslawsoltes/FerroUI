use crate::document::DesignDocument;
use crate::protocol::{DragOrigin, DropPosition, DropTarget, EditResult, Placement};
use ferroui_designer_core::{Document, Edit, NodeId};
use ferroui_designer_language::LineIndex;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct PlacementPolicy {
    containers: BTreeSet<NodeId>,
    expressions: BTreeMap<NodeId, BTreeSet<String>>,
}

impl PlacementPolicy {
    pub(crate) fn of(design: &DesignDocument<'_>) -> Option<Self> {
        design.rsx.as_ref()?;
        let mut result = Self {
            containers: BTreeSet::new(),
            expressions: BTreeMap::new(),
        };
        for node in design.xaml.ast().elements() {
            if let Some((_, authored)) = design.rsx_node(node.id) {
                if authored.editable_children() {
                    result.containers.insert(node.id);
                }
                if !authored.expressions.is_empty() {
                    result
                        .expressions
                        .insert(node.id, authored.expressions.keys().cloned().collect());
                }
            }
        }
        Some(result)
    }

    pub(crate) fn editable_children(&self, node: NodeId) -> bool {
        self.containers.contains(&node)
    }

    pub(crate) fn validate(
        &self,
        origin: &DragOrigin,
        document: &Document,
        edits: &[Edit],
    ) -> Result<(), String> {
        let moving = match origin {
            DragOrigin::Node { node, .. } => std::slice::from_ref(node),
            DragOrigin::Nodes { nodes, .. } => nodes,
            _ => &[],
        };
        for node in moving {
            let parent = document.ast().node(*node).and_then(|node| node.parent);
            if !parent.is_some_and(|parent| self.editable_children(parent)) {
                return Err("Edit the source container's computed child slot in Rust".into());
            }
        }
        for edit in edits {
            match edit {
                Edit::InsertChild { parent, .. } | Edit::MoveNode { parent, .. }
                    if !self.editable_children(*parent) =>
                {
                    return Err("Drop into a Rust container with an authored child slot".into());
                }
                Edit::SetAttribute { node, name, .. } | Edit::RemoveAttribute { node, name }
                    if self
                        .expressions
                        .get(node)
                        .is_some_and(|fields| fields.contains(name)) =>
                {
                    return Err(format!(
                        "Edit the Rust {name} expression before changing this layout"
                    ));
                }
                _ => (),
            }
        }
        Ok(())
    }
}

pub(crate) fn edit(
    design: &DesignDocument<'_>,
    draft: &Document,
    selected: Option<NodeId>,
    placement: &Placement,
    edits: &[Edit],
) -> Result<EditResult, String> {
    let Some(selected) = selected else {
        return crate::edits::editor_edits(design, Vec::new());
    };
    let projection = design.rsx.as_ref().ok_or("Open an RSX design")?;
    let after = match placement.target {
        DropTarget::Outline {
            node,
            position: DropPosition::After,
        } => Some(node),
        _ => None,
    };
    let plan = projection.plan_child_edit(&design.xaml, draft, edits, after)?;
    let index = LineIndex::new(&plan.authored_source);
    let range = |id| {
        let span = plan
            .controls
            .get(&id)
            .ok_or("The placed control has no Rust source range")?;
        index.range(*span).map_err(|error| error.to_string())
    };
    let nodes = match &placement.origin {
        DragOrigin::Nodes { nodes, .. } => nodes.as_slice(),
        _ => &[],
    };
    let retained = nodes
        .iter()
        .copied()
        .filter(|node| *node != selected)
        .chain(std::iter::once(selected))
        .map(range)
        .collect::<Result<Vec<_>, _>>()?;
    let mut result = crate::layer_edits::rust(design, plan)?;
    result.selection = retained.last().cloned();
    result.retained_selection = retained;
    Ok(result)
}
