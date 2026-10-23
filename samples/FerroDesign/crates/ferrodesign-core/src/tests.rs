use crate::*;
use std::collections::BTreeMap;

fn rectangle(d: &mut Document, parent: NodeId, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.insert_node(
        parent,
        Node::new(NodeKind::Rectangle, "Rectangle", x, y, w, h),
    )
    .unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "expected {a} ~= {b}")
}
fn point_close(a: Point, b: Point) {
    close(a.x, b.x);
    close(a.y, b.y)
}
fn make_document() -> Document {
    Document::new("Tests")
}

#[test]
fn affine_inverse_and_composition_survive_reflection() {
    let t = Transform::translation(23.0, -17.0)
        * Transform::rotation(37.0)
        * Transform::scale(-2.0, 0.75);
    let p = Point::new(-13.5, 44.2);
    point_close(t.inverse().unwrap().apply(t.apply(p)), p);
    assert!(Transform::scale(0.0, 1.0).inverse().is_none());
    let rect = t.transform_rect(Rect::new(0.0, 0.0, 40.0, 20.0));
    for point in Rect::new(0.0, 0.0, 40.0, 20.0).corners() {
        assert!(rect.inflate(1e-8).contains(t.apply(point)))
    }
}

#[test]
fn viewport_zoom_keeps_cursor_world_position() {
    let mut v = Viewport::default();
    let screen = Point::new(453.0, 272.0);
    let before = v.screen_to_world(screen);
    v.zoom_at(screen, 3.7);
    point_close(before, v.screen_to_world(screen));
    v.fit(Rect::new(-300.0, 100.0, 800.0, 600.0), 20.0);
    assert!(v
        .visible_rect()
        .contains_rect(Rect::new(-300.0, 100.0, 800.0, 600.0)));
}

#[test]
fn grouping_rotated_children_preserves_global_geometry_and_z_order() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let frame = d
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "Rotated parent", 150.0, 80.0, 600.0, 500.0),
        )
        .unwrap();
    d.nodes.get_mut(&frame).unwrap().rotation = 29.0;
    let a = rectangle(&mut d, frame, 40.0, 70.0, 80.0, 40.0);
    let b = rectangle(&mut d, frame, 240.0, 110.0, 110.0, 75.0);
    let c = rectangle(&mut d, frame, 400.0, 200.0, 20.0, 20.0);
    d.nodes.get_mut(&a).unwrap().rotation = -17.0;
    d.nodes.get_mut(&b).unwrap().flip_x = true;
    let before = [d.world_transform(a).unwrap(), d.world_transform(b).unwrap()];
    let mut e = Editor::new(d);
    e.selection = vec![a, b];
    let group = e.group_selection().unwrap();
    assert_eq!(e.document.nodes[&frame].children, vec![group, c]);
    for (id, t) in [a, b].into_iter().zip(before) {
        for p in [Point::default(), Point::new(10.0, 30.0)] {
            point_close(e.document.world_transform(id).unwrap().apply(p), t.apply(p))
        }
    }
    e.ungroup_selection();
    assert_eq!(e.document.nodes[&frame].children, vec![a, b, c]);
    e.document.validate().unwrap();
    for (id, t) in [a, b].into_iter().zip(before) {
        point_close(
            e.document
                .world_transform(id)
                .unwrap()
                .apply(Point::new(12.0, 22.0)),
            t.apply(Point::new(12.0, 22.0)),
        )
    }
}

#[test]
fn move_world_delta_under_rotated_parent_is_exact() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let parent = d
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "Parent", 100.0, 100.0, 400.0, 300.0),
        )
        .unwrap();
    d.nodes.get_mut(&parent).unwrap().rotation = 90.0;
    let child = rectangle(&mut d, parent, 10.0, 20.0, 50.0, 60.0);
    let mut e = Editor::new(d);
    e.selection = vec![child];
    let before = e.document.world_origin(child).unwrap();
    e.move_selection(32.0, -12.0);
    point_close(
        e.document.world_origin(child).unwrap(),
        before + Point::new(32.0, -12.0),
    );
}

