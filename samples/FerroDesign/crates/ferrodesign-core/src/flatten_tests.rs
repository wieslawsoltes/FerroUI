use super::*;

fn fixture() -> (Editor, NodeId, NodeId) {
    let mut document = Document::new("Flatten regression");
    let mut parent = Node::new(NodeKind::Frame, "Affine parent", 30.0, 20.0, 200.0, 150.0);
    parent.fills.clear();
    parent.clip_content = false;
    parent.affine_transform = Some(Transform {
        a: 1.1,
        b: 0.2,
        c: 0.35,
        d: 0.9,
        e: 0.0,
        f: 0.0,
    });
    let parent = document.insert_node(1, parent).unwrap();
    let mut shape = Node::new(
        NodeKind::Rectangle,
        "Preserved name",
        12.0,
        17.0,
        60.0,
        40.0,
    )
    .with_radius(8.0);
    shape.rotation = 23.0;
    shape.flip_x = true;
    shape.fills = vec![Fill::solid(Color::rgb(30, 140, 230))];
    shape.strokes = vec![Stroke::new(Color::rgb(220, 30, 90), 3.0)];
    shape.effects = vec![Effect::for_kind(EffectKind::LayerBlur)];
    shape.effects[0].radius = 1.5;
    let shape = document.insert_node(parent, shape).unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![shape];
    (editor, parent, shape)
}

fn unchanged_on_error(
    editor: &mut Editor,
    operation: impl FnOnce(&mut Editor) -> Result<NodeId, DocumentError>,
) {
    let before = editor.document.to_json().unwrap();
    let selection = editor.selection.clone();
    let revision = editor.document.revision;
    let history = editor.history_len();
    let redo = editor.can_redo();
    assert!(operation(editor).is_err());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.selection, selection);
    assert_eq!(editor.document.revision, revision);
    assert_eq!(editor.history_len(), history);
    assert_eq!(editor.can_redo(), redo);
}

#[test]
fn flatten_shape_retains_identity_hierarchy_affine_paints_strokes_effects_and_history() {
    let (mut editor, parent, id) = fixture();
    let before = editor.document.to_json().unwrap();
    let original = editor.document.nodes[&id].clone();
    let world = editor.document.world_transform(id).unwrap();
    let history = editor.history_len();
    let token = editor.render_change_token();
    assert_eq!(editor.flatten_selection(), Some(id));
    assert!(
        matches!(editor.render_changes_since(&token), EditorRenderChanges::Nodes { nodes, .. } if nodes == vec![id])
    );
    let result = &editor.document.nodes[&id];
    assert_eq!(result.kind, NodeKind::Vector);
    assert_eq!(result.name, original.name);
    assert_eq!(result.parent, Some(parent));
    assert_eq!(editor.document.children(parent), Some([id].as_slice()));
    assert_eq!(editor.document.world_transform(id), Some(world));
    assert_eq!(result.fills, original.fills);
    assert_eq!(result.strokes, original.strokes);
    assert_eq!(result.effects, original.effects);
    assert_eq!(result.path.fill_rule, FillRule::NonZero);
    assert!(!result.path.segments.is_empty());
    assert_eq!(editor.history_len(), history + 1);
    let after = editor.document.to_json().unwrap();
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert!(editor.redo());
    assert_eq!(editor.document.to_json().unwrap(), after);
}

#[test]
fn flatten_kind_changes_keep_structural_container_mask_and_component_invalidation() {
    for kind in [
        NodeKind::Frame,
        NodeKind::Boolean,
        NodeKind::Component,
        NodeKind::Instance,
        NodeKind::Rectangle,
    ] {
        let mut document = Document::new("Conservative kind invalidation");
        let mut node = Node::new(kind, "Structural role", 0.0, 0.0, 20.0, 20.0);
        node.clip_content = false;
        node.is_mask = kind == NodeKind::Rectangle;
        let id = document.insert_node(1, node).unwrap();
        let mut editor = Editor::new(document);
        let token = editor.render_change_token();
        editor.update_node(id, "Change kind", |node| node.kind = NodeKind::Vector);
        assert_eq!(
            editor.render_changes_since(&token),
            EditorRenderChanges::Full,
            "{kind:?}"
        );
    }
    for owned_by_component in [false, true] {
        let mut document = Document::new("Owned leaf invalidation");
        let component = document
            .insert_node(
                1,
                Node::new(NodeKind::Component, "Master", 0.0, 0.0, 100.0, 100.0),
            )
            .unwrap();
        let mut node = Node::new(NodeKind::Rectangle, "Owned leaf", 0.0, 0.0, 20.0, 20.0);
        if owned_by_component {
            node.component_source = Some(component);
        } else {
            node.layout = Some(AutoLayout::default());
        }
        let id = document.insert_node(1, node).unwrap();
        let mut editor = Editor::new(document);
        let token = editor.render_change_token();
        editor.update_node(id, "Change kind", |node| node.kind = NodeKind::Vector);
        assert_eq!(
            editor.render_changes_since(&token),
            EditorRenderChanges::Full
        );
    }
}

