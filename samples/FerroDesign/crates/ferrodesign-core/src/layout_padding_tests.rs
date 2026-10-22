use super::*;

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
}

fn fixture() -> (Editor, NodeId, [NodeId; 2]) {
    let mut document = Document::new("Four-side padding");
    let page = document.pages[0].id;
    let mut frame = Node::new(NodeKind::Frame, "Row", 40.0, 50.0, 300.0, 200.0);
    frame.layout = Some(AutoLayout { padding: Padding::all(10.0), gap: 7.0, cross_gap: 11.0,
        width: SizingMode::Fixed, height: SizingMode::Fixed, ..AutoLayout::default() });
    frame.metadata.insert("customer".into(), "unchanged".into());
    let parent = document.insert_node(page, frame).unwrap();
    let a = document.insert_node(parent, Node::new(NodeKind::Rectangle, "A", 0.0, 0.0, 40.0, 20.0)).unwrap();
    let b = document.insert_node(parent, Node::new(NodeKind::Rectangle, "B", 0.0, 0.0, 50.0, 30.0)).unwrap();
    assert!(document.layout_all().errors.is_empty());
    let mut editor = Editor::new(document);
    editor.selection = vec![parent];
    (editor, parent, [a, b])
}

#[test]
fn independent_padding_reflows_without_replacing_other_layout_settings_and_has_exact_history() {
    let (mut editor, parent, [a, b]) = fixture();
    let before = editor.document.to_json().unwrap();
    let old_layout = editor.document.nodes[&parent].layout.clone().unwrap();
    let padding = Padding { top: 12.5, right: 24.75, bottom: 6.25, left: 33.125 };
    editor.set_layout_padding(parent, padding).unwrap();
    let mut expected_layout = old_layout;
    expected_layout.padding = padding;
    assert_eq!(editor.document.nodes[&parent].layout, Some(expected_layout));
    close(editor.document.nodes[&a].x, 33.125);
    close(editor.document.nodes[&a].y, 12.5);
    close(editor.document.nodes[&b].x, 80.125);
    close(editor.document.nodes[&b].y, 12.5);
    assert_eq!(editor.document.nodes[&parent].metadata["customer"], "unchanged");
    assert_eq!(editor.document.nodes[&parent].children, [a, b]);
    assert_eq!(editor.selection, [parent]);
    assert_eq!(editor.history_len(), 1);
    let after = editor.document.to_json().unwrap();
    assert_eq!(Document::from_json(&after).unwrap().to_json().unwrap(), after);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
}

#[test]
fn padding_updates_explicit_grid_available_space_without_touching_tracks_or_placements() {
    let (mut editor, parent, children) = fixture();
    let layout = editor.document.nodes.get_mut(&parent).unwrap().layout.as_mut().unwrap();
    layout.axis = LayoutAxis::Grid;
    layout.columns = 2;
    layout.column_tracks = vec![GridTrack::Fixed(60.0), GridTrack::Fraction(1.0)];
    layout.row_tracks = vec![GridTrack::Fraction(1.0)];
    for id in children {
        let child = &mut editor.document.nodes.get_mut(&id).unwrap().layout_child;
        child.width = SizingMode::Fill;
        child.height = SizingMode::Fill;
    }
    editor.document.layout_all(); editor.refresh();
    let before_layout = editor.document.nodes[&parent].layout.clone().unwrap();
    editor.set_layout_padding(parent, Padding { top: 11.5, right: 37.0, bottom: 19.25, left: 23.0 }).unwrap();
    let layout = editor.document.nodes[&parent].layout.as_ref().unwrap();
    assert_eq!(layout.column_tracks, before_layout.column_tracks);
    assert_eq!(layout.row_tracks, before_layout.row_tracks);
    close(editor.document.nodes[&children[0]].x, 23.0);
    close(editor.document.nodes[&children[0]].width, 60.0);
    close(editor.document.nodes[&children[1]].x, 90.0);
    close(editor.document.nodes[&children[1]].width, 173.0);
    for id in children {
        close(editor.document.nodes[&id].y, 11.5);
        close(editor.document.nodes[&id].height, 169.25);
        assert!(editor.document.nodes[&id].layout_child.grid.is_none());
    }
}

