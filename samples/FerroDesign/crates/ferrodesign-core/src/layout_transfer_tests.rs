use super::*;

fn frame(
    document: &mut Document,
    parent: NodeId,
    name: &str,
    x: f64,
    y: f64,
    axis: Option<LayoutAxis>,
) -> NodeId {
    let mut frame = Node::new(NodeKind::Frame, name, x, y, 260.0, 160.0);
    frame.layout = axis.map(|axis| AutoLayout {
        axis,
        gap: 8.0,
        cross_gap: 9.0,
        padding: Padding::all(10.0),
        height: SizingMode::Fixed,
        columns: 3,
        ..AutoLayout::default()
    });
    document.insert_node(parent, frame).unwrap()
}

fn child(document: &mut Document, parent: NodeId, name: &str) -> NodeId {
    document
        .insert_node(
            parent,
            Node::new(NodeKind::Rectangle, name, 0.0, 0.0, 32.0, 24.0),
        )
        .unwrap()
}

fn editor(mut document: Document, selection: &[NodeId]) -> Editor {
    let layout = document.layout_all();
    assert!(layout.errors.is_empty(), "{:?}", layout.errors);
    document.validate().unwrap();
    let mut editor = Editor::new(document);
    editor.selection = selection.to_vec();
    editor
}

fn drag(editor: &Editor, anchor: NodeId) -> LayoutTransferSession {
    let origin = editor
        .document
        .world_transform(anchor)
        .unwrap()
        .apply(editor.document.nodes[&anchor].local_bounds().center());
    LayoutTransferSession::new(&editor.document, &editor.selection, anchor, origin).unwrap()
}

fn before(editor: &Editor, parent: NodeId, child: NodeId) -> Point {
    let node = &editor.document.nodes[&child];
    let row = editor.document.nodes[&parent].layout.as_ref().unwrap().axis != LayoutAxis::Column;
    let p = if row {
        Point::new(node.x, node.y + node.height * 0.5)
    } else {
        Point::new(node.x + node.width * 0.5, node.y)
    };
    editor.document.world_transform(parent).unwrap().apply(p)
}

fn linear_close(a: Transform, b: Transform) {
    for (a, b) in [(a.a, b.a), (a.b, b.b), (a.c, b.c), (a.d, b.d)] {
        assert!((a - b).abs() < 1e-8, "{a} != {b}");
    }
}

#[test]
fn cross_frame_multi_selection_keeps_visual_order_and_exact_single_history() {
    let mut document = Document::new("Transfer");
    let page = document.pages[0].id;
    let source = frame(
        &mut document,
        page,
        "Source",
        10.0,
        20.0,
        Some(LayoutAxis::Row),
    );
    let destination = frame(
        &mut document,
        page,
        "Destination",
        400.0,
        20.0,
        Some(LayoutAxis::Column),
    );
    let a: Vec<_> = (0..5)
        .map(|i| child(&mut document, source, &format!("A{i}")))
        .collect();
    let b: Vec<_> = (0..3)
        .map(|i| child(&mut document, destination, &format!("B{i}")))
        .collect();
    let mut editor = editor(document, &[a[3], a[1]]);
    let original = editor.to_json().unwrap();
    let session = drag(&editor, a[1]);
    let target = session.target(before(&editor, destination, b[1])).unwrap();
    assert_eq!(
        (target.parent, target.index, target.kind),
        (destination, 1, LayoutTransferKind::Flow)
    );
    assert_eq!(editor.to_json().unwrap(), original);
    editor.commit_layout_transfer(&session, target).unwrap();
    assert_eq!(editor.document.nodes[&source].children, [a[0], a[2], a[4]]);
    assert_eq!(
        editor.document.nodes[&destination].children,
        [b[0], a[1], a[3], b[1], b[2]]
    );
    assert_eq!(editor.selection, [a[3], a[1]]);
    assert_eq!(editor.document.nodes[&a[1]].parent, Some(destination));
    assert_eq!(editor.document.nodes[&a[3]].y, 74.0);
    assert_eq!(editor.history_len(), 1);
    let dropped = editor.to_json().unwrap();
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), original);
    assert!(editor.redo());
    assert_eq!(editor.to_json().unwrap(), dropped);
}

