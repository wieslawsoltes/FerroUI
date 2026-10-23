//! A cached row projection over the editor's complete, bounded change feed.
//! Native controls consume these rows only when their displayed metadata changes.
use ferrodesign_core::{Editor, EditorRenderChanges, EditorRenderToken, Node, NodeId, NodeKind};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct LayerRow {
    pub(super) id: NodeId,
    pub(super) name: String,
    pub(super) kind: NodeKind,
    pub(super) depth: usize,
    pub(super) expanded: bool,
    pub(super) has_children: bool,
    pub(super) visible: bool,
    pub(super) locked: bool,
    pub(super) assets: bool,
}

impl LayerRow {
    fn matches_node(&self, node: &Node) -> bool {
        self.id == node.id
            && self.name == node.name
            && self.kind == node.kind
            && self.has_children == !node.children.is_empty()
            && self.visible == node.visible
            && self.locked == node.locked
    }
}

struct ProjectionContext {
    token: EditorRenderToken,
    page: NodeId,
    assets: bool,
    filter: String,
    expanded: BTreeSet<NodeId>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct LayerProjectionStats {
    pub(super) builds: u64,
    pub(super) reuses: u64,
}

#[derive(Default)]
pub(super) struct LayerProjection {
    rows: Vec<LayerRow>,
    indices: BTreeMap<NodeId, usize>,
    context: Option<ProjectionContext>,
    stats: LayerProjectionStats,
}

impl LayerProjection {
    pub(super) fn rows(&self) -> &[LayerRow] {
        &self.rows
    }

    pub(super) fn stats(&self) -> LayerProjectionStats {
        self.stats
    }

    /// Refresh pure row data; return whether the native ItemsSource must change.
    /// The caller releases its editor/cache borrows before any native setters.
    pub(super) fn refresh(
        &mut self,
        editor: &Editor,
        assets: bool,
        filter: &str,
        expanded: &BTreeSet<NodeId>,
    ) -> bool {
        if self.can_reuse(editor, assets, filter, expanded) {
            // Consume empty or equivalent changes too, so repeated no-op UI
            // refreshes cannot unnecessarily age this token out of the feed.
            self.context.as_mut().unwrap().token = editor.render_change_token();
            self.stats.reuses = self.stats.reuses.saturating_add(1);
            return false;
        }

        let rows = project_rows(editor, assets, filter, expanded);
        self.stats.builds = self.stats.builds.saturating_add(1);
        let changed = self.rows != rows;
        if changed {
            self.indices = rows.iter().enumerate().map(|(index, row)| (row.id, index)).collect();
            self.rows = rows;
        }
        self.context = Some(ProjectionContext {
            token: editor.render_change_token(),
            page: editor.active_page,
            assets,
            filter: filter.to_owned(),
            // Snapshot only on a full projection. Equality may still compare
            // expanded IDs on reuse; the flat document's empty set is constant
            // work, and no expansion mutation can silently bypass this guard.
            expanded: expanded.clone(),
        });
        changed
    }

