use super::*;
use ferroui_base::input::{Pointer, PointerType};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::{IPlatformRenderInterface, PixelFormat};
use ferroui_base::{FerroLocator, PixelRect, PixelSize};
use std::collections::BTreeMap;

fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(Rc::new(ferroui_vello::PlatformRenderInterface::default()));
    scope
}

fn fixture() -> (Ref<DesignCanvas>, NodeId) {
    let mut document = core::Document::new("Native gradient transactions");
    let mut node = Node::new(NodeKind::Rectangle, "Gradient", 20.0, 30.0, 200.0, 120.0);
    let value = LinearGradientEdit {
        start: Point::new(0.1, 0.5),
        end: Point::new(0.9, 0.5),
        stops: vec![
            core::GradientStop {
                offset: 0.0,
                color: Color::BLACK,
            },
            core::GradientStop {
                offset: 0.5,
                color: Color::rgb(255, 0, 0),
            },
            core::GradientStop {
                offset: 0.5,
                color: Color::rgb(0, 0, 255),
            },
            core::GradientStop {
                offset: 1.0,
                color: Color::WHITE,
            },
        ],
    };
    node.fills = vec![
        core::Fill::solid(Color::rgb(10, 20, 30)),
        core::Fill {
            paint: value.paint(),
            opacity: 0.75,
            visible: true,
        },
        core::Fill {
            paint: core::Paint::Solid(Color::WHITE),
            opacity: 0.2,
            visible: false,
        },
    ];
    node.fill_style = Some(900);
    document.styles.insert(
        900,
        core::Style {
            id: 900,
            name: "Shared gradient".into(),
            description: String::new(),
            value: core::StyleValue::Fill {
                fills: node.fills.clone(),
            },
        },
    );
    node.corner_radius = 6.0;
    node.variable_bindings.insert("corner_radius".into(), 901);
    document.variables.insert(
        901,
        core::Variable {
            id: 901,
            name: "Radius".into(),
            collection: "Shapes".into(),
            values: BTreeMap::from([("Default".into(), core::VariableValue::Number(6.0))]),
        },
    );
    let id = document.insert_node(1, node).unwrap();
    document.next_id = 902;
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    let canvas = DesignCanvas::new(Rc::new(RefCell::new(editor)));
    canvas.viewport.set(Viewport {
        width: 500.0,
        height: 360.0,
        zoom: 1.0,
        pan: Point::default(),
    });
    assert!(canvas.edit_linear_gradient(1));
    (canvas, id)
}

fn json(canvas: &DesignCanvas) -> String {
    canvas.editor.borrow().document.to_json().unwrap()
}
fn value(canvas: &DesignCanvas, id: NodeId) -> LinearGradientEdit {
    LinearGradientEdit::from_node(&canvas.editor.borrow().document.nodes[&id], 1).unwrap()
}
fn prepared(canvas: &DesignCanvas, id: NodeId) -> Rc<PreparedGradient> {
    canvas
        .prepared_gradient(&canvas.editor.borrow(), id, 1)
        .unwrap()
}
fn marker(canvas: &DesignCanvas, id: NodeId, part: Part) -> Point {
    let prepared = prepared(canvas, id);
    prepared.viewport.screen_to_world(
        prepared
            .handles
            .iter()
            .find(|handle| handle.part == part)
            .unwrap()
            .marker,
    )
}
fn press(canvas: &DesignCanvas, id: NodeId, part: Part) -> Point {
    let point = marker(canvas, id, part);
    assert!(canvas.linear_gradient_move(point));
    assert_eq!(canvas.linear_gradient.borrow().hover, Some(part));
    assert!(canvas.linear_gradient_press(point, 1));
    assert!(canvas.linear_gradient_active());
    assert_eq!(canvas.linear_gradient.borrow().selected, Some(part));
    point
}
fn make_redo(canvas: &DesignCanvas, id: NodeId) {
    canvas
        .editor
        .borrow_mut()
        .update_node(id, "Previous edit", |node| {
            node.name = "Previous edit".into()
        });
    assert!(canvas.editor.borrow_mut().undo());
    canvas.reconcile_linear_gradient();
    assert!(canvas.editor.borrow().can_redo());
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
}

