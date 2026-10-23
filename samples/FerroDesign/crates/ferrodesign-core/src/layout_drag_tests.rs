use super::*;

fn fixture(axis: LayoutAxis, reverse: bool, wrap: bool) -> (Editor, NodeId, Vec<NodeId>) {
    let mut document = Document::new("Canvas layout drag");
    let page = document.pages[0].id;
    let mut frame = Node::new(
        NodeKind::Frame,
        "Layout",
        30.0,
        50.0,
        if wrap && axis == LayoutAxis::Row {
            120.0
        } else {
            300.0
        },
        if wrap && axis == LayoutAxis::Column {
            90.0
        } else {
            210.0
        },
    );
    frame.layout = Some(AutoLayout {
        axis,
        reverse,
        wrap,
        gap: 10.0,
        cross_gap: 12.0,
        columns: 3,
        padding: Padding::all(10.0),
        height: SizingMode::Fixed,
        ..AutoLayout::default()
    });
    let parent = document.insert_node(page, frame).unwrap();
    let children = (0..5)
        .map(|index| {
            document
                .insert_node(
                    parent,
                    Node::new(
                        NodeKind::Rectangle,
                        format!("Child {index}"),
                        800.0,
                        900.0,
                        40.0,
                        30.0,
                    ),
                )
                .unwrap()
        })
        .collect();
    assert!(document.layout_all().errors.is_empty());
    (Editor::new(document), parent, children)
}

fn world(document: &Document, parent: NodeId, point: Point) -> Point {
    document.world_transform(parent).unwrap().apply(point)
}

fn leading(document: &Document, parent: NodeId, child: NodeId) -> Point {
    let bounds = document.nodes[&child].bounds();
    let point = if document.nodes[&parent].layout.as_ref().unwrap().axis == LayoutAxis::Column {
        Point::new(bounds.center().x, bounds.y + 1.0)
    } else {
        Point::new(bounds.x + 1.0, bounds.center().y)
    };
    world(document, parent, point)
}

fn trailing(document: &Document, parent: NodeId, child: NodeId) -> Point {
    let bounds = document.nodes[&child].bounds();
    let point = if document.nodes[&parent].layout.as_ref().unwrap().axis == LayoutAxis::Column {
        Point::new(bounds.center().x, bounds.bottom() - 1.0)
    } else {
        Point::new(bounds.right() - 1.0, bounds.center().y)
    };
    world(document, parent, point)
}

#[test]
fn row_and_column_drag_reflow_once_and_restore_exact_history_and_selection() {
    for axis in [LayoutAxis::Row, LayoutAxis::Column] {
        let (mut editor, parent, children) = fixture(axis, false, false);
        let source = children[1];
        editor.selection = vec![source];
        let before = editor.document.to_json().unwrap();
        let revision = editor.revision;
        let session = LayoutDragSession::new(&editor.document, source).unwrap();
        let end = trailing(&editor.document, parent, children[4]);
        for _ in 0..100 {
            let target = session.target(end).unwrap();
            assert_eq!(target.index, 4);
            assert!(session.changes_order(target));
        }
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert_eq!(editor.revision, revision);
        assert!(!editor.has_active_transaction());
        let target = session.target(end).unwrap();
        let result = editor.commit_layout_drag(&session, target).unwrap();
        assert!(!result.changed.is_empty());
        assert_eq!(
            editor.document.nodes[&parent].children,
            vec![children[0], children[2], children[3], children[4], source]
        );
        assert_eq!(editor.history_len(), 1);
        assert_eq!(editor.undo_label(), Some("Reorder layout child"));
        assert_eq!(editor.selection, vec![source]);
        let after = editor.document.to_json().unwrap();
        assert!(editor.undo());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert_eq!(editor.selection, vec![source]);
        assert!(!editor.can_undo());
        assert!(editor.redo());
        assert_eq!(editor.document.to_json().unwrap(), after);
        assert_eq!(editor.selection, vec![source]);
        assert!(
            editor.commit_layout_drag(&session, target).is_err(),
            "A committed session cannot be replayed"
        );
    }
}