#[test]
fn nested_hug_padding_moves_siblings_and_restores_the_entire_tree_in_one_step() {
    let mut document = Document::new("Nested padding");
    let page = document.pages[0].id;
    let mut outer = Node::new(NodeKind::Frame, "Column", 20.0, 30.0, 1.0, 1.0);
    outer.layout = Some(AutoLayout { axis: LayoutAxis::Column, gap: 8.0, padding: Padding::all(5.0),
        width: SizingMode::Hug, height: SizingMode::Hug, ..AutoLayout::default() });
    let outer = document.insert_node(page, outer).unwrap();
    let mut inner = Node::new(NodeKind::Frame, "Row", 0.0, 0.0, 1.0, 1.0);
    inner.layout = Some(AutoLayout { padding: Padding::all(10.0), width: SizingMode::Hug,
        height: SizingMode::Hug, ..AutoLayout::default() });
    let inner = document.insert_node(outer, inner).unwrap();
    document.insert_node(inner, Node::new(NodeKind::Rectangle, "Content", 0.0, 0.0, 40.0, 20.0)).unwrap();
    let sibling = document.insert_node(outer, Node::new(NodeKind::Rectangle, "Sibling", 0.0, 0.0, 50.0, 15.0)).unwrap();
    assert!(document.layout_all().errors.is_empty());
    let mut editor = Editor::new(document);
    let before = editor.document.to_json().unwrap();
    editor.set_layout_padding(inner, Padding { top: 4.5, right: 31.0, bottom: 7.0, left: 20.0 }).unwrap();
    close(editor.document.nodes[&inner].width, 91.0);
    close(editor.document.nodes[&inner].height, 31.5);
    close(editor.document.nodes[&outer].width, 101.0);
    close(editor.document.nodes[&outer].height, 64.5);
    close(editor.document.nodes[&sibling].y, 44.5);
    let after = editor.document.to_json().unwrap();
    assert_eq!(editor.history_len(), 1);
    editor.undo(); assert_eq!(editor.document.to_json().unwrap(), before);
    editor.redo(); assert_eq!(editor.document.to_json().unwrap(), after);
}

#[test]
fn empty_hug_frame_can_author_zero_and_fractional_sides() {
    let mut document = Document::new("Empty padding");
    let page = document.pages[0].id;
    let mut frame = Node::new(NodeKind::Frame, "Empty", 10.0, 20.0, 30.0, 40.0);
    frame.layout = Some(AutoLayout { width: SizingMode::Hug, height: SizingMode::Hug, ..AutoLayout::default() });
    let parent = document.insert_node(page, frame).unwrap();
    let mut editor = Editor::new(document);
    editor.set_layout_padding(parent, Padding { top: 0.0, right: 22.75, bottom: 13.125, left: 0.0 }).unwrap();
    close(editor.document.nodes[&parent].width, 22.75);
    close(editor.document.nodes[&parent].height, 13.125);
    assert!(editor.document.nodes[&parent].children.is_empty());
    assert_eq!(editor.history_len(), 1);
}

#[test]
fn invalid_padding_and_overflow_are_atomic_and_preserve_real_redo() {
    let (mut editor, parent, _) = fixture();
    editor.set_layout_padding(parent, Padding::all(20.0)).unwrap();
    assert!(editor.undo());
    let before = editor.document.to_json().unwrap();
    let token = editor.render_change_token();
    for index in 0..4 {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.125] {
            let mut sides = [12.0, 13.0, 14.0, 15.0]; sides[index] = invalid;
            let padding = Padding { top: sides[0], right: sides[1], bottom: sides[2], left: sides[3] };
            assert!(editor.set_layout_padding(parent, padding).is_err());
            assert_eq!(editor.document.to_json().unwrap(), before);
            assert_eq!(editor.render_change_token(), token);
            assert_eq!(editor.history_len(), 0);
            assert!(editor.can_redo());
        }
    }
    assert!(editor.redo());
    assert_eq!(editor.document.nodes[&parent].layout.as_ref().unwrap().padding, Padding::all(20.0));
    let layout = editor.document.nodes.get_mut(&parent).unwrap().layout.as_mut().unwrap();
    layout.width = SizingMode::Hug;
    editor.refresh();
    let before = editor.document.to_json().unwrap();
    let token = editor.render_change_token();
    assert!(editor.set_layout_padding(parent, Padding::all(f64::MAX)).is_err());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.render_change_token(), token);
}