#[test]
fn flatten_existing_vector_is_noop_and_preserves_evenodd_and_redo() {
    let (mut editor, _, id) = fixture();
    editor.flatten_selection().unwrap();
    editor.document.nodes.get_mut(&id).unwrap().path.fill_rule = FillRule::EvenOdd;
    let other = editor.add_node(NodeKind::Ellipse, 0.0, 0.0, 10.0, 10.0);
    assert!(editor.undo());
    assert!(!editor.document.nodes.contains_key(&other));
    editor.selection = vec![id];
    let before = editor.document.to_json().unwrap();
    let history = editor.history_len();
    assert_eq!(editor.flatten_selection(), Some(id));
    assert_eq!(editor.document.to_json().unwrap(), before);
    assert_eq!(editor.history_len(), history);
    assert!(editor.can_redo());
}

#[test]
fn flatten_region_paints_rejects_groups_atomically_and_keeps_single_vectors_unchanged() {
    for fills in [
        vec![
            Fill {
                paint: Paint::LinearGradient {
                    start: Point::new(0.0, 0.0),
                    end: Point::new(1.0, 1.0),
                    stops: vec![
                        GradientStop { offset: 0.0, color: Color::WHITE },
                        GradientStop { offset: 1.0, color: Color::BLACK },
                    ],
                },
                opacity: 0.4,
                visible: true,
            },
            Fill { visible: false, ..Fill::solid(Color::rgb(40, 150, 220)) },
        ],
        Vec::new(), // Explicit transparency also replaces the inherited fill.
    ] {
        let mut document = Document::new("Flatten region paints");
        let group = document.insert_node(
            1, Node::new(NodeKind::Group, "Group", 0.0, 0.0, 80.0, 40.0),
        ).unwrap();
        let path = VectorPath::polygon(&[
            Point::new(0.0, 0.0), Point::new(40.0, 0.0),
            Point::new(40.0, 40.0), Point::new(0.0, 40.0),
        ], true);
        let mut network = VectorNetwork::from_path(&path).unwrap();
        let region = *network.regions.keys().next().unwrap();
        network.set_region_fills(region, Some(fills.clone())).unwrap();
        let mut vector = Node::new(NodeKind::Vector, "Painted face", 0.0, 0.0, 40.0, 40.0);
        vector.path = path;
        vector.vector_network = Some(network.clone());
        let vector = document.insert_node(group, vector).unwrap();
        document.insert_node(group,
            Node::new(NodeKind::Rectangle, "Sibling", 40.0, 0.0, 40.0, 40.0),
        ).unwrap();
        let mut editor = Editor::new(document);
        let redo_node = editor.add_node(NodeKind::Ellipse, 100.0, 0.0, 10.0, 10.0);
        assert!(editor.undo());
        assert!(editor.can_redo());

        editor.selection = vec![group];
        let allocator = editor.document.next_id;
        let token = editor.render_change_token();
        unchanged_on_error(&mut editor, |editor| {
            let result = editor.flatten_selection_with_geometry(|_, _| {
                panic!("region paint loss must be rejected before geometry extraction")
            });
            assert!(matches!(&result, Err(DocumentError::Invalid(message))
                if message.contains("independent region paints")));
            result
        });
        assert_eq!(editor.document.next_id, allocator);
        assert_eq!(editor.render_changes_since(&token), EditorRenderChanges::Nodes {
            nodes: Vec::new(), ancestors: Vec::new(),
        });

        editor.selection = vec![vector];
        let before = editor.document.to_json().unwrap();
        let revision = editor.document.revision;
        let history = editor.history_len();
        assert_eq!(editor.flatten_selection_with_geometry(|_, _| {
            panic!("an editable vector does not need new geometry")
        }).unwrap(), vector);
        assert_eq!(editor.flatten_selection(), Some(vector));
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert_eq!(editor.document.revision, revision);
        assert_eq!(editor.document.next_id, allocator);
        assert_eq!(editor.selection, vec![vector]);
        assert_eq!(editor.history_len(), history);
        assert_eq!(editor.document.nodes[&vector].vector_network.as_ref(), Some(&network));
        assert_eq!(editor.render_changes_since(&token), EditorRenderChanges::Nodes {
            nodes: Vec::new(), ancestors: Vec::new(),
        });
        assert!(editor.can_redo());
        assert!(editor.redo());
        assert!(editor.document.nodes.contains_key(&redo_node));
    }
}

