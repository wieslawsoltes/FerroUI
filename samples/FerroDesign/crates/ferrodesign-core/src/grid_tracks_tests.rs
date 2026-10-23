use super::*;

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-7, "{actual} != {expected}");
}

fn fixture(count: usize) -> (Editor, NodeId, Vec<NodeId>) {
    let mut document = Document::new("Explicit grid tracks");
    let page = document.pages[0].id;
    let mut frame = Node::new(NodeKind::Frame, "Grid", 20.0, 30.0, 340.0, 200.0);
    frame.layout = Some(AutoLayout {
        axis: LayoutAxis::Grid,
        columns: 3,
        padding: Padding::all(10.0),
        gap: 10.0,
        cross_gap: 5.0,
        width: SizingMode::Fixed,
        height: SizingMode::Fixed,
        ..AutoLayout::default()
    });
    let parent = document.insert_node(page, frame).unwrap();
    let children = (0..count)
        .map(|index| {
            document
                .insert_node(
                    parent,
                    Node::new(
                        NodeKind::Rectangle,
                        format!("Child {index}"),
                        0.0,
                        0.0,
                        40.0,
                        24.0,
                    ),
                )
                .unwrap()
        })
        .collect();
    (Editor::new(document), parent, children)
}

#[test]
fn grid_tracks_fixed_and_weighted_axes_arrange_exact_cells_and_undo_redo() {
    let (mut editor, parent, children) = fixture(9);
    for child in &children {
        let settings = &mut editor.document.nodes.get_mut(child).unwrap().layout_child;
        settings.width = SizingMode::Fill;
        settings.height = SizingMode::Fill;
    }
    editor.refresh();
    editor.selection = vec![children[5]];
    let before = editor.document.to_json().unwrap();
    editor
        .set_grid_tracks(
            parent,
            vec![
                GridTrack::Fixed(60.0),
                GridTrack::Fraction(1.0),
                GridTrack::Fraction(2.0),
            ],
            vec![
                GridTrack::Fixed(30.0),
                GridTrack::Fraction(2.0),
                GridTrack::Fraction(1.0),
            ],
        )
        .unwrap();
    for (index, child) in children.iter().enumerate() {
        let node = &editor.document.nodes[child];
        close(node.x, [10.0, 80.0, 170.0][index % 3]);
        close(node.width, [60.0, 80.0, 160.0][index % 3]);
        close(node.y, [10.0, 45.0, 45.0 + 280.0 / 3.0 + 5.0][index / 3]);
        close(node.height, [30.0, 280.0 / 3.0, 140.0 / 3.0][index / 3]);
    }
    assert_eq!(editor.history_len(), 1);
    assert_eq!(editor.selection, vec![children[5]]);
    let after = editor.document.to_json().unwrap();
    assert!(editor.document.layout_all().changed.is_empty());
    editor.undo();
    assert_eq!(editor.document.to_json().unwrap(), before);
    editor.redo();
    assert_eq!(editor.document.to_json().unwrap(), after);
}

#[test]
fn grid_tracks_content_and_fraction_share_space_without_resizing_fixed_children() {
    let (mut editor, parent, children) = fixture(3);
    editor.document.nodes.get_mut(&children[1]).unwrap().width = 70.0;
    editor
        .document
        .nodes
        .get_mut(&children[2])
        .unwrap()
        .layout_child
        .width = SizingMode::Fill;
    editor
        .set_grid_tracks(
            parent,
            vec![
                GridTrack::Fixed(40.0),
                GridTrack::Content,
                GridTrack::Fraction(1.0),
            ],
            vec![GridTrack::Content],
        )
        .unwrap();
    close(editor.document.nodes[&children[0]].x, 10.0);
    close(editor.document.nodes[&children[1]].x, 60.0);
    close(editor.document.nodes[&children[1]].width, 70.0);
    close(editor.document.nodes[&children[2]].x, 140.0);
    close(editor.document.nodes[&children[2]].width, 190.0);
    assert!(editor.document.layout_all().changed.is_empty());
}