#[test]
fn native_gradient_hover_reuses_bounded_preparation_and_paints_distinct_tied_stop_markers() {
    let scope = backend();
    let (canvas, id) = fixture();
    let before = json(&canvas);
    let token = canvas.editor.borrow().render_change_token();
    assert!(canvas.can_edit_linear_gradient(1));
    assert!(!canvas.can_edit_linear_gradient(0));
    assert!(!canvas.can_edit_linear_gradient(99));
    let first = marker(&canvas, id, Part::Stop(1));
    let second = marker(&canvas, id, Part::Stop(2));
    assert!(first.distance(second) > HIT_RADIUS * 2.0);
    let endpoint = marker(&canvas, id, Part::Start);
    assert!(endpoint.distance(marker(&canvas, id, Part::Stop(0))) > HIT_RADIUS * 2.0);
    for _ in 0..100 {
        assert!(canvas.linear_gradient_move(second));
    }
    assert_eq!(canvas.linear_gradient.borrow().preparations, 1);
    assert_eq!(canvas.linear_gradient.borrow().hover, Some(Part::Stop(2)));
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.editor.borrow().render_change_token(), token);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    assert!(!canvas.editor.borrow().has_active_transaction());
    let bitmap = RenderTargetBitmap::new(PixelSize::new(260, 180));
    {
        let mut context = bitmap.create_drawing_context();
        assert!(canvas.render_linear_gradient(&mut context, &canvas.editor.borrow(), id));
    }
    let mut pixels = vec![0; 260 * 180 * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, 260, 180), &mut pixels, 260 * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }
    for point in [first, second, endpoint] {
        let point = canvas.viewport.get().world_to_screen(point);
        assert!(
            pixels[(point.y.round() as usize * 260 + point.x.round() as usize) * 4 + 3] > 20,
            "Each native marker must paint real DrawingContext pixels"
        );
    }
    assert_eq!(pixels[(10 * 260 + 10) * 4 + 3], 0);
    scope.dispose();
}

#[test]
fn affine_gradient_endpoint_and_axis_drags_keep_other_fills_and_commit_one_scoped_history_step() {
    let scope = backend();
    let (canvas, id) = fixture();
    {
        let mut editor = canvas.editor.borrow_mut();
        editor
            .document
            .nodes
            .get_mut(&id)
            .unwrap()
            .set_local_transform(core::Transform {
                a: -1.1,
                b: 0.2,
                c: 0.4,
                d: 0.9,
                e: 300.0,
                f: 40.0,
            });
        editor.refresh();
    }
    let before = json(&canvas);
    let original_node = canvas.editor.borrow().document.nodes[&id].clone();
    let transform = canvas.editor.borrow().document.world_transform(id).unwrap();
    let start = press(&canvas, id, Part::End);
    assert_eq!(
        json(&canvas),
        before,
        "Marker press is not an authored change"
    );
    for step in 1..=25 {
        canvas.linear_gradient_move(
            start + transform.apply_vector(Point::new(step as f64 * 0.5, -step as f64 * 0.3)),
        );
    }
    let changed = value(&canvas, id);
    near(changed.end.x, 0.9625);
    near(changed.end.y, 0.4375);
    let mut expected_node = original_node.clone();
    expected_node.fills[1].paint = changed.paint();
    expected_node.fill_style = None;
    expected_node.overridden_properties.insert("fills".into());
    assert_eq!(
        serde_json::to_value(&canvas.editor.borrow().document.nodes[&id]).unwrap(),
        serde_json::to_value(&expected_node).unwrap(),
        "Every authored non-paint field is preserved"
    );
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    canvas.finish_linear_gradient_edit();
    let endpoint = json(&canvas);
    assert_eq!(canvas.editor.borrow().history_len(), 1);
    assert!(canvas.editor.borrow_mut().undo());
    canvas.reconcile_linear_gradient();
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.linear_gradient_state()["node"], id);
    assert!(canvas.editor.borrow_mut().redo());
    canvas.reconcile_linear_gradient();
    assert_eq!(json(&canvas), endpoint);
    let original = value(&canvas, id);
    let plan = prepared(&canvas, id);
    let axis_press = plan.axis[0] + (plan.axis[1] - plan.axis[0]) * 0.25;
    assert!(canvas.linear_gradient_press(axis_press, 1));
    assert_eq!(canvas.linear_gradient.borrow().selected, Some(Part::Axis));
    canvas.linear_gradient_move(axis_press + transform.apply_vector(Point::new(-10.0, 15.0)));
    let moved = value(&canvas, id);
    near(moved.start.x, original.start.x - 0.05);
    near(moved.start.y, original.start.y + 0.125);
    near(moved.end.x, original.end.x - 0.05);
    near(moved.end.y, original.end.y + 0.125);
    assert_eq!(moved.stops, original.stops);
    canvas.finish_linear_gradient_edit();
    assert_eq!(canvas.editor.borrow().history_len(), 2);
    assert!(canvas.editor.borrow_mut().undo());
    assert_eq!(json(&canvas), endpoint);
    scope.dispose();
}

