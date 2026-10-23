use super::*;
use crate::{Color, Document, LocalStyleKind, NodeKind, StyleValue};

fn columns(alignment: LayoutGridAlignment, count: Option<u32>) -> LayoutGrid {
    LayoutGrid { size: 20.0, pattern: LayoutGridPattern::Columns {
        axis: LayoutGridAxis { alignment, count, gutter: 10.0, offset: 15.0 }
    }, ..LayoutGrid::default() }
}
fn fixture() -> (Editor, NodeId, NodeId) {
    let mut document = Document::new("Layout guides");
    let a = document.insert_node(1, Node::new(NodeKind::Frame, "A", 0.0, 0.0, 150.0, 100.0)).unwrap();
    let page = document.add_page("Other page");
    let b = document.insert_node(page, Node::new(NodeKind::Frame, "B", 0.0, 0.0, 150.0, 100.0)).unwrap();
    (Editor::new(document), a, b)
}

#[test]
fn fixed_guides_resolve_exact_start_center_end_alignment() {
    for (alignment, start) in [(LayoutGridAlignment::Start, 15.0), (LayoutGridAlignment::Center, 35.0), (LayoutGridAlignment::End, 55.0)] {
        let bands = columns(alignment, Some(3)).bands(150.0).unwrap();
        assert_eq!(bands, LayoutGridBands { start, size: 20.0, step: 30.0, count: 3 });
    }
}

#[test]
fn stretch_tracks_retain_margins_and_gutters_across_frame_resize() {
    let grid = columns(LayoutGridAlignment::Stretch, Some(3));
    let a = grid.bands(150.0).unwrap();
    let b = grid.bands(300.0).unwrap();
    assert_eq!(a.start, 15.0); assert_eq!(b.start, 15.0);
    for (extent, bands) in [(150.0, a), (300.0, b)] {
        assert!((bands.start + (bands.count - 1) as f64 * bands.step + bands.size - (extent - 15.0)).abs() < 1e-10);
        assert_eq!(bands.step - bands.size, 10.0);
    }
    assert!(grid.bands(50.0).is_none(), "no positive width remains after fixed margins and gutters");
}

#[test]
fn automatic_counts_are_finite_and_fixed_sections_fit_available_extent() {
    let start = columns(LayoutGridAlignment::Start, None).bands(150.0).unwrap();
    assert_eq!(start.count, 4);
    assert_eq!(start.size, 20.0);
    let centered = columns(LayoutGridAlignment::Center, None).bands(150.0).unwrap();
    assert_eq!(centered.count, 5);
    assert_eq!(centered.start, 5.0);
    let stretched = columns(LayoutGridAlignment::Stretch, None).bands(150.0).unwrap();
    assert_eq!(stretched.count, 4);
    assert_eq!(stretched.size, 22.5);
    assert!(columns(LayoutGridAlignment::Start, None).bands(20.0).is_none());
    assert!(columns(LayoutGridAlignment::End, None).bands(f64::INFINITY).is_none());
}

#[test]
fn bands_compute_viewport_intersection_without_scanning_authored_count() {
    let grid = columns(LayoutGridAlignment::Start, Some(MAX_LAYOUT_GUIDE_SECTIONS));
    let bands = grid.bands(1e9).unwrap();
    assert_eq!(bands.visible_range(15.0, 35.0), 0..1);
    assert_eq!(bands.visible_range(35.0, 45.0), 1..1, "gutter has no band");
    assert_eq!(bands.visible_range(46.0, 47.0), 1..2);
    assert_eq!(bands.visible_range(1e8, 1e8 + 100.0), MAX_LAYOUT_GUIDE_SECTIONS..MAX_LAYOUT_GUIDE_SECTIONS);
    assert_eq!(bands.visible_range(-1e9, -1e8), 0..0);
    assert_eq!(bands.visible_range(f64::NAN, 50.0), 0..0);
}

