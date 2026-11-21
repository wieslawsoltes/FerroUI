//! Source-preserving placement. Native layout supplies the intended projected tree,
//! while the Rust parser supplies the original child slots and opaque boundaries.
mod removals;

use crate::{RsxProjection, placement::ChildBoundary, source_edits::merge_insertions, xaml::XamlTree};
use ferroui_designer_core::{Document, Edit, NodeId, NodeKind, SourceSpan, TextPatch};
use removals::RemovedSubtrees;
use std::collections::{BTreeMap, BTreeSet};

pub struct RsxChildEdit {
    pub patches: Vec<TextPatch>,
    /// New authored tag-name ranges for the projected node identities in the edit draft.
    pub controls: BTreeMap<NodeId, SourceSpan>,
}

/// A child-list transaction can combine placements and removals with mapped
/// layout values on the affected controls.
pub fn is_child_list_edit(edits: &[Edit]) -> bool {
    edits.iter().any(|edit| {
        matches!(edit, Edit::InsertChild { .. } | Edit::MoveNode { .. } | Edit::RemoveNode { .. })
    }) && edits.iter().all(|edit| {
        matches!(
            edit,
            Edit::InsertChild { .. } | Edit::MoveNode { .. } | Edit::RemoveNode { .. }
                | Edit::SetAttribute { .. } | Edit::RemoveAttribute { .. }
        )
    })
}