#[test]
fn gesture_has_one_undo_step_and_cancel_restores_baseline() {
    let mut e = Editor::default();
    let id = e.add_node(NodeKind::Rectangle, 10.0, 20.0, 100.0, 100.0);
    e.clear_history();
    e.begin_transaction("Drag");
    for _ in 0..25 {
        e.document.nodes.get_mut(&id).unwrap().x += 2.0;
        e.refresh();
    }
    assert!(e.commit_transaction());
    assert_eq!(e.history_len(), 1);
    close(e.document.nodes[&id].x, 60.0);
    assert!(e.undo());
    close(e.document.nodes[&id].x, 10.0);
    assert!(e.redo());
    close(e.document.nodes[&id].x, 60.0);
    e.begin_transaction("Cancelled drag");
    e.document.nodes.get_mut(&id).unwrap().x = 800.0;
    e.refresh();
    e.cancel_transaction();
    close(e.document.nodes[&id].x, 60.0);
    assert_eq!(e.history_len(), 1);
    e.begin_transaction("No operation");
    e.refresh();
    assert!(!e.commit_transaction());
    assert_eq!(e.history_len(), 1);
}

#[test]
fn selection_parent_and_child_move_only_once() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let parent = d
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "Frame", 20.0, 30.0, 200.0, 200.0),
        )
        .unwrap();
    let child = rectangle(&mut d, parent, 10.0, 10.0, 30.0, 30.0);
    let mut e = Editor::new(d);
    e.selection = vec![parent, child];
    e.move_selection(5.0, 7.0);
    close(e.document.nodes[&parent].x, 25.0);
    close(e.document.nodes[&child].x, 10.0);
}

#[test]
fn history_restores_deleted_subtree_and_prototype_references() {
    let mut e = Editor::default();
    let a = e.add_node(NodeKind::Frame, 0.0, 0.0, 100.0, 100.0);
    let b = e.add_node(NodeKind::Frame, 200.0, 0.0, 100.0, 100.0);
    let child = e
        .insert_node(
            b,
            Node::new(NodeKind::Text, "Target child", 0.0, 0.0, 40.0, 20.0),
        )
        .unwrap();
    e.add_interaction(a, b, Transition::default());
    e.document.pages[0].prototype_start = Some(b);
    e.selection = vec![b];
    e.delete_selection();
    assert!(!e.document.nodes.contains_key(&child));
    assert!(e.document.nodes[&a].interactions.is_empty());
    assert_eq!(e.document.pages[0].prototype_start, None);
    assert!(e.undo());
    assert!(e.document.nodes.contains_key(&child));
    assert_eq!(e.document.nodes[&a].interactions.len(), 1);
    assert_eq!(e.document.pages[0].prototype_start, Some(b));
    e.document.validate().unwrap();
}

#[test]
fn constrained_resize_honors_end_center_stretch_scale() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let parent = d
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "Frame", 0.0, 0.0, 200.0, 100.0),
        )
        .unwrap();
    let end = rectangle(&mut d, parent, 140.0, 10.0, 40.0, 20.0);
    let center = rectangle(&mut d, parent, 80.0, 40.0, 40.0, 20.0);
    let stretch = rectangle(&mut d, parent, 20.0, 70.0, 160.0, 20.0);
    let scale = rectangle(&mut d, parent, 20.0, 10.0, 40.0, 30.0);
    d.nodes.get_mut(&end).unwrap().constraints = Constraints {
        horizontal: Constraint::End,
        vertical: Constraint::End,
    };
    d.nodes.get_mut(&center).unwrap().constraints = Constraints {
        horizontal: Constraint::Center,
        vertical: Constraint::Center,
    };
    d.nodes.get_mut(&stretch).unwrap().constraints = Constraints {
        horizontal: Constraint::Stretch,
        vertical: Constraint::Start,
    };
    d.nodes.get_mut(&scale).unwrap().constraints = Constraints {
        horizontal: Constraint::Scale,
        vertical: Constraint::Scale,
    };
    d.apply_constraints(parent, 200.0, 100.0, 300.0, 200.0);
    close(d.nodes[&end].x, 240.0);
    close(d.nodes[&end].y, 110.0);
    close(d.nodes[&center].x, 130.0);
    close(d.nodes[&center].y, 90.0);
    close(d.nodes[&stretch].width, 260.0);
    close(d.nodes[&scale].x, 30.0);
    close(d.nodes[&scale].width, 60.0);
    close(d.nodes[&scale].height, 60.0);
}