#[test]
fn flatten_rejects_locked_selection_and_unavailable_outline_atomically() {
    let (mut editor, parent, id) = fixture();
    editor.document.nodes.get_mut(&parent).unwrap().locked = true;
    unchanged_on_error(&mut editor, |editor| {
        editor
            .flatten_selection_with_geometry(|_, _| panic!("locked geometry must not be requested"))
    });
    editor.document.nodes.get_mut(&parent).unwrap().locked = false;
    let other = editor.add_node(NodeKind::Ellipse, 0.0, 0.0, 10.0, 10.0);
    assert!(editor.undo());
    editor.selection = vec![id];
    unchanged_on_error(&mut editor, |editor| {
        editor.flatten_selection_with_geometry(|_, _| Err(invalid("unsupported renderer")))
    });
    assert!(editor.redo());
    assert!(editor.document.nodes.contains_key(&other));
}

#[test]
fn flatten_rejects_translucent_text_paints_but_retains_whole_layer_opacity() {
    let (mut editor, _, id) = fixture();
    let redo = editor.add_node(NodeKind::Ellipse, 0.0, 0.0, 10.0, 10.0);
    assert!(editor.undo());
    editor.selection = vec![id];
    for color_alpha in [false, true] {
        let node = editor.document.nodes.get_mut(&id).unwrap();
        node.kind = NodeKind::Text;
        node.text = "HH".into();
        node.letter_spacing = -16.0;
        node.text_decoration = TextDecoration::Strikethrough;
        node.fills[0].opacity = if color_alpha { 1.0 } else { 0.5 };
        let mut color = Color::BLACK;
        color.a = if color_alpha { 128 } else { 255 };
        node.fills[0].paint = Paint::Solid(color);
        unchanged_on_error(&mut editor, |editor| {
            editor.flatten_selection_with_geometry(|_, _| {
                panic!("translucent text must fail before geometry extraction")
            })
        });
        assert!(editor.can_redo());
    }
    let node = editor.document.nodes.get_mut(&id).unwrap();
    node.fills[0] = Fill::solid(Color::BLACK);
    node.opacity = 0.5;
    assert_eq!(
        editor
            .flatten_selection_with_geometry(|_, _| Ok(VectorPath::polygon(
                &[
                    Point::new(0.0, 0.0),
                    Point::new(50.0, 0.0),
                    Point::new(50.0, 30.0),
                    Point::new(0.0, 30.0),
                ],
                true
            )))
            .unwrap(),
        id
    );
    assert_eq!(editor.document.nodes[&id].opacity, 0.5);
    assert!(!editor.document.nodes.contains_key(&redo));
}

