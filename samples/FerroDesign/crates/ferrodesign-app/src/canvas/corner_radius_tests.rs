use super::*;
use ferroui_base::input::{Pointer, PointerType};
use ferroui_base::media::imaging::RenderTargetBitmap;
use ferroui_base::platform::{IPlatformRenderInterface, PixelFormat};
use ferroui_base::{FerroLocator, PixelRect, PixelSize};

fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(Rc::new(ferroui_vello::PlatformRenderInterface::default()));
    scope
}

fn fixture(kind: NodeKind, radius: f64, bound: bool) -> (Ref<DesignCanvas>, NodeId) {
    let mut document = core::Document::new("Corner pointer transactions");
    let mut node = Node::new(kind, "Editable corner", 20.0, 30.0, 200.0, 160.0);
    node.corner_radius = radius;
    node.polygon_sides = 5;
    node.star_inner_radius = 0.45;
    if bound {
        document.variables.insert(
            900,
            core::Variable {
                id: 900,
                name: "Radius".into(),
                collection: "Shapes".into(),
                values: std::collections::BTreeMap::from([(
                    "Default".into(),
                    core::VariableValue::Number(radius),
                )]),
            },
        );
        node.variable_bindings.insert("corner_radius".into(), 900);
    }
    let id = document.insert_node(1, node).unwrap();
    let mut editor = Editor::new(document);
    editor.selection = vec![id];
    let canvas = DesignCanvas::new(Rc::new(RefCell::new(editor)));
    canvas.viewport.set(Viewport {
        width: 500.0,
        height: 360.0,
        zoom: 1.0,
        pan: Point::default(),
    });
    (canvas, id)
}

fn handles(canvas: &DesignCanvas) -> Rc<Vec<Handle>> {
    let editor = canvas.editor.borrow();
    let (id, transform) = canvas.radius_target(&editor).unwrap();
    canvas.prepared_radius_handles(&editor, id, transform)
}
fn handle(canvas: &DesignCanvas, corner: usize) -> Handle {
    *handles(canvas)
        .iter()
        .find(|handle| handle.source.corner == corner)
        .unwrap()
}
fn json(canvas: &DesignCanvas) -> String {
    canvas.editor.borrow().document.to_json().unwrap()
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn press(canvas: &DesignCanvas, corner: usize, individual: bool) -> Handle {
    let handle = handle(canvas, corner);
    let modifiers = if individual {
        KeyModifiers::ALT
    } else {
        KeyModifiers::empty()
    };
    assert!(canvas.corner_radius_hover(handle.world, modifiers));
    assert!(canvas.corner_radius_press(handle.world, modifiers));
    assert!(canvas.corner_radius_active());
    assert_eq!(canvas.corner_radius_state()["corner"], corner);
    let expected_individual = individual
        && canvas.editor.borrow().document.nodes[&canvas.editor.borrow().selection[0]].kind
            == NodeKind::Rectangle;
    assert_eq!(
        canvas.corner_radius_state()["individual"],
        expected_individual
    );
    handle
}
fn move_by(canvas: &DesignCanvas, handle: Handle, delta: f64) {
    assert!(canvas.corner_radius_move(handle.world + handle.world_per_radius * delta));
}

#[test]
fn radius_hover_retains_the_affordance_and_paints_public_native_pixels_without_document_edits() {
    let scope = backend();
    let (canvas, id) = fixture(NodeKind::Rectangle, 8.0, true);
    let before = json(&canvas);
    let token = canvas.editor.borrow().render_change_token();
    let handle = handle(&canvas, 0);
    near(handle.screen.distance(handle.vertex_screen), MIN_INSET);
    for index in 0..100 {
        assert!(canvas.corner_radius_hover(
            handle.world + Point::new(index as f64 * 0.01, 0.0),
            KeyModifiers::empty()
        ));
    }
    assert_eq!(canvas.corner_radius.borrow().preparations, 1);
    assert_eq!(canvas.corner_radius.borrow().spacing_preparations, 1);
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.editor.borrow().render_change_token(), token);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    assert!(!canvas.editor.borrow().has_active_transaction());
    assert_eq!(canvas.corner_radius_state()["hover"], 0);
    let bitmap = RenderTargetBitmap::new(PixelSize::new(260, 220));
    {
        let mut context = bitmap.create_drawing_context();
        canvas.render_corner_radius(&mut context, &canvas.editor.borrow(), id);
    }
    let mut pixels = vec![0; 260 * 220 * 4];
    bitmap.copy_pixels(PixelRect::new(0, 0, 260, 220), &mut pixels, 260 * 4);
    if bitmap.format() == Some(PixelFormat::BGRA8888) {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
    }
    let alpha = |x: usize, y: usize| pixels[(y * 260 + x) * 4 + 3];
    assert!(
        alpha(
            handle.screen.x.round() as usize,
            handle.screen.y.round() as usize
        ) > 20,
        "The native circle must paint actual DrawingContext pixels"
    );
    assert_eq!(alpha(150, 100), 0);
    canvas.clear_corner_radius_hover();
    assert!(canvas.corner_radius_state()["hover"].is_null());
    scope.dispose();
}

