use super::*;

fn fixture() -> (Editor, NodeId, NodeId, NodeId) {
    let mut document = core::Document::new("Affine authoring");
    let page = document.pages[0].id;
    let frame = document
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "Parent", 30.0, 40.0, 300.0, 200.0),
        )
        .unwrap();
    document
        .nodes
        .get_mut(&frame)
        .unwrap()
        .set_local_transform(Transform {
            a: 0.9,
            b: 0.2,
            c: 0.1,
            d: 1.1,
            e: 30.0,
            f: 40.0,
        });
    let group = document
        .insert_node(
            frame,
            Node::new(NodeKind::Group, "Group", 20.0, 15.0, 100.0, 60.0),
        )
        .unwrap();
    let child = document
        .insert_node(
            group,
            Node::new(NodeKind::Rectangle, "Child", 7.0, 9.0, 23.0, 17.0),
        )
        .unwrap();
    let mut editor = Editor::new(document);
    editor.select(group, false);
    (editor, frame, group, child)
}

#[test]
fn affine_authoring_preserves_child_geometry_parent_coordinates_and_complete_undo() {
    let (mut editor, frame, group, child) = fixture();
    let before = editor.document.to_json().unwrap();
    let child_before = serde_json::to_value(&editor.document.nodes[&child]).unwrap();
    let local_child = editor.document.nodes[&child].local_transform();
    let authored = Transform {
        a: 1.2,
        b: -0.15,
        c: 0.4,
        d: 0.85,
        e: 71.0,
        f: 33.0,
    };
    assert!(apply_affine(&mut editor, group, authored).unwrap());
    assert_eq!(editor.history_len(), 1);
    assert_eq!(
        serde_json::to_value(&editor.document.nodes[&child]).unwrap(),
        child_before
    );
    let actual = editor.document.world_transform(child).unwrap();
    let expected = editor.document.world_transform(frame).unwrap() * authored * local_child;
    for point in [
        Point::new(0.0, 0.0),
        Point::new(23.0, 17.0),
        Point::new(11.0, 8.0),
    ] {
        assert!(actual.apply(point).distance(expected.apply(point)) < 1e-8);
    }
    let after = editor.document.to_json().unwrap();
    assert!(!apply_affine(&mut editor, group, authored).unwrap());
    assert_eq!(editor.history_len(), 1);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
}

#[test]
fn affine_authoring_invalid_locked_stale_and_external_drafts_never_mutate_or_take_history() {
    let (mut editor, frame, group, _) = fixture();
    let before = editor.document.to_json().unwrap();
    for transform in [
        Transform {
            a: 0.0,
            d: 0.0,
            ..Transform::IDENTITY
        },
        Transform {
            c: f64::NAN,
            ..Transform::IDENTITY
        },
        Transform {
            e: 1e8,
            ..Transform::IDENTITY
        },
    ] {
        assert!(apply_affine(&mut editor, group, transform).is_err());
    }
    assert_eq!(editor.document.to_json().unwrap(), before);
    editor.selection.clear();
    assert!(apply_affine(&mut editor, group, Transform::IDENTITY).is_err());
    editor.select(group, false);
    editor.document.nodes.get_mut(&frame).unwrap().locked = true;
    assert!(apply_affine(&mut editor, group, Transform::IDENTITY).is_err());
    editor.document.nodes.get_mut(&frame).unwrap().locked = false;
    editor.begin_transaction("External field");
    let token = editor.active_transaction_token();
    assert!(apply_affine(&mut editor, group, Transform::IDENTITY).is_err());
    assert_eq!(editor.active_transaction_token(), token);
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.history_len(), 0);
}

#[test]
fn constraint_diagram_edges_drive_actual_resize_without_losing_child_content() {
    let mut document = core::Document::new("Graphical constraints");
    let page = document.pages[0].id;
    let frame = document
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "Frame", 0.0, 0.0, 200.0, 100.0),
        )
        .unwrap();
    let child = document
        .insert_node(
            frame,
            Node::new(NodeKind::Rectangle, "Pinned", 20.0, 10.0, 40.0, 30.0)
                .with_fill(Color::WHITE),
        )
        .unwrap();
    let mut editor = Editor::new(document);
    editor.update_nodes_with_layout(&[child], "Pin edges", |node| {
        node.constraints.horizontal = toggle_edge(Constraint::Start, true);
        node.constraints.vertical = Constraint::Center;
    });
    let before_resize = editor.document.to_json().unwrap();
    editor.resize_node(frame, 300.0, 160.0);
    let node = &editor.document.nodes[&child];
    assert_eq!(
        (node.x, node.y, node.width, node.height),
        (20.0, 40.0, 140.0, 30.0)
    );
    assert_eq!(node.fill_color(), Color::WHITE);
    assert_eq!(node.constraints.horizontal, Constraint::Stretch);
    assert_eq!(editor.history_len(), 2);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before_resize);
}