#[test]
fn flatten_box_drop_shadow_requires_an_opaque_covering_fill_atomically() {
    let mut translucent = Color::BLACK;
    translucent.a = 128;
    let gradient = |alpha| Fill {
        paint: Paint::LinearGradient {
            start: Point::new(0.0, 0.0),
            end: Point::new(1.0, 1.0),
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: Color::WHITE,
                },
                GradientStop {
                    offset: 1.0,
                    color: Color {
                        a: alpha,
                        ..Color::BLACK
                    },
                },
            ],
        },
        opacity: 1.0,
        visible: true,
    };
    let unsupported = [
        Vec::new(),
        vec![Fill::solid(translucent)],
        vec![Fill {
            opacity: 0.5,
            ..Fill::solid(Color::BLACK)
        }],
        vec![Fill {
            visible: false,
            ..Fill::solid(Color::BLACK)
        }],
        vec![gradient(128)],
        vec![Fill {
            paint: Paint::Image {
                href: "data:image/png;base64,unknown-alpha".into(),
                scale: ImageScale::Fill,
                crop: None,
            },
            opacity: 1.0,
            visible: true,
        }],
    ];
    for kind in [NodeKind::Rectangle, NodeKind::Frame, NodeKind::Section] {
        let (mut editor, _, id) = fixture();
        editor.add_node(NodeKind::Ellipse, 0.0, 0.0, 10.0, 10.0);
        assert!(editor.undo());
        editor.selection = vec![id];
        for fills in &unsupported {
            let node = editor.document.nodes.get_mut(&id).unwrap();
            node.kind = kind;
            node.effects = vec![Effect::shadow(Color::BLACK, Point::new(4.0, 0.0), 2.0)];
            node.fills = fills.clone();
            unchanged_on_error(&mut editor, |editor| {
                editor.flatten_selection_with_geometry(|_, _| {
                    panic!("a box-shadow paint mismatch must fail before extraction")
                })
            });
            assert!(editor.can_redo());
        }
        // An opaque gradient hides the shadow even when other fills and the
        // whole isolated layer are translucent. There is no per-fill blend
        // operator in this model; each fill uses ordinary source-over.
        let node = editor.document.nodes.get_mut(&id).unwrap();
        node.fills = vec![gradient(255), Fill::solid(translucent)];
        node.opacity = 0.5;
        assert_eq!(editor.flatten_selection(), Some(id));
        assert_eq!(editor.document.nodes[&id].opacity, 0.5);
    }
}

#[test]
fn flatten_empty_container_rejects_inner_shadow_stroke_order_change_atomically() {
    for kind in [NodeKind::Frame, NodeKind::Section] {
        let (mut editor, _, id) = fixture();
        editor.add_node(NodeKind::Ellipse, 0.0, 0.0, 10.0, 10.0);
        assert!(editor.undo());
        editor.selection = vec![id];
        let node = editor.document.nodes.get_mut(&id).unwrap();
        node.kind = kind;
        node.effects = vec![Effect::for_kind(EffectKind::InnerShadow)];
        assert!(node.children.is_empty());
        unchanged_on_error(&mut editor, |editor| {
            editor.flatten_selection_with_geometry(|_, _| {
                panic!("an inner-shadow/stroke order mismatch must fail before extraction")
            })
        });
        assert!(editor.can_redo());
        // With no painted stroke the container/leaf ordering distinction has
        // no second paint to affect, so an otherwise ordinary frame can pass.
        editor.document.nodes.get_mut(&id).unwrap().strokes[0].visible = false;
        assert_eq!(editor.flatten_selection(), Some(id));
    }
}

#[test]
fn flatten_box_shadow_outward_spread_rejects_square_corners_atomically() {
    for kind in [NodeKind::Rectangle, NodeKind::Frame, NodeKind::Section] {
        for (effect_kind, spread) in [
            (EffectKind::DropShadow, 12.0),
            (EffectKind::InnerShadow, -12.0),
        ] {
            let (mut editor, _, id) = fixture();
            editor.add_node(NodeKind::Ellipse, 0.0, 0.0, 10.0, 10.0);
            assert!(editor.undo());
            editor.selection = vec![id];
            let node = editor.document.nodes.get_mut(&id).unwrap();
            node.kind = kind;
            node.strokes.clear();
            node.effects = vec![Effect::for_kind(effect_kind)];
            node.effects[0].spread = spread;
            node.corner_radii = Some([8.0, 0.0, 8.0, 8.0]);
            unchanged_on_error(&mut editor, |editor| {
                editor.flatten_selection_with_geometry(|_, _| {
                    panic!("square shadow corners must fail before extraction")
                })
            });
            assert!(editor.can_redo());
            // A single zero corner is enough to differ; all round corners
            // keep their radii through the outward spread and remain eligible.
            editor.document.nodes.get_mut(&id).unwrap().corner_radii = Some([8.0; 4]);
            assert_eq!(editor.flatten_selection(), Some(id));
        }
    }
}

#[test]
fn flatten_preserves_exact_host_fill_rule_and_curves() {
    let (mut editor, _, id) = fixture();
    let path = VectorPath {
        fill_rule: FillRule::EvenOdd,
        segments: vec![
            PathSegment::MoveTo {
                point: Point::new(0.0, 0.0),
            },
            PathSegment::CubicTo {
                control1: Point::new(0.0, 30.0),
                control2: Point::new(60.0, 30.0),
                point: Point::new(60.0, 0.0),
            },
            PathSegment::Close,
        ],
        ..VectorPath::default()
    };
    let expected = path.clone();
    editor
        .flatten_selection_with_geometry(|_, request| {
            assert_eq!(request.roots, vec![id]);
            Ok(path.clone())
        })
        .unwrap();
    assert_eq!(editor.document.nodes[&id].path, expected);
}