#[test]
fn captured_uniform_radius_has_no_press_jump_and_many_previews_commit_one_exact_history_step() {
    let scope = backend();
    let (canvas, id) = fixture(NodeKind::Rectangle, 8.0, true);
    let before = json(&canvas);
    let handle = press(&canvas, 0, false);
    assert_eq!(
        json(&canvas),
        before,
        "A minimum-inset press must not detach a binding"
    );
    assert!(canvas.editor.borrow().has_active_transaction());
    for step in 1..=21 {
        move_by(&canvas, handle, step as f64 * 0.5);
    }
    let node = canvas.editor.borrow().document.nodes[&id].clone();
    near(node.corner_radius, 18.5);
    assert_eq!(node.corner_radii, None);
    assert!(!node.variable_bindings.contains_key("corner_radius"));
    assert!(node.overridden_properties.contains("corner_radius"));
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    assert_eq!(
        canvas.corner_radius.borrow().spacing_preparations,
        1,
        "Changing radius reuses the nearest-vertex spacing plan"
    );
    assert!(canvas.finish_corner_radius_edit());
    let after = json(&canvas);
    assert_eq!(canvas.editor.borrow().history_len(), 1);
    assert!(canvas.editor.borrow_mut().undo());
    assert_eq!(json(&canvas), before);
    assert!(canvas.editor.borrow_mut().redo());
    assert_eq!(json(&canvas), after);
    scope.dispose();
}

#[test]
fn maximum_uniform_and_independent_rectangles_keep_every_alt_corner_separately_accessible() {
    let scope = backend();
    for (radius, independent) in [
        (100.0, None),
        (1e6, None),
        (8.0, Some([200.0, 0.0, 200.0, 0.0])),
    ] {
        let (canvas, id) = fixture(NodeKind::Rectangle, radius, false);
        {
            let mut editor = canvas.editor.borrow_mut();
            let node = editor.document.nodes.get_mut(&id).unwrap();
            node.height = 200.0;
            node.corner_radii = independent;
            editor.refresh();
        }
        let plan = handles(&canvas);
        assert_eq!(plan.len(), 4);
        for (index, handle) in plan.iter().enumerate() {
            for other in plan.iter().skip(index + 1) {
                assert!(handle.screen.distance(other.screen) > 2.0 * HIT_RADIUS);
            }
        }
        for corner in 0..4 {
            let before = json(&canvas);
            let handle = press(&canvas, corner, true);
            move_by(&canvas, handle, 2.75);
            let radii = canvas.editor.borrow().document.nodes[&id]
                .corner_radii
                .unwrap();
            let mut expected = independent.unwrap_or([radius; 4]);
            expected[corner] += 2.75;
            for index in 0..4 {
                near(radii[index], expected[index]);
            }
            assert_eq!(
                canvas.editor.borrow().document.nodes[&id].corner_radius,
                radius
            );
            canvas.finish_corner_radius_edit();
            assert!(canvas.editor.borrow_mut().undo());
            assert_eq!(json(&canvas), before);
        }
    }
    let (canvas, _) = fixture(NodeKind::Polygon, 1e6, false);
    let plan = handles(&canvas);
    assert_eq!(plan.len(), 5);
    for (index, handle) in plan.iter().enumerate() {
        assert!(plan
            .iter()
            .skip(index + 1)
            .all(|other| handle.screen.distance(other.screen) > 2.0 * HIT_RADIUS));
    }
    scope.dispose();
}