#[test]
fn legacy_square_and_new_stack_documents_round_trip_without_migration_loss() {
    let (mut editor, a, _) = fixture();
    editor.edit_layout_guides(&[a], LayoutGuideEdit::Add(LayoutGrid::default())).unwrap();
    let old = editor.to_json().unwrap();
    assert!(!old.contains("layout_grid_overlays")); assert!(!old.contains("\"pattern\""));
    assert_eq!(Document::from_json(&old).unwrap().to_json().unwrap(), old);
    editor.edit_layout_guides(&[a], LayoutGuideEdit::Add(columns(LayoutGridAlignment::End, None))).unwrap();
    let new = editor.to_json().unwrap();
    assert!(new.contains("layout_grid_overlays"));
    assert_eq!(Document::from_json(&new).unwrap().to_json().unwrap(), new);
}

#[test]
fn stack_add_edit_duplicate_reorder_remove_are_exact_single_history_transactions() {
    let (mut editor, a, _) = fixture();
    for edit in [
        LayoutGuideEdit::Add(LayoutGrid::default()),
        LayoutGuideEdit::Add(columns(LayoutGridAlignment::Stretch, Some(4))),
        LayoutGuideEdit::Duplicate(1),
        LayoutGuideEdit::Set { index: 2, property: LayoutGuideProperty::Color(Color::BLACK) },
        LayoutGuideEdit::Move { index: 2, to: 0 },
        LayoutGuideEdit::Remove(1),
        LayoutGuideEdit::Clear,
    ] {
        let before = editor.to_json().unwrap();
        let depth = editor.history_len();
        assert_eq!(editor.edit_layout_guides(&[a, a], edit).unwrap(), 1);
        let after = editor.to_json().unwrap();
        assert_ne!(before, after); assert_eq!(editor.history_len(), depth + 1);
        assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), before);
        assert!(editor.redo()); assert_eq!(editor.to_json().unwrap(), after);
    }
}

#[test]
fn indexed_edits_leave_missing_rows_and_effective_locks_unchanged() {
    let (mut editor, a, b) = fixture();
    editor.edit_layout_guides(&[a], LayoutGuideEdit::Add(LayoutGrid::default())).unwrap();
    assert_eq!(editor.edit_layout_guides(&[a, b], LayoutGuideEdit::Set { index: 1, property: LayoutGuideProperty::Size(20.0) }).unwrap(), 0);
    editor.document.nodes.get_mut(&b).unwrap().locked = true;
    assert_eq!(editor.edit_layout_guides(&[a, b], LayoutGuideEdit::Add(columns(LayoutGridAlignment::Start, Some(3)))).unwrap(), 1);
    assert_eq!(editor.document.nodes[&a].layout_guide_count(), 2);
    assert_eq!(editor.document.nodes[&b].layout_guide_count(), 0);
}

#[test]
fn orientation_changes_preserve_each_targets_latest_fields_with_exact_history() {
    let (mut editor, a, b) = fixture();
    editor.edit_layout_guides(&[a, b], LayoutGuideEdit::Add(columns(LayoutGridAlignment::Stretch, Some(12)))).unwrap();
    // The UI constructs this command before fields are edited without rebuilding
    // its combo box. Applying it must resolve each target's current axis values.
    let rows = LayoutGuideEdit::Set { index: 0, property: LayoutGuideProperty::Kind(LayoutGridKind::Rows) };
    for (id, alignment, count, gutter, offset, size, color, visible) in [
        (a, LayoutGridAlignment::End, Some(3), 8.0, 12.0, 36.0, Color::rgba(34, 197, 94, 128), true),
        (b, LayoutGridAlignment::Start, None, 6.0, 9.0, 18.0, Color::BLACK, false),
    ] {
        for property in [LayoutGuideProperty::Alignment(alignment), LayoutGuideProperty::Count(count),
            LayoutGuideProperty::Gutter(gutter), LayoutGuideProperty::Offset(offset), LayoutGuideProperty::Size(size),
            LayoutGuideProperty::Color(color), LayoutGuideProperty::Visible(visible)] {
            editor.edit_layout_guides(&[id], LayoutGuideEdit::Set { index: 0, property }).unwrap();
        }
    }
    let originals = [a, b].map(|id| *editor.document.nodes[&id].layout_guide(0).unwrap());
    let before = editor.to_json().unwrap(); let depth = editor.history_len();
    assert_eq!(editor.edit_layout_guides(&[a, b], rows).unwrap(), 2);
    for (id, original) in [a, b].into_iter().zip(originals) {
        assert_eq!(editor.document.nodes[&id].layout_guide(0), Some(&LayoutGrid {
            pattern: LayoutGridPattern::Rows { axis: original.pattern.axis().unwrap() }, ..original
        }));
    }
    let after = editor.to_json().unwrap();
    assert_ne!(before, after); assert_eq!(editor.history_len(), depth + 1);
    assert!(editor.undo()); assert_eq!(editor.to_json().unwrap(), before);
    assert!(editor.redo()); assert_eq!(editor.to_json().unwrap(), after);
    assert_eq!(editor.edit_layout_guides(&[a, b], rows).unwrap(), 0);
    assert_eq!(editor.history_len(), depth + 1);
    editor.edit_layout_guides(&[a, b], LayoutGuideEdit::Set {
        index: 0, property: LayoutGuideProperty::Kind(LayoutGridKind::Columns),
    }).unwrap();
    for (id, original) in [a, b].into_iter().zip(originals) {
        assert_eq!(editor.document.nodes[&id].layout_guide(0), Some(&original));
    }
}