#[test]
fn auto_layout_flexible_children_redistribute_after_max_clamp() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let parent = d
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "Row", 0.0, 0.0, 400.0, 100.0),
        )
        .unwrap();
    d.nodes.get_mut(&parent).unwrap().layout = Some(AutoLayout {
        gap: 10.0,
        padding: Padding::all(20.0),
        height: SizingMode::Fixed,
        cross_align: LayoutAlign::Center,
        ..AutoLayout::default()
    });
    let fixed = rectangle(&mut d, parent, 0.0, 0.0, 60.0, 30.0);
    let a = rectangle(&mut d, parent, 0.0, 0.0, 20.0, 20.0);
    let b = rectangle(&mut d, parent, 0.0, 0.0, 20.0, 40.0);
    for id in [a, b] {
        d.nodes.get_mut(&id).unwrap().layout_child.width = SizingMode::Fill;
    }
    d.nodes.get_mut(&a).unwrap().layout_child.max_width = Some(80.0);
    d.layout_all();
    close(d.nodes[&fixed].x, 20.0);
    close(d.nodes[&a].x, 90.0);
    close(d.nodes[&a].width, 80.0);
    close(d.nodes[&b].x, 180.0);
    close(d.nodes[&b].width, 200.0);
    close(d.nodes[&b].y, 30.0);
}

#[test]
fn wrapped_layout_hugs_multiple_lines_and_ignores_absolute_children() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let parent = d
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "Wrap", 0.0, 0.0, 220.0, 100.0),
        )
        .unwrap();
    d.nodes.get_mut(&parent).unwrap().layout = Some(AutoLayout {
        gap: 10.0,
        cross_gap: 15.0,
        padding: Padding::all(10.0),
        wrap: true,
        ..AutoLayout::default()
    });
    let a = rectangle(&mut d, parent, 0.0, 0.0, 90.0, 30.0);
    let b = rectangle(&mut d, parent, 0.0, 0.0, 90.0, 50.0);
    let c = rectangle(&mut d, parent, 0.0, 0.0, 90.0, 40.0);
    let absolute = rectangle(&mut d, parent, 800.0, 900.0, 1000.0, 1000.0);
    d.nodes.get_mut(&absolute).unwrap().layout_child.absolute = true;
    d.layout_all();
    close(d.nodes[&a].y, 10.0);
    close(d.nodes[&b].x, 110.0);
    close(d.nodes[&c].y, 75.0);
    close(d.nodes[&parent].height, 125.0);
    close(d.nodes[&absolute].x, 800.0);
}

#[test]
fn grid_layout_sizes_last_partial_row() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let parent = d
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "Grid", 0.0, 0.0, 330.0, 10.0),
        )
        .unwrap();
    d.nodes.get_mut(&parent).unwrap().layout = Some(AutoLayout {
        axis: LayoutAxis::Grid,
        columns: 3,
        gap: 15.0,
        cross_gap: 12.0,
        padding: Padding::all(10.0),
        ..AutoLayout::default()
    });
    let mut ids = Vec::new();
    for _ in 0..5 {
        let id = rectangle(&mut d, parent, 0.0, 0.0, 50.0, 40.0);
        d.nodes.get_mut(&id).unwrap().layout_child.width = SizingMode::Fill;
        ids.push(id)
    }
    d.layout_all();
    close(d.nodes[&ids[0]].width, 280.0 / 3.0);
    close(d.nodes[&ids[3]].x, 10.0);
    close(d.nodes[&ids[3]].y, 62.0);
    close(d.nodes[&parent].height, 112.0);
}

#[test]
fn duplicate_remaps_internal_links_and_preserves_external_links() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let group = d
        .insert_node(
            page,
            Node::new(NodeKind::Group, "Flow", 10.0, 20.0, 200.0, 100.0),
        )
        .unwrap();
    let a = rectangle(&mut d, group, 0.0, 0.0, 50.0, 50.0);
    let b = rectangle(&mut d, group, 70.0, 0.0, 50.0, 50.0);
    let external = rectangle(&mut d, page, 500.0, 0.0, 50.0, 50.0);
    let mut e = Editor::new(d);
    e.add_interaction(a, b, Transition::default());
    e.add_interaction(b, external, Transition::default());
    e.selection = vec![group];
    let duplicate = e.duplicate_selection()[0];
    let children = e.document.nodes[&duplicate].children.clone();
    assert_ne!(children[0], a);
    assert_ne!(children[1], b);
    assert_eq!(
        e.document.nodes[&children[0]].interactions[0].action,
        PrototypeAction::NavigateTo {
            target: children[1]
        }
    );
    assert_eq!(
        e.document.nodes[&children[1]].interactions[0].action,
        PrototypeAction::NavigateTo { target: external }
    );
    e.document.validate().unwrap();
}