#[test]
fn returning_to_the_press_restores_bindings_independent_arrays_overrides_and_redo_before_release() {
    let scope = backend();
    for individual in [false, true] {
        let (canvas, id) = fixture(NodeKind::Rectangle, 8.0, true);
        {
            let mut editor = canvas.editor.borrow_mut();
            editor.document.nodes.get_mut(&id).unwrap().corner_radii = Some([3.0, 5.0, 7.0, 9.0]);
            editor.refresh();
            editor.set_corner_radius(&[id], 20.0).unwrap();
            editor.undo();
        }
        let before = json(&canvas);
        press(&canvas, 2, individual);
        assert_eq!(json(&canvas), before);
        canvas.finish_corner_radius_edit();
        assert_eq!(json(&canvas), before);
        assert!(canvas.editor.borrow().can_redo());
        let handle = press(&canvas, 2, individual);
        move_by(&canvas, handle, 12.5);
        assert_ne!(json(&canvas), before);
        move_by(&canvas, handle, 0.0);
        assert_eq!(
            json(&canvas),
            before,
            "Drag-back restores metadata while capture is still owned"
        );
        assert!(canvas.corner_radius_active());
        assert!(canvas.editor.borrow().has_active_transaction());
        assert!(canvas.editor.borrow().can_redo());
        assert_eq!(canvas.editor.borrow().history_len(), 0);
        canvas.finish_corner_radius_edit();
        assert_eq!(json(&canvas), before);
        assert_eq!(canvas.editor.borrow().history_len(), 0);
        assert!(canvas.editor.borrow().can_redo());
        // A later move after restoring the snapshot gets its own new token but
        // remains one pointer transaction against the same original document.
        let handle = press(&canvas, 2, individual);
        move_by(&canvas, handle, 10.0);
        move_by(&canvas, handle, 0.0);
        move_by(&canvas, handle, 2.5);
        canvas.finish_corner_radius_edit();
        assert_eq!(canvas.editor.borrow().history_len(), 1);
        canvas.editor.borrow_mut().undo();
        assert_eq!(json(&canvas), before);
    }
    scope.dispose();
}

#[test]
fn escape_native_capture_loss_and_undo_cancel_only_the_owned_radius_transaction() {
    let scope = backend();
    let (canvas, id) = fixture(NodeKind::Rectangle, 8.0, true);
    canvas
        .editor
        .borrow_mut()
        .set_corner_radius(&[id], 20.0)
        .unwrap();
    canvas.editor.borrow_mut().undo();
    let before = json(&canvas);
    for cancel in 0..3 {
        let handle = press(&canvas, 0, false);
        move_by(&canvas, handle, 12.5);
        match cancel {
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
            _ => {
                assert!(canvas.cancel_active_edit());
            }
        }
        assert_eq!(json(&canvas), before);
        assert_eq!(canvas.editor.borrow().selection, [id]);
        assert!(!canvas.corner_radius_active());
        assert!(!canvas.editor.borrow().has_active_transaction());
        assert!(canvas.editor.borrow().can_redo());
        assert!(!canvas.corner_radius_move(handle.world + handle.world_per_radius * 20.0));
        assert!(!canvas.finish_corner_radius_edit());
        assert_eq!(json(&canvas), before);
    }
    let handle = press(&canvas, 0, false);
    move_by(&canvas, handle, 3.0);
    {
        let mut editor = canvas.editor.borrow_mut();
        editor.cancel_transaction();
        editor.begin_node_transaction(&[id], "Foreign draft");
        editor.update_node(id, "Foreign draft", |node| {
            node.name = "Retain external edit".into()
        });
    }
    let external = json(&canvas);
    let token = canvas.editor.borrow().active_transaction_token();
    assert!(canvas.cancel_corner_radius_gesture());
    assert_eq!(json(&canvas), external);
    assert_eq!(canvas.editor.borrow().active_transaction_token(), token);
    canvas.editor.borrow_mut().cancel_transaction();
    // A mutation that joined the existing transaction still invalidates the
    // expected render token. The stale pointer cannot undo that other work.
    let handle = press(&canvas, 0, false);
    move_by(&canvas, handle, 1.0);
    canvas
        .editor
        .borrow_mut()
        .update_node(id, "Joined external edit", |node| {
            node.name = "Retain same-transaction external edit".into();
        });
    let external = json(&canvas);
    let token = canvas.editor.borrow().active_transaction_token();
    assert!(canvas.finish_corner_radius_edit());
    assert_eq!(json(&canvas), external);
    assert_eq!(canvas.editor.borrow().active_transaction_token(), token);
    canvas.editor.borrow_mut().cancel_transaction();
    scope.dispose();
}