#[test]
fn invalid_batches_and_guide_limits_preserve_every_field_and_redo() {
    let (mut editor, a, b) = fixture();
    editor.edit_layout_guides(&[a, b], LayoutGuideEdit::Add(columns(LayoutGridAlignment::Start, Some(3)))).unwrap();
    editor.edit_layout_guides(&[a], LayoutGuideEdit::Set { index: 0, property: LayoutGuideProperty::Size(16.0) }).unwrap();
    editor.undo();
    let before = editor.to_json().unwrap(); let revision = editor.revision; let depth = editor.history_len();
    for property in [LayoutGuideProperty::Count(Some(0)), LayoutGuideProperty::Count(Some(MAX_LAYOUT_GUIDE_SECTIONS + 1)),
        LayoutGuideProperty::Gutter(-1.0), LayoutGuideProperty::Offset(f64::NAN), LayoutGuideProperty::Size(f64::INFINITY)] {
        assert!(editor.edit_layout_guides(&[a, b], LayoutGuideEdit::Set { index: 0, property }).is_err());
    }
    assert!(editor.edit_layout_guides(&[a, 99999], LayoutGuideEdit::Clear).is_err());
    assert_eq!(editor.to_json().unwrap(), before); assert_eq!(editor.revision, revision); assert_eq!(editor.history_len(), depth);
    assert!(editor.redo());
    let node = editor.document.nodes.get_mut(&b).unwrap(); node.set_layout_guides(&vec![LayoutGrid::default(); MAX_LAYOUT_GUIDES]);
    let before = editor.to_json().unwrap();
    assert!(editor.edit_layout_guides(&[a, b], LayoutGuideEdit::Add(LayoutGrid::default())).is_err());
    assert_eq!(editor.to_json().unwrap(), before, "full target rejects the entire batch before changing an earlier target");
}

#[test]
fn grid_stack_styles_propagate_across_pages_preserving_local_visibility_and_detach_on_geometry() {
    let (mut editor, a, b) = fixture();
    editor.edit_layout_guides(&[a], LayoutGuideEdit::Add(LayoutGrid::default())).unwrap();
    editor.edit_layout_guides(&[a], LayoutGuideEdit::Add(columns(LayoutGridAlignment::Stretch, Some(3)))).unwrap();
    let style = editor.create_local_style_for_nodes(&[a, b], LocalStyleKind::Grid, "Responsive", "Two guides").unwrap();
    assert!(matches!(editor.document.styles[&style].value, StyleValue::GridStack { .. }));
    editor.edit_layout_guides(&[b], LayoutGuideEdit::Set { index: 1, property: LayoutGuideProperty::Visible(false) }).unwrap();
    assert_eq!(editor.document.nodes[&b].grid_style, Some(style));
    editor.document.nodes.get_mut(&b).unwrap().locked = true;
    let mut grids: Vec<_> = editor.document.nodes[&a].layout_guides().copied().collect();
    grids[1].pattern = columns(LayoutGridAlignment::Stretch, Some(6)).pattern;
    let result = editor.update_local_style(style, "Responsive", "Six columns", StyleValue::GridStack { grids: grids.clone() }).unwrap();
    assert_eq!(result.updated_nodes, 2);
    assert_eq!(editor.document.nodes[&b].layout_grid_overlays[0].pattern, grids[1].pattern);
    assert!(!editor.document.nodes[&b].layout_grid_overlays[0].visible);
    editor.edit_layout_guides(&[a], LayoutGuideEdit::Set { index: 1, property: LayoutGuideProperty::Gutter(8.0) }).unwrap();
    assert_eq!(editor.document.nodes[&a].grid_style, None);
    let retained: Vec<_> = editor.document.nodes[&b].layout_guides().copied().collect();
    editor.remove_local_style(style).unwrap();
    assert_eq!(editor.document.nodes[&b].layout_guides().copied().collect::<Vec<_>>(), retained);
}