#[test]
fn component_sync_applies_updates_while_retaining_text_override() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let master = d
        .insert_node(
            page,
            Node::new(NodeKind::Component, "Button", 0.0, 0.0, 180.0, 50.0),
        )
        .unwrap();
    let label = d
        .insert_node(
            master,
            Node::new(NodeKind::Text, "Label", 10.0, 10.0, 160.0, 30.0).with_text(
                "Continue",
                14.0,
                Color::BLACK,
            ),
        )
        .unwrap();
    let mut e = Editor::new(d);
    let instance = e.create_instance(master, 300.0, 70.0).unwrap();
    let instance_label = e.document.nodes[&instance].children[0];
    e.set_text(instance_label, "Pay now");
    e.transact("Change master", |d| {
        d.nodes.get_mut(&label).unwrap().font_size = 18.0;
        d.nodes.get_mut(&label).unwrap().fills = vec![Fill::solid(Color::rgb(200, 20, 100))];
        d.nodes.get_mut(&master).unwrap().width = 220.0;
    });
    let new_child = e
        .insert_node(
            master,
            Node::new(NodeKind::Ellipse, "Dot", 185.0, 20.0, 8.0, 8.0),
        )
        .unwrap();
    assert_eq!(e.sync_instances(master), 1);
    assert_eq!(e.document.nodes[&instance_label].text, "Pay now");
    close(e.document.nodes[&instance_label].font_size, 18.0);
    close(e.document.nodes[&instance].x, 300.0);
    close(e.document.nodes[&instance].width, 220.0);
    assert!(e.document.nodes[&instance]
        .children
        .iter()
        .any(|id| e.document.nodes[id].component_source == Some(new_child)));
    e.transact("Remove master child", |d| {
        d.remove_subtree(new_child);
    });
    e.sync_instances(master);
    assert_eq!(e.document.nodes[&instance].children.len(), 1);
    assert!(e.detach_instance(instance));
    assert_eq!(e.document.nodes[&instance].kind, NodeKind::Frame);
    assert_eq!(e.document.nodes[&instance_label].component_source, None);
    e.document.validate().unwrap();
}

#[test]
fn style_variables_alias_modes_and_cycles_are_deterministic() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let id = rectangle(&mut d, page, 0.0, 0.0, 10.0, 10.0);
    let a = d.allocate_id();
    let b = d.allocate_id();
    let c = d.allocate_id();
    d.variables.insert(
        a,
        Variable {
            id: a,
            name: "Base".into(),
            collection: "Theme".into(),
            values: BTreeMap::from([
                ("Default".into(), VariableValue::Color(Color::BLACK)),
                ("Dark".into(), VariableValue::Color(Color::WHITE)),
            ]),
        },
    );
    d.variables.insert(
        b,
        Variable {
            id: b,
            name: "Alias".into(),
            collection: "Theme".into(),
            values: BTreeMap::from([("Default".into(), VariableValue::Alias(a))]),
        },
    );
    d.nodes
        .get_mut(&id)
        .unwrap()
        .variable_bindings
        .insert("fill".into(), b);
    d.variable_mode = "Dark".into();
    d.apply_variables();
    assert_eq!(d.nodes[&id].fill_color(), Color::WHITE);
    d.variables.insert(
        c,
        Variable {
            id: c,
            name: "Cycle".into(),
            collection: "Theme".into(),
            values: BTreeMap::from([("Default".into(), VariableValue::Alias(c))]),
        },
    );
    assert!(d.resolve_variable(c, "Default").is_none());
}

#[test]
fn hierarchy_validation_rejects_cycles_unreachable_nodes_and_bad_utf8_ranges() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let a = d
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "A", 0.0, 0.0, 100.0, 100.0),
        )
        .unwrap();
    let b = d
        .insert_node(a, Node::new(NodeKind::Frame, "B", 0.0, 0.0, 50.0, 50.0))
        .unwrap();
    assert!(d.reparent(a, b, 0).is_err());
    let mut corrupt = d.clone();
    corrupt.nodes.get_mut(&b).unwrap().children.push(a);
    assert!(corrupt.validate().is_err());
    let mut corrupt = d.clone();
    corrupt.pages[0].children.clear();
    assert!(corrupt.validate().is_err());
    let mut corrupt = d.clone();
    corrupt.schema_version = 999;
    assert!(matches!(
        Document::from_json(&corrupt.to_json().unwrap()),
        Err(DocumentError::UnsupportedVersion(999))
    ));
    let text = d
        .insert_node(
            a,
            Node::new(NodeKind::Text, "Unicode", 0.0, 0.0, 80.0, 20.0).with_text(
                "Żółć",
                14.0,
                Color::BLACK,
            ),
        )
        .unwrap();
    d.nodes.get_mut(&text).unwrap().text_runs.push(TextRun {
        start: 1,
        end: 4,
        font_family: None,
        font_size: None,
        font_weight: None,
        color: None,
        italic: None,
        decoration: None,
    });
    assert!(d.validate().is_err());
}