#[test]
fn reversed_visual_flow_maps_back_to_stored_sibling_order() {
    for axis in [LayoutAxis::Row, LayoutAxis::Column, LayoutAxis::Grid] {
        let (mut editor, parent, children) = fixture(axis, true, false);
        let source = children[1];
        let session = LayoutDragSession::new(&editor.document, source).unwrap();
        let target = session
            .target(leading(&editor.document, parent, children[4]))
            .unwrap();
        assert_eq!(target.index, 0);
        editor.commit_layout_drag(&session, target).unwrap();
        assert_eq!(
            editor.document.nodes[&parent].children,
            vec![children[0], children[2], children[3], children[4], source]
        );
        assert!(editor.document.layout_all().changed.is_empty());
    }
}

#[test]
fn wrap_targets_distinguish_rows_columns_and_both_edges_at_a_line_break() {
    for axis in [LayoutAxis::Row, LayoutAxis::Column] {
        let (mut editor, parent, children) = fixture(axis, false, true);
        let session = LayoutDragSession::new(&editor.document, children[4]).unwrap();
        let before_second_line = session
            .target(leading(&editor.document, parent, children[2]))
            .unwrap();
        let after_first_line = session
            .target(trailing(&editor.document, parent, children[1]))
            .unwrap();
        assert_eq!(before_second_line.index, 2);
        assert_eq!(after_first_line.index, 2);
        assert_ne!(
            before_second_line.marker, after_first_line.marker,
            "A wrapped slot has a visible insertion edge on either adjacent line"
        );
        editor
            .commit_layout_drag(&session, before_second_line)
            .unwrap();
        assert_eq!(
            editor.document.nodes[&parent].children,
            vec![
                children[0],
                children[1],
                children[4],
                children[2],
                children[3]
            ]
        );
        let moved = editor.document.nodes[&children[4]].bounds();
        let preceding = editor.document.nodes[&children[1]].bounds();
        if axis == LayoutAxis::Row {
            assert!(moved.y > preceding.y);
        } else {
            assert!(moved.x > preceding.x);
        }
    }
}

#[test]
fn grid_reorder_preserves_hidden_absolute_and_pinned_list_slots_and_metadata() {
    for reverse in [false, true] {
        let (mut editor, parent, children) = fixture(LayoutAxis::Grid, reverse, false);
        let pinned = children[2];
        editor
            .document
            .nodes
            .get_mut(&pinned)
            .unwrap()
            .layout_child
            .grid = Some(GridPlacement {
            row: Some(0),
            column: Some(1),
            ..GridPlacement::default()
        });
        let mut hidden = Node::new(NodeKind::Rectangle, "Hidden", 888.0, 777.0, 15.0, 20.0);
        hidden.visible = false;
        let hidden = editor.document.insert_node(parent, hidden).unwrap();
        let mut absolute = Node::new(NodeKind::Rectangle, "Absolute", 555.0, 666.0, 20.0, 15.0);
        absolute.layout_child.absolute = true;
        let absolute = editor.document.insert_node(parent, absolute).unwrap();
        editor.document.nodes.get_mut(&parent).unwrap().children = vec![
            children[0],
            hidden,
            children[1],
            pinned,
            children[3],
            absolute,
            children[4],
        ];
        assert!(editor.document.layout_all().errors.is_empty());
        editor.refresh();
        let excluded: Vec<_> = [hidden, absolute, pinned]
            .iter()
            .map(|id| (*id, editor.document.nodes[id].clone()))
            .collect();
        let source = if reverse { children[4] } else { children[0] };
        let last = if reverse { children[0] } else { children[4] };
        let session = LayoutDragSession::new(&editor.document, source).unwrap();
        let target = session
            .target(trailing(&editor.document, parent, last))
            .unwrap();
        assert_eq!(target.index, 3);
        editor.commit_layout_drag(&session, target).unwrap();
        let order = &editor.document.nodes[&parent].children;
        assert_eq!(order[1], hidden);
        assert_eq!(order[3], pinned);
        assert_eq!(order[5], absolute);
        for (id, mut expected) in excluded {
            expected.revision = editor.document.nodes[&id].revision;
            assert_eq!(editor.document.nodes[&id], expected);
        }
        assert!(editor.document.layout_all().changed.is_empty());
    }
}