#[test]
fn gradient_noop_and_drag_back_restore_style_unresolved_fill_binding_overrides_and_real_redo() {
    let scope = backend();
    let (canvas, id) = fixture();
    // A portable imported snapshot can carry a stale fill binding. Resolving
    // it would replace the whole stack; canvas authoring must detach it only
    // after a real edit, and cancellation must restore it byte for byte.
    {
        let mut editor = canvas.editor.borrow_mut();
        editor.document.variables.insert(
            902,
            core::Variable {
                id: 902,
                name: "Unresolved fill".into(),
                collection: "Paints".into(),
                values: BTreeMap::from([(
                    "Default".into(),
                    core::VariableValue::Color(Color::WHITE),
                )]),
            },
        );
        editor
            .document
            .nodes
            .get_mut(&id)
            .unwrap()
            .variable_bindings
            .insert("fill".into(), 902);
        editor.document.next_id = 903;
        editor.refresh();
    }
    make_redo(&canvas, id);
    let before = json(&canvas);
    press(&canvas, id, Part::Stop(2));
    canvas.finish_linear_gradient_edit();
    assert_eq!(json(&canvas), before);
    assert!(canvas.editor.borrow().can_redo());
    let start = press(&canvas, id, Part::Start);
    canvas.linear_gradient_move(start + Point::new(20.0, 10.0));
    assert!(!canvas.editor.borrow().document.nodes[&id]
        .variable_bindings
        .contains_key("fill"));
    canvas.linear_gradient_move(start);
    assert_eq!(
        json(&canvas),
        before,
        "The original metadata returns while the pointer is still held"
    );
    assert!(canvas.linear_gradient_active());
    assert!(canvas.editor.borrow().has_active_transaction());
    assert!(canvas.editor.borrow().can_redo());
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    canvas.finish_linear_gradient_edit();
    assert_eq!(json(&canvas), before);
    assert!(canvas.editor.borrow().can_redo());
    let start = press(&canvas, id, Part::Start);
    canvas.linear_gradient_move(start + Point::new(20.0, 0.0));
    canvas.linear_gradient_move(start);
    canvas.linear_gradient_move(start + Point::new(5.0, 0.0));
    canvas.finish_linear_gradient_edit();
    assert_eq!(canvas.editor.borrow().history_len(), 1);
    assert!(canvas.editor.borrow_mut().undo());
    assert_eq!(json(&canvas), before);
    scope.dispose();
}