#[test]
fn id_allocator_after_import_cannot_overwrite_existing_nodes() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let a = rectangle(&mut d, page, 0.0, 0.0, 50.0, 50.0);
    d.next_id = 1;
    let mut imported = Document::from_json(&d.to_json().unwrap()).unwrap();
    let b = rectangle(&mut imported, page, 0.0, 0.0, 50.0, 50.0);
    assert_ne!(a, b);
    assert!(b > a);
    assert_eq!(imported.nodes.len(), 2);
}

#[test]
fn clip_and_shape_hit_testing_excludes_hidden_locked_and_ellipse_corners() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let frame = d
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "Clip", 100.0, 100.0, 100.0, 100.0),
        )
        .unwrap();
    let child = rectangle(&mut d, frame, 80.0, 20.0, 60.0, 40.0);
    let circle = d
        .insert_node(
            page,
            Node::new(NodeKind::Ellipse, "Circle", 300.0, 100.0, 100.0, 100.0),
        )
        .unwrap();
    let mut e = Editor::new(d);
    assert_eq!(e.hit_test(Point::new(190.0, 130.0), 0.0), Some(child));
    assert_eq!(e.hit_test(Point::new(225.0, 130.0), 0.0), None);
    assert_eq!(e.hit_test(Point::new(301.0, 101.0), 0.0), None);
    assert_eq!(e.hit_test(Point::new(350.0, 150.0), 0.0), Some(circle));
    e.toggle_locked(frame);
    assert_eq!(e.hit_test(Point::new(190.0, 130.0), 0.0), None);
    e.toggle_visibility(circle);
    assert_eq!(e.hit_test(Point::new(350.0, 150.0), 0.0), None);
}

#[test]
fn spatial_index_ten_thousand_nodes_and_huge_frames_remain_queryable() {
    let mut index = SpatialIndex::new(128.0);
    for i in 0..10_000u64 {
        let x = (i % 100) as f64 * 32.0;
        let y = (i / 100) as f64 * 32.0;
        index.insert(i + 1, Rect::new(x, y, 20.0, 20.0));
    }
    let query = index.query(Rect::new(320.0, 640.0, 21.0, 21.0));
    assert_eq!(query, vec![2011]);
    index.insert(10001, Rect::new(-1e15, -1e15, 2e15, 2e15));
    let query = index.query(Rect::new(320.0, 640.0, 21.0, 21.0));
    assert_eq!(query, vec![2011, 10001]);
    index.remove(2011);
    assert_eq!(
        index.query(Rect::new(320.0, 640.0, 21.0, 21.0)),
        vec![10001]
    );
    assert_eq!(
        index.query(Rect::new(-1e15, -1e15, 2e15, 2e15)).len(),
        10000
    );
}

#[test]
fn bezier_flattening_retains_endpoints_and_evenodd_holes() {
    let mut out = vec![Point::new(0.0, 0.0)];
    flatten_cubic(
        Point::new(0.0, 0.0),
        Point::new(0.0, 100.0),
        Point::new(100.0, 100.0),
        Point::new(100.0, 0.0),
        0.25,
        &mut out,
    );
    assert!(out.len() > 10);
    assert_eq!(out.first(), Some(&Point::new(0.0, 0.0)));
    assert_eq!(out.last(), Some(&Point::new(100.0, 0.0)));
    let mut path = VectorPath::polygon(&Rect::new(0.0, 0.0, 100.0, 100.0).corners(), true);
    path.segments
        .extend(VectorPath::polygon(&Rect::new(25.0, 25.0, 50.0, 50.0).corners(), true).segments);
    path.fill_rule = FillRule::EvenOdd;
    assert!(path.contains(Point::new(10.0, 10.0)));
    assert!(!path.contains(Point::new(50.0, 50.0)));
}

#[test]
fn boolean_operations_evaluate_rotated_operand_geometry() {
    let mut e = Editor::default();
    let a = e.add_node(NodeKind::Rectangle, 0.0, 0.0, 100.0, 100.0);
    let b = e.add_node(NodeKind::Ellipse, 50.0, 0.0, 100.0, 100.0);
    e.selection = vec![a, b];
    let id = e.boolean_selection(BooleanOperation::Subtract).unwrap();
    assert!(e.document.hit_test_node(id, Point::new(20.0, 50.0), 0.0));
    assert!(!e.document.hit_test_node(id, Point::new(75.0, 50.0), 0.0));
    e.document.nodes.get_mut(&id).unwrap().boolean_operation = BooleanOperation::Intersect;
    assert!(!e.document.hit_test_node(id, Point::new(20.0, 50.0), 0.0));
    assert!(e.document.hit_test_node(id, Point::new(75.0, 50.0), 0.0));
    e.document.validate().unwrap();
}