#[test]
fn automatic_grid_spans_keep_sizing_and_placement_during_reorder() {
    let (mut editor, parent, children) = fixture(LayoutAxis::Grid, false, false);
    let source = children[4];
    let placement = GridPlacement {
        column_span: 2,
        row_span: 2,
        ..GridPlacement::default()
    };
    let node = editor.document.nodes.get_mut(&source).unwrap();
    node.layout_child.grid = Some(placement);
    node.layout_child.width = SizingMode::Fill;
    node.layout_child.height = SizingMode::Fill;
    assert!(editor.document.layout_all().errors.is_empty());
    editor.refresh();
    let settings = editor.document.nodes[&source].layout_child;
    let session = LayoutDragSession::new(&editor.document, source).unwrap();
    let own_center = world(
        &editor.document,
        parent,
        editor.document.nodes[&source].bounds().center(),
    );
    assert!(!session.changes_order(session.target(own_center).unwrap()));
    let target = session
        .target(leading(&editor.document, parent, children[0]))
        .unwrap();
    assert_eq!(target.index, 0);
    editor.commit_layout_drag(&session, target).unwrap();
    assert_eq!(editor.document.nodes[&source].layout_child, settings);
    assert_eq!(editor.document.nodes[&parent].children[0], source);
    assert!(editor.document.validate().is_ok());
}

#[test]
fn affine_reflected_ancestors_transform_hit_edges_and_preview_outline_exactly() {
    let (mut editor, parent, children) = fixture(LayoutAxis::Row, false, true);
    let page = editor.active_page;
    let mut outer = Node::new(
        NodeKind::Frame,
        "Transformed ancestor",
        420.0,
        120.0,
        700.0,
        550.0,
    );
    outer.rotation = 27.0;
    outer.flip_x = true;
    outer.affine_transform = Some(Transform {
        a: 1.4,
        b: 0.2,
        c: 0.45,
        d: 0.65,
        e: 0.0,
        f: 0.0,
    });
    let outer = editor.document.insert_node(page, outer).unwrap();
    editor.document.pages[0].children.retain(|id| *id != parent);
    editor
        .document
        .nodes
        .get_mut(&outer)
        .unwrap()
        .children
        .push(parent);
    editor.document.nodes.get_mut(&parent).unwrap().parent = Some(outer);
    editor.document.nodes.get_mut(&parent).unwrap().rotation = -11.0;
    let source = children[4];
    editor.document.nodes.get_mut(&source).unwrap().rotation = 18.0;
    editor.refresh();
    let session = LayoutDragSession::new(&editor.document, source).unwrap();
    let bounds = editor.document.nodes[&children[2]].bounds();
    let target = session
        .target(leading(&editor.document, parent, children[2]))
        .unwrap();
    assert_eq!(target.index, 2);
    let transform = editor.document.world_transform(parent).unwrap();
    assert!(
        target
            .marker
            .start
            .distance(transform.apply(Point::new(bounds.x, bounds.y)))
            < 1e-9
    );
    assert!(
        target
            .marker
            .end
            .distance(transform.apply(Point::new(bounds.x, bounds.bottom())))
            < 1e-9
    );
    let source_transform = editor.document.world_transform(source).unwrap();
    assert_eq!(
        session.source_corners(),
        editor.document.nodes[&source]
            .local_bounds()
            .corners()
            .map(|point| source_transform.apply(point))
    );
    let before = editor.document.to_json().unwrap();
    editor.commit_layout_drag(&session, target).unwrap();
    assert_eq!(editor.document.nodes[&source].rotation, 18.0);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
}

#[test]
fn nested_wrap_reorder_records_ancestor_and_other_sibling_reflow_in_one_history_entry() {
    let mut document = Document::new("Nested reflow");
    let page = document.pages[0].id;
    let mut outer = Node::new(NodeKind::Frame, "Column", 0.0, 0.0, 200.0, 400.0);
    outer.layout = Some(AutoLayout {
        axis: LayoutAxis::Column,
        padding: Padding::all(0.0),
        ..AutoLayout::default()
    });
    let outer = document.insert_node(page, outer).unwrap();
    let mut inner = Node::new(NodeKind::Frame, "Wrapped row", 0.0, 0.0, 100.0, 400.0);
    inner.layout = Some(AutoLayout {
        wrap: true,
        gap: 10.0,
        cross_gap: 10.0,
        padding: Padding::all(0.0),
        ..AutoLayout::default()
    });
    let inner = document.insert_node(outer, inner).unwrap();
    let tail = document
        .insert_node(
            outer,
            Node::new(
                NodeKind::Rectangle,
                "Following sibling",
                0.0,
                0.0,
                80.0,
                20.0,
            ),
        )
        .unwrap();
    let a = document
        .insert_node(
            inner,
            Node::new(NodeKind::Rectangle, "Wide", 0.0, 0.0, 60.0, 20.0),
        )
        .unwrap();
    let b = document
        .insert_node(
            inner,
            Node::new(NodeKind::Rectangle, "Tall", 0.0, 0.0, 40.0, 80.0),
        )
        .unwrap();
    document
        .insert_node(
            inner,
            Node::new(NodeKind::Rectangle, "Small", 0.0, 0.0, 40.0, 10.0),
        )
        .unwrap();
    assert!(document.layout_all().errors.is_empty());
    let mut editor = Editor::new(document);
    editor.selection = vec![a];
    let before = editor.document.to_json().unwrap();
    let old_tail = editor.document.nodes[&tail].y;
    let old_height = editor.document.nodes[&outer].height;
    let session = LayoutDragSession::new(&editor.document, a).unwrap();
    let target = session
        .target(trailing(&editor.document, inner, b))
        .unwrap();
    assert_eq!(target.index, 1);
    editor.commit_layout_drag(&session, target).unwrap();
    assert!(editor.document.nodes[&tail].y > old_tail);
    assert!(editor.document.nodes[&outer].height > old_height);
    assert_eq!(editor.history_len(), 1);
    let after = editor.document.to_json().unwrap();
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
}