#[test]
fn multi_flow_reorder_handles_row_column_wrap_grid_and_reverse() {
    for axis in [LayoutAxis::Row, LayoutAxis::Column, LayoutAxis::Grid] {
        for reverse in [false, true] {
            for wrap in [false, true] {
                let mut document = Document::new("Flows");
                let page = document.pages[0].id;
                let parent = frame(&mut document, page, "Flow", 0.0, 0.0, Some(axis));
                let layout = document
                    .nodes
                    .get_mut(&parent)
                    .unwrap()
                    .layout
                    .as_mut()
                    .unwrap();
                layout.reverse = reverse;
                layout.wrap = wrap;
                let ids: Vec<_> = (0..6)
                    .map(|i| child(&mut document, parent, &i.to_string()))
                    .collect();
                let mut visual = ids.clone();
                if reverse {
                    visual.reverse();
                }
                let mut editor = editor(document, &[visual[3], visual[1]]);
                let session = drag(&editor, visual[1]);
                let target = session
                    .target_in(parent, before(&editor, parent, visual[0]))
                    .unwrap();
                assert_eq!(target.index, 0);
                editor.commit_layout_transfer(&session, target).unwrap();
                let mut expected = vec![
                    visual[1], visual[3], visual[0], visual[2], visual[4], visual[5],
                ];
                if reverse {
                    expected.reverse();
                }
                assert_eq!(
                    editor.document.nodes[&parent].children, expected,
                    "{axis:?}, reverse={reverse}, wrap={wrap}"
                );
                assert_eq!(editor.history_len(), 1);
            }
        }
    }
}

#[test]
fn unselected_pinned_hidden_and_absolute_slots_remain_exact_during_same_parent_reorder() {
    let mut document = Document::new("Reserved slots");
    let page = document.pages[0].id;
    let parent = frame(
        &mut document,
        page,
        "Grid",
        0.0,
        0.0,
        Some(LayoutAxis::Grid),
    );
    document
        .nodes
        .get_mut(&parent)
        .unwrap()
        .layout
        .as_mut()
        .unwrap()
        .reverse = true;
    let a = child(&mut document, parent, "A");
    let hidden = child(&mut document, parent, "Hidden");
    document.nodes.get_mut(&hidden).unwrap().visible = false;
    let b = child(&mut document, parent, "B");
    let pinned = child(&mut document, parent, "Pinned");
    document.nodes.get_mut(&pinned).unwrap().layout_child.grid = Some(GridPlacement {
        row: Some(2),
        column: Some(1),
        ..GridPlacement::default()
    });
    let c = child(&mut document, parent, "C");
    let absolute = child(&mut document, parent, "Absolute");
    document
        .nodes
        .get_mut(&absolute)
        .unwrap()
        .layout_child
        .absolute = true;
    let d = child(&mut document, parent, "D");
    let mut editor = editor(document, &[a, c]);
    let reserved: Vec<_> = [hidden, pinned, absolute]
        .iter()
        .map(|id| editor.document.nodes[id].clone())
        .collect();
    let session = drag(&editor, c);
    let target = session
        .target_in(parent, before(&editor, parent, b))
        .unwrap();
    editor.commit_layout_transfer(&session, target).unwrap();
    assert_eq!(
        editor.document.nodes[&parent].children,
        [b, hidden, a, pinned, c, absolute, d]
    );
    for node in reserved {
        assert_eq!(editor.document.nodes[&node.id], node);
    }
}