#[test]
fn prototype_navigation_has_back_history_and_overlay_stack() {
    let mut e = Editor::default();
    let a = e.add_node(NodeKind::Frame, 0.0, 0.0, 100.0, 100.0);
    let b = e.add_node(NodeKind::Frame, 200.0, 0.0, 100.0, 100.0);
    e.document.pages[0].prototype_start = Some(a);
    e.add_interaction(a, b, Transition::default());
    e.document
        .nodes
        .get_mut(&b)
        .unwrap()
        .interactions
        .push(Interaction {
            trigger: Trigger::Click,
            action: PrototypeAction::Back,
            transition: Transition::default(),
            delay_ms: 0,
            key: None,
        });
    let mut player = PrototypePlayer::default();
    assert!(player.start(&e.document, e.active_page));
    assert_eq!(player.current, Some(a));
    assert!(player.trigger(&mut e.document, a, Trigger::Click));
    assert_eq!(player.current, Some(b));
    assert_eq!(player.history, vec![a]);
    assert!(player.trigger(&mut e.document, b, Trigger::Click));
    assert_eq!(player.current, Some(a));
    assert!(player.history.is_empty());
}

#[test]
fn svg_export_preserves_gradients_paths_unicode_escaping_and_world_position() {
    let mut d = make_document();
    let page = d.pages[0].id;
    let frame = d
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "A < B & C", 100.0, 200.0, 300.0, 200.0),
        )
        .unwrap();
    let text = d
        .insert_node(
            frame,
            Node::new(NodeKind::Text, "Text", 20.0, 30.0, 200.0, 60.0).with_text(
                "Zażółć <>&\nSecond line",
                16.0,
                Color::BLACK,
            ),
        )
        .unwrap();
    d.nodes.get_mut(&text).unwrap().text_runs = vec![TextRun {
        start: 0,
        end: "Zażółć".len(),
        font_family: None,
        font_size: Some(20.0),
        font_weight: Some(700),
        color: Some(Color::rgb(90, 20, 200)),
        italic: Some(true),
        decoration: None,
    }];
    d.nodes.get_mut(&frame).unwrap().fills = vec![Fill {
        paint: Paint::LinearGradient {
            start: Point::new(0.0, 0.0),
            end: Point::new(1.0, 1.0),
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: Color::BLACK,
                },
                GradientStop {
                    offset: 1.0,
                    color: Color::WHITE,
                },
            ],
        },
        opacity: 1.0,
        visible: true,
    }];
    let svg = d.export_svg(page, &[]);
    assert!(svg.contains("<linearGradient"));
    assert!(svg.contains("Zażółć"));
    assert!(svg.contains("&lt;&gt;&amp;"));
    assert!(svg.contains("A &lt; B &amp; C"));
    assert!(svg.contains("font-weight=\"700\""));
    assert!(!svg.contains("<script"));
    let svg = d.export_svg(page, &[text]);
    assert!(svg.contains("viewBox=\"120 230 200 60\""));
    assert!(svg.contains("matrix(1 0 0 1 120 230)"));
}

#[test]
fn svg_image_export_rejects_script_urls() {
    let mut e = Editor::default();
    let id = e.add_node(NodeKind::Rectangle, 0.0, 0.0, 100.0, 100.0);
    e.document.nodes.get_mut(&id).unwrap().fills = vec![Fill {
        paint: Paint::Image {
            href: "javascript:alert(1)".into(),
            scale: ImageScale::Fill,
            crop: None,
        },
        opacity: 1.0,
        visible: true,
    }];
    assert!(!e.export_svg().contains("javascript:"));
}

#[test]
fn seed_is_fully_editable_serializable_and_prototyped() {
    let d = seeded_document();
    d.validate().unwrap();
    assert!(d
        .nodes
        .values()
        .any(|n| n.kind == NodeKind::Frame && n.name == "Dashboard · iPhone 16"));
    assert!(d
        .nodes
        .values()
        .any(|n| n.kind == NodeKind::Frame && n.name == "Cards · iPhone 16"));
    assert!(d
        .nodes
        .values()
        .any(|n| n.kind == NodeKind::Component && n.name == "Button / Primary"));
    assert!(d.nodes.values().any(|n| !n.interactions.is_empty()));
    assert!(d.styles.len() >= 9);
    assert!(d.variables.len() >= 5);
    assert_eq!(d.pages.len(), 2);
    assert!(d.pages[0].prototype_start.is_some());
    let json = d.to_json().unwrap();
    let restored = Document::from_json(&json).unwrap();
    assert_eq!(restored, d);
    let svg = d.export_svg(d.pages[0].id, &[]);
    assert!(svg.contains("Your money,"));
    assert!(svg.contains("TOTAL BALANCE"));
    assert!(svg.contains("linearGradient"));
}