#[test]
fn old_grid_styles_can_be_upgraded_to_stacks_without_changing_style_identity() {
    let (mut editor, a, b) = fixture();
    let style = editor.create_local_style_for_nodes(&[a, b], LocalStyleKind::Grid, "Grid", "").unwrap();
    assert!(matches!(editor.document.styles[&style].value, StyleValue::Grid { .. }));
    let grids = vec![LayoutGrid::default(), columns(LayoutGridAlignment::Stretch, Some(4))];
    assert_eq!(editor.update_local_style(style, "Grid", "", StyleValue::GridStack { grids }).unwrap().updated_nodes, 2);
    assert_eq!(editor.document.nodes[&a].layout_guide_count(), 2);
    assert_eq!(editor.document.nodes[&b].grid_style, Some(style));
}

#[test]
fn unsupported_nodes_and_orphan_overlays_fail_document_validation() {
    let (mut editor, a, _) = fixture();
    editor.document.nodes.get_mut(&a).unwrap().layout_grid_overlays.push(LayoutGrid::default());
    assert!(editor.document.validate().is_err());
    let node = editor.document.nodes.get_mut(&a).unwrap(); node.layout_grid = Some(LayoutGrid::default()); node.kind = NodeKind::Rectangle;
    assert!(editor.document.validate().is_err());
}

#[test]
fn clearing_an_empty_shared_stack_detaches_ownership_in_one_edit() {
    let (mut editor, a, _) = fixture();
    let style = editor.create_local_style("Empty guides", "", StyleValue::GridStack { grids: vec![] }).unwrap();
    editor.apply_local_style(&[a], style).unwrap();
    assert_eq!(editor.document.nodes[&a].grid_style, Some(style));
    let before = editor.to_json().unwrap();
    assert_eq!(editor.edit_layout_guides(&[a], LayoutGuideEdit::Clear).unwrap(), 1);
    assert_eq!(editor.document.nodes[&a].grid_style, None);
    editor.undo(); assert_eq!(editor.to_json().unwrap(), before);
}

#[test]
fn component_refresh_preserves_independent_stack_and_per_index_visibility_overrides() {
    let (mut editor, a, _) = fixture();
    editor.selection = vec![a];
    let component = editor.create_component().unwrap();
    editor.edit_layout_guides(&[component], LayoutGuideEdit::Add(LayoutGrid::default())).unwrap();
    editor.edit_layout_guides(&[component], LayoutGuideEdit::Add(columns(LayoutGridAlignment::Stretch, Some(3)))).unwrap();
    let instance = editor.create_instance(component, 400.0, 10.0).unwrap();
    editor.edit_layout_guides(&[instance], LayoutGuideEdit::Set { index: 1, property: LayoutGuideProperty::Visible(false) }).unwrap();
    editor.edit_layout_guides(&[component], LayoutGuideEdit::Set { index: 1, property: LayoutGuideProperty::Count(Some(4)) }).unwrap();
    editor.refresh_component_instances(component).unwrap();
    assert!(!editor.document.nodes[&instance].layout_grid_overlays[0].visible);
    assert_eq!(editor.document.nodes[&instance].layout_grid_overlays[0].pattern.axis().unwrap().count, Some(4));
    editor.edit_layout_guides(&[instance], LayoutGuideEdit::Duplicate(1)).unwrap();
    let own: Vec<_> = editor.document.nodes[&instance].layout_guides().copied().collect();
    editor.edit_layout_guides(&[component], LayoutGuideEdit::Clear).unwrap();
    editor.refresh_component_instances(component).unwrap();
    assert_eq!(editor.document.nodes[&instance].layout_guides().copied().collect::<Vec<_>>(), own);
}