#[test]
fn selected_pins_become_automatic_without_losing_spans_or_child_sizing() {
    let mut document = Document::new("Pinned transfer");
    let page = document.pages[0].id;
    let source = frame(
        &mut document,
        page,
        "Source",
        0.0,
        0.0,
        Some(LayoutAxis::Grid),
    );
    let destination = frame(
        &mut document,
        page,
        "Destination",
        400.0,
        0.0,
        Some(LayoutAxis::Grid),
    );
    let pin = child(&mut document, source, "Pinned span");
    document.nodes.get_mut(&pin).unwrap().layout_child = LayoutChild {
        grid: Some(GridPlacement {
            row: Some(1),
            column: Some(0),
            row_span: 2,
            column_span: 2,
        }),
        min_width: 14.0,
        max_width: Some(120.0),
        grow: 2.5,
        ..LayoutChild::default()
    };
    let reserved = child(&mut document, destination, "Reserved");
    document.nodes.get_mut(&reserved).unwrap().layout_child.grid = Some(GridPlacement {
        row: Some(0),
        column: Some(0),
        ..GridPlacement::default()
    });
    let mut editor = editor(document, &[pin]);
    let session = drag(&editor, pin);
    let previous = editor.document.nodes[&pin].layout_child;
    let reserved_before = editor.document.nodes[&reserved].clone();
    let target = session
        .target_in(destination, Point::new(420.0, 20.0))
        .unwrap();
    editor.commit_layout_transfer(&session, target).unwrap();
    assert_eq!(
        editor.document.nodes[&pin].layout_child,
        LayoutChild {
            grid: Some(GridPlacement {
                row: None,
                column: None,
                row_span: 2,
                column_span: 2
            }),
            ..previous
        }
    );
    assert_eq!(editor.document.nodes[&reserved], reserved_before);
    assert_eq!(editor.history_len(), 1);
}

#[test]
fn affine_cross_frame_and_page_drops_preserve_translated_world_geometry_and_absolute_flags() {
    let mut document = Document::new("Affine transfer");
    let page = document.pages[0].id;
    let source = frame(
        &mut document,
        page,
        "Source",
        0.0,
        0.0,
        Some(LayoutAxis::Row),
    );
    document
        .nodes
        .get_mut(&source)
        .unwrap()
        .set_local_transform(Transform {
            a: 1.3,
            b: 0.2,
            c: 0.5,
            d: 0.9,
            e: 10.0,
            f: 20.0,
        });
    let destination = frame(&mut document, page, "Free", 500.0, 40.0, None);
    document
        .nodes
        .get_mut(&destination)
        .unwrap()
        .set_local_transform(Transform {
            a: -1.0,
            b: 0.25,
            c: 0.3,
            d: 1.2,
            e: 700.0,
            f: 40.0,
        });
    let a = child(&mut document, source, "A");
    let b = child(&mut document, source, "Absolute");
    document.nodes.get_mut(&a).unwrap().rotation = 27.0;
    document.nodes.get_mut(&b).unwrap().layout_child.absolute = true;
    let mut editor = editor(document, &[a, b]);
    let original = editor.to_json().unwrap();
    let session = drag(&editor, a);
    let target = session
        .target_in(destination, Point::new(570.0, 100.0))
        .unwrap();
    let delta = target.position - session.origin;
    let expected: Vec<_> = session
        .nodes
        .iter()
        .map(|node| {
            (
                node.geometry.id,
                Transform::translation(delta.x, delta.y) * node.world,
            )
        })
        .collect();
    editor.commit_layout_transfer(&session, target).unwrap();
    for (id, expected) in expected {
        let actual = editor.document.world_transform(id).unwrap();
        linear_close(actual, expected);
        assert!((actual.e - expected.e).abs() < 1e-8 && (actual.f - expected.f).abs() < 1e-8);
    }
    assert!(editor.document.nodes[&b].layout_child.absolute);
    assert!(editor.document.nodes[&source].children.is_empty());
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), original);
    let session = drag(&editor, a);
    let target = session.target(Point::new(-700.0, -700.0)).unwrap();
    assert_eq!(
        (target.parent, target.kind),
        (page, LayoutTransferKind::Free)
    );
    editor.commit_layout_transfer(&session, target).unwrap();
    assert_eq!(editor.document.nodes[&a].parent, Some(page));
    assert_eq!(editor.document.nodes[&b].parent, Some(page));
}