#[test]
fn grid_tracks_spans_fixed_tracks_and_child_limits_are_resolved_together() {
    let (mut editor, parent, children) = fixture(2);
    let first = editor.document.nodes.get_mut(&children[0]).unwrap();
    first.width = 200.0;
    first.height = 80.0;
    first.layout_child.grid = Some(GridPlacement {
        row: Some(0),
        column: Some(0),
        column_span: 2,
        row_span: 2,
    });
    let second = editor.document.nodes.get_mut(&children[1]).unwrap();
    second.layout_child.grid = Some(GridPlacement {
        row: Some(0),
        column: Some(2),
        ..GridPlacement::default()
    });
    second.layout_child.width = SizingMode::Fill;
    second.layout_child.max_width = Some(90.0);
    editor
        .set_grid_tracks(
            parent,
            vec![
                GridTrack::Fixed(60.0),
                GridTrack::Content,
                GridTrack::Fraction(1.0),
            ],
            vec![GridTrack::Fixed(30.0), GridTrack::Content],
        )
        .unwrap();
    close(editor.document.nodes[&children[0]].width, 200.0);
    close(editor.document.nodes[&children[1]].x, 220.0);
    close(editor.document.nodes[&children[1]].width, 90.0);
    assert!(editor.document.layout_all().changed.is_empty());
}

#[test]
fn grid_tracks_hug_overlapping_fill_spans_converge_and_do_not_accumulate_size() {
    let (mut editor, parent, children) = fixture(3);
    let layout = editor
        .document
        .nodes
        .get_mut(&parent)
        .unwrap()
        .layout
        .as_mut()
        .unwrap();
    layout.width = SizingMode::Hug;
    layout.height = SizingMode::Hug;
    for (index, child) in children.iter().enumerate() {
        let node = editor.document.nodes.get_mut(child).unwrap();
        node.width = [100.0, 130.0, 180.0][index];
        node.layout_child.width = SizingMode::Fill;
        node.layout_child.grid = Some(GridPlacement {
            row: Some(index),
            column: Some(if index == 2 { 1 } else { 0 }),
            column_span: if index == 0 { 1 } else { 2 },
            ..GridPlacement::default()
        });
    }
    editor
        .set_grid_tracks(parent, vec![GridTrack::Content; 3], Vec::new())
        .unwrap();
    let width = editor.document.nodes[&parent].width;
    close(width, 340.0);
    for _ in 0..20 {
        assert!(editor.document.layout_all().changed.is_empty());
        close(editor.document.nodes[&parent].width, width);
    }
}

#[test]
fn grid_tracks_empty_authored_frames_and_implicit_rows_keep_their_track_extents() {
    let (mut editor, parent, _) = fixture(0);
    let layout = editor
        .document
        .nodes
        .get_mut(&parent)
        .unwrap()
        .layout
        .as_mut()
        .unwrap();
    layout.width = SizingMode::Hug;
    layout.height = SizingMode::Hug;
    editor
        .set_grid_tracks(
            parent,
            vec![
                GridTrack::Fixed(80.0),
                GridTrack::Content,
                GridTrack::Fraction(2.0),
            ],
            vec![GridTrack::Fixed(30.0), GridTrack::Fixed(50.0)],
        )
        .unwrap();
    close(editor.document.nodes[&parent].width, 120.0);
    close(editor.document.nodes[&parent].height, 105.0);
    let page = editor.active_page;
    let child = editor
        .document
        .insert_node(
            page,
            Node::new(NodeKind::Rectangle, "Incoming", 500.0, 40.0, 40.0, 24.0),
        )
        .unwrap();
    editor.refresh();
    editor.selection = vec![child];
    let start = editor.document.world_bounds(child).unwrap().center();
    let session =
        LayoutTransferSession::new(&editor.document, &editor.selection, child, start).unwrap();
    let target = session.target_in(parent, Point::new(40.0, 50.0)).unwrap();
    editor.commit_layout_transfer(&session, target).unwrap();
    assert_eq!(editor.document.nodes[&child].parent, Some(parent));
    close(editor.document.nodes[&child].x, 10.0);
    close(editor.document.nodes[&parent].height, 105.0);
}

#[test]
fn grid_tracks_reverse_hidden_and_absolute_children_retain_their_geometry_and_metadata() {
    let (mut editor, parent, children) = fixture(5);
    editor
        .document
        .nodes
        .get_mut(&parent)
        .unwrap()
        .layout
        .as_mut()
        .unwrap()
        .reverse = true;
    editor.document.nodes.get_mut(&children[1]).unwrap().visible = false;
    editor
        .document
        .nodes
        .get_mut(&children[3])
        .unwrap()
        .layout_child
        .absolute = true;
    editor.refresh();
    let hidden = editor.document.nodes[&children[1]].clone();
    let absolute = editor.document.nodes[&children[3]].clone();
    editor
        .set_grid_tracks(
            parent,
            vec![
                GridTrack::Fixed(40.0),
                GridTrack::Fixed(70.0),
                GridTrack::Fixed(100.0),
            ],
            Vec::new(),
        )
        .unwrap();
    close(editor.document.nodes[&children[4]].x, 10.0);
    close(editor.document.nodes[&children[2]].x, 60.0);
    close(editor.document.nodes[&children[0]].x, 140.0);
    assert_eq!(editor.document.nodes[&children[1]], hidden);
    assert_eq!(editor.document.nodes[&children[3]], absolute);
}