#[test]
fn native_escape_capture_loss_and_undo_cancel_gradient_gestures_without_touching_foreign_transactions(
) {
    let scope = backend();
    let (canvas, id) = fixture();
    make_redo(&canvas, id);
    let before = json(&canvas);
    for cancellation in 0..3 {
        let start = press(&canvas, id, Part::Stop(1));
        canvas.linear_gradient_move(start + Point::new(20.0, 0.0));
        match cancellation {
            0 => {
                let mut event = KeyEventArgs::new();
                event.key = Key::Escape;
                <DesignCanvas as InputElementImpl>::on_key_down(&canvas, &event);
                assert!(event.handled());
            }
            1 => {
                let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true);
                let event = PointerCaptureLostEventArgs::new(canvas.clone(), pointer);
                <DesignCanvas as InputElementImpl>::on_pointer_capture_lost(&canvas, &event);
            }
            _ => assert!(canvas.cancel_active_edit()),
        }
        assert_eq!(json(&canvas), before);
        assert!(!canvas.linear_gradient_active());
        assert!(!canvas.editor.borrow().has_active_transaction());
        assert!(canvas.editor.borrow().can_redo());
        assert_eq!(canvas.linear_gradient_state()["node"], id);
        canvas.linear_gradient_move(start + Point::new(40.0, 0.0));
        assert!(!canvas.finish_linear_gradient_edit());
        assert_eq!(json(&canvas), before);
    }
    let start = press(&canvas, id, Part::End);
    canvas.linear_gradient_move(start + Point::new(5.0, 0.0));
    {
        let mut editor = canvas.editor.borrow_mut();
        editor.cancel_transaction();
        editor.begin_node_transaction(&[id], "Foreign gradient-adjacent draft");
        editor.update_node(id, "Foreign name", |node| {
            node.name = "Preserve foreign state".into()
        });
    }
    let foreign = json(&canvas);
    let token = canvas.editor.borrow().active_transaction_token();
    canvas.cancel_linear_gradient_gesture();
    assert_eq!(json(&canvas), foreign);
    assert_eq!(canvas.editor.borrow().active_transaction_token(), token);
    canvas.editor.borrow_mut().cancel_transaction();
    let start = press(&canvas, id, Part::End);
    canvas.linear_gradient_move(start + Point::new(5.0, 0.0));
    canvas
        .editor
        .borrow_mut()
        .update_node(id, "Joined mutation", |node| {
            node.name = "Preserve joined state".into()
        });
    let foreign = json(&canvas);
    let token = canvas.editor.borrow().active_transaction_token();
    canvas.finish_linear_gradient_edit();
    assert_eq!(json(&canvas), foreign);
    assert_eq!(canvas.editor.borrow().active_transaction_token(), token);
    canvas.editor.borrow_mut().cancel_transaction();
    scope.dispose();
}