impl RsxProjection<'_> {
    /// Translate a validated projected layout draft without rewriting its Rust invocation.
    /// Outline drops can supply an explicit after-anchor at a visible authored sibling.
    pub fn plan_child_edit(
        &self,
        current: &Document,
        draft: &Document,
        edits: &[Edit],
        after: Option<NodeId>,
    ) -> Result<RsxChildEdit, String> {
        let markup = self.preview_markup();
        if !current.is_valid() || current.source() != markup.source || !draft.is_valid() {
            return Err("Use the current Rust projection before editing its child list".into());
        }
        let indices = current
            .ast()
            .elements()
            .filter_map(|node| {
                Some((
                    node.id,
                    markup.controls.binary_search(&node.span.start).ok()?,
                ))
            })
            .collect::<BTreeMap<_, _>>();
        let index = |node: NodeId| {
            indices
                .get(&node)
                .copied()
                .ok_or("Choose an authored Rust control")
        };
        let mut destinations = BTreeSet::new();
        let mut moving = BTreeSet::new();
        let mut removals = BTreeMap::new();
        let mut departures = BTreeMap::<NodeId, Vec<usize>>::new();
        let mut values = BTreeMap::<usize, Vec<TextPatch>>::new();
        for edit in edits {
            match edit {
                Edit::MoveNode { node, .. } | Edit::RemoveNode { node } => {
                    let source_parent = current
                        .ast()
                        .node(*node)
                        .and_then(|node| node.parent)
                        .ok_or("The Rust control needs an authored parent")?;
                    if !self.nodes[index(source_parent)?].editable_children() {
                        return Err(
                            "Edit the source container's computed child slot in Rust".into()
                        );
                    }
                    let moving_index = index(*node)?;
                    if matches!(edit, Edit::RemoveNode { .. }) {
                        removals.insert(moving_index, source_parent);
                        continue;
                    }
                    if moving.insert(moving_index) {
                        departures.entry(source_parent).or_default().push(moving_index);
                    }
                    destinations.insert(
                        draft
                            .ast()
                            .node(*node)
                            .and_then(|node| node.parent)
                            .ok_or("The Rust control needs an authored destination")?,
                    );
                }
                Edit::InsertChild { parent, .. } => {
                    destinations.insert(*parent);
                }
                Edit::SetAttribute { node, name, value } if !is_namespace(name) => {
                    let index = index(*node)?;
                    values.entry(index).or_default().push(self.layout_value(
                        index,
                        name,
                        Some(value),
                    )?);
                }
                Edit::RemoveAttribute { node, name } => {
                    let index = index(*node)?;
                    values
                        .entry(index)
                        .or_default()
                        .push(self.layout_value(index, name, None)?);
                }
                Edit::SetAttribute { .. } => {}
                _ => return Err("Choose a child insertion, move, removal, or mapped layout value".into()),
            }
        }
        let removed = RemovedSubtrees::record(self, removals, &mut departures);
        if !moving.is_disjoint(&removed.indices) {
            return Err("Move the Rust control before removing its source subtree".into());
        }
        let departing = moving.union(&removed.indices).copied().collect();
        for edits in values.values_mut() {
            merge_insertions(edits);
        }
        let mut patches = Vec::new();
        let mut relocated = BTreeSet::new();
        for parent in destinations {
            let parent_index = index(parent)?;
            let children = control_children(draft, parent).collect::<Vec<_>>();
            let transferred = |node| {
                indices
                    .get(&node)
                    .is_none_or(|index| moving.contains(index))
            };
            let mut at = 0;
            while at < children.len() {
                if !transferred(children[at]) {
                    at += 1;
                    continue;
                }
                let start = at;
                at += children[at..]
                    .iter()
                    .take_while(|&&node| transferred(node))
                    .count();
                let previous = start.checked_sub(1).map(|i| children[i]);
                let boundary =
                    if let Some(anchor) = after.filter(|&anchor| previous == Some(anchor)) {
                        ChildBoundary::After(index(anchor)?)
                    } else if let Some(next) = children.get(at) {
                        ChildBoundary::Before(index(*next)?)
                    } else {
                        ChildBoundary::End
                    };
                let mut items = Vec::new();
                for &node in &children[start..at] {
                    if let Some(&index) = indices.get(&node) {
                        items.push((
                            self.moved_child(
                                index,
                                parent_index,
                                values.remove(&index).unwrap_or_default(),
                            )?,
                            false,
                        ));
                        relocated.insert(index);
                    } else {
                        items.push((XamlTree::fragment(draft, self.catalog, node)?, true));
                    }
                }
                patches.push(self.insert_elements(parent_index, boundary, &items, &departing)?);
            }
        }
        if relocated != moving {
            return Err("The placement lost an authored Rust control".into());
        }
        for (parent, removed) in departures {
            patches.extend(self.detach_children(
                index(parent)?,
                &removed,
                control_children(draft, parent).next().is_none(),
            )?);
        }
        patches.extend(values.into_values().flatten());
        let edited = TextPatch::apply_all(self.source, patches.clone())
            .map_err(|error| error.to_string())?;
        let modules = self.draft_module_sources(&edited)?;
        let actual = self.catalog.project_with_modules(&edited, self.invocation.start, modules.as_ref())?;
        if actual.invocation.start != self.invocation.start
            || actual.omitted_count != self.omitted_count - removed.omitted
            || actual.animation_issues.len() != self.animation_issues.len() - removed.animation_issues
            || actual.style_issues.len() != self.style_issues.len() - removed.style_issues
        {
            return Err("This edit would change an unmapped Rust child or declaration outside the edited controls".into());
        }
        let state_classes = indices
            .iter()
            .filter_map(|(&node, &index)| {
                let groups = self.states.get(&index)?;
                Some((
                    node,
                    (
                        groups.anchor.as_str(),
                        self.nodes[index].properties.contains_key("Classes"),
                    ),
                ))
            })
            .collect();
        if actual.tree != XamlTree::layout(draft, self.catalog, &state_classes)?.tree
            || program_count(self, &removed.indices) != program_count(&actual, &BTreeSet::new())
        {
            return Err("This placement cannot preserve the authored Rust layout".into());
        }
        let mut order = Vec::new();
        layout_order(draft, draft.ast().root(), &mut order);
        if order.len() != actual.nodes.len() {
            return Err("The placed controls have no matching Rust source ranges".into());
        }
        for (&table, node) in order.iter().zip(&actual.nodes) {
            if let Some(columns) = &node.columns {
                if !columns.matches_layout(draft, table)? {
                    return Err("This placement would change the authored Rust columns".into());
                }
            }
        }
        let controls = order
            .into_iter()
            .zip(actual.nodes.iter().map(|node| node.name))
            .collect();
        Ok(RsxChildEdit { patches, controls })
    }

    fn layout_value(
        &self,
        index: usize,
        name: &str,
        value: Option<&str>,
    ) -> Result<TextPatch, String> {
        if matches!(name, "Name" | "Classes") {
            return Err("Edit the control's name or classes separately before moving it".into());
        }
        let patch = self.edit_property(index, name, value)?;
        if !patch.span.is_empty()
            && !crate::blocks::keep_comments(&self.source[patch.span.range()])?.is_empty()
        {
            return Err(format!(
                "Edit the Rust {name} value in source to preserve the comments"
            ));
        }
        Ok(patch)
    }
}

fn program_count(projection: &RsxProjection<'_>, removed: &BTreeSet<usize>) -> (usize, usize) {
    (
        projection
            .styles
            .iter()
            .filter(|(index, _)| !removed.contains(*index))
            .flat_map(|(_, styles)| styles)
            .map(|style| style.clips.len())
            .sum(),
        projection
            .states
            .iter()
            .filter(|(index, _)| !removed.contains(*index))
            .map(|(_, states)| states.groups.len())
            .sum(),
    )
}

fn is_namespace(name: &str) -> bool {
    name == "xmlns" || name.starts_with("xmlns:")
}

fn control_children(document: &Document, parent: NodeId) -> impl Iterator<Item = NodeId> + '_ {
    document
        .ast()
        .element_children(parent)
        .into_iter()
        .filter(|node| {
            document
                .ast()
                .node(*node)
                .is_some_and(|node| node.kind == NodeKind::Element)
        })
}

fn layout_order(document: &Document, node: NodeId, order: &mut Vec<NodeId>) {
    order.push(node);
    for child in control_children(document, node) {
        layout_order(document, child, order);
    }
}