#[test]
fn incremental_scene_update_matches_rebuild_when_clipped_subtree_moves_into_view() {
    let mut document = Document::new("Clipped subtree");
    let page = document.pages[0].id;
    let outer = document
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "Outer", 100.0, 100.0, 120.0, 120.0),
        )
        .unwrap();
    document.nodes.get_mut(&outer).unwrap().rotation = 21.0;
    let inner = document
        .insert_node(
            outer,
            Node::new(NodeKind::Frame, "Initially outside", 200.0, 0.0, 70.0, 70.0),
        )
        .unwrap();
    let child = rectangle(&mut document, inner, 10.0, 10.0, 40.0, 40.0);
    let mut editor = Editor::new(document);
    assert!(editor.scene.spatial.bounds(child).is_none());
    editor.begin_transaction("Move clipped frame");
    for x in [90.0, 55.0, 20.0, -90.0, 35.0] {
        editor.document.nodes.get_mut(&inner).unwrap().x = x;
        editor.refresh();
        let mut rebuilt = SceneIndex::default();
        rebuilt.rebuild(&editor.document, page);
        assert_eq!(editor.scene.draw_order, rebuilt.draw_order);
        for id in [outer, inner, child] {
            assert_eq!(
                editor.scene.transforms.get(&id),
                rebuilt.transforms.get(&id)
            );
            assert_eq!(editor.scene.spatial.bounds(id), rebuilt.spatial.bounds(id));
        }
        let point = editor
            .document
            .world_transform(child)
            .unwrap()
            .apply(Point::new(20.0, 20.0));
        assert_eq!(
            editor.hit_test(point, 0.0),
            rebuilt.hit_test(&editor.document, point, 0.0, false)
        );
    }
    editor.commit_transaction();
    assert!(editor.scene.spatial.bounds(child).is_some());
    editor.undo();
    assert!(editor.scene.spatial.bounds(child).is_none());
}

#[test]
fn top_level_reorder_updates_hit_test_and_undo_restores_stacking() {
    let mut editor = Editor::default();
    let back = editor.add_node(NodeKind::Rectangle, 0.0, 0.0, 100.0, 100.0);
    let front = editor.add_node(NodeKind::Rectangle, 0.0, 0.0, 100.0, 100.0);
    let point = Point::new(50.0, 50.0);
    assert_eq!(editor.hit_test(point, 0.0), Some(front));
    editor.selection = vec![back];
    editor.arrange_selection(Arrange::BringToFront);
    assert_eq!(editor.hit_test(point, 0.0), Some(back));
    editor.undo();
    assert_eq!(editor.hit_test(point, 0.0), Some(front));
}

#[test]
fn oversized_corner_radius_uses_the_same_clamp_as_rendered_geometry() {
    let node = Node::new(NodeKind::Rectangle, "Rounded", 0.0, 0.0, 50.0, 50.0).with_radius(200.0);
    assert!(node.hit_test_local(Point::new(25.0, 25.0), 0.0));
    assert!(!node.hit_test_local(Point::new(1.0, 1.0), 0.0));
    assert!(node.hit_test_local(Point::new(25.0, 0.0), 0.0));
}