#[test]
fn gradient_stop_pointer_insertion_keyboard_order_and_delete_preserve_authored_ties_and_minimum() {
    let scope = backend();
    let (canvas, id) = fixture();
    let before = json(&canvas);
    let original = value(&canvas, id);
    let start = press(&canvas, id, Part::Stop(2));
    canvas.linear_gradient_move(start + Point::new(20.0, 0.0));
    let moved = value(&canvas, id);
    near(moved.stops[2].offset, 0.625);
    for index in [0, 1, 3] {
        assert_eq!(moved.stops[index], original.stops[index]);
    }
    assert_eq!(moved.stops[2].color, original.stops[2].color);
    canvas.finish_linear_gradient_edit();
    assert!(canvas.editor.borrow_mut().undo());
    assert_eq!(json(&canvas), before);
    let plan = prepared(&canvas, id);
    let insertion = plan.axis[0] + (plan.axis[1] - plan.axis[0]) * 0.25;
    assert!(canvas.linear_gradient_press(insertion, 2));
    let added = value(&canvas, id);
    assert_eq!(added.stops.len(), 5);
    assert_eq!(&added.stops[..4], &original.stops);
    near(added.stops[4].offset, 0.25);
    assert_eq!(added.stops[4].color, Color::rgb(128, 0, 0));
    assert_eq!(
        canvas.linear_gradient.borrow().selected,
        Some(Part::Stop(4))
    );
    canvas.finish_linear_gradient_edit();
    assert_eq!(canvas.editor.borrow().history_len(), 1);
    assert!(canvas.linear_gradient_key(Key::Tab, KeyModifiers::empty()));
    assert_eq!(canvas.linear_gradient.borrow().selected, Some(Part::Start));
    assert!(canvas.linear_gradient_key(Key::Tab, KeyModifiers::SHIFT));
    assert_eq!(
        canvas.linear_gradient.borrow().selected,
        Some(Part::Stop(4))
    );
    canvas.linear_gradient_key(Key::Delete, KeyModifiers::empty());
    assert_eq!(canvas.linear_gradient.borrow().selected, None);
    assert_eq!(value(&canvas, id), original);
    assert_eq!(canvas.editor.borrow().history_len(), 2);
    assert!(canvas.editor.borrow_mut().undo());
    assert_eq!(value(&canvas, id), added);
    assert!(canvas.editor.borrow_mut().undo());
    assert_eq!(json(&canvas), before);
    {
        let mut two = value(&canvas, id);
        two.stops = vec![two.stops[0].clone(), two.stops[3].clone()];
        let mut editor = canvas.editor.borrow_mut();
        // Direct fixture replacement is deliberately outside an input gesture.
        editor.document.nodes.get_mut(&id).unwrap().fills[1].paint = two.paint();
        editor.refresh();
    }
    let notices = Rc::new(RefCell::new(Vec::new()));
    let output = Rc::clone(&notices);
    canvas.on_change(move |event| {
        if let CanvasEvent::Notice(message) = event {
            output.borrow_mut().push(message);
        }
    });
    let minimum = json(&canvas);
    let depth = canvas.editor.borrow().history_len();
    press(&canvas, id, Part::Stop(0));
    canvas.finish_linear_gradient_edit();
    canvas.linear_gradient_key(Key::Delete, KeyModifiers::empty());
    assert_eq!(json(&canvas), minimum);
    assert_eq!(canvas.editor.borrow().history_len(), depth);
    assert_eq!(
        canvas.linear_gradient.borrow().selected,
        Some(Part::Stop(0))
    );
    assert_eq!(&*notices.borrow(), &["Keep at least two gradient stops."]);
    scope.dispose();
}

#[test]
fn degenerate_gradient_endpoints_remain_separate_nojump_targets_and_recover_direction() {
    let scope = backend();
    let (canvas, id) = fixture();
    {
        let mut editor = canvas.editor.borrow_mut();
        let node = editor.document.nodes.get_mut(&id).unwrap();
        if let core::Paint::LinearGradient { start, end, .. } = &mut node.fills[1].paint {
            *start = Point::new(0.5, 0.5);
            *end = *start;
        }
        editor.refresh();
    }
    let before = json(&canvas);
    let plan = prepared(&canvas, id);
    assert_eq!(plan.axis[0], plan.axis[1]);
    assert!(
        marker(&canvas, id, Part::Start).distance(marker(&canvas, id, Part::End))
            >= MIN_ENDPOINT_SEPARATION
    );
    let start = press(&canvas, id, Part::Stop(1));
    canvas.linear_gradient_move(start + Point::new(20.0, 0.0));
    canvas.finish_linear_gradient_edit();
    assert_eq!(json(&canvas), before);
    let start = press(&canvas, id, Part::End);
    canvas.linear_gradient_move(start);
    assert_eq!(json(&canvas), before);
    canvas.linear_gradient_move(start + Point::new(20.0, 12.0));
    let changed = value(&canvas, id);
    assert_eq!(changed.start, Point::new(0.5, 0.5));
    near(changed.end.x, 0.6);
    near(changed.end.y, 0.6);
    canvas.finish_linear_gradient_edit();
    assert_eq!(canvas.editor.borrow().history_len(), 1);
    assert!(canvas.editor.borrow_mut().undo());
    assert_eq!(json(&canvas), before);
    scope.dispose();
}