#[test]
fn equal_padding_preserves_unsettled_geometry_render_token_and_redo() {
    let (mut editor, parent, [a, _]) = fixture();
    // Imported geometry may not yet have been laid out. Equal submission must
    // not use a document-wide layout as an incidental repair operation.
    editor.document.nodes.get_mut(&a).unwrap().x = 777.25;
    editor.refresh();
    editor.set_layout_padding(parent, Padding::all(18.0)).unwrap();
    let after = editor.document.to_json().unwrap();
    editor.undo();
    let before = editor.document.to_json().unwrap();
    let token = editor.render_change_token();
    let revision = editor.revision;
    for _ in 0..3 { assert_eq!(editor.set_layout_padding(parent, Padding::all(10.0)).unwrap(), LayoutResult::default()); }
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.render_change_token(), token);
    assert_eq!(editor.revision, revision);
    assert_eq!(editor.history_len(), 0);
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
}

#[test]
fn padding_respects_inherited_locks_missing_layout_and_transaction_ownership() {
    let (mut editor, parent, [a, _]) = fixture();
    let page = editor.active_page;
    let ancestor = editor.document.insert_node(page, Node::new(NodeKind::Frame, "Locked ancestor", 0.0, 0.0, 500.0, 500.0)).unwrap();
    editor.document.pages.iter_mut().find(|p| p.id == page).unwrap().children.retain(|id| *id != parent);
    editor.document.nodes.get_mut(&ancestor).unwrap().children.push(parent);
    editor.document.nodes.get_mut(&parent).unwrap().parent = Some(ancestor);
    editor.document.nodes.get_mut(&ancestor).unwrap().locked = true;
    editor.refresh();
    let before = editor.document.to_json().unwrap();
    let token = editor.render_change_token();
    for id in [parent, a, u64::MAX] {
        assert!(editor.set_layout_padding(id, Padding::all(30.0)).is_err());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert_eq!(editor.render_change_token(), token);
    }
    editor.document.nodes.get_mut(&ancestor).unwrap().locked = false;
    editor.refresh();
    editor.begin_transaction("Owned gesture");
    let before = editor.document.to_json().unwrap();
    assert!(editor.set_layout_padding(parent, Padding::all(30.0)).is_err());
    assert!(editor.has_active_transaction());
    assert_eq!(editor.document.to_json().unwrap(), before);
    editor.cancel_transaction();
    assert_eq!(editor.history_len(), 0);
}

#[test]
fn changed_instance_padding_is_an_override_but_equal_padding_is_not() {
    let (mut editor, component, _) = fixture();
    editor.document.nodes.get_mut(&component).unwrap().kind = NodeKind::Component;
    let instance = editor.document.create_component_instance(component, editor.active_page, 400.0, 50.0).unwrap();
    editor.document.layout_all(); editor.refresh();
    let before = editor.document.to_json().unwrap();
    editor.set_layout_padding(instance, Padding::all(10.0)).unwrap();
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(!editor.document.nodes[&instance].overridden_properties.contains("layout"));
    let independent = Padding { top: 2.5, right: 6.25, bottom: 11.75, left: 20.0 };
    editor.set_layout_padding(instance, independent).unwrap();
    assert!(editor.document.nodes[&instance].overridden_properties.contains("layout"));
    let after = editor.document.to_json().unwrap();
    editor.undo(); assert_eq!(editor.document.to_json().unwrap(), before);
    editor.redo(); assert_eq!(editor.document.to_json().unwrap(), after);
    editor.set_layout_padding(component, Padding::all(30.0)).unwrap();
    editor.sync_instances(component);
    assert_eq!(editor.document.nodes[&instance].layout.as_ref().unwrap().padding, independent);
    assert_eq!(editor.document.nodes[&component].layout.as_ref().unwrap().padding, Padding::all(30.0));
}