#[test]
fn comments_stay_on_their_page_and_follow_reparented_layers_through_history() {
    let mut editor = Editor::default();
    let first_page = editor.active_page;
    let position = Point::new(40.0, 60.0);
    let first = editor.add_comment(None, position, "Author", "First page", "2026-10-18");
    assert_eq!(editor.document.comments[0].node, Some(first_page));

    let second_page = editor.add_page("Playground");
    editor.clear_history();
    let second = editor.add_comment(None, position, "Author", "Second page", "2026-10-18");
    let second_comment = editor.document.comments.last().unwrap().clone();
    assert_eq!(second_comment.node, Some(second_page));
    assert_eq!(editor.history_len(), 1);
    assert!(editor.undo());
    assert!(!editor.document.comments.iter().any(|c| c.id == second));
    assert!(editor.redo());
    assert_eq!(editor.document.comments.last(), Some(&second_comment));

    let layer = editor.add_node(NodeKind::Rectangle, 20.0, 30.0, 100.0, 80.0);
    let attached = editor.add_comment(
        Some(layer),
        position,
        "Author",
        "Follows this layer",
        "2026-10-18",
    );
    let pages = |document: &Document| {
        document
            .comments
            .iter()
            .map(|comment| (comment.id, document.comment_page(comment)))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        pages(&editor.document),
        vec![
            (first, Some(first_page)),
            (second, Some(second_page)),
            (attached, Some(second_page)),
        ]
    );

    editor.clear_history();
    let before_move = editor.to_json().unwrap();
    editor
        .transact("Move commented layer", |document| {
            document.reparent(layer, first_page, 0)
        })
        .unwrap();
    let expected_pages = vec![
        (first, Some(first_page)),
        (second, Some(second_page)),
        (attached, Some(first_page)),
    ];
    assert_eq!(pages(&editor.document), expected_pages);
    let after_move = editor.to_json().unwrap();
    assert_eq!(editor.history_len(), 1);
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before_move);
    assert!(editor.redo());
    assert_eq!(editor.to_json().unwrap(), after_move);

    let mut restored = Document::from_json(&after_move).unwrap();
    assert_eq!(restored.comments, editor.document.comments);
    assert_eq!(pages(&restored), expected_pages);
    restored.comments[0].node = None;
    let restored = Document::from_json(&restored.to_json().unwrap()).unwrap();
    assert_eq!(restored.comments[0].node, None);
    assert_eq!(restored.comment_page(&restored.comments[0]), Some(first_page));

    let mut invalid = restored.comments[0].clone();
    invalid.node = Some(u64::MAX);
    assert_eq!(restored.comment_page(&invalid), None);
    invalid.node = Some(layer);
    let mut detached = restored;
    detached.nodes.get_mut(&layer).unwrap().parent = None;
    assert_eq!(detached.comment_page(&invalid), None);
}

#[test]
fn resize_and_position_refresh_once_with_constraints_spatial_bounds_and_history() {
    let mut document = make_document();
    let page = document.pages[0].id;
    let frame = document
        .insert_node(
            page,
            Node::new(NodeKind::Frame, "Frame", 20.0, 30.0, 100.0, 80.0),
        )
        .unwrap();
    let child = rectangle(&mut document, frame, 70.0, 55.0, 20.0, 15.0);
    document.nodes.get_mut(&child).unwrap().constraints = Constraints {
        horizontal: Constraint::End,
        vertical: Constraint::End,
    };
    let mut editor = Editor::new(document);
    editor.selection = vec![frame];
    let before = editor.to_json().unwrap();
    let original_frame_bounds = Rect::new(20.0, 30.0, 100.0, 80.0);
    let original_child_bounds = Rect::new(90.0, 85.0, 20.0, 15.0);
    editor.begin_transaction("Resize gesture");
    let revision = editor.revision;
    editor.resize_node_and_position(frame, 200.0, 120.0, Point::new(200.0, 300.0));
    assert_eq!(editor.revision, revision + 1);
    assert_eq!(editor.history_len(), 0);
    let frame_bounds = Rect::new(200.0, 300.0, 200.0, 120.0);
    let child_bounds = Rect::new(370.0, 395.0, 20.0, 15.0);
    assert_eq!(editor.document.nodes[&frame].bounds(), frame_bounds);
    assert_eq!(
        editor.document.nodes[&child].bounds(),
        Rect::new(170.0, 95.0, 20.0, 15.0)
    );
    assert_eq!(editor.scene.spatial.bounds(frame), Some(frame_bounds));
    assert_eq!(editor.scene.spatial.bounds(child), Some(child_bounds));
    assert_eq!(editor.hit_test(child_bounds.center(), 0.0), Some(child));

    assert!(editor.commit_transaction());
    assert_eq!(editor.history_len(), 1);
    let after = editor.to_json().unwrap();
    assert!(editor.undo());
    assert_eq!(editor.to_json().unwrap(), before);
    assert_eq!(editor.scene.spatial.bounds(frame), Some(original_frame_bounds));
    assert_eq!(editor.scene.spatial.bounds(child), Some(original_child_bounds));
    assert!(editor.redo());
    assert_eq!(editor.to_json().unwrap(), after);
    assert_eq!(editor.scene.spatial.bounds(frame), Some(frame_bounds));
    assert_eq!(editor.scene.spatial.bounds(child), Some(child_bounds));
}