#[test]
fn reflected_sheared_polygon_star_and_frame_drags_use_local_radius_units_and_finite_clamps() {
    let scope = backend();
    for kind in [NodeKind::Polygon, NodeKind::Star, NodeKind::Frame] {
        let (canvas, id) = fixture(kind, 4.0, false);
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
                    f: 30.0,
                });
            editor.refresh();
        }
        let before = json(&canvas);
        let transform = canvas.editor.borrow().document.world_transform(id).unwrap();
        let handle = press(&canvas, 0, true);
        assert!(
            !canvas.corner_radius_state()["individual"]
                .as_bool()
                .unwrap(),
            "Alt edits one corner only on Rectangle"
        );
        move_by(&canvas, handle, 8.75);
        near(
            canvas.editor.borrow().document.nodes[&id].corner_radius,
            12.75,
        );
        assert_eq!(
            canvas.editor.borrow().document.world_transform(id),
            Some(transform)
        );
        canvas.finish_corner_radius_edit();
        assert_eq!(canvas.editor.borrow().history_len(), 1);
        canvas.editor.borrow_mut().undo();
        assert_eq!(json(&canvas), before);
        let handle = press(&canvas, 0, false);
        move_by(&canvas, handle, -10.0);
        near(
            canvas.editor.borrow().document.nodes[&id].corner_radius,
            0.0,
        );
        canvas.corner_radius_move(Point::new(f64::INFINITY, f64::NAN));
        near(
            canvas.editor.borrow().document.nodes[&id].corner_radius,
            0.0,
        );
        canvas.editor.borrow().document.validate().unwrap();
        canvas.cancel_corner_radius_gesture();
        assert_eq!(json(&canvas), before);
    }
    scope.dispose();
}

#[test]
fn lifecycle_finish_commits_once_and_late_capture_or_motion_cannot_reopen_the_draft() {
    let scope = backend();
    let (canvas, id) = fixture(NodeKind::Rectangle, 8.0, false);
    let documents = Rc::new(Cell::new(0));
    let captured = documents.clone();
    canvas.on_change(move |event| {
        if matches!(event, CanvasEvent::Document) {
            captured.set(captured.get() + 1);
        }
    });
    let handle = press(&canvas, 1, false);
    move_by(&canvas, handle, 6.25);
    assert_eq!(
        documents.get(),
        0,
        "Held previews do not trigger autosave/inspector document refreshes"
    );
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    assert!(canvas.finish_corner_radius_edit());
    let after = json(&canvas);
    assert_eq!(documents.get(), 1);
    assert!(!canvas.finish_corner_radius_edit());
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let event = PointerCaptureLostEventArgs::new(canvas.clone(), pointer);
    <DesignCanvas as InputElementImpl>::on_pointer_capture_lost(&canvas, &event);
    assert!(!canvas.corner_radius_move(handle.world + handle.world_per_radius * 30.0));
    assert_eq!(json(&canvas), after);
    assert_eq!(canvas.editor.borrow().history_len(), 1);
    assert_eq!(documents.get(), 1);
    assert!(!canvas.editor.borrow().has_active_transaction());
    near(
        canvas.editor.borrow().document.nodes[&id].corner_radius,
        14.25,
    );
    scope.dispose();
}