#[test]
fn multi_source_parents_and_ancestor_selection_move_each_subtree_once() {
    let mut document = Document::new("Roots");
    let page = document.pages[0].id;
    let first = frame(
        &mut document,
        page,
        "First",
        0.0,
        0.0,
        Some(LayoutAxis::Row),
    );
    let second = frame(
        &mut document,
        page,
        "Second",
        400.0,
        0.0,
        Some(LayoutAxis::Column),
    );
    let destination = frame(
        &mut document,
        page,
        "Destination",
        800.0,
        0.0,
        Some(LayoutAxis::Row),
    );
    let outer = frame(
        &mut document,
        first,
        "Selected frame",
        0.0,
        0.0,
        Some(LayoutAxis::Row),
    );
    let nested = child(&mut document, outer, "Selected descendant");
    let other = child(&mut document, second, "Other source");
    let mut editor = editor(document, &[nested, other, outer]);
    let session = drag(&editor, nested);
    assert_eq!(session.nodes.len(), 2);
    let origin = session.target(session.origin).unwrap();
    assert_eq!(origin.kind, LayoutTransferKind::Restore);
    editor.commit_layout_transfer(&session, origin).unwrap();
    assert_eq!(editor.history_len(), 0);
    let target = session
        .target_in(destination, Point::new(810.0, 20.0))
        .unwrap();
    editor.commit_layout_transfer(&session, target).unwrap();
    assert_eq!(editor.document.nodes[&destination].children, [outer, other]);
    assert_eq!(editor.document.nodes[&nested].parent, Some(outer));
    assert_eq!(editor.selection, [nested, other, outer]);
    assert_eq!(editor.history_len(), 1);
}

#[test]
fn sole_child_empty_frame_and_returned_selection_preserve_noop_redo() {
    let mut document = Document::new("Empty transfer");
    let page = document.pages[0].id;
    let source = frame(
        &mut document,
        page,
        "Source",
        0.0,
        0.0,
        Some(LayoutAxis::Row),
    );
    let destination = frame(
        &mut document,
        page,
        "Empty",
        400.0,
        0.0,
        Some(LayoutAxis::Column),
    );
    let a = child(&mut document, source, "Only");
    let mut editor = editor(document, &[a]);
    let session = drag(&editor, a);
    let original = editor.to_json().unwrap();
    let revision = editor.revision;
    editor
        .commit_layout_transfer(
            &session,
            session.target_in(source, Point::new(200.0, 20.0)).unwrap(),
        )
        .unwrap();
    assert_eq!(editor.revision, revision);
    assert_eq!(editor.history_len(), 0);
    let target = session
        .target_in(destination, Point::new(410.0, 20.0))
        .unwrap();
    editor.commit_layout_transfer(&session, target).unwrap();
    assert!(editor.undo());
    let session = drag(&editor, a);
    let revision = editor.revision;
    editor
        .commit_layout_transfer(&session, session.target(session.origin).unwrap())
        .unwrap();
    assert_eq!(editor.to_json().unwrap(), original);
    assert_eq!(editor.revision, revision);
    assert!(editor.can_redo());
}