    fn can_reuse(
        &self,
        editor: &Editor,
        assets: bool,
        filter: &str,
        expanded: &BTreeSet<NodeId>,
    ) -> bool {
        let Some(context) = &self.context else { return false; };
        if context.page != editor.active_page
            || context.assets != assets
            || context.filter != filter
            || &context.expanded != expanded
            // An excluded node can become a search match after a rename.
            // Keep filtering conservative instead of caching every node name.
            || !filter.is_empty()
        {
            return false;
        }
        let EditorRenderChanges::Nodes { nodes, ancestors } =
            editor.render_changes_since(&context.token)
        else {
            return false;
        };
        for id in nodes.into_iter().chain(ancestors) {
            let Some(&index) = self.indices.get(&id) else {
                // The feed makes hierarchy, ordering, kind and membership
                // changes Full. With an empty search, a changed node outside
                // this page, under a collapsed parent, or outside Assets
                // cannot enter the projection through a content-only edit.
                continue;
            };
            let Some(node) = editor.document.nodes.get(&id) else { return false; };
            if !self.rows[index].matches_node(node) {
                return false;
            }
        }
        true
    }
}

fn project_rows(
    editor: &Editor,
    assets: bool,
    filter: &str,
    expanded: &BTreeSet<NodeId>,
) -> Vec<LayerRow> {
    let roots = if assets {
        editor.document.nodes.values()
            .filter(|node| node.kind == NodeKind::Component)
            .map(|node| node.id)
            .collect::<Vec<_>>()
    } else {
        editor.document.page(editor.active_page)
            .map(|page| page.children.clone())
            .unwrap_or_default()
    };
    // Search visits each matching ancestor chain once, preserving the existing
    // reverse sibling order without realizing native controls for all rows.
    let mut included = BTreeSet::new();
    if !filter.is_empty() {
        for node in editor.document.nodes.values()
            .filter(|node| node.name.to_lowercase().contains(filter))
        {
            let mut id = Some(node.id);
            while let Some(current) = id {
                if !included.insert(current) { break; }
                id = editor.document.nodes.get(&current).and_then(|node| node.parent);
            }
        }
    }
    let mut pending: Vec<_> = roots.into_iter().map(|id| (id, 0)).collect();
    let mut rows = Vec::new();
    while let Some((id, depth)) = pending.pop() {
        if !filter.is_empty() && !included.contains(&id) { continue; }
        let Some(node) = editor.document.nodes.get(&id) else { continue; };
        let open = expanded.contains(&id) || !filter.is_empty();
        rows.push(LayerRow {
            id,
            name: node.name.clone(),
            kind: node.kind,
            depth,
            expanded: open,
            has_children: !node.children.is_empty(),
            visible: node.visible,
            locked: node.locked,
            assets,
        });
        if open && !assets {
            pending.extend(node.children.iter().map(|&child| (child, depth + 1)));
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrodesign_core::Document;

    fn flat_editor(count: usize) -> (Editor, Vec<NodeId>) {
        let mut document = Document::new("Layer cache");
        let page = document.pages[0].id;
        let ids = (0..count).map(|index| {
            document.insert_node(page, Node::new(NodeKind::Rectangle,
                format!("Layer {index}"), index as f64 * 20.0, 0.0, 12.0, 12.0)).unwrap()
        }).collect();
        (Editor::new(document), ids)
    }

    fn tree_editor() -> (Editor, NodeId, NodeId, NodeId) {
        let mut document = Document::new("Layer tree");
        let page = document.pages[0].id;
        let parent = document.insert_node(page,
            Node::new(NodeKind::Frame, "Frame", 0.0, 0.0, 100.0, 100.0)).unwrap();
        let child = document.insert_node(parent,
            Node::new(NodeKind::Rectangle, "Child", 1.0, 2.0, 10.0, 10.0)).unwrap();
        let sibling = document.insert_node(page,
            Node::new(NodeKind::Rectangle, "Sibling", 120.0, 0.0, 10.0, 10.0)).unwrap();
        (Editor::new(document), parent, child, sibling)
    }

    fn ids(projection: &LayerProjection) -> Vec<NodeId> {
        projection.rows().iter().map(|row| row.id).collect()
    }

    #[test]
    fn layer_projection_scoped_move_undo_redo_reuses_10000_rows_and_names() {
        let (mut editor, nodes) = flat_editor(10_000);
        let expanded = BTreeSet::new();
        let mut projection = LayerProjection::default();
        assert!(projection.refresh(&editor, false, "", &expanded));
        assert_eq!(projection.rows().len(), 10_000);
        let row_buffer = projection.rows().as_ptr();
        let first_name = projection.rows()[0].name.as_ptr();
        editor.selection = vec![nodes[5_000]];
        editor.move_selection(1.0, 0.0);
        assert!(!projection.refresh(&editor, false, "", &expanded));
        assert!(editor.undo());
        assert!(!projection.refresh(&editor, false, "", &expanded));
        assert!(editor.redo());
        assert!(!projection.refresh(&editor, false, "", &expanded));
        assert_eq!(projection.stats(), LayerProjectionStats { builds: 1, reuses: 3 });
        assert_eq!(projection.rows().as_ptr(), row_buffer);
        assert_eq!(projection.rows()[0].name.as_ptr(), first_name);
        assert_eq!(ids(&projection), nodes.into_iter().rev().collect::<Vec<_>>());
    }

    #[test]
    fn layer_projection_preview_cancel_and_repeated_noops_advance_the_token() {
        let (mut editor, nodes) = flat_editor(2);
        let expanded = BTreeSet::new();
        let mut projection = LayerProjection::default();
        projection.refresh(&editor, false, "", &expanded);
        editor.begin_node_transaction(&[nodes[0]], "Move preview");
        editor.document.nodes.get_mut(&nodes[0]).unwrap().x += 5.0;
        editor.refresh_preview_nodes(&[nodes[0]]);
        assert!(!projection.refresh(&editor, false, "", &expanded));
        editor.cancel_transaction();
        assert!(!projection.refresh(&editor, false, "", &expanded));
        assert_eq!(editor.history_len(), 0);
        // More than the feed's retained revision count: every successful reuse
        // must advance its token, including unchanged scoped refreshes.
        for _ in 0..160 {
            editor.refresh_preview_nodes(&[]);
            assert!(!projection.refresh(&editor, false, "", &expanded));
        }
        assert_eq!(projection.stats(), LayerProjectionStats { builds: 1, reuses: 162 });
    }

    #[test]
    fn layer_projection_rebuilds_changed_names_lock_and_visibility() {
        let (mut editor, nodes) = flat_editor(1);
        let expanded = BTreeSet::new();
        let mut projection = LayerProjection::default();
        projection.refresh(&editor, false, "", &expanded);
        editor.update_node(nodes[0], "Rename", |node| node.name = "Renamed".into());
        assert!(projection.refresh(&editor, false, "", &expanded));
        assert_eq!(projection.rows()[0].name, "Renamed");
        editor.toggle_locked(nodes[0]);
        assert!(projection.refresh(&editor, false, "", &expanded));
        assert!(projection.rows()[0].locked);
        editor.toggle_visibility(nodes[0]);
        assert!(projection.refresh(&editor, false, "", &expanded));
        assert!(!projection.rows()[0].visible);
        assert!(editor.undo());
        assert!(projection.refresh(&editor, false, "", &expanded));
        assert!(projection.rows()[0].visible);
        assert_eq!(projection.stats(), LayerProjectionStats { builds: 5, reuses: 0 });
    }

    #[test]
    fn layer_projection_expansion_snapshot_exposes_previously_excluded_edits() {
        let (mut editor, parent, child, sibling) = tree_editor();
        let mut expanded = BTreeSet::new();
        let mut projection = LayerProjection::default();
        projection.refresh(&editor, false, "", &expanded);
        assert_eq!(ids(&projection), vec![sibling, parent]);
        editor.update_node(child, "Rename hidden row", |node| node.name = "Updated child".into());
        assert!(!projection.refresh(&editor, false, "", &expanded));
        expanded.insert(parent);
        assert!(projection.refresh(&editor, false, "", &expanded));
        assert_eq!(ids(&projection), vec![sibling, parent, child]);
        assert_eq!(projection.rows()[2].name, "Updated child");
        assert_eq!(projection.rows()[2].depth, 1);
        assert!(projection.rows()[1].expanded);
        expanded.clear();
        assert!(projection.refresh(&editor, false, "", &expanded));
        assert_eq!(ids(&projection), vec![sibling, parent]);
        assert_eq!(projection.stats(), LayerProjectionStats { builds: 3, reuses: 1 });
    }

    #[test]
    fn layer_projection_topology_order_and_membership_use_full_projection() {
        let (mut editor, parent, child, sibling) = tree_editor();
        let expanded = BTreeSet::from([parent]);
        let mut projection = LayerProjection::default();
        projection.refresh(&editor, false, "", &expanded);
        let page = editor.active_page;
        editor.transact("Reparent", |document| document.reparent(child, page, 0).unwrap());
        assert!(projection.refresh(&editor, false, "", &expanded));
        assert_eq!(ids(&projection), vec![sibling, parent, child]);
        assert_eq!(projection.rows()[2].depth, 0);
        assert!(!projection.rows()[1].has_children);
        editor.transact("Reorder roots", |document| document.page_mut(page).unwrap().children.reverse());
        assert!(projection.refresh(&editor, false, "", &expanded));
        assert_eq!(ids(&projection), vec![child, parent, sibling]);
        editor.selection = vec![parent];
        editor.delete_selection();
        assert!(projection.refresh(&editor, false, "", &expanded));
        assert_eq!(ids(&projection), vec![child, sibling]);
        assert_eq!(projection.stats(), LayerProjectionStats { builds: 4, reuses: 0 });
    }

    #[test]
    fn layer_projection_search_rebuilds_when_an_excluded_node_becomes_a_match() {
        let (mut editor, parent, child, sibling) = tree_editor();
        let expanded = BTreeSet::new();
        let mut projection = LayerProjection::default();
        projection.refresh(&editor, false, "missing", &expanded);
        assert!(projection.rows().is_empty());
        editor.update_node(child, "Rename", |node| node.name = "Now missing matches".into());
        assert!(projection.refresh(&editor, false, "missing", &expanded));
        assert_eq!(ids(&projection), vec![parent, child]);
        editor.update_node(child, "Rename", |node| node.name = "Child".into());
        assert!(projection.refresh(&editor, false, "missing", &expanded));
        assert!(projection.rows().is_empty());
        assert!(projection.refresh(&editor, false, "", &expanded));
        assert_eq!(ids(&projection), vec![sibling, parent]);
        assert!(!projection.refresh(&editor, false, "", &expanded));
        assert_eq!(projection.stats(), LayerProjectionStats { builds: 4, reuses: 1 });
    }

    #[test]
    fn layer_projection_pages_assets_and_kind_changes_are_context_boundaries() {
        let (mut editor, parent, child, sibling) = tree_editor();
        let first_page = editor.active_page;
        let second_page = editor.document.add_page("Second page");
        let component = editor.document.insert_node(second_page,
            Node::new(NodeKind::Component, "Component", 0.0, 0.0, 10.0, 10.0)).unwrap();
        editor.refresh();
        let expanded = BTreeSet::new();
        let mut projection = LayerProjection::default();
        projection.refresh(&editor, false, "", &expanded);
        assert_eq!(ids(&projection), vec![sibling, parent]);
        editor.update_node(component, "Move on another page", |node| node.x += 1.0);
        assert!(!projection.refresh(&editor, false, "", &expanded));
        assert!(editor.set_active_page(second_page));
        assert!(projection.refresh(&editor, false, "", &expanded));
        assert_eq!(ids(&projection), vec![component]);
        assert!(editor.set_active_page(first_page));
        assert!(projection.refresh(&editor, false, "", &expanded));
        assert!(projection.refresh(&editor, true, "", &expanded));
        assert_eq!(ids(&projection), vec![component]);
        assert!(projection.rows()[0].assets);
        editor.update_node(child, "Rename non-asset", |node| node.name = "Changed".into());
        assert!(!projection.refresh(&editor, true, "", &expanded));
        editor.update_node(sibling, "Make component", |node| node.kind = NodeKind::Component);
        assert!(projection.refresh(&editor, true, "", &expanded));
        assert_eq!(ids(&projection), vec![component, sibling]);
        assert_eq!(projection.stats(), LayerProjectionStats { builds: 5, reuses: 2 });
    }

    #[test]
    fn layer_projection_identity_full_refresh_and_expired_feed_are_conservative() {
        let (mut editor, nodes) = flat_editor(1);
        let expanded = BTreeSet::new();
        let mut projection = LayerProjection::default();
        projection.refresh(&editor, false, "", &expanded);
        let replacement = Editor::new(editor.document.clone());
        assert!(!projection.refresh(&replacement, false, "", &expanded));
        assert_eq!(projection.stats().builds, 2, "equal rows do not imply a valid editor identity");
        editor = replacement;
        editor.refresh();
        assert!(!projection.refresh(&editor, false, "", &expanded));
        assert_eq!(projection.stats().builds, 3);
        for _ in 0..160 {
            editor.update_node(nodes[0], "Move", |node| node.x += 1.0);
        }
        assert!(!projection.refresh(&editor, false, "", &expanded));
        assert_eq!(projection.stats().builds, 4, "a missed ledger window rebuilds even equal rows");
        editor.document.revision = editor.document.revision.wrapping_add(1);
        assert!(!projection.refresh(&editor, false, "", &expanded));
        assert_eq!(projection.stats().builds, 5, "revision mismatch cannot reuse cached rows");
    }
}