#[test]
fn rejected_sources_and_instance_descendants_cannot_start_reorder_sessions() {
    let (editor, parent, children) = fixture(LayoutAxis::Grid, false, false);
    let child = children[0];
    for case in 0..7 {
        let mut document = editor.document.clone();
        match case {
            0 => document.nodes.get_mut(&child).unwrap().visible = false,
            1 => document.nodes.get_mut(&child).unwrap().locked = true,
            2 => {
                document
                    .nodes
                    .get_mut(&child)
                    .unwrap()
                    .layout_child
                    .absolute = true
            }
            3 => {
                document.nodes.get_mut(&child).unwrap().layout_child.grid = Some(GridPlacement {
                    row: Some(0),
                    ..GridPlacement::default()
                })
            }
            4 => {
                document.nodes.get_mut(&child).unwrap().layout_child.grid = Some(GridPlacement {
                    column: Some(0),
                    ..GridPlacement::default()
                })
            }
            5 => document.nodes.get_mut(&parent).unwrap().kind = NodeKind::Instance,
            6 => {
                document.nodes.get_mut(&parent).unwrap().affine_transform =
                    Some(Transform::scale(0.0, 1.0))
            }
            _ => unreachable!(),
        }
        assert!(
            LayoutDragSession::new(&document, child).is_err(),
            "case {case}"
        );
    }
    let mut nested = editor.document.clone();
    let instance = nested
        .insert_node(
            editor.active_page,
            Node::new(NodeKind::Instance, "Instance", 0.0, 0.0, 800.0, 600.0),
        )
        .unwrap();
    nested.pages[0].children.retain(|id| *id != parent);
    nested.nodes.get_mut(&parent).unwrap().parent = Some(instance);
    nested
        .nodes
        .get_mut(&instance)
        .unwrap()
        .children
        .push(parent);
    assert!(LayoutDragSession::new(&nested, child).is_err());
    let mut nested_editor = Editor::new(nested);
    assert!(nested_editor
        .reorder_layout_child(child, children[1], true)
        .is_err());
    assert!(!nested_editor.can_undo());
    let mut instance_child = editor.document.clone();
    instance_child.nodes.get_mut(&child).unwrap().kind = NodeKind::Instance;
    assert!(
        LayoutDragSession::new(&instance_child, child).is_ok(),
        "An instance may move inside an editable parent"
    );
}

#[test]
fn cancellation_and_noop_drop_preserve_document_revision_history_and_redo() {
    let (mut editor, parent, children) = fixture(LayoutAxis::Row, false, false);
    editor.selection = vec![children[1]];
    editor.update_node(children[2], "Rename", |node| {
        node.name = "Renamed".into();
    });
    assert!(editor.undo());
    assert!(editor.can_redo());
    let before = editor.document.to_json().unwrap();
    let revision = editor.revision;
    let session = LayoutDragSession::new(&editor.document, children[1]).unwrap();
    let moved = session
        .target(trailing(&editor.document, parent, children[4]))
        .unwrap();
    assert!(session.changes_order(moved));
    let returned = session
        .target(world(
            &editor.document,
            parent,
            editor.document.nodes[&children[1]].bounds().center(),
        ))
        .unwrap();
    assert!(!session.changes_order(returned));
    for _ in 0..3 {
        assert!(editor
            .commit_layout_drag(&session, returned)
            .unwrap()
            .changed
            .is_empty());
    }
    assert_eq!(editor.revision, revision);
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.can_redo());
    assert!(!editor.can_undo());
    assert!(!editor.has_active_transaction());
    drop(session);
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.document.nodes[&children[2]].name, "Renamed");
}