#[test]
fn grid_tracks_invalid_counts_spans_weights_and_overflow_are_atomic() {
    let (mut editor, parent, children) = fixture(1);
    editor
        .document
        .nodes
        .get_mut(&children[0])
        .unwrap()
        .layout_child
        .grid = Some(GridPlacement {
        row: Some(0),
        column: Some(2),
        ..GridPlacement::default()
    });
    let before = editor.document.to_json().unwrap();
    let revision = editor.revision;
    for columns in [
        vec![GridTrack::Fraction(1.0); 2],
        vec![GridTrack::Fixed(-1.0); 3],
        vec![GridTrack::Fraction(0.0); 3],
        vec![GridTrack::Fraction(f64::NAN); 3],
        vec![GridTrack::Fixed(f64::MAX); 3],
        vec![GridTrack::Content; MAX_GRID_COLUMNS + 1],
    ] {
        assert!(editor.set_grid_tracks(parent, columns, Vec::new()).is_err());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert_eq!(editor.revision, revision);
        assert!(!editor.can_undo());
    }
    editor.begin_transaction("Other draft");
    assert!(editor
        .set_grid_tracks(parent, vec![GridTrack::Content; 3], Vec::new())
        .is_err());
    assert!(editor.has_active_transaction());
}

#[test]
fn grid_tracks_noop_and_reset_keep_history_redo_and_legacy_serialization() {
    let (mut editor, parent, _) = fixture(3);
    let original = editor.document.to_json().unwrap();
    assert!(!original.contains("column_tracks") && !original.contains("row_tracks"));
    let columns = vec![GridTrack::Fixed(40.0); 3];
    editor
        .set_grid_tracks(parent, columns.clone(), Vec::new())
        .unwrap();
    let after = editor.document.to_json().unwrap();
    let revision = editor.revision;
    editor.set_grid_tracks(parent, columns, Vec::new()).unwrap();
    assert_eq!(editor.revision, revision);
    assert_eq!(editor.history_len(), 1);
    editor.undo();
    let revision = editor.revision;
    editor
        .set_grid_tracks(parent, Vec::new(), Vec::new())
        .unwrap();
    assert_eq!(editor.revision, revision);
    assert!(editor.can_redo());
    assert_eq!(editor.document.to_json().unwrap(), original);
    editor.redo();
    assert_eq!(editor.document.to_json().unwrap(), after);
    editor
        .set_grid_tracks(parent, Vec::new(), Vec::new())
        .unwrap();
    assert!(!editor.document.to_json().unwrap().contains("column_tracks"));
}

#[test]
fn grid_tracks_roundtrip_import_validates_authored_tracks() {
    let (mut editor, parent, _) = fixture(3);
    editor
        .set_grid_tracks(
            parent,
            vec![
                GridTrack::Fixed(60.0),
                GridTrack::Content,
                GridTrack::Fraction(2.0),
            ],
            vec![GridTrack::Fixed(30.0), GridTrack::Fraction(1.0)],
        )
        .unwrap();
    let json = editor.document.to_json().unwrap();
    let parsed = Document::from_json(&json).unwrap();
    assert_eq!(
        parsed.nodes[&parent].layout,
        editor.document.nodes[&parent].layout
    );
    let invalid = json.replace("\"value\": 60.0", "\"value\": -60.0");
    assert_ne!(invalid, json);
    assert!(Document::from_json(&invalid).is_err());
}

#[test]
fn grid_tracks_native_syntax_roundtrips_and_rejects_ambiguous_or_invalid_items() {
    let tracks = parse_grid_tracks(" 120px, Auto, 2fr, 0, 0.25fr ", 5).unwrap();
    assert_eq!(
        tracks,
        vec![
            GridTrack::Fixed(120.0),
            GridTrack::Content,
            GridTrack::Fraction(2.0),
            GridTrack::Fixed(0.0),
            GridTrack::Fraction(0.25)
        ]
    );
    assert_eq!(
        parse_grid_tracks(&format_grid_tracks(&tracks), 5).unwrap(),
        tracks
    );
    assert!(parse_grid_tracks("", 0).unwrap().is_empty());
    for value in [
        "1,",
        ",1",
        "1,,2",
        "-1",
        "0fr",
        "-2fr",
        "NaN",
        "inf",
        "NaNfr",
        "fr",
        "auto auto",
        "1fr,2fr,3fr",
    ] {
        assert!(parse_grid_tracks(value, 2).is_err(), "{value}");
    }
}
