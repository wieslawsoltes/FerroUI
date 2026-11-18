use crate::{DesignerSession, RuntimeNode};
use ferroui_base::{FerroObject, Ref};
use ferroui_designer_core::NodeId;
use ferroui_inspect::{Inspector, PropertyInfo, TreeKind};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeOutlineMode {
    Visual,
    Logical,
    Combined,
}

/// A live object with provenance in the visual and logical hierarchies.
#[derive(Clone, Debug)]
pub struct RuntimeOutlineRow {
    pub node: RuntimeNode,
    pub visual: bool,
    pub logical: bool,
    pub logical_parent: Option<u64>,
}

impl DesignerSession {
    pub fn runtime_outline(&self, mode: RuntimeOutlineMode) -> Vec<RuntimeOutlineRow> {
        let visual = if mode != RuntimeOutlineMode::Logical {
            self.runtime_tree(TreeKind::Visual)
        } else {
            Vec::new()
        };
        let logical = if mode != RuntimeOutlineMode::Visual {
            self.runtime_tree(TreeKind::Logical)
        } else {
            Vec::new()
        };
        combine(visual, logical)
    }

    pub fn authored_preview_properties(&self, node: NodeId) -> Vec<PropertyInfo> {
        self.preview
            .visual(node)
            .map_or_else(Vec::new, |visual| Inspector::properties(&visual))
    }

    pub fn inspected_preview_properties(&self, id: u64) -> Result<Vec<PropertyInfo>, String> {
        if !self
            .runtime_outline(RuntimeOutlineMode::Combined)
            .iter()
            .any(|row| row.node.inspector_id == id)
        {
            return Err("The runtime object no longer belongs to this preview".into());
        }
        let object: Ref<FerroObject> = Inspector::shared().borrow().resolve(id)?;
        Ok(Inspector::properties(&object))
    }
}

fn combine(visual: Vec<RuntimeNode>, logical: Vec<RuntimeNode>) -> Vec<RuntimeOutlineRow> {
    let visual_ids: BTreeSet<_> = visual.iter().map(|node| node.inspector_id).collect();
    let logical_parents: BTreeMap<_, _> = logical
        .iter()
        .map(|node| (node.inspector_id, node.parent))
        .collect();
    let mut branches = BTreeMap::<_, Vec<RuntimeOutlineRow>>::new();
    // Visual rows retain their paint-tree hierarchy. A logical-only row is grafted
    // onto its logical parent, which may already have a place in the visual tree.
    for node in visual.into_iter().chain(
        logical
            .into_iter()
            .filter(|node| !visual_ids.contains(&node.inspector_id)),
    ) {
        let id = node.inspector_id;
        branches
            .entry(node.parent)
            .or_default()
            .push(RuntimeOutlineRow {
                node,
                visual: visual_ids.contains(&id),
                logical: logical_parents.contains_key(&id),
                logical_parent: logical_parents.get(&id).copied().flatten(),
            });
    }
    let mut rows = Vec::new();
    let mut pending: Vec<_> = branches
        .remove(&None)
        .unwrap_or_default()
        .into_iter()
        .rev()
        .map(|root| (root, 0))
        .collect();
    while let Some((mut row, depth)) = pending.pop() {
        row.node.depth = depth;
        if let Some(children) = branches.remove(&Some(row.node.inspector_id)) {
            pending.extend(children.into_iter().rev().map(|child| (child, depth + 1)));
        }
        rows.push(row);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u64, parent: Option<u64>) -> RuntimeNode {
        RuntimeNode {
            inspector_id: id,
            parent,
            source_id: None,
            depth: 0,
            type_name: "Control".into(),
            name: None,
            bounds: None,
        }
    }

    #[test]
    fn combined_tree_preserves_visual_parents_and_includes_logical_only_nodes() {
        let rows = combine(
            vec![node(1, None), node(2, Some(1)), node(3, Some(2))],
            vec![
                node(1, None),
                node(4, Some(1)),
                node(3, Some(4)),
                node(5, Some(3)),
            ],
        );
        assert_eq!(
            rows.iter()
                .map(|row| (row.node.inspector_id, row.node.depth))
                .collect::<Vec<_>>(),
            vec![(1, 0), (2, 1), (3, 2), (5, 3), (4, 1)]
        );
        let shared = &rows[2];
        assert_eq!(shared.node.parent, Some(2));
        assert_eq!(shared.logical_parent, Some(4));
        assert!(shared.logical && shared.visual);
        assert!(rows[3].logical && !rows[3].visual);
    }
}
