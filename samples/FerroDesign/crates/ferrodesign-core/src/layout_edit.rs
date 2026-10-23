//! Validated layout authoring uses a complete transaction because arranging one
//! child can resize ancestors and move siblings in other nested layout passes.
use crate::*;

fn layout_error(message: impl Into<String>) -> DocumentError { DocumentError::Invalid(message.into()) }

fn validate_child_settings(settings: LayoutChild) -> Result<(), DocumentError> {
    if ![settings.min_width, settings.min_height, settings.grow].iter()
        .all(|value| value.is_finite() && *value >= 0.0)
        || settings.max_width.is_some_and(|value| !value.is_finite() || value < settings.min_width)
        || settings.max_height.is_some_and(|value| !value.is_finite() || value < settings.min_height)
    {
        return Err(layout_error("Minimum sizes and growth must be nonnegative; maximum sizes must be at least their minimum"));
    }
    if let Some(grid) = settings.grid { grid.validate()?; }
    Ok(())
}

impl Editor {
    /// Set all four padding sides atomically, then arrange the complete layout
    /// tree. An equal value is a true no-op, including on an unsettled import:
    /// it neither creates an instance override nor consumes a redo entry.
    pub fn set_layout_padding(&mut self, parent: NodeId, padding: Padding) -> Result<LayoutResult, DocumentError> {
        if self.has_active_transaction() {
            return Err(layout_error("Finish the current edit before changing layout padding"));
        }
        if ![padding.top, padding.right, padding.bottom, padding.left].iter()
            .all(|value| value.is_finite() && *value >= 0.0)
        {
            return Err(layout_error("Padding must contain four finite nonnegative numbers"));
        }
        let node = self.document.nodes.get(&parent).ok_or(DocumentError::MissingNode(parent))?;
        let layout = node.layout.as_ref().ok_or_else(|| layout_error("Select an auto-layout frame"))?;
        if self.document.effective_locked(parent) {
            return Err(layout_error("The auto-layout frame is locked"));
        }
        if layout.padding == padding { return Ok(LayoutResult::default()); }
        let mut candidate = self.document.clone();
        let node = candidate.nodes.get_mut(&parent).unwrap();
        let bindings = crate::layout_variables::LayoutBindingSnapshot::capture(node);
        node.layout.as_mut().unwrap().padding = padding;
        bindings.detach_changed(node);
        if node.component_source.is_some() { node.overridden_properties.insert("layout".into()); }
        candidate.validate_layout_node_settings(parent)?;
        self.commit_layout_candidate("Edit layout padding", candidate)
    }

    fn layout_parent_for_edit(&self, child: NodeId) -> Result<NodeId, DocumentError> {
        let node = self.document.nodes.get(&child).ok_or(DocumentError::MissingNode(child))?;
        let parent = node.parent.and_then(|id| self.document.nodes.get(&id))
            .filter(|parent| parent.layout.is_some())
            .ok_or_else(|| layout_error("Select a direct child of an auto-layout frame"))?;
        if self.document.effective_locked(child) || self.document.effective_locked(parent.id) {
            return Err(layout_error("The layout child or its frame is locked"));
        }
        Ok(parent.id)
    }

    pub(crate) fn commit_layout_candidate(&mut self, label: &str, mut candidate: Document) -> Result<LayoutResult, DocumentError> {
        candidate.validate()?;
        let result = candidate.layout_all();
        if !result.errors.is_empty() { return Err(layout_error(result.errors.join("\n"))); }
        candidate.validate()?;
        self.transact(label, |document| *document = candidate);
        Ok(result)
    }

    /// Apply sizing, absolute positioning and optional grid placement together.
    /// Failed validation never changes selection, history or the live document.
    pub fn set_layout_child(&mut self, child: NodeId, settings: LayoutChild) -> Result<LayoutResult, DocumentError> {
        validate_child_settings(settings)?;
        let parent = self.layout_parent_for_edit(child)?;
        let mut candidate = self.document.clone();
        let node = candidate.nodes.get_mut(&child).unwrap();
        if node.layout_child != settings {
            if let Some(layout) = &mut node.layout {
                let sizing_changed = node.layout_child.width != settings.width || node.layout_child.height != settings.height;
                if node.layout_child.width != settings.width { layout.width = settings.width; }
                if node.layout_child.height != settings.height { layout.height = settings.height; }
                if sizing_changed && node.component_source.is_some() { node.overridden_properties.insert("layout".into()); }
            }
            node.layout_child = settings;
            if node.component_source.is_some() { node.overridden_properties.insert("layout_child".into()); }
        }
        candidate.validate_layout_node_settings(parent)?;
        self.commit_layout_candidate("Edit layout child", candidate)
    }