#[test]
fn candidate_frame_hit_testing_honors_painter_order_affine_clips_and_cycles() {
    let mut document = Document::new("Destination hit");
    let page = document.pages[0].id;
    let source = frame(
        &mut document,
        page,
        "Source",
        -400.0,
        0.0,
        Some(LayoutAxis::Row),
    );
    let a = child(&mut document, source, "A");
    let back = frame(
        &mut document,
        page,
        "Back",
        400.0,
        0.0,
        Some(LayoutAxis::Row),
    );
    let nested = frame(
        &mut document,
        back,
        "Nested",
        0.0,
        0.0,
        Some(LayoutAxis::Column),
    );
    let front = frame(
        &mut document,
        page,
        "Front",
        400.0,
        0.0,
        Some(LayoutAxis::Grid),
    );
    let clip = frame(&mut document, page, "Clip", 800.0, 0.0, None);
    document.nodes.get_mut(&clip).unwrap().width = 80.0;
    document.nodes.get_mut(&clip).unwrap().clip_content = true;
    let overflow = frame(
        &mut document,
        clip,
        "Overflow",
        60.0,
        0.0,
        Some(LayoutAxis::Row),
    );
    let editor = editor(document, &[a]);
    let session = drag(&editor, a);
    assert_eq!(
        session.target(Point::new(450.0, 50.0)).unwrap().parent,
        front
    );
    assert_eq!(
        session.target(Point::new(870.0, 50.0)).unwrap().parent,
        overflow
    );
    assert_eq!(
        session.target(Point::new(900.0, 50.0)).unwrap().parent,
        page
    );
    assert_ne!(
        session.target(Point::new(450.0, 50.0)).unwrap().parent,
        nested
    );
    let mut editor = editor;
    editor.selection = vec![back];
    let session = drag(&editor, back);
    assert!(session.target_in(nested, Point::new(450.0, 50.0)).is_none());
    let target = LayoutTransferTarget {
        parent: nested,
        index: 0,
        marker: session.original_marker,
        kind: LayoutTransferKind::Flow,
        position: Point::new(450.0, 50.0),
    };
    assert!(editor.commit_layout_transfer(&session, target).is_err());
}

#[test]
fn instance_descendants_locked_frames_and_invalid_spans_are_rejected_atomically() {
    let mut document = Document::new("Protections");
    let page = document.pages[0].id;
    let component = document
        .insert_node(
            page,
            Node::new(NodeKind::Component, "Component", 0.0, 0.0, 120.0, 80.0),
        )
        .unwrap();
    document.nodes.get_mut(&component).unwrap().layout = Some(AutoLayout::default());
    child(&mut document, component, "Content");
    let instance = document
        .create_component_instance(component, page, 300.0, 0.0)
        .unwrap();
    let descendant = document.nodes[&instance].children[0];
    let source = frame(
        &mut document,
        page,
        "Source",
        -300.0,
        0.0,
        Some(LayoutAxis::Grid),
    );
    let a = child(&mut document, source, "Span");
    document.nodes.get_mut(&a).unwrap().layout_child.grid = Some(GridPlacement {
        column_span: 2,
        ..GridPlacement::default()
    });
    let destination = frame(
        &mut document,
        page,
        "Narrow",
        700.0,
        0.0,
        Some(LayoutAxis::Grid),
    );
    document
        .nodes
        .get_mut(&destination)
        .unwrap()
        .layout
        .as_mut()
        .unwrap()
        .columns = 1;
    let locked = frame(
        &mut document,
        page,
        "Locked",
        1000.0,
        0.0,
        Some(LayoutAxis::Row),
    );
    document.nodes.get_mut(&locked).unwrap().locked = true;
    let mut editor = editor(document, &[descendant]);
    assert!(LayoutTransferSession::new(
        &editor.document,
        &editor.selection,
        descendant,
        Point::new(320.0, 20.0)
    )
    .is_err());
    editor.selection = vec![a];
    let session = drag(&editor, a);
    assert!(session
        .target_in(instance, Point::new(320.0, 20.0))
        .is_none());
    assert!(session
        .target_in(locked, Point::new(1020.0, 20.0))
        .is_none());
    let original = editor.to_json().unwrap();
    let revision = editor.revision;
    let target = session
        .target_in(destination, Point::new(710.0, 20.0))
        .unwrap();
    assert!(editor.commit_layout_transfer(&session, target).is_err());
    assert_eq!(editor.to_json().unwrap(), original);
    assert_eq!(editor.revision, revision);
    assert_eq!(editor.history_len(), 0);
    editor.selection = vec![instance];
    assert!(LayoutTransferSession::new(
        &editor.document,
        &editor.selection,
        instance,
        Point::new(320.0, 20.0)
    )
    .is_ok());
}

