//! Selection-only navigation. Geometry queries stay in the retained spatial
//! index, and these commands never open a transaction or invalidate rendering.
use crate::{Document, Editor, NodeId, NodeKind, Point, MAX_DOCUMENT_DEPTH};
use std::collections::{BTreeMap, BTreeSet};

mod matching;
pub use matching::SelectionMatch;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionNavigation {
    Parent,
    Children,
    NextSibling,
    PreviousSibling,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionCycle {
    Behind,
    InFront,
}

/// Ancestor membership of the current selection. Keeping this small context
/// separate from geometry lets a canvas cache it across hover events. No node
/// paint payloads, document snapshots or full-scene walks are needed.
#[derive(Clone, Debug, Default)]
pub struct SelectionContext {
    selected: BTreeSet<NodeId>,
    entered: BTreeSet<NodeId>,
}

impl SelectionContext {
    pub fn new(document: &Document, selection: &[NodeId]) -> Self {
        let mut context = Self {
            selected: selection.iter().copied().collect(),
            entered: BTreeSet::new(),
        };
        for id in selection {
            let mut parent = document.nodes.get(id).and_then(|node| node.parent);
            for _ in 0..MAX_DOCUMENT_DEPTH {
                let Some(id) = parent else { break };
                if !context.entered.insert(id) {
                    // This ancestor's complete chain was already recorded.
                    break;
                }
                parent = document.nodes.get(&id).and_then(|node| node.parent);
            }
        }
        context
    }

    /// Resolve an already visible geometry hit to the first unopened frame or
    /// group. Previously selected descendants keep their ancestor scopes open.
    /// Double-click descends through the selected boundary by one level. A
    /// Section organizes independent frames and does not swallow their picks.
    pub fn grouped_target(
        &self,
        document: &Document,
        hit: NodeId,
        descend: bool,
    ) -> Option<NodeId> {
        let mut chain = Vec::new();
        let mut next = Some(hit);
        for _ in 0..MAX_DOCUMENT_DEPTH {
            let Some(id) = next else { break };
            let Some(node) = document.nodes.get(&id) else {
                break;
            };
            chain.push(id);
            next = node.parent;
        }
        for id in chain.into_iter().rev() {
            let node = &document.nodes[&id];
            if id == hit {
                return Some(id);
            }
            if matches!(
                node.kind,
                NodeKind::Frame | NodeKind::Group | NodeKind::Component | NodeKind::Instance
            ) && !self.entered.contains(&id)
                && !(descend && self.selected.contains(&id))
            {
                return Some(id);
            }
        }
        None
    }
}

impl Editor {
    pub fn hit_test_candidates(&self, point: Point, tolerance: f64) -> Vec<NodeId> {
        self.scene
            .hit_test_candidates(&self.document, point, tolerance, false)
    }

    pub fn hit_test_filtered(
        &self,
        point: Point,
        tolerance: f64,
        accept: impl FnMut(NodeId) -> bool,
    ) -> Option<NodeId> {
        self.scene
            .hit_test_filtered(&self.document, point, tolerance, false, accept)
    }

    pub fn prototype_hit_test_filtered(
        &self,
        root: NodeId,
        point: Point,
        tolerance: f64,
        accept: impl FnMut(NodeId) -> bool,
    ) -> Option<NodeId> {
        self.scene.hit_test_in_subtree_filtered(
            &self.document,
            root,
            point,
            tolerance,
            true,
            accept,
        )
    }

    /// Move through an already filtered front-to-back overlap list. The list
    /// may come from exact renderer geometry, alpha coverage or a layer menu.
    /// Revalidate IDs before using it so a stale UI cannot select a deleted,
    /// hidden, locked or cross-page node. Duplicates never introduce extra steps.
    pub fn cycle_selection(&mut self, candidates: &[NodeId], direction: SelectionCycle) -> bool {
        if self.has_active_transaction() {
            return false;
        }
        let mut seen = BTreeSet::new();
        let candidates: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|id| seen.insert(*id) && selectable(self, *id))
            .collect();
        if candidates.is_empty() {
            return false;
        }
        let current = candidates.iter().position(|id| self.selection.contains(id));
        let index = match (current, direction) {
            (Some(index), SelectionCycle::Behind) => (index + 1) % candidates.len(),
            (Some(index), SelectionCycle::InFront) => {
                (index + candidates.len() - 1) % candidates.len()
            }
            (None, SelectionCycle::Behind) => 0,
            (None, SelectionCycle::InFront) => candidates.len() - 1,
        };
        let next = vec![candidates[index]];
        if next == self.selection {
            return false;
        }
        self.selection = next;
        true
    }

    /// Navigate the authored hierarchy in Layers-panel order (frontmost
    /// siblings first). Enter selects immediate children, Shift+Enter their
    /// parent, and Tab/Shift+Tab wrap through eligible siblings. Multi-selection
    /// advances independently within each parent; selecting several siblings
    /// first narrows to the first/last selected sibling in that direction.
    pub fn navigate_selection(&mut self, navigation: SelectionNavigation) -> bool {
        if self.has_active_transaction() {
            return false;
        }
        let selected: Vec<_> = self
            .selection
            .iter()
            .copied()
            .filter(|id| selectable(self, *id))
            .collect();
        let mut next = Vec::new();
        match navigation {
            SelectionNavigation::Parent => {
                for id in selected {
                    let parent = self.document.nodes[&id].parent;
                    next.push(
                        parent
                            .filter(|&parent| selectable(self, parent))
                            .unwrap_or(id),
                    );
                }
            }
            SelectionNavigation::Children => {
                let selected_set: BTreeSet<_> = selected.iter().copied().collect();
                for id in selected {
                    if has_selected_ancestor(&self.document, id, &selected_set) {
                        continue;
                    }
                    let children: Vec<_> = self.document.nodes[&id]
                        .children
                        .iter()
                        .rev()
                        .copied()
                        .filter(|id| selectable(self, *id))
                        .collect();
                    if children.is_empty() {
                        next.push(id);
                    } else {
                        next.extend(children);
                    }
                }
            }
            SelectionNavigation::NextSibling | SelectionNavigation::PreviousSibling => {
                let forward = navigation == SelectionNavigation::NextSibling;
                let mut groups = BTreeMap::<NodeId, Vec<NodeId>>::new();
                let mut parents = Vec::new();
                for id in selected {
                    let Some(parent) = self.document.nodes[&id].parent else {
                        continue;
                    };
                    if !groups.contains_key(&parent) {
                        parents.push(parent);
                    }
                    groups.entry(parent).or_default().push(id);
                }
                if parents.is_empty() {
                    parents.push(self.active_page);
                }
                for parent in parents {
                    let siblings: Vec<_> = self
                        .document
                        .children(parent)
                        .unwrap_or_default()
                        .iter()
                        .rev()
                        .copied()
                        .filter(|id| selectable(self, *id))
                        .collect();
                    if siblings.is_empty() {
                        continue;
                    }
                    let selected = groups.get(&parent).map(Vec::as_slice).unwrap_or_default();
                    let index = if selected.len() == 1 {
                        let index = siblings
                            .iter()
                            .position(|id| *id == selected[0])
                            .unwrap_or(0);
                        if forward {
                            (index + 1) % siblings.len()
                        } else {
                            (index + siblings.len() - 1) % siblings.len()
                        }
                    } else if forward {
                        siblings
                            .iter()
                            .position(|id| selected.contains(id))
                            .unwrap_or(0)
                    } else {
                        siblings
                            .iter()
                            .rposition(|id| selected.contains(id))
                            .unwrap_or(siblings.len() - 1)
                    };
                    next.push(siblings[index]);
                }
            }
        }
        let mut seen = BTreeSet::new();
        next.retain(|id| seen.insert(*id));
        if next.is_empty() || next == self.selection {
            return false;
        }
        self.selection = next;
        true
    }
}

fn selectable(editor: &Editor, id: NodeId) -> bool {
    editor.document.nodes.contains_key(&id)
        && editor.document.page_of(id) == Some(editor.active_page)
        && editor.document.effective_visible(id)
        && !editor.document.effective_locked(id)
}

fn has_selected_ancestor(document: &Document, id: NodeId, selected: &BTreeSet<NodeId>) -> bool {
    let mut parent = document.nodes.get(&id).and_then(|node| node.parent);
    for _ in 0..MAX_DOCUMENT_DEPTH {
        let Some(id) = parent else { break };
        if selected.contains(&id) {
            return true;
        }
        parent = document.nodes.get(&id).and_then(|node| node.parent);
    }
    false
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