    pub fn set_grid_placement(&mut self, child: NodeId, placement: Option<GridPlacement>) -> Result<LayoutResult, DocumentError> {
        let parent = self.layout_parent_for_edit(child)?;
        if self.document.nodes[&parent].layout.as_ref().unwrap().axis != LayoutAxis::Grid {
            return Err(layout_error("Grid placement requires a grid auto-layout frame"));
        }
        let mut settings = self.document.nodes[&child].layout_child;
        settings.grid = placement.filter(|grid| *grid != GridPlacement::default());
        self.set_layout_child(child, settings)
    }

    /// Move in visual flow order. Reversed layouts translate that order back
    /// into the stored child list; pinned grid coordinates remain unchanged.
    pub fn reorder_layout_child(&mut self, child: NodeId, anchor: NodeId, after: bool) -> Result<LayoutResult, DocumentError> {
        let parent = self.layout_parent_for_edit(child)?;
        crate::layout_drag::validate_layout_reorder_ancestry(&self.document, parent)?;
        if child == anchor { return Err(layout_error("Choose a different sibling as the reorder target")); }
        if self.document.nodes.get(&anchor).and_then(|node| node.parent) != Some(parent) {
            return Err(layout_error("Layout reorder requires two children of the same frame"));
        }
        let mut candidate = self.document.clone();
        let frame = candidate.nodes.get_mut(&parent).unwrap();
        let reverse = frame.layout.as_ref().unwrap().reverse;
        if reverse { frame.children.reverse(); }
        frame.children.retain(|id| *id != child);
        let index = frame.children.iter().position(|id| *id == anchor)
            .ok_or_else(|| layout_error("Reorder target is missing from its frame"))?;
        frame.children.insert(index + usize::from(after), child);
        if reverse { frame.children.reverse(); }
        candidate.validate_layout_node_settings(parent)?;
        self.commit_layout_candidate(if after { "Move layout child after" } else { "Move layout child before" }, candidate)
    }
}