#[test]
fn destination_changes_pending_transactions_selection_and_bad_targets_never_partially_apply() {
    let mut document = Document::new("Atomic transfer");
    let page = document.pages[0].id;
    let source = frame(
        &mut document,
        page,
        "Source",
        0.0,
        0.0,
        Some(LayoutAxis::Row),
    );
    let a = child(&mut document, source, "A");
    let destination = frame(
        &mut document,
        page,
        "Destination",
        400.0,
        0.0,
        Some(LayoutAxis::Row),
    );
    let mut editor = editor(document, &[a]);
    let session = drag(&editor, a);
    let target = session
        .target_in(destination, Point::new(410.0, 20.0))
        .unwrap();
    for change in 0..5 {
        let mut changed = Editor::new(editor.document.clone());
        changed.selection = vec![a];
        let session = drag(&changed, a);
        let target = session.target_in(destination, target.position).unwrap();
        match change {
            0 => {
                let frame = changed.document.nodes.get_mut(&destination).unwrap();
                frame.clip_content = !frame.clip_content;
            }
            1 => {
                changed
                    .document
                    .nodes
                    .get_mut(&destination)
                    .unwrap()
                    .layout
                    .as_mut()
                    .unwrap()
                    .gap += 1.0
            }
            2 => changed.selection.clear(),
            3 => changed.begin_transaction("External pending edit"),
            _ => changed.document.nodes.get_mut(&destination).unwrap().locked = true,
        }
        let original = changed.to_json().unwrap();
        let revision = changed.revision;
        assert!(
            changed.commit_layout_transfer(&session, target).is_err(),
            "change {change}"
        );
        assert_eq!(changed.to_json().unwrap(), original);
        assert_eq!(changed.revision, revision);
        assert_eq!(changed.history_len(), 0);
        assert_eq!(changed.has_active_transaction(), change == 3);
    }
    let original = editor.to_json().unwrap();
    assert!(editor
        .commit_layout_transfer(
            &session,
            LayoutTransferTarget {
                index: usize::MAX,
                ..target
            }
        )
        .is_err());
    assert!(session.target(Point::new(f64::NAN, 0.0)).is_none());
    assert_eq!(editor.to_json().unwrap(), original);
}

#[test]
fn frame_broad_phase_matches_exhaustive_geometry_across_a_thousand_destinations() {
    let mut document = Document::new("Frame query");
    let page = document.pages[0].id;
    let source = frame(
        &mut document,
        page,
        "Source",
        -500.0,
        -500.0,
        Some(LayoutAxis::Row),
    );
    let a = child(&mut document, source, "A");
    for i in 0..1000 {
        let parent = frame(
            &mut document,
            page,
            "Destination",
            (i % 50) as f64 * 300.0,
            (i / 50) as f64 * 200.0,
            Some(LayoutAxis::Row),
        );
        child(&mut document, parent, "Child");
    }
    let editor = editor(document, &[a]);
    let session = drag(&editor, a);
    for i in 0..240 {
        let point = Point::new((i * 7919 % 15000) as f64, (i * 37 % 4000) as f64);
        let expected = session
            .frames
            .iter()
            .filter(|frame| {
                frame.id != page
                    && frame.eligible
                    && frame.bounds.contains(frame.inverse.apply(point))
                    && session.inside_clips(frame.clip, point)
            })
            .max_by_key(|frame| frame.order)
            .map(|frame| frame.id)
            .unwrap_or(page);
        assert_eq!(session.target(point).unwrap().parent, expected);
    }
}