#[test]
fn flatten_multiple_layers_rejects_different_parents_colors_and_interleaved_order() {
    let mut document = Document::new("Multiple");
    let a = document
        .insert_node(1, Node::new(NodeKind::Rectangle, "A", 0.0, 0.0, 20.0, 20.0))
        .unwrap();
    let gap = document
        .insert_node(
            1,
            Node::new(NodeKind::Rectangle, "Gap", 20.0, 0.0, 20.0, 20.0),
        )
        .unwrap();
    let b = document
        .insert_node(
            1,
            Node::new(NodeKind::Rectangle, "B", 40.0, 0.0, 20.0, 20.0),
        )
        .unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![b, a];
    unchanged_on_error(&mut editor, |editor| {
        editor.flatten_selection_with_geometry(|_, _| panic!("interleaved layers cannot combine"))
    });
    editor.selection = vec![a, gap];
    editor.document.nodes.get_mut(&gap).unwrap().fills = vec![Fill::solid(Color::BLACK)];
    unchanged_on_error(&mut editor, |editor| {
        editor.flatten_selection_with_geometry(|_, _| panic!("different paints cannot combine"))
    });
}

#[test]
fn flatten_adjacent_uniform_shapes_retains_parent_stack_and_one_step_history() {
    let mut document = Document::new("Multiple");
    let parent = document
        .insert_node(
            1,
            Node::new(NodeKind::Frame, "Parent", 10.0, 20.0, 200.0, 100.0),
        )
        .unwrap();
    let a = document
        .insert_node(
            parent,
            Node::new(NodeKind::Rectangle, "A", 0.0, 0.0, 20.0, 20.0),
        )
        .unwrap();
    let b = document
        .insert_node(
            parent,
            Node::new(NodeKind::Rectangle, "B", 20.0, 0.0, 20.0, 20.0),
        )
        .unwrap();
    let c = document
        .insert_node(
            parent,
            Node::new(NodeKind::Rectangle, "C", 40.0, 0.0, 20.0, 20.0),
        )
        .unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![b, a];
    let before = editor.document.to_json().unwrap();
    editor
        .flatten_selection_with_geometry(|document, request| {
            assert_eq!(request.roots, vec![a, b]);
            assert_eq!(
                request.destination_world,
                document.world_transform(a).unwrap()
            );
            Ok(outline_path(&Node::new(
                NodeKind::Rectangle,
                "Union",
                0.0,
                0.0,
                40.0,
                20.0,
            )))
        })
        .unwrap();
    assert_eq!(editor.document.children(parent), Some([a, c].as_slice()));
    assert_eq!(editor.document.nodes[&a].width, 40.0);
    assert_eq!(editor.selection, vec![a]);
    assert!(!editor.document.nodes.contains_key(&b));
    assert_eq!(editor.history_len(), 1);
    assert!(editor.undo());
    assert_eq!(editor.document.to_json().unwrap(), before);
}

#[test]
fn flatten_group_checks_locked_descendants_and_retains_references() {
    let mut document = Document::new("Group");
    let group = document
        .insert_node(1, Node::new(NodeKind::Group, "Group", 0.0, 0.0, 50.0, 50.0))
        .unwrap();
    let child = document
        .insert_node(
            group,
            Node::new(NodeKind::Rectangle, "Child", 0.0, 0.0, 20.0, 20.0),
        )
        .unwrap();
    document.nodes.get_mut(&child).unwrap().locked = true;
    let mut editor = Editor::new(document);
    editor.selection = vec![group];
    unchanged_on_error(&mut editor, |editor| {
        editor
            .flatten_selection_with_geometry(|_, _| panic!("locked descendants cannot be removed"))
    });
    editor.document.nodes.get_mut(&child).unwrap().locked = false;
    editor.document.comments.push(CommentThread {
        id: 100,
        node: Some(child),
        position: Point::default(),
        resolved: false,
        messages: vec![],
    });
    unchanged_on_error(&mut editor, |editor| {
        editor.flatten_selection_with_geometry(|_, _| panic!("referenced child cannot be removed"))
    });
}