#[cfg(test)]
#[path = "layout_padding_tests.rs"]
mod padding_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) { assert!((actual - expected).abs() < 1e-7, "{actual} != {expected}"); }
    fn grid_document(columns: usize) -> (Document, NodeId) {
        let mut document = Document::new("Grid authoring");
        let page = document.pages[0].id;
        let mut frame = Node::new(NodeKind::Frame, "Grid", 40.0, 50.0, 340.0, 100.0);
        frame.layout = Some(AutoLayout { axis: LayoutAxis::Grid, columns, gap: 10.0, cross_gap: 5.0,
            padding: Padding::all(10.0), ..AutoLayout::default() });
        let frame = document.insert_node(page, frame).unwrap();
        (document, frame)
    }
    fn child(document: &mut Document, frame: NodeId, name: &str, height: f64, grid: Option<GridPlacement>) -> NodeId {
        let mut node = Node::new(NodeKind::Rectangle, name, 700.0, 800.0, 20.0, height);
        node.layout_child.width = SizingMode::Fill;
        node.layout_child.grid = grid;
        document.insert_node(frame, node).unwrap()
    }
    fn at(row: usize, column: usize, row_span: usize, column_span: usize) -> GridPlacement {
        GridPlacement { row: Some(row), column: Some(column), row_span, column_span }
    }
    fn round_trip(document: &Document) {
        let json = document.to_json().unwrap();
        assert_eq!(Document::from_json(&json).unwrap().to_json().unwrap(), json);
    }

    #[test]
    fn absent_grid_metadata_keeps_legacy_json_and_partial_row_geometry() {
        let (mut document, frame) = grid_document(5);
        let a = child(&mut document, frame, "A", 20.0, None);
        let b = child(&mut document, frame, "B", 30.0, None);
        let before = document.to_json().unwrap();
        assert!(!before.contains("\"grid\":"));
        let result = document.layout_all();
        assert!(result.errors.is_empty());
        close(document.nodes[&a].width, 155.0);
        close(document.nodes[&b].x, 175.0);
        close(document.nodes[&frame].height, 50.0);
        let geometry = document.nodes.clone();
        document.nodes.get_mut(&a).unwrap().layout_child.grid = Some(GridPlacement::default());
        document.layout_all();
        for (id, previous) in geometry {
            assert_eq!(document.nodes[&id].bounds(), previous.bounds());
        }
        document.nodes.get_mut(&a).unwrap().layout_child.grid = None;
        round_trip(&document);
    }

    #[test]
    fn explicit_partial_and_spanning_cells_reserve_nonoverlapping_tracks() {
        let (mut document, frame) = grid_document(3);
        let a = child(&mut document, frame, "Auto", 20.0, None);
        let pinned = child(&mut document, frame, "Pinned", 30.0, Some(at(0, 1, 1, 2)));
        let span = child(&mut document, frame, "Span", 70.0,
            Some(GridPlacement { row_span: 2, column_span: 2, ..GridPlacement::default() }));
        document.nodes.get_mut(&span).unwrap().layout_child.height = SizingMode::Fill;
        let d = child(&mut document, frame, "Auto next", 15.0, None);
        let row = child(&mut document, frame, "Fixed row", 20.0,
            Some(GridPlacement { row: Some(3), ..GridPlacement::default() }));
        let column = child(&mut document, frame, "Fixed column", 25.0,
            Some(GridPlacement { column: Some(2), ..GridPlacement::default() }));
        let hidden = child(&mut document, frame, "Hidden", 99.0, Some(at(0, 1, 1, 2)));
        document.nodes.get_mut(&hidden).unwrap().visible = false;
        let absolute = child(&mut document, frame, "Absolute", 98.0, Some(at(0, 1, 1, 2)));
        document.nodes.get_mut(&absolute).unwrap().layout_child.absolute = true;
        let result = document.layout_all();
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        close(document.nodes[&a].x, 10.0);
        close(document.nodes[&a].width, 100.0);
        close(document.nodes[&pinned].x, 120.0);
        close(document.nodes[&pinned].width, 210.0);
        close(document.nodes[&span].x, 10.0);
        close(document.nodes[&span].y, 45.0);
        close(document.nodes[&span].width, 210.0);
        close(document.nodes[&span].height, 70.0);
        close(document.nodes[&d].x, 230.0);
        close(document.nodes[&column].y, 82.5);
        close(document.nodes[&row].y, 120.0);
        close(document.nodes[&frame].height, 150.0);
        for id in [hidden, absolute] {
            close(document.nodes[&id].x, 700.0);
            close(document.nodes[&id].y, 800.0);
            assert!(!result.changed.contains(&id));
        }
        assert!(document.layout_all().changed.is_empty());
        round_trip(&document);
    }

    #[test]
    fn grid_hug_fill_minimum_and_maximum_apply_to_whole_spans() {
        let (mut document, frame) = grid_document(3);
        document.nodes.get_mut(&frame).unwrap().layout.as_mut().unwrap().width = SizingMode::Hug;
        let span = child(&mut document, frame, "Wide", 30.0, Some(at(0, 0, 1, 2)));
        {
            let node = document.nodes.get_mut(&span).unwrap();
            node.width = 150.0;
            node.layout_child.min_width = 160.0;
            node.layout_child.max_width = Some(170.0);
            node.layout_child.min_height = 40.0;
            node.layout_child.max_height = Some(50.0);
        }
        let other = child(&mut document, frame, "Other", 15.0, None);
        document.nodes.get_mut(&other).unwrap().width = 50.0;
        document.layout_all();
        close(document.nodes[&frame].width, 265.0);
        close(document.nodes[&span].width, 160.0);
        close(document.nodes[&span].height, 40.0);
        close(document.nodes[&other].width, 75.0);
        close(document.nodes[&other].x, 180.0);
        {
            let layout = document.nodes.get_mut(&frame).unwrap().layout.as_mut().unwrap();
            layout.width = SizingMode::Fixed;
            layout.height = SizingMode::Fixed;
            layout.cross_align = LayoutAlign::Stretch;
        }
        document.nodes.get_mut(&frame).unwrap().width = 340.0;
        document.nodes.get_mut(&frame).unwrap().height = 120.0;
        document.layout_all();
        close(document.nodes[&span].width, 170.0);
        close(document.nodes[&span].height, 50.0);
        close(document.nodes[&other].height, 100.0);
        assert!(document.layout_all().changed.is_empty());
        round_trip(&document);
    }

    #[test]
    fn overlapping_fill_row_spans_have_stable_hug_sizes_and_noop_history() {
        let (mut document, frame) = grid_document(2);
        {
            let layout = document.nodes.get_mut(&frame).unwrap().layout.as_mut().unwrap();
            layout.padding = Padding::all(0.0);
            layout.gap = 0.0;
            layout.cross_gap = 0.0;
        }
        let a = child(&mut document, frame, "A", 100.0, Some(at(0, 0, 2, 1)));
        let b = child(&mut document, frame, "B", 100.0, Some(at(1, 1, 2, 1)));
        for id in [a, b] { document.nodes.get_mut(&id).unwrap().layout_child.height = SizingMode::Fill; }
        assert!(document.layout_all().errors.is_empty());
        close(document.nodes[&frame].height, 150.0);
        close(document.nodes[&a].height, 100.0);
        close(document.nodes[&b].height, 100.0);
        close(document.nodes[&b].y, 50.0);
        assert!(document.layout_all().changed.is_empty());
        let mut editor = Editor::new(document);
        let before = editor.document.to_json().unwrap();
        let settings = editor.document.nodes[&a].layout_child;
        for _ in 0..3 { editor.set_layout_child(a, settings).unwrap(); }
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(!editor.can_undo(), "Hug/Fill must not drift into a new history entry on an unchanged Apply");
        round_trip(&editor.document);
    }

    #[test]
    fn invalid_grid_edits_are_atomic_and_direct_layout_reports_bounded_errors() {
        let (mut document, frame) = grid_document(3);
        let a = child(&mut document, frame, "Pinned", 30.0, Some(at(0, 0, 1, 2)));
        let b = child(&mut document, frame, "Auto", 20.0, None);
        document.layout_all();
        let mut editor = Editor::new(document);
        editor.selection = vec![b];
        editor.set_grid_placement(b, Some(at(1, 0, 1, 1))).unwrap();
        assert!(editor.undo());
        let before = editor.document.to_json().unwrap();
        let revision = editor.revision;
        for placement in [at(0, 1, 1, 1), at(0, 2, 1, 2), at(0, 0, 0, 1),
            at(usize::MAX, 0, 2, 1), at(MAX_GRID_ROWS - 1, 0, 2, 1)] {
            assert!(editor.set_grid_placement(b, Some(placement)).is_err());
            assert_eq!(editor.document.to_json().unwrap(), before);
            assert_eq!(editor.revision, revision);
            assert_eq!(editor.selection, vec![b]);
            assert!(editor.can_redo());
        }
        let mut invalid = editor.document.clone();
        invalid.nodes.get_mut(&b).unwrap().layout_child.grid = Some(at(0, 1, 1, 1));
        assert!(invalid.validate().is_err());
        let invalid_before = invalid.to_json().unwrap();
        let result = invalid.layout_node(frame);
        assert!(result.changed.is_empty());
        assert!(!result.errors.is_empty());
        assert_eq!(invalid.to_json().unwrap(), invalid_before);
        invalid.nodes.get_mut(&b).unwrap().layout_child.grid = Some(at(MAX_GRID_ROWS - 1, 0, 1, 1));
        invalid.nodes.get_mut(&frame).unwrap().layout.as_mut().unwrap().columns = MAX_GRID_COLUMNS;
        assert!(!invalid.layout_node(frame).errors.is_empty());
        assert_eq!(invalid.nodes[&a].width, editor.document.nodes[&a].width);
        assert!(Document::from_json(&invalid.to_json().unwrap()).is_err());
        invalid.nodes.get_mut(&b).unwrap().layout_child.grid = None;
        let layout = invalid.nodes.get_mut(&frame).unwrap().layout.as_mut().unwrap();
        layout.columns = 3;
        layout.gap = f64::MAX;
        let overflow_before = invalid.to_json().unwrap();
        let overflow = invalid.layout_node(frame);
        assert!(overflow.changed.is_empty());
        assert!(overflow.errors.iter().any(|error| error.contains("overflow")));
        assert_eq!(invalid.to_json().unwrap(), overflow_before);
    }

    #[test]
    fn native_layout_transaction_reorders_reversed_flow_and_restores_exact_history() {
        let (mut document, frame) = grid_document(3);
        let a = child(&mut document, frame, "A", 20.0, None);
        let b = child(&mut document, frame, "B", 20.0, None);
        let c = child(&mut document, frame, "C", 20.0, None);
        {
            let layout = document.nodes.get_mut(&frame).unwrap().layout.as_mut().unwrap();
            layout.axis = LayoutAxis::Row;
            layout.reverse = true;
        }
        for id in [a, b, c] { document.nodes.get_mut(&id).unwrap().layout_child.width = SizingMode::Fixed; }
        document.layout_all();
        let mut editor = Editor::new(document);
        editor.selection = vec![a];
        let before = editor.document.to_json().unwrap();
        editor.reorder_layout_child(a, c, false).unwrap();
        assert_eq!(editor.document.nodes[&frame].children, vec![b, c, a]);
        close(editor.document.nodes[&a].x, 10.0);
        close(editor.document.nodes[&c].x, 40.0);
        close(editor.document.nodes[&b].x, 70.0);
        assert_eq!(editor.selection, vec![a]);
        let reordered = editor.document.to_json().unwrap();
        editor.reorder_layout_child(a, c, false).unwrap();
        assert!(editor.undo());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(!editor.can_undo(), "Repeated unchanged reorder must not create history");
        assert!(editor.redo());
        assert_eq!(editor.document.to_json().unwrap(), reordered);
        editor.reorder_layout_child(a, b, true).unwrap();
        assert_eq!(editor.document.nodes[&frame].children, vec![a, b, c]);
        let after = editor.document.to_json().unwrap();
        assert!(editor.undo()); assert!(editor.redo());
        assert_eq!(editor.document.to_json().unwrap(), after);
        round_trip(&editor.document);
    }

    #[test]
    fn pinned_grid_cells_survive_reorder_and_child_sizing_undo_is_atomic() {
        let (mut document, frame) = grid_document(3);
        let a = child(&mut document, frame, "A", 20.0, None);
        let pinned = child(&mut document, frame, "Pinned", 30.0, Some(at(0, 1, 1, 1)));
        let b = child(&mut document, frame, "B", 40.0, None);
        document.layout_all();
        let mut editor = Editor::new(document);
        let pinned_bounds = editor.document.nodes[&pinned].bounds();
        let before = editor.document.to_json().unwrap();
        editor.reorder_layout_child(b, a, false).unwrap();
        close(editor.document.nodes[&b].x, 10.0);
        close(editor.document.nodes[&a].x, 230.0);
        assert_eq!(editor.document.nodes[&pinned].bounds(), pinned_bounds);
        let mut settings = editor.document.nodes[&a].layout_child;
        settings.grid = Some(at(1, 0, 1, 3));
        settings.height = SizingMode::Fill;
        settings.min_height = 65.0;
        editor.set_layout_child(a, settings).unwrap();
        close(editor.document.nodes[&a].width, 320.0);
        close(editor.document.nodes[&a].height, 65.0);
        close(editor.document.nodes[&frame].height, 130.0);
        let after = editor.document.to_json().unwrap();
        assert!(editor.undo()); assert!(editor.undo());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert!(editor.redo()); assert!(editor.redo());
        assert_eq!(editor.document.to_json().unwrap(), after);
        let mut bad = settings;
        bad.max_height = Some(10.0);
        assert!(editor.set_layout_child(a, bad).is_err());
        assert_eq!(editor.document.to_json().unwrap(), after);
        round_trip(&editor.document);
    }
}
