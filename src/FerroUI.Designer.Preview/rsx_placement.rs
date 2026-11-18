use crate::document::DesignDocument;
use crate::protocol::DragOrigin;
use ferroui_designer_core::{Document, Edit, NodeId, TextPatch};
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
            if !parent.is_some_and(|parent| self.containers.contains(&parent)) {
                return Err(
                    "The dragged control has dynamic Rust siblings. Move it in source.".into(),
                );
            }
        }
        for edit in edits {
            match edit {
                Edit::InsertChild { parent, .. } | Edit::MoveNode { parent, .. }
                    if !self.containers.contains(parent) =>
                {
                    return Err("Drop into a Rust container with fully mapped children".into());
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

pub(crate) fn patches(
    design: &DesignDocument<'_>,
    draft: &Document,
    id: NodeId,
    origin: &DragOrigin,
    planned: &[Edit],
) -> Result<Vec<TextPatch>, String> {
    let rsx = design.rsx.as_ref().ok_or("Open an RSX design")?;
    if matches!(origin, DragOrigin::Nodes { .. }) {
        return rsx.apply_xaml(draft.source());
    }
    let (parent, before) = boundary(draft, id)?;
    let parent = design
        .rsx_node(parent)
        .ok_or("Select an authored Rust container")?
        .0;
    let before = before
        .map(|next| {
            design
                .rsx_node(next)
                .map(|entry| entry.0)
                .ok_or("The Rust insertion anchor changed")
        })
        .transpose()?;
    let patches = match origin {
        DragOrigin::Asset { .. } | DragOrigin::Image { .. } => {
            let fragment = ferroui_designer_tools::component_fragment(draft, id)
                .map_err(|error| error.to_string())?;
            vec![rsx.insert_child(parent, before, &fragment)?]
        }
        DragOrigin::Node { node, .. } => {
            let (index, _) = design
                .rsx_node(*node)
                .ok_or("Select an authored Rust control")?;
            let (original_parent, original_before) = boundary(&design.xaml, *node)?;
            let original = design
                .rsx_node(original_parent)
                .ok_or("Select an authored Rust container")?;
            if !original.1.editable_children() {
                return Err(
                    "Move controls in the Rust editor when their siblings contain dynamic children"
                        .into(),
                );
            }
            let original_before =
                original_before.and_then(|node| design.rsx_node(node).map(|entry| entry.0));
            let mut values = Vec::new();
            for edit in planned {
                match edit {
                    Edit::SetAttribute {
                        node: target,
                        name,
                        value,
                    } if *target == *node => {
                        values.push(rsx.edit_property(index, name, Some(value))?)
                    }
                    Edit::RemoveAttribute { node: target, name } if *target == *node => {
                        values.push(rsx.edit_property(index, name, None)?)
                    }
                    Edit::MoveNode { node: target, .. } if *target == *node => (),
                    _ => {
                        return Err(
                            "This placement requires editing the Rust layout expressions".into(),
                        );
                    }
                }
            }
            if parent == original.0 && before == original_before {
                merge_insertions(values)
            } else {
                rsx.move_child(index, parent, before, merge_insertions(values))?
            }
        }
        DragOrigin::Nodes { .. } => unreachable!("group placement handled above"),
    };
    let mut draft_source = design.source.to_owned();
    let mut ordered: Vec<_> = patches.iter().collect();
    ordered.sort_by_key(|patch| std::cmp::Reverse(patch.span.start));
    for patch in ordered {
        draft_source.replace_range(patch.span.range(), &patch.replacement);
    }
    let reparsed = design.rsx_catalog().project(&draft_source, rsx.invocation.start)?;
    if !reparsed.matches_xaml(draft.source())? {
        return Err("This placement cannot preserve the authored Rust layout".into());
    }
    Ok(patches)
}

fn boundary(document: &Document, id: NodeId) -> Result<(NodeId, Option<NodeId>), String> {
    let parent = document
        .ast()
        .node(id)
        .and_then(|node| node.parent)
        .ok_or("The control has no authored container")?;
    let siblings = document.ast().element_children(parent);
    let index = siblings
        .iter()
        .position(|node| *node == id)
        .ok_or("The control has no authored sibling order")?;
    Ok((parent, siblings.get(index + 1).copied()))
}

fn merge_insertions(mut values: Vec<TextPatch>) -> Vec<TextPatch> {
    values.sort_by_key(|patch| (patch.span.start, patch.span.end));
    let mut merged: Vec<TextPatch> = Vec::new();
    for patch in values {
        if let Some(previous) = merged
            .last_mut()
            .filter(|item| item.span == patch.span && patch.span.is_empty())
        {
            previous.replacement.push_str(&patch.replacement);
        } else {
            merged.push(patch);
        }
    }
    merged
}