#[test]
fn gradient_lifecycle_finish_is_once_and_stale_selection_viewport_or_editor_cannot_commit() {
    let scope = backend();
    let (canvas, id) = fixture();
    let before = json(&canvas);
    let start = press(&canvas, id, Part::End);
    canvas.linear_gradient_move(start + Point::new(10.0, 0.0));
    assert!(canvas.finish_linear_gradient_edit());
    let saved = json(&canvas);
    assert!(!canvas.finish_linear_gradient_edit());
    assert_eq!(canvas.editor.borrow().history_len(), 1);
    canvas.linear_gradient_move(start + Point::new(15.0, 0.0));
    assert_eq!(json(&canvas), saved);
    assert!(canvas.editor.borrow_mut().undo());
    assert_eq!(json(&canvas), before);
    let start = press(&canvas, id, Part::End);
    canvas.linear_gradient_move(start + Point::new(10.0, 0.0));
    canvas.viewport.set(Viewport {
        zoom: 2.0,
        ..canvas.viewport.get()
    });
    canvas.reconcile_linear_gradient();
    assert!(!canvas.linear_gradient_active());
    assert_eq!(json(&canvas), before);
    canvas.viewport.set(Viewport {
        zoom: 1.0,
        ..canvas.viewport.get()
    });
    let start = press(&canvas, id, Part::End);
    canvas.linear_gradient_move(start + Point::new(10.0, 0.0));
    canvas.editor.borrow_mut().selection.clear();
    canvas.reconcile_linear_gradient();
    assert_eq!(json(&canvas), before);
    assert!(canvas.editor.borrow().selection.is_empty());
    assert!(canvas.linear_gradient_state()["node"].is_null());
    canvas.editor.borrow_mut().selection = vec![id];
    assert!(canvas.edit_linear_gradient(1));
    let start = press(&canvas, id, Part::End);
    canvas.linear_gradient_move(start + Point::new(10.0, 0.0));
    let replacement = core::Document::new("Replacement editor");
    *canvas.editor.borrow_mut() = Editor::new(replacement);
    let replacement = json(&canvas);
    canvas.reconcile_linear_gradient();
    assert_eq!(json(&canvas), replacement);
    assert!(!canvas.editor.borrow().has_active_transaction());
    assert!(canvas.linear_gradient_state()["node"].is_null());
    for unsupported in 0..7 {
        let (canvas, id) = fixture();
        canvas.leave_linear_gradient_edit();
        {
            let mut editor = canvas.editor.borrow_mut();
            let node = editor.document.nodes.get_mut(&id).unwrap();
            match unsupported {
                0 => node.locked = true,
                1 => node.visible = false,
                2 => node.set_local_transform(core::Transform {
                    a: 0.0,
                    b: 0.0,
                    c: 0.0,
                    d: 1.0,
                    e: 20.0,
                    f: 30.0,
                }),
                3 => node.kind = NodeKind::Star,
                4 => {
                    if let core::Paint::LinearGradient { stops, .. } = &mut node.fills[1].paint {
                        stops.resize(core::MAX_CANVAS_GRADIENT_STOPS + 1, stops[0].clone());
                    }
                }
                5 => {}
                _ => {
                    // The authored transform is finite and its determinant is
                    // one, but inverse translation overflows. Both the button
                    // predicate and entry must reject it before preparation.
                    node.x = f64::MAX;
                    node.y = -f64::MAX;
                    node.affine_transform = Some(core::Transform {
                        a: 1.0,
                        b: 1.0,
                        c: 0.0,
                        d: 1.0,
                        e: 0.0,
                        f: 0.0,
                    });
                }
            }
            if unsupported == 5 {
                editor.active_page = editor.document.add_page("Another active page");
            }
            editor.refresh();
        }
        let before = json(&canvas);
        assert!(!canvas.can_edit_linear_gradient(1));
        assert!(!canvas.edit_linear_gradient(1));
        assert_eq!(json(&canvas), before);
        assert_eq!(canvas.editor.borrow().history_len(), 0);
        assert!(!canvas.editor.borrow().has_active_transaction());
    }
    scope.dispose();
}