#[test]
fn stale_geometry_parent_order_revision_and_transaction_ownership_fail_atomically() {
    for case in 0..6 {
        let (mut editor, parent, children) = fixture(LayoutAxis::Row, false, false);
        let session = LayoutDragSession::new(&editor.document, children[0]).unwrap();
        let target = session
            .target(trailing(&editor.document, parent, children[4]))
            .unwrap();
        match case {
            0 => editor.document.nodes.get_mut(&children[2]).unwrap().x += 1.0,
            1 => editor
                .document
                .nodes
                .get_mut(&parent)
                .unwrap()
                .children
                .swap(1, 2),
            2 => editor.document.nodes.get_mut(&parent).unwrap().rotation = 30.0,
            3 => editor.refresh(),
            4 => editor.begin_transaction("Another live edit"),
            5 => editor.document.nodes.get_mut(&children[0]).unwrap().locked = true,
            _ => unreachable!(),
        }
        let before = editor.document.to_json().unwrap();
        let revision = editor.revision;
        assert!(
            editor.commit_layout_drag(&session, target).is_err(),
            "case {case}"
        );
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert_eq!(editor.revision, revision);
        assert!(!editor.can_undo());
        assert_eq!(editor.has_active_transaction(), case == 4);
    }
}

#[test]
fn invalid_pointer_and_drop_indices_never_create_history() {
    let (mut editor, parent, children) = fixture(LayoutAxis::Row, false, false);
    let session = LayoutDragSession::new(&editor.document, children[0]).unwrap();
    for point in [Point::new(f64::NAN, 0.0), Point::new(0.0, f64::INFINITY)] {
        assert!(session.target(point).is_none());
    }
    let mut target = session
        .target(trailing(&editor.document, parent, children[4]))
        .unwrap();
    target.index = usize::MAX;
    let before = editor.document.to_json().unwrap();
    assert!(editor.commit_layout_drag(&session, target).is_err());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(!editor.can_undo());
}

#[test]
fn retained_edge_tree_matches_exhaustive_geometric_nearest_targets_on_dense_affine_grid() {
    let (mut editor, parent, mut children) = fixture(LayoutAxis::Grid, false, false);
    for index in 5..1200 {
        children.push(
            editor
                .document
                .insert_node(
                    parent,
                    Node::new(
                        NodeKind::Rectangle,
                        format!("Cell {index}"),
                        0.0,
                        0.0,
                        12.0 + (index % 9) as f64,
                        10.0 + (index % 7) as f64,
                    ),
                )
                .unwrap(),
        );
    }
    let frame = editor.document.nodes.get_mut(&parent).unwrap();
    frame.width = 900.0;
    frame.height = 1600.0;
    frame.rotation = 19.0;
    frame.affine_transform = Some(Transform {
        a: 0.8,
        b: 0.3,
        c: -0.4,
        d: 1.2,
        e: 0.0,
        f: 0.0,
    });
    frame.layout.as_mut().unwrap().columns = 30;
    assert!(editor.document.layout_all().errors.is_empty());
    editor.refresh();
    let session = LayoutDragSession::new(&editor.document, children[500]).unwrap();
    let mut seed = 117u64;
    for _ in 0..240 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let x = ((seed >> 32) % 1400) as f64 - 200.0;
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let y = ((seed >> 32) % 1900) as f64 - 200.0;
        let point = world(&editor.document, parent, Point::new(x, y));
        if session
            .source_bounds
            .contains(session.inverse_world.apply(point))
        {
            continue;
        }
        let expected = session
            .targets
            .iter()
            .min_by(|a, b| {
                segment_distance(point, a.marker.start, a.marker.end)
                    .total_cmp(&segment_distance(point, b.marker.start, b.marker.end))
                    .then_with(|| {
                        (a.index.abs_diff(session.original.index), a.index)
                            .cmp(&(b.index.abs_diff(session.original.index), b.index))
                    })
            })
            .unwrap();
        let actual = session.target(point).unwrap();
        assert_eq!(actual.index, expected.index, "at {point:?}");
    }
}