#[test]
fn alt_radius_markers_inside_a_rounded_clip_cutout_remain_distinct_from_layer_picking() {
    let scope = backend();
    let mut document = core::Document::new("Affine rounded clip and radius controls");
    let back = document
        .insert_node(
            1,
            Node::new(NodeKind::Rectangle, "Backdrop", 40.0, 40.0, 300.0, 220.0)
                .with_fill(Color::WHITE),
        )
        .unwrap();
    let mut frame = Node::new(NodeKind::Frame, "Rounded clip", 60.0, 60.0, 160.0, 140.0);
    frame.corner_radius = 40.0;
    frame.clip_content = true;
    frame.fills.clear();
    frame.affine_transform = Some(core::Transform {
        a: 1.0,
        b: 0.2,
        c: 0.3,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    });
    let parent = document.insert_node(1, frame).unwrap();
    let child = document
        .insert_node(
            parent,
            Node::new(NodeKind::Rectangle, "Clipped child", 0.0, 0.0, 160.0, 140.0)
                .with_fill(Color::WHITE),
        )
        .unwrap();
    let mut editor = Editor::new(document);
    editor.update_node(child, "Seed redo", |node| node.name = "Redo child".into());
    assert!(editor.undo());
    editor.selection = vec![child];
    let canvas = DesignCanvas::new(Rc::new(RefCell::new(editor)));
    canvas.viewport.set(Viewport {
        width: 1016.0,
        height: 956.0,
        zoom: 2.953333333333333,
        pan: Point::new(-53.133333333333326, 35.0),
    });
    let before = json(&canvas);
    let world = |x, y| Point::new(60.0 + x + 0.3 * y, 60.0 + 0.2 * x + y);
    assert_eq!(canvas.pick(world(50.0, 50.0)), Some(child));
    let control_point = world(3.0, 3.0);
    assert_eq!(canvas.pick(control_point), Some(back));
    assert!(
        handle(&canvas, 0)
            .screen
            .distance(canvas.viewport.get().world_to_screen(control_point))
            < HIT_RADIUS,
        "The historical clipped-picking probe actually hit a radius control"
    );
    assert!(canvas.corner_radius_press(control_point, KeyModifiers::ALT));
    assert!(canvas.corner_radius_state()["individual"]
        .as_bool()
        .unwrap());
    assert_eq!(canvas.editor.borrow().selection, [child]);
    assert!(canvas.finish_corner_radius_edit());
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    assert!(canvas.editor.borrow().can_redo());

    let local = Point::new(3.0, 20.0);
    assert!(local.distance(Point::new(40.0, 40.0)) > 40.0);
    let empty = world(local.x, local.y);
    assert!(handles(&canvas).iter().all(|handle| handle
        .screen
        .distance(canvas.viewport.get().world_to_screen(empty))
        > 2.0 * HIT_RADIUS));
    assert_eq!(canvas.handle_at(empty), None);
    assert!(!canvas.corner_radius_press(empty, KeyModifiers::ALT));
    assert_eq!(canvas.pick(empty), Some(back));
    canvas.cycle_selection_at(empty, core::SelectionCycle::Behind);
    assert_eq!(canvas.editor.borrow().selection, [back]);
    assert_eq!(json(&canvas), before);
    assert_eq!(canvas.editor.borrow().history_len(), 0);
    assert!(canvas.editor.borrow().can_redo());
    assert!(!canvas.editor.borrow().has_active_transaction());
    scope.dispose();
}

#[test]
fn tiny_hidden_locked_singular_and_exclusive_modes_preserve_resize_and_deep_selection_precedence() {
    let scope = backend();
    let (canvas, id) = fixture(NodeKind::Rectangle, 0.0, false);
    let handle = handle(&canvas, 0);
    let before = json(&canvas);
    assert!(!canvas.corner_radius_press(handle.vertex_world, KeyModifiers::empty()));
    assert!(canvas.handle_at(handle.vertex_world).is_some());
    assert!(!canvas.corner_radius_press(handle.world, KeyModifiers::CONTROL));
    assert!(!canvas.corner_radius_press(handle.world, KeyModifiers::META));
    assert_eq!(json(&canvas), before);
    assert!(!canvas.editor.borrow().has_active_transaction());
    for mode in 0..3 {
        {
            let mut editor = canvas.editor.borrow_mut();
            let node = editor.document.nodes.get_mut(&id).unwrap();
            node.locked = mode == 0;
            node.visible = mode != 1;
            node.affine_transform = (mode == 2).then_some(core::Transform::scale(0.0, 1.0));
            editor.refresh();
        }
        assert!(canvas.corner_radius_state()["node"].is_null());
        assert!(!canvas.corner_radius_press(handle.world, KeyModifiers::empty()));
    }
    {
        let mut editor = canvas.editor.borrow_mut();
        let node = editor.document.nodes.get_mut(&id).unwrap();
        node.locked = false;
        node.visible = true;
        node.affine_transform = None;
        node.width = 12.0;
        node.height = 12.0;
        editor.refresh();
    }
    assert!(
        handles(&canvas).is_empty(),
        "Unusable tiny affordances do not steal resize hits"
    );
    let mut viewport = canvas.viewport.get();
    viewport.zoom = 8.0;
    canvas.viewport.set(viewport);
    assert_eq!(
        handles(&canvas).len(),
        4,
        "Zoom makes the same authored corners accessible"
    );
    {
        let mut editor = canvas.editor.borrow_mut();
        editor.document.nodes.get_mut(&id).unwrap().strokes =
            vec![core::Stroke::new(Color::BLACK, 4.0)];
        editor.refresh();
    }
    assert!(canvas.edit_stroke_width());
    assert!(canvas.corner_radius_state()["node"].is_null());
    canvas.leave_stroke_width_edit();
    let handle = press(&canvas, 0, false);
    move_by(&canvas, handle, 2.0);
    canvas.editor.borrow_mut().selection.clear();
    canvas.reconcile_corner_radius();
    assert!(!canvas.corner_radius_active());
    assert!(canvas.editor.borrow().selection.is_empty());
    assert!(!canvas.editor.borrow().has_active_transaction());
    scope.dispose();
}
